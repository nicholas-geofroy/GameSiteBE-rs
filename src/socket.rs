use crate::lobby::Lobby;
use crate::models::lobby::InMsg;
use crate::models::lobby::{LobbyInMsg, LobbyOutMsg};
use axum::extract::ws::{Message, WebSocket};
use eyre::{eyre, WrapErr};
use serde_json;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::{join, select, sync::mpsc};

struct UserManager {
    user_id: String,
    lobby: Lobby,

    socket: WebSocket,
    c_out: Sender<InMsg>,
    c_in: Receiver<LobbyOutMsg>,
}

async fn handle_join(mut socket: WebSocket, mut lobby: Lobby) -> Result<UserManager, String> {
    let lobby_chan: mpsc::Sender<InMsg> = lobby.in_msg.clone();
    let (tx, rx) = mpsc::channel(100);

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

    lobby.add_user(user_id.clone(), tx.clone()).await;
    let res = lobby_chan
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
            &lobby.id,
            res.unwrap_err()
        );
    }

    Ok(UserManager {
        user_id,
        lobby,
        socket,
        c_out: lobby_chan,
        c_in: rx,
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
                                    println!("client sent str: {:?}", t);
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

                        let (_, _) = join!(
                            um.c_out.send(InMsg {uid: um.user_id.clone(), cmd: LobbyInMsg::Leave }),
                            um.lobby.remove_user(um.user_id)
                        );
                        return
                    }
                }

            },
            lobby_res = um.c_in.recv() => {
                match lobby_res {
                    Some(msg) => {
                        dbg!("Send message", &msg);
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

pub async fn handle_socket(socket: WebSocket, lobby: Lobby) {
    let um = match handle_join(socket, lobby).await {
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
