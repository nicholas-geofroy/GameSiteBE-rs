use std::sync::Arc;

use crate::lobby_manager::LobbyManager;
use crate::models::lobby::InMsg;
use crate::models::lobby::{LobbyInMsg, LobbyOutMsg};
use axum::extract::ws::{Message, WebSocket};
use eyre::{eyre, WrapErr};
use serde_json;
use tokio::select;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;

struct UserManager {
    user_id: String,
    lobby_id: String,
    lm: Arc<Mutex<LobbyManager>>,

    socket: WebSocket,
    c_out: Sender<InMsg>,
    c_in: Receiver<LobbyOutMsg>,
}

async fn handle_join(
    mut socket: WebSocket,
    lm_mutex: Arc<Mutex<LobbyManager>>,
    lobby_id: String,
) -> Result<UserManager, String> {
    let Some(res) = socket.recv().await else {
        return Err("Socked closed before join message".to_owned());
    };
    let Ok(ws_msg) = res else {
        return Err("Socked closed before join message".to_owned());
    };
    let Message::Text(txt) = ws_msg else {
        let _ = socket.send(Message::Text("Error: Expected join message with user id".to_string())).await;
        return Err(format!("Initial message from socket was not a text message. Msg: {:?}", &ws_msg));
    };
    let Ok(LobbyInMsg::Join { user_id }) = serde_json::from_str(&txt) else {
        let _ = socket.send(Message::Text("Error: Expected join message with user id".to_string())).await;
        return Err(format!("Initial message from socket was not join message. Msg {}", txt));
    };

    let mut lm = lm_mutex.lock().await;

    lm.create_lobby(lobby_id.clone());

    let (lobby_in, lobby_out) = match lm.add_user(&lobby_id, &user_id).await {
        Ok(c) => c,
        Err(e) => return Err(format!("Error joining lobby {}: {:?}", lobby_id, e)),
    };

    let res = lobby_in
        .send(InMsg {
            uid: user_id.clone(),
            cmd: LobbyInMsg::Join {
                user_id: user_id.clone(),
            },
        })
        .await;

    if res.is_err() {
        println!(
            "Could not join lobby {}, error: {}",
            &lobby_id,
            res.unwrap_err()
        );
    }

    Ok(UserManager {
        user_id,
        lobby_id,
        lm: lm_mutex.clone(),
        socket,
        c_out: lobby_in,
        c_in: lobby_out,
    })
}

async fn communicate(mut um: UserManager) {
    loop {
        select! {
            res = um.socket.recv() => {
                match res
                    .map(|r| match r {
                        Ok(Message::Close(_)) => Err(eyre!("Socket closed by client {}", &um.user_id)),
                        x => x.wrap_err(format!("Client {} Socket Error", &um.user_id))
                    })
                    .unwrap_or_else(|| Err(eyre!("Socket Closed"))) {
                    Ok(msg)=> {
                        match msg {
                            Message::Text(t) => {
                                if !t.eq("ping") {
                                    let msg: Result<LobbyInMsg, _> = serde_json::from_str(&t);
                                    match msg {
                                        Ok(msg) => {
                                            um.c_out.send(InMsg { uid: um.user_id.clone(), cmd: msg }).await.unwrap_or_else(|e| println!("{}", e));
                                        },
                                        Err(e) => {
                                            let msg = serde_json::to_string(
                                                &LobbyOutMsg::Error { msg: format!("{}", e)}
                                            ).unwrap_or_else(|_| "Internal Server Error...".to_owned());

                                            um.socket.send(Message::Text(msg)).await.unwrap_or_else(|err| {
                                                println!("Error message failed to send {}", err);
                                            });
                                        }
                                    }
                                }
                            }
                            Message::Binary(_) => {
                                println!("client sent binary data... ignoring");
                            }
                            Message::Ping(_) => {
                                println!("socket ping");
                            }
                            Message::Pong(_) => {
                                println!("socket pong");
                            }
                            Message::Close(_) => {
                                panic!("Close should have been mapped to err");
                            }
                        }

                    },
                    Err(e) => {
                        println!("{:?}", e);

                        let mut lm = um.lm.lock().await;
                        lm.disconnect_user(&um.lobby_id, &um.user_id).await;

                        um.c_out.send(InMsg {uid: um.user_id.clone(), cmd: LobbyInMsg::Leave }).await;
                        return
                    }
                }

            },
            lobby_res = um.c_in.recv() => {
                match lobby_res {
                    Some(msg) => {
                        let res = match serde_json::to_string(&msg) {
                            Ok(txt) => um.socket.send(Message::Text(txt)).await,
                            Err(_) => {
                                println!("Unable to encode {:?}", msg);
                                Ok(())
                            }
                        };
                        if let Err(e) = res {
                            println!("Error Sending msg {:?} {}", msg, e)
                        }
                    },
                    None => {
                        um.socket.send(Message::Text("Error: Lobby Closed".to_string())).await.unwrap_or_else(|err| {
                            println!("Lobby Closed message failed to send {}", err);
                        });
                    }
                }
            }
        }
    }
}

pub async fn handle_socket(socket: WebSocket, lm: Arc<Mutex<LobbyManager>>, lobby_id: String) {
    let um = match handle_join(socket, lm, lobby_id).await {
        Ok(c) => c,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };

    tokio::spawn(async move {
        communicate(um).await;
    });
}
