//! Typed decisions that complete an in-flight chess action.

use std::borrow::Cow;

use cozy_chess::{Piece, Square};
use reactant::rules::PromptData;

use crate::reactant_game::ChessGame;

/// A player decision requested by the chess rules worker.
pub enum ChessPrompt<'a> {
  /// Selects the piece created by one pawn move.
  Promotion(Cow<'a, PromotionPrompt>),
}

/// The missing choice required to complete one legal promotion move.
#[derive(Clone)]
pub struct PromotionPrompt {
  /// Pawn's source square, retained so the dialog can describe the pending move.
  pub from: Square,
  /// Promotion square selected by the player.
  pub to: Square,
  /// Responses accepted by the rules worker.
  pub choices: [Piece; 4],
}

impl PromotionPrompt {
  /// Creates the decision payload before the move mutates logical state.
  ///
  /// In Reactant, a rules action may pause on typed prompt data. The component
  /// submits one of these advertised choices, after which the same action resumes.
  pub const fn new(from: Square, to: Square) -> Self {
    Self {
      from,
      to,
      choices: [Piece::Queen, Piece::Rook, Piece::Bishop, Piece::Knight],
    }
  }
}

impl PromptData<ChessGame> for PromotionPrompt {
  type ResponseType = Piece;

  /// Enumerates choices for generic prompt consumers and validation.
  fn options(&self) -> impl Iterator<Item = Piece> {
    self.choices.into_iter()
  }

  /// Rejects stale or fabricated responses before rules execution resumes.
  fn is_valid_response(&self, response: &Piece) -> bool {
    self.choices.contains(response)
  }

  /// Borrows this payload when presenting a prompt from an in-flight action.
  fn as_prompt(&self) -> ChessPrompt<'_> {
    ChessPrompt::Promotion(Cow::Borrowed(self))
  }

  /// Owns this payload when it must outlive the action stack, such as in the UI.
  fn into_prompt(self) -> ChessPrompt<'static> {
    ChessPrompt::Promotion(Cow::Owned(self))
  }
}
