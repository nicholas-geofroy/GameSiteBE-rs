use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::games::GameType;

pub struct InMsg {
    pub uid: String,
    pub cmd: LobbyInMsg,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "msgType", content = "data")]
#[serde(rename_all = "camelCase")]
pub enum LobbyInMsg {
    #[serde(rename_all = "camelCase")]
    Join { user_id: String },
    #[serde(rename_all = "camelCase")]
    Leave,
    #[serde(rename_all = "camelCase")]
    Start,
    #[serde(rename_all = "camelCase")]
    GetUsers,
    #[serde(rename_all = "camelCase")]
    GetGameData,
    #[serde(rename_all = "camelCase")]
    GameMove { action: Value },
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum LobbyState {
    InLobby,
    InGame,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "msgType", content = "data")]
pub enum LobbyOutMsg {
    Error { msg: String },
    Members(Vec<String>),
    SelectedGame(GameType),
    GameState(Value),
}
