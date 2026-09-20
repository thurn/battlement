use cozy_chess::Board;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct SavedGame {
  position: String,
}

impl SavedGame {
  pub(crate) fn new(board: &Board) -> Self {
    Self {
      position: board.to_string(),
    }
  }

  pub(crate) fn board(&self) -> Option<Board> {
    self.position.parse().ok()
  }
}
