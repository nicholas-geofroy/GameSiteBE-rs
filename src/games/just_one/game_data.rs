use super::round::*;

use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::fs::read_to_string;

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameData<'a> {
    players: &'a Vec<String>,

    round: usize,
    rounds: Vec<RoundData<'a>>,

    #[serde(skip)]
    words: Arc<WordList>,
    #[serde(skip)]
    cur_word: usize,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "actionType", content = "data")]
#[serde(rename_all = "camelCase")]
enum JustOneMove {
    Guess(String),
    Hint(String),
    #[serde(rename_all = "camelCase")]
    SetDuplicate {
        hint_id: String,
    },
    #[serde(rename_all = "camelCase")]
    SetUnique {
        hint_id: String,
    },
    RevealHints,
    CorrectGuess,
    WrongGuess,
    NextRound,
}

#[derive(Deserialize, Debug, Clone)]
struct WordList {
    description: String,
    words: Vec<String>,
}

impl<'a> GameData<'a> {
    pub async fn new(players: &'a Vec<String>) -> GameData<'a> {
        let words = read_to_string("assets/nouns.json")
            .await
            .expect("Expected nouns file to exist");

        let mut word_list: WordList =
            serde_json::from_str(&words).expect("Could not parse words from file");
        word_list.words.shuffle(&mut thread_rng());

        let mut game = GameData {
            players,
            round: 0,
            rounds: Vec::new(),
            words: Arc::new(word_list),
            cur_word: 0,
        };
        game.new_round();

        return game;
    }

    fn new_round(&mut self) {
        self.rounds.push(RoundData::new(
            self.players.clone(),
            self.players[self.round % self.players.len()].as_str(),
            self.words
                .words
                .get(self.cur_word % self.words.words.len())
                .expect("Index out of range")
                .to_owned(),
        ));
        self.cur_word += 1;
        self.round += 1;
    }

    fn cur_round(&mut self) -> &mut RoundData<'a> {
        return &mut self.rounds[self.round - 1];
    }

    pub fn filter(&self, user: &str) -> GameData<'a> {
        let mut rounds = self.rounds.clone();

        if let Some(last) = rounds.pop() {
            rounds.push(last.filter(user))
        }

        return GameData {
            players: self.players,
            round: self.round,
            rounds,
            words: self.words.clone(),
            cur_word: self.cur_word,
        };
    }

    pub fn make_move(&mut self, req_uid: &String, m: Value) -> Result<&GameData<'a>, InvalidMove> {
        let res = serde_json::from_value(m);

        if let Err(e) = res {
            return Err(InvalidMove::CouldNotParse { msg: e.to_string() });
        }

        let cur_roud = self.cur_round();

        return match res.unwrap() {
            JustOneMove::Guess(guess) => cur_roud.guess(req_uid, guess),
            JustOneMove::Hint(hint) => cur_roud.give_hint(req_uid, hint),
            JustOneMove::SetDuplicate { hint_id } => cur_roud.set_duplicate(&req_uid, &hint_id),
            JustOneMove::SetUnique { hint_id } => cur_roud.set_unique(&req_uid, &hint_id),
            JustOneMove::RevealHints => cur_roud.done_removing_dupes(&req_uid),
            JustOneMove::CorrectGuess => cur_roud.set_guess_correctness(&req_uid, true),
            JustOneMove::WrongGuess => cur_roud.set_guess_correctness(&req_uid, false),
            JustOneMove::NextRound => Ok(self.new_round()),
        }
        .map(|_| &*self);
    }
}
