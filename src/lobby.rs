use crate::models::lobby::{LobbyInMsg, LobbyOutMsg};
use futures::future::join_all;
use tokio::sync::mpsc::Sender;
use tokio::sync::{mpsc, Mutex};
use std::collections::HashMap;
use std::sync::Arc;
use eyre::Result;

type UserMap = Arc<Mutex<HashMap<String, Sender<LobbyOutMsg>>>>;

#[derive(Clone)]
pub struct Lobby {
    pub id: String,
    pub in_msg: Sender<LobbyInMsg>,
    users: UserMap
}

impl Lobby {
    pub fn new(id: String) -> Lobby {
        let (tx, mut rx) = mpsc::channel(100);
        let users = Arc::new(Mutex::new(HashMap::new()));
        let lobby = Lobby { id: id.clone(), in_msg: tx, users };
        let ret = lobby.clone();

        tokio::spawn(async move {
            while let Some(cmd) = rx.recv().await {
                use LobbyInMsg::*;
                match cmd {
                    Join { user_id } => {
                        println!("User {} joined lobby {}", &user_id, &id);
                        lobby
                            .broadcast(LobbyOutMsg::Members(lobby.get_members().await)).await
                            .unwrap_or_else(|e| println!("Error in broadcast {}", e))
                    },
                    Leave { user_id } => println!("User {} left lobby {}", &user_id, &id),
                    StartGame => println!("Start Game"),
                    GetUsers{req_id} => println!("Get Users {}", req_id),
                    GetGameData{req_id} => println!("Get Game Data {}", req_id),
                }


            }
        });
        ret
        
    }

    async fn get_members(& self) -> Vec<String> {
        self.users.lock().await
            .iter()
            .map(|(id, _)| id.clone())
            .collect()
    }

    async fn broadcast(& self, msg: LobbyOutMsg) -> Result<(), String> {
        let users = self.users.lock().await;
        let sends = users.iter()
            .map(|(_, tx)| async {
                tx.send(msg.clone()).await.map_err(|e| format!("Unable to send {}", e))
            });
        let errors: Vec<String> = join_all(sends).await
            .into_iter()
            .filter(|r| r.is_err())
            .map(|r| r.expect_err("Expected list to only contain errors"))
            .collect();
        if errors.len() > 0 {
            Err(errors.concat())
        } else {
            Ok(())
        }
    }

    async fn send(& self, user: String, msg: LobbyOutMsg) -> Result<(), String> {
        let um = self.users.lock().await;

        if ! um.contains_key(&user){
            return Err(format!("User {} not found", &user));
        }

        let tx = um.get(&user).unwrap();

        return tx.send(msg).await.map_err(|e| format!("Unable to send {}", e))
    }

    pub async fn remove_user(&mut self, user_id: String) {
        self.users.lock().await.remove(&user_id);
    }

    pub async fn add_user(&mut self, user_id: String, tx: Sender<LobbyOutMsg>) {
        self.users.lock().await.insert(user_id, tx);
    }

}

pub struct LobbyManager {
    lobbies: HashMap<String, Lobby>
}

impl LobbyManager {
    pub fn new() -> LobbyManager {
        LobbyManager { lobbies: HashMap::new() }
    }

    pub fn get(&mut self, id: String) -> Lobby {
        match self.lobbies.get(&id){
            Some(l) => l.clone(),
            None => {
                let l = Lobby::new(id);
                self.lobbies.insert(l.id.clone(), l.clone());
                l
            
            }
        }
    }
}
