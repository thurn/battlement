//! Semantic presentation states used by diagnostics and native Ditto scenarios.
//!
//! These names describe user-visible outcomes rather than component internals.
//! That makes automated scenarios durable even when the visual tree is refactored.

use battlement::ObjectId;
use cozy_chess::{Board, Color as PieceColor, GameStatus, Move, Piece};

/// Stable identity of the Reactant document root inspected by native scenarios.
pub const ROOT_ID: ObjectId = battlement::object_id!("43000000-0000-4000-8000-000000000002");

/// Finite user-visible presentation families recognized by the Chess engine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VisualState {
  Title,
  Initial,
  Selected,
  PlayerMove,
  AiResponse,
  Capture,
  Castle,
  EnPassant,
  Promotion,
  Check,
  PlayerWin,
  ComputerWin,
  Draw,
  Paused,
  Refreshed,
  Restarted,
  Resumed,
}

/// Deterministic board and expected semantic state for review scenarios.
pub struct SemanticFixture {
  /// Starting logical position.
  pub board: Board,
  /// Initial presentation family used when the fixture mounts.
  pub state: VisualState,
}

/// Resolves a named review fixture without adding test-only branches to components.
///
/// FEN keeps the fixtures compact and exercises the same application assembly as
/// a real session. Returning `None` lets the entry point report unknown names.
pub fn semantic_fixture(name: &str) -> Option<SemanticFixture> {
  let (fen, state) = match name {
    "title" => {
      return Some(SemanticFixture {
        board: Board::default(),
        state: VisualState::Title,
      });
    }
    "initial" => (
      "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
      VisualState::Initial,
    ),
    "capture" => ("4k3/8/8/4p3/3B4/8/8/4K3 w - - 0 1", VisualState::Initial),
    "castling" => ("4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1", VisualState::Initial),
    "en passant" => ("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1", VisualState::Initial),
    "promotion capture" => ("1r2k3/P7/8/8/8/8/8/4K3 w - - 0 1", VisualState::Initial),
    "promotion" => ("4k3/P7/8/8/8/8/8/4K3 w - - 0 1", VisualState::Initial),
    "check" => ("4k3/8/8/8/8/8/R7/4K3 w - - 0 1", VisualState::Initial),
    "checkmating capture" => ("7k/5Kp1/6Q1/8/8/8/8/8 w - - 0 1", VisualState::Initial),
    "player win" => ("7k/5K2/6Q1/8/8/8/8/8 w - - 0 1", VisualState::Initial),
    "computer win" => ("8/8/8/8/8/5kq1/8/7K b - - 0 1", VisualState::Initial),
    "draw" => ("k7/2Q5/2K5/8/8/8/8/8 w - - 0 1", VisualState::Initial),
    "resumed board" => ("7k/8/5KQ1/8/8/8/8/8 w - - 0 1", VisualState::Resumed),
    _ => return None,
  };
  Some(SemanticFixture {
    board: fen.parse().expect("checked Chess semantic fixture FEN"),
    state,
  })
}

/// Classifies the most specific user-visible outcome of an accepted move.
///
/// Classification happens beside the rules transition because it needs both the
/// old and new boards. Components consume the resulting semantic value instead
/// of duplicating chess inference in the view layer.
pub fn after_move(
  board_before: &Board,
  board_after: &Board,
  mv: Move,
  mover: PieceColor,
) -> VisualState {
  match board_after.status() {
    GameStatus::Won if mover == PieceColor::White => VisualState::PlayerWin,
    GameStatus::Won => VisualState::ComputerWin,
    GameStatus::Drawn => VisualState::Draw,
    GameStatus::Ongoing if mv.promotion.is_some() => VisualState::Promotion,
    GameStatus::Ongoing if is_castle(board_before, mv, mover) => VisualState::Castle,
    GameStatus::Ongoing if is_en_passant(board_before, mv) => VisualState::EnPassant,
    GameStatus::Ongoing if board_before.piece_on(mv.to).is_some() => VisualState::Capture,
    GameStatus::Ongoing if !board_after.checkers().is_empty() => VisualState::Check,
    GameStatus::Ongoing if mover == PieceColor::Black => VisualState::AiResponse,
    GameStatus::Ongoing => VisualState::PlayerMove,
  }
}

/// Recognizes cozy-chess's king-to-friendly-rook representation of castling.
fn is_castle(board: &Board, mv: Move, mover: PieceColor) -> bool {
  board.piece_on(mv.from) == Some(Piece::King) && board.color_on(mv.to) == Some(mover)
}

/// Recognizes a diagonal pawn move whose captured piece is not on the destination.
fn is_en_passant(board: &Board, mv: Move) -> bool {
  board.piece_on(mv.from) == Some(Piece::Pawn)
    && mv.from.file() != mv.to.file()
    && board.piece_on(mv.to).is_none()
}
