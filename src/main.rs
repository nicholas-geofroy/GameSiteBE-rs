mod user_manager; mod models;
mod lobby;
mod games;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::{sync::{Mutex, mpsc}, select, join};
use axum::extract::Extension;
use lobby::{Lobby, LobbyManager};
use eyre::{eyre, WrapErr};
use serde_json;
use crate::models::lobby::{LobbyInMsg, LobbyOutMsg};

use axum::{
    routing::get,
    Router,
    response::{Html, IntoResponse}, extract::{WebSocketUpgrade, ws::{Message, WebSocket}, Path}, TypedHeader,
};

type LMRef = Arc<Mutex<LobbyManager>>;

#[tokio::main]
async fn main() {

    let lm = Arc::new(Mutex::new(LobbyManager::new()));
    let app = Router::new()
        .route("/", get(handler))
        .route("/lobby/:id/ws", get(ws_handler))
        .layer(Extension(lm));

    let addr = SocketAddr::from(([127,0,0,1], 9000));
    println!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}

#[axum::debug_handler]
async fn ws_handler(Path(lobby_id): Path<String>, Extension(lm): Extension<LMRef>, ws: WebSocketUpgrade, user_agent: Option<TypedHeader<headers::UserAgent>>) -> impl IntoResponse {
    println!("Try connecting to lobby {}", lobby_id);

    let mut lm = lm.lock().await;

    let lobby = lm.get(lobby_id);

    if let Some(TypedHeader(user_agent)) = user_agent {
        println!("`{}` connected to lobby {}", user_agent.as_str(), lobby.id);
    }

    ws.on_upgrade(move |socket| {
        handle_socket(socket, lobby)
    })

} 

async fn handle_socket(mut socket: WebSocket, mut lobby: Lobby) {
    let lobby_chan = lobby.in_msg.clone();
    let (tx, mut rx) = mpsc::channel(100);

    let _user_thread = tokio::spawn(async move {
        let Some(res) = socket.recv().await else {
            println!("Socked closed before join message");
            return
        };
        let Ok(ws_msg) = res else {
            print!("Socked closed before join message");
            return
        };
        let Message::Text(txt) = ws_msg else {
            print!("Initial message from socket was not a text message. Msg: {:?}", &ws_msg);
            let _ = socket.send(Message::Text("Error: Expected join message with user id".to_string())).await;
            return
        };
        let Ok(LobbyInMsg::Join { user_id }) = serde_json::from_str(&txt) else {
            println!("Initial message from socket was not join message. Msg {}", txt);
            let _ = socket.send(Message::Text("Error: Expected join message with user id".to_string())).await;
            return
        };

        lobby.add_user(user_id.clone(), tx).await;
        let res = lobby_chan.send(models::lobby::LobbyInMsg::Join { user_id: user_id.clone() }).await;
    
        if res.is_err() {
            println!("Could not join lobby {}, error: {}", &lobby.id, res.unwrap_err());
        }

        loop {
            select! {
                res = socket.recv() => {
                    match res
                        .map(|r| match r {
                            Ok(Message::Close(_)) => Err(eyre!("Socket closed by client {}", &user_id)),
                            x => x.wrap_err(format!("Client {} Socket Error", &user_id))
                        })
                        .unwrap_or_else(|| Err(eyre!("Socket Closed"))) {
                        Ok(msg)=> {
                            match msg {
                                Message::Text(t) => {
                                    println!("client sent str: {:?}", t);
                                    if !t.eq("ping") {
                                        let msg: Result<LobbyInMsg, _> = serde_json::from_str(&t);
                                        match msg {
                                            Ok(msg) => {
                                                lobby_chan.send(msg).await.unwrap_or_else(|e| println!("{}", e));
                                            },
                                            Err(e) => {
                                                let msg = serde_json::to_string(
                                                    &LobbyOutMsg::Error { msg: format!("{}", e)}
                                                ).unwrap_or_else(|_| "Internal Server Error...".to_owned());
    
                                                socket.send(Message::Text(msg)).await.unwrap_or_else(|err| {
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
                                lobby_chan.send(LobbyInMsg::Leave{user_id: user_id.clone()}),
                                lobby.remove_user(user_id)
                            );
                            return
                        }
                    }

                },
                lobby_res = rx.recv() => {
                    match lobby_res {
                        Some(msg) => {
                            dbg!("Send message", &msg);
                            let res = match serde_json::to_string(&msg) {
                                Ok(txt) => socket.send(Message::Text(txt)).await,
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
                            socket.send(Message::Text("Error: Lobby Closed".to_string())).await.unwrap_or_else(|err| {
                                println!("Lobby Closed message failed to send {}", err);
                            });
                        }
                    }
                }
            }
        }
    });
    
}

