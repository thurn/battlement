//! Semantic presentation states used by diagnostics and native Ditto scenarios.
//!
//! These names describe user-visible outcomes rather than component internals.
//! That makes automated scenarios durable even when the visual tree is refactored.

use battlement::{ObjectId, object_id};
use cozy_chess::{Board, Color as PieceColor, GameStatus, Move, Piece};

#[cfg(test)]
const DITTO_VISUAL_STATE_REGISTRY: &str = include_str!("../../ditto-visual-states.toml");

/// Stable identity of the Reactant document root inspected by native scenarios.
pub const ROOT_ID: ObjectId = object_id!("43000000-0000-4000-8000-000000000002");

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
  /// State marker to expose when the fixture mounts.
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
    "promotion" => ("4k3/P7/8/8/8/8/8/4K3 w - - 0 1", VisualState::Initial),
    "check" => ("4k3/8/8/8/8/8/R7/4K3 w - - 0 1", VisualState::Initial),
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

impl VisualState {
  /// Every visual state in registry order.
  pub const ALL: [Self; 17] = [
    Self::Title,
    Self::Initial,
    Self::Selected,
    Self::PlayerMove,
    Self::AiResponse,
    Self::Capture,
    Self::Castle,
    Self::EnPassant,
    Self::Promotion,
    Self::Check,
    Self::PlayerWin,
    Self::ComputerWin,
    Self::Draw,
    Self::Paused,
    Self::Refreshed,
    Self::Restarted,
    Self::Resumed,
  ];

  /// Returns the canonical Ditto registry key.
  pub const fn registry_key(self) -> &'static str {
    match self {
      Self::Title => "screen.title",
      Self::Initial => "board.initial",
      Self::Selected => "selection.legal-targets",
      Self::PlayerMove => "move.committed",
      Self::AiResponse => "turn.ai-response",
      Self::Capture => "move.capture",
      Self::Castle => "special.castle",
      Self::EnPassant => "special.en-passant",
      Self::Promotion => "special.promotion",
      Self::Check => "feedback.check",
      Self::PlayerWin => "terminal.player-win",
      Self::ComputerWin => "terminal.computer-win",
      Self::Draw => "terminal.draw",
      Self::Paused => "menu.paused",
      Self::Refreshed => "board.refreshed",
      Self::Restarted => "board.restarted",
      Self::Resumed => "board.resumed",
    }
  }

  /// Returns the hidden semantic label exposed for the active state.
  ///
  /// The labels are not player-facing layout. They give native black-box tests
  /// an accessible assertion surface without coupling them to Rust state.
  pub const fn label(self) -> &'static str {
    match self {
      Self::Title => "CHESS · START A NEW GAME",
      Self::Initial => "YOUR TURN · CHOOSE A PIECE",
      Self::Selected => "PIECE SELECTED · LEGAL TARGETS SHOWN",
      Self::PlayerMove => "MOVE COMMITTED · COMPUTER THINKING",
      Self::AiResponse => "COMPUTER MOVED · YOUR TURN",
      Self::Capture => "PIECE CAPTURED",
      Self::Castle => "CASTLING COMPLETE",
      Self::EnPassant => "EN PASSANT COMPLETE",
      Self::Promotion => "PAWN PROMOTED TO QUEEN",
      Self::Check => "CHECK",
      Self::PlayerWin => "CHECKMATE · YOU WIN",
      Self::ComputerWin => "CHECKMATE · COMPUTER WINS",
      Self::Draw => "DRAW",
      Self::Paused => "PAUSED · REFRESH STARTS A NEW GAME",
      Self::Refreshed => "NEW GAME · BOARD REFRESHED",
      Self::Restarted => "NEW GAME · RESTART SHORTCUT",
      Self::Resumed => "SAVED GAME RESUMED",
    }
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

#[cfg(test)]
mod tests {
  use crate::visual_state::{
    DITTO_VISUAL_STATE_REGISTRY, VisualState, after_move, semantic_fixture,
  };
  use cozy_chess::{Board, Color, Move, Piece, Square};

  #[test]
  /// Keeps the semantic registry synchronized with deterministic review fixtures.
  fn deterministic_visual_states_and_semantic_fixtures_are_registered() {
    assert_eq!(VisualState::ALL.len(), 17);
    let deterministic_states = [
      VisualState::Title,
      VisualState::ComputerWin,
      VisualState::Resumed,
    ];
    assert_eq!(
      DITTO_VISUAL_STATE_REGISTRY.matches("[[states]]").count(),
      deterministic_states.len()
    );
    for state in deterministic_states {
      assert!(!state.registry_key().is_empty());
      assert!(
        DITTO_VISUAL_STATE_REGISTRY.contains(&format!("key = \"{}\"", state.registry_key())),
        "registry is missing {}",
        state.registry_key()
      );
    }
    for name in [
      "capture",
      "castling",
      "en passant",
      "promotion",
      "check",
      "player win",
      "computer win",
      "draw",
      "resumed board",
    ] {
      assert!(semantic_fixture(name).is_some(), "missing fixture {name}");
    }
  }

  #[test]
  /// Distinguishes equivalent quiet moves by which side committed them.
  fn normal_moves_distinguish_player_commit_from_ai_response() {
    let mut board = Board::default();
    let player_move = Move {
      from: Square::E2,
      to: Square::E4,
      promotion: None,
    };
    let before_player = board.clone();
    board.play(player_move);
    assert_eq!(
      after_move(&before_player, &board, player_move, Color::White),
      VisualState::PlayerMove
    );

    let ai_move = Move {
      from: Square::E7,
      to: Square::E5,
      promotion: None,
    };
    let before_ai = board.clone();
    board.play(ai_move);
    assert_eq!(
      after_move(&before_ai, &board, ai_move, Color::Black),
      VisualState::AiResponse
    );
  }

  #[test]
  /// Proves the draw fixture reaches the intended stalemate through legal play.
  fn draw_fixture_reaches_stalemate_through_a_legal_move() {
    let mut board = semantic_fixture("draw").unwrap().board;
    let mv = Move {
      from: Square::C7,
      to: Square::B6,
      promotion: None,
    };
    assert!(board.is_legal(mv));
    let before = board.clone();
    board.play(mv);
    assert_eq!(
      after_move(&before, &board, mv, Color::White),
      VisualState::Draw
    );
    assert_eq!(board.status(), cozy_chess::GameStatus::Drawn);
    assert_eq!(board.piece_on(Square::B6), Some(Piece::Queen));
  }
}
