use crate::games::just_one::GameData; // TODO: dynamic game
use crate::games::GameType;
use crate::models::lobby::{InMsg, LobbyInMsg, LobbyOutMsg};
use futures::future::join_all;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{mpsc, Mutex};

type UserMap = Arc<Mutex<HashMap<String, Sender<LobbyOutMsg>>>>;

#[derive(Clone)]
pub struct Lobby {
    pub id: String,
    pub in_msg: Sender<InMsg>,
    users: UserMap,
    game: GameType,
}

impl Lobby {
    pub fn new(id: String) -> Lobby {
        let (tx, rx) = mpsc::channel(100);
        let users = Arc::new(Mutex::new(HashMap::new()));
        let mut lobby = Lobby {
            id: id.clone(),
            in_msg: tx,
            users,
            game: GameType::JustOne,
        };
        let ret = lobby.clone();

        tokio::spawn(async move {
            lobby.lobby_loop(rx).await;
        });
        ret
    }

    async fn lobby_loop(&mut self, mut rx: Receiver<InMsg>) {
        while let Some(msg) = rx.recv().await {
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
                    self.game_loop(&mut rx, GameData::new(&users).await).await;
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

    async fn game_loop(&mut self, rx: &mut Receiver<InMsg>, mut game: GameData<'_>) {
        self.broadcast_state(&game).await;
        while let Some(msg) = rx.recv().await {
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

        let sends = users.iter().map(|(u_id, tx)| async {
            tx.send(f(u_id))
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

        let tx = um.get(&user).unwrap();

        if let Err(e) = tx.send(msg).await {
            println!("Unable to send {}", e);
        }
    }

    pub async fn remove_user(&mut self, user_id: String) {
        self.users.lock().await.remove(&user_id);
    }

    pub async fn add_user(&mut self, user_id: String, tx: Sender<LobbyOutMsg>) {
        self.users.lock().await.insert(user_id, tx);
    }
}

//TODO: could probably use Rc<String> to reduce copiess
pub struct LobbyManager {
    lobbies: HashMap<String, Lobby>,
}

impl LobbyManager {
    pub fn new() -> LobbyManager {
        LobbyManager {
            lobbies: HashMap::new(),
        }
    }

    pub fn get(&mut self, id: String) -> Lobby {
        match self.lobbies.get(&id) {
            Some(l) => l.clone(),
            None => {
                let l = Lobby::new(id);
                self.lobbies.insert(l.id.clone(), l.clone());
                l
            }
        }
    }
}
