use crate::model::*;
use crate::gameplay::*;

#[derive(Clone)]
struct GameStateWithHistory {
    state: GameState,
    history: Vec<MoveAction>,
}
impl GameStateWithHistory {
    pub fn merge(mut self, other: GameStateWithHistory) -> GameStateWithHistory {
        assert !(self.state == other.state, "Cannot merge different game states");
        let shortest_history = if self.history.len() < other.history.len() {
            self.history.clone()
        } else {
            other.history.clone()
        };
        GameStateWithHistory {
            state: self.state,
            history: shortest_history,
        }
    }
}
impl PartialEq for GameStateWithHistory {
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state
    }
}
impl Eq for GameStateWithHistory {}

pub struct Solver {
    starting_state: GameState,
    considering_states: Vec<GameStateWithHistory>,
    visited_states: Vec<GameStateWithHistory>,
}

impl Solver {
    pub fn new(starting_state: GameState) -> Solver {
        Solver {
            starting_state: starting_state.clone(),
            considering_states: vec![GameStateWithHistory {
                state: starting_state,
                history: vec![],
            }],
            visited_states: vec![],
        }
    }
    fn consider_state(&mut self, state_with_history: GameStateWithHistory) {
        if !self
            .visited_states
            .iter()
            .any(|s| s.state == state_with_history.state)
        {
            self.considering_states.push(state_with_history);
        }
    }
}