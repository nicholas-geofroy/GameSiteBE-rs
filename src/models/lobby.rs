use serde::{Serialize, Deserialize};

use crate::games::GameType;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "msgType", content = "data")]
#[serde(rename_all = "camelCase")]
pub enum LobbyInMsg {
    #[serde(rename_all = "camelCase")]
    Join{user_id: String},
    #[serde(rename_all = "camelCase")]
    Leave{user_id: String},
    #[serde(rename_all = "camelCase")]
    StartGame,
    #[serde(rename_all = "camelCase")]
    GetUsers{req_uid: String},
    #[serde(rename_all = "camelCase")]
    GetGameData{req_uid: String},
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum LobbyState {
    InLobby,
    InGame
}


#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "msgType", content = "data")]
pub enum LobbyOutMsg {
    Error{ msg: String },
    Members(Vec<String>),
    SelectedGame(GameType),
}

