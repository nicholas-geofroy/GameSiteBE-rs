use std::{collections::HashMap, sync::Arc};
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex,
};

use crate::{
    lobby::Lobby,
    models::lobby::{InMsg, LobbyOutMsg},
};

#[derive(Debug)]
pub enum LobbyError {
    LobbyDoesNotExist,
    UserAlreadyConnected,
}

pub struct LobbyManager {
    lobbies: HashMap<String, LobbyData>,
}

impl LobbyManager {
    pub fn new() -> LobbyManager {
        LobbyManager {
            lobbies: HashMap::new(),
        }
    }

    pub fn create_lobby(&mut self, id: String) {
        self.lobbies.entry(id.clone()).or_insert(LobbyData::new(id));
    }

    pub async fn add_user(
        &mut self,
        l_id: &String,
        u_id: &String,
    ) -> Result<(Sender<InMsg>, Receiver<LobbyOutMsg>), LobbyError> {
        let lobby = match self.lobbies.get(l_id) {
            Some(l) => l,
            None => return Err(LobbyError::LobbyDoesNotExist),
        };

        let mut users = lobby.users.lock().await;

        if users.contains_key(u_id) {
            if users.get(u_id).unwrap().is_conn {
                return Err(LobbyError::UserAlreadyConnected);
            }
        }

        let (tx, rx) = mpsc::channel(100);
        users.insert(
            u_id.clone(),
            User {
                out: tx,
                is_conn: true,
            },
        );
        Ok((lobby.msg_sender.clone(), rx))
    }

    pub async fn disconnect_user(
        &mut self,
        l_id: &String,
        u_id: &String,
    ) -> Result<(), LobbyError> {
        todo!()
    }
}

pub struct User {
    pub out: Sender<LobbyOutMsg>,
    pub is_conn: bool,
}

pub struct LobbyData {
    id: String,
    users: Arc<Mutex<HashMap<String, User>>>,
    msg_sender: Sender<InMsg>,
}

impl LobbyData {
    fn new(id: String) -> LobbyData {
        let (tx, rx) = mpsc::channel(100);
        let users = Arc::new(Mutex::new(HashMap::new()));

        let mut lobby = Lobby::new(id.clone(), users.clone(), rx);

        tokio::spawn(async move {
            lobby.lobby_loop().await;
        });

        return LobbyData {
            id,
            users,
            msg_sender: tx,
        };
    }
}
