//! Typed decisions that complete an in-flight chess action.

use std::borrow::Cow;

use cozy_chess::{Piece, Square};
use reactant::rules::PromptData;

use crate::reactant_game::ChessGame;

/// A player decision requested by the chess rules worker.
pub(crate) enum ChessPrompt<'a> {
  /// Selects the piece created by one pawn move.
  Promotion(Cow<'a, PromotionPrompt>),
}

/// The missing choice required to complete one legal promotion move.
#[derive(Clone)]
pub(crate) struct PromotionPrompt {
  pub(crate) from: Square,
  pub(crate) to: Square,
  pub(crate) choices: [Piece; 4],
}

impl PromotionPrompt {
  pub(crate) const fn new(from: Square, to: Square) -> Self {
    Self {
      from,
      to,
      choices: [Piece::Queen, Piece::Rook, Piece::Bishop, Piece::Knight],
    }
  }
}

impl PromptData<ChessGame> for PromotionPrompt {
  type ResponseType = Piece;

  fn options(&self) -> impl Iterator<Item = Piece> {
    self.choices.into_iter()
  }

  fn is_valid_response(&self, response: &Piece) -> bool {
    self.choices.contains(response)
  }

  fn as_prompt(&self) -> ChessPrompt<'_> {
    ChessPrompt::Promotion(Cow::Borrowed(self))
  }

  fn into_prompt(self) -> ChessPrompt<'static> {
    ChessPrompt::Promotion(Cow::Owned(self))
  }
}
