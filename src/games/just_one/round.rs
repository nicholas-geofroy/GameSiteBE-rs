use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug)]
pub enum InvalidMove {
    CouldNotParse { msg: String },
    NotYourTurn { msg: String },
    WrongState { msg: String },
    InvalidUser { msg: String },
}

#[derive(Serialize, Deserialize, PartialEq, PartialOrd, Debug, Clone, Copy)]
enum RoundState {
    GivingHints,
    RemovingDuplicates,
    Guessing,
    RoundFinished,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Guess {
    val: String,
    is_correct: bool,
    user_check: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Hint {
    val: String,
    duplicate: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RoundData<'a> {
    players: Vec<String>,
    guesser: &'a str,
    hints: HashMap<String, Hint>,
    guesses: Vec<Guess>,
    word: String,
    cur_state: RoundState,
}

impl<'a> RoundData<'a> {
    pub fn new(players: Vec<String>, guesser: &'a str, word: String) -> RoundData<'a> {
        return RoundData {
            players,
            guesser,
            hints: HashMap::new(),
            guesses: Vec::new(),
            word,
            cur_state: RoundState::GivingHints,
        };
    }

    pub fn give_hint(&mut self, user: &String, hint: String) -> Result<(), InvalidMove> {
        if self.guesser == user {
            return Err(InvalidMove::NotYourTurn {
                msg: "The guesser cannot give hints".to_owned(),
            });
        }

        if self.cur_state != RoundState::GivingHints {
            return Err(InvalidMove::WrongState {
                msg: format!("Can't give hint during {:?}", self.cur_state),
            });
        }

        self.hints
            .entry(user.clone())
            .and_modify(|h| {
                h.val = hint.clone();
            })
            .or_insert(Hint {
                val: hint,
                duplicate: false,
            });

        let h_count: HashMap<String, usize> = self
            .hints
            .iter()
            .map(|(_, v)| v.val.to_lowercase())
            .group_by(|k| k.to_owned())
            .into_iter()
            .map(|e| (e.0, e.1.count()))
            .collect();

        for (_, h) in self.hints.iter_mut() {
            h.duplicate = *h_count.get(&h.val.to_lowercase()).unwrap() > 1
        }

        if self.hints.len() == self.players.len() - 1 {
            self.cur_state = RoundState::RemovingDuplicates;
        }

        Ok(())
    }

    pub fn guess(&mut self, user: &str, val: String) -> Result<(), InvalidMove> {
        if self.guesser != user {
            return Err(InvalidMove::NotYourTurn {
                msg: "Only the guesser can guess".to_owned(),
            });
        }
        if self.cur_state != RoundState::Guessing {
            return Err(InvalidMove::WrongState {
                msg: format!("Can't guess during {:?}", self.cur_state),
            });
        }

        let is_correct = val.to_lowercase().eq(&self.word.to_lowercase());
        self.guesses.push(Guess {
            val,
            is_correct,
            user_check: false,
        });
        if is_correct {
            self.cur_state = RoundState::RoundFinished
        }

        Ok(())
    }

    pub fn done_removing_dupes(&mut self, user: &str) -> Result<(), InvalidMove> {
        if self.guesser == user {
            return Err(InvalidMove::NotYourTurn {
                msg: "The guesser cannot say all duplicates have been removed".to_owned(),
            });
        }

        self.cur_state = RoundState::Guessing;

        return Ok(());
    }

    pub fn set_duplicate(&mut self, user: &str, hint_user: &str) -> Result<(), InvalidMove> {
        if self.guesser == user {
            return Err(InvalidMove::NotYourTurn {
                msg: "Cannot set duplicate when you're the guesser".to_string(),
            });
        }
        self.hints
            .get_mut(hint_user)
            .map(|hint| {
                hint.duplicate = true;
            })
            .ok_or(InvalidMove::InvalidUser {
                msg: format!("User {} does not exist", user),
            })
    }

    pub fn set_unique(&mut self, user: &str, hint_user: &str) -> Result<(), InvalidMove> {
        if self.guesser == user {
            return Err(InvalidMove::NotYourTurn {
                msg: "Cannot set duplicate when you're the guesser".to_string(),
            });
        }
        self.hints
            .get_mut(hint_user)
            .map(|hint| {
                hint.duplicate = false;
            })
            .ok_or(InvalidMove::InvalidUser {
                msg: format!("User {} does not exist", user),
            })
    }

    pub fn set_guess_correctness(
        &mut self,
        user: &str,
        is_correct: bool,
    ) -> Result<(), InvalidMove> {
        if self.guesser == user {
            return Err(InvalidMove::NotYourTurn {
                msg: "Cannot set Guesses when you're the guesser".to_string(),
            });
        } else if self.cur_state != RoundState::Guessing {
            return Err(InvalidMove::WrongState {
                msg: "Must be in guessing state to set guesses as correct/incorrect".to_string(),
            });
        }

        self.guesses
            .last_mut()
            .map(|guess| {
                guess.is_correct = is_correct;
                guess.user_check = true;
            })
            .ok_or(InvalidMove::WrongState {
                msg: "No guess has been made yet".to_string(),
            })
    }

    pub fn filter(&self, user: &str) -> RoundData<'a> {
        let hints: HashMap<String, Hint> = self
            .hints
            .iter()
            .map(|(u, h)| {
                if self.guesser == user && (self.cur_state < RoundState::Guessing || h.duplicate) {
                    (
                        u.clone(),
                        Hint {
                            val: "".to_owned(),
                            duplicate: h.duplicate,
                        },
                    )
                } else {
                    (u.clone(), h.clone())
                }
            })
            .collect();

        let word = if self.guesser == user && self.cur_state != RoundState::RoundFinished {
            "".to_owned()
        } else {
            self.word.clone()
        };

        RoundData {
            players: self.players.clone(),
            guesser: self.guesser,
            hints,
            guesses: self.guesses.clone(),
            word,
            cur_state: self.cur_state,
        }
    }
}
