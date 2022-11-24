use futures::future::join_all;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc::Receiver, Mutex};

use crate::{
    games::{just_one::GameData, GameType},
    lobby_manager::User,
    models::lobby::{InMsg, LobbyInMsg, LobbyOutMsg},
};

pub struct Lobby {
    id: String,
    users: Arc<Mutex<HashMap<String, User>>>,
    rx: Receiver<InMsg>,
    game: GameType,
}

impl Lobby {
    pub fn new(id: String, users: Arc<Mutex<HashMap<String, User>>>, rx: Receiver<InMsg>) -> Lobby {
        return Lobby {
            id,
            users,
            rx,
            game: GameType::JustOne,
        };
    }
    pub async fn lobby_loop(&mut self) {
        while let Some(msg) = self.rx.recv().await {
            use LobbyInMsg::*;
            use LobbyOutMsg::*;
            let req_uid = msg.uid;

            match msg.cmd {
                Join { user_id } => {
                    println!("User {} joined lobby {}", &user_id, &self.id);
                    let members = self.get_members().await;

                    self.broadcast(|_| Members(members.clone())).await;
                    self.send(user_id, SelectedGame(self.game)).await;
                }
                Leave => println!("User {} left lobby {}", &req_uid, &self.id),
                Start => {
                    println!("Start Game");
                    let users: Vec<String> = self.get_members().await;
                    self.game_loop(GameData::new(&users).await).await;
                }
                GetUsers => {
                    println!("Get Users {}", req_uid);
                    self.send(req_uid, Members(self.get_members().await)).await;
                }
                GetGameData => {
                    println!("Get Game Data {}", req_uid);
                    self.send(req_uid, SelectedGame(self.game)).await;
                }
                GameMove(_) => {
                    self.send(
                        req_uid,
                        Error {
                            msg: "Invalid Msg. Cannot make move during the lobby".to_string(),
                        },
                    )
                    .await
                }
            }
        }
    }

    async fn game_loop(&mut self, mut game: GameData<'_>) {
        self.broadcast_state(&game).await;
        while let Some(msg) = self.rx.recv().await {
            use LobbyInMsg::*;
            use LobbyOutMsg::*;

            let req_uid = msg.uid;
            match msg.cmd {
                Join { user_id } => {
                    println!("User {} joined lobby {}", &user_id, &self.id);
                    let members = self.get_members().await;
                    self.broadcast(|_| Members(members.clone())).await;
                    self.send(user_id, SelectedGame(self.game)).await;
                }
                Leave => println!("User {} left lobby {}", &req_uid, &self.id),
                Start => {
                    self.send(
                        req_uid,
                        Error {
                            msg: "Invalid Msg. Cannot start a game during an existing game"
                                .to_string(),
                        },
                    )
                    .await
                }
                GetUsers => {
                    println!("Get Users {}", req_uid);
                    self.send(req_uid, Members(self.get_members().await)).await;
                }
                GetGameData => {
                    println!("Get Game Data {}", req_uid);
                    self.send(req_uid, SelectedGame(self.game)).await;
                }
                GameMove(action) => match game.make_move(&req_uid, action) {
                    Ok(state) => {
                        self.broadcast_state(&state).await;
                    }
                    Err(e) => {
                        self.send(
                            req_uid,
                            Error {
                                msg: format!("Invalid Move: {:?}", e),
                            },
                        )
                        .await
                    }
                },
            }
        }
    }

    async fn broadcast_state(&self, state: &GameData<'_>) {
        self.broadcast(|u| match serde_json::to_value(&state.filter(u)) {
            Ok(s) => {
                println!("Sending State {}", s);
                LobbyOutMsg::GameState(s)
            }
            Err(e) => LobbyOutMsg::Error { msg: e.to_string() },
        })
        .await;
    }

    async fn get_members(&self) -> Vec<String> {
        self.users
            .lock()
            .await
            .iter()
            .map(|(id, _)| id.clone())
            .collect()
    }

    async fn broadcast(&self, f: impl Fn(&str) -> LobbyOutMsg) {
        let users = self.users.lock().await;

        let sends = users.iter().map(|(u_id, u)| async {
            u.out
                .send(f(u_id))
                .await
                .map_err(|e| format!("Unable to send {}", e))
        });
        let errors: Vec<String> = join_all(sends)
            .await
            .into_iter()
            .filter(|r| r.is_err())
            .map(|r| r.expect_err("Expected list to only contain errors"))
            .collect();
        if errors.len() > 0 {
            println!("Error in broadcast: {}", errors.concat());
        }
    }

    async fn send(&self, user: String, msg: LobbyOutMsg) {
        let um = self.users.lock().await;

        if !um.contains_key(&user) {
            println!("Tried to send message to {} who was not found", &user);
        }

        let u = um.get(&user).unwrap();

        if let Err(e) = u.out.send(msg).await {
            println!("Unable to send {}", e);
        }
    }
}
