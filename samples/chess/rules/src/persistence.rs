//! Minimal persisted state for resuming a chess session.

use cozy_chess::Board;
use serde::{Deserialize, Serialize};

/// Serialized game data stored by Reactant's host-backed persistence hook.
///
/// The sample stores FEN rather than presentation state. Stable piece identities,
/// selection, effects, and animation state are reconstructed when the app mounts.
#[derive(Clone, PartialEq, Deserialize, Serialize)]
pub struct SavedGame {
  position: String,
}

impl SavedGame {
  /// Captures the logical board in the portable FEN representation.
  pub fn new(board: &Board) -> Self {
    Self {
      position: board.to_string(),
    }
  }

  /// Restores the saved board, returning `None` when persisted data is invalid.
  ///
  /// Treating corrupt host data as absent lets the component fall back to its
  /// configured starting state instead of making persistence a startup failure.
  pub fn board(&self) -> Option<Board> {
    self.position.parse().ok()
  }
}
