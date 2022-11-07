use itertools::Itertools;
use std::collections::HashMap;

enum InvalidMove {
    NotYourTurn { msg: String },
    WrongState { msg: String },
}

#[derive(PartialEq, Debug)]
enum RoundState {
    GivingHints,
    RemovingDuplicates,
    Guessing,
    RoundFinished,
}
struct Guess {
    val: String,
    is_correct: bool,
}

struct Hint {
    val: String,
    duplicate: bool,
}

struct RoundData<'a> {
    players: Vec<&'a String>,
    guesser: &'a str,
    hints: HashMap<&'a str, Hint>,
    guesses: Vec<Guess>,
    word: String,
    cur_state: RoundState,
}

impl<'a> RoundData<'a> {
    fn new(players: Vec<&'a String>, guesser: &'a str) -> RoundData<'a> {
        return RoundData {
            players,
            guesser,
            hints: HashMap::new(),
            guesses: Vec::new(),
            word: "word".to_owned(), //TODO: Random word
            cur_state: RoundState::GivingHints,
        };
    }

    fn give_hint(&mut self, user: &'a str, hint: String) -> Result<(), InvalidMove> {
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
            .entry(user)
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

        if self.hints.len() == self.players.len() {
            self.cur_state = RoundState::RemovingDuplicates;
        }

        Ok(())
    }

    fn guess(&mut self, user: &'a str, val: String) -> Result<(), InvalidMove> {
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
        self.guesses.push(Guess { val, is_correct });
        if is_correct {
            self.cur_state = RoundState::RoundFinished
        }

        Ok(())
    }

    fn done_removing_dupes(&mut self, user: &'a str) -> Result<(), InvalidMove> {
        if self.guesser == user {
            return Err(InvalidMove::NotYourTurn {
                msg: "The guesser cannot say all duplicates have been removed".to_owned(),
            });
        }

        self.cur_state = RoundState::Guessing;

        return Ok(());
    }
}

struct GameData<'a> {
    players: Vec<&'a String>,

    round: usize,
    rounds: Vec<Box<RoundData<'a>>>,
}

impl<'a> GameData<'a> {
    fn new(players: Vec<&'a String>) -> GameData<'a> {
        return GameData {
            players,
            round: 0,
            rounds: Vec::new(),
        };
    }

    fn new_round(&mut self) {
        self.rounds.push(Box::new(RoundData::new(
            self.players.clone(),
            self.players[self.round % self.players.len()],
        )));
        self.round += 1;
    }

    fn cur_round(&mut self) -> &mut Box<RoundData<'a>> {
        return &mut self.rounds[self.round - 1];
    }

    fn guess(&mut self, user: &'a str, val: String) -> Result<&GameData<'a>, InvalidMove> {
        self.cur_round().guess(user, val).map(|_| &*self)
    }

    fn give_hint(&mut self, user: &'a str, val: String) -> Result<&GameData<'a>, InvalidMove> {
        self.cur_round().give_hint(user, val).map(|_| &*self)
    }

    fn done_removing_dupes(&mut self, user: &'a str) -> Result<&GameData<'a>, InvalidMove> {
        self.cur_round().done_removing_dupes(user).map(|_| &*self)
    }
}
