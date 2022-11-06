pub mod just_one;

use serde::{Serialize, Deserialize};
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
pub enum GameType {
    #[default]
    JustOne
}
