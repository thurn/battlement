use battlement::{
  Color, Command, CommandBody, Label, ObjectId, PickingMode, Position, Style, UiDocument, UiNode,
  object_id,
};
use cozy_chess::{Board, Color as PieceColor, GameStatus, Move, Piece};

const DOCUMENT_ID: ObjectId = object_id!("43000000-0000-4000-8000-000000000001");
const ROOT_ID: ObjectId = object_id!("43000000-0000-4000-8000-000000000002");
const STATE_IDS: [ObjectId; 17] = [
  object_id!("43000000-0000-4000-8000-000000000101"),
  object_id!("43000000-0000-4000-8000-000000000102"),
  object_id!("43000000-0000-4000-8000-000000000103"),
  object_id!("43000000-0000-4000-8000-000000000104"),
  object_id!("43000000-0000-4000-8000-000000000105"),
  object_id!("43000000-0000-4000-8000-000000000106"),
  object_id!("43000000-0000-4000-8000-000000000107"),
  object_id!("43000000-0000-4000-8000-000000000108"),
  object_id!("43000000-0000-4000-8000-000000000109"),
  object_id!("43000000-0000-4000-8000-000000000110"),
  object_id!("43000000-0000-4000-8000-000000000111"),
  object_id!("43000000-0000-4000-8000-000000000112"),
  object_id!("43000000-0000-4000-8000-000000000113"),
  object_id!("43000000-0000-4000-8000-000000000114"),
  object_id!("43000000-0000-4000-8000-000000000115"),
  object_id!("43000000-0000-4000-8000-000000000116"),
  object_id!("43000000-0000-4000-8000-000000000117"),
];

/// Finite user-visible presentation families exercised by the Chess Ditto suite.
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

  pub(crate) const fn object_id(self) -> ObjectId {
    STATE_IDS[self as usize]
  }

  const fn label(self) -> &'static str {
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

pub(crate) struct SemanticFixture {
  pub(crate) board: Board,
  pub(crate) state: VisualState,
}

pub(crate) fn semantic_fixture(name: &str) -> Option<SemanticFixture> {
  let (fen, state) = match name {
    "opening board move and ai" | "pause refresh and restart" => {
      return Some(SemanticFixture {
        board: Board::default(),
        state: VisualState::Title,
      });
    }
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

pub(crate) fn after_move(
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

pub(crate) fn document(state: VisualState) -> UiDocument {
  UiDocument::with_root_id(DOCUMENT_ID, ROOT_ID)
    .name("chess-state")
    .picking_mode(PickingMode::Ignore)
    .style(
      Style::new()
        .position(Position::Absolute)
        .top(0)
        .left(0)
        .right(0),
    )
    .child(self::node(state))
}

pub(crate) fn transition(from: VisualState, to: VisualState) -> [CommandBody; 2] {
  [
    Command::destroy_visual_element(from.object_id()).body,
    Command::create_visual_element(ROOT_ID, self::node(to)).body,
  ]
}

fn node(state: VisualState) -> UiNode {
  UiNode::new(
    state.object_id(),
    Label::new(state.label())
      .name(state.registry_key())
      .picking_mode(PickingMode::Ignore)
      .style(
        Style::new()
          .position(Position::Absolute)
          .top(18)
          .left(24)
          .padding((9, 15))
          .font_size(18)
          .color(Color::rgb(0.96, 0.97, 0.91))
          .background_color(Color::rgba(0.03, 0.04, 0.035, 0.88))
          .border_radius(6),
      ),
  )
}

fn is_castle(board: &Board, mv: Move, mover: PieceColor) -> bool {
  board.piece_on(mv.from) == Some(Piece::King) && board.color_on(mv.to) == Some(mover)
}

fn is_en_passant(board: &Board, mv: Move) -> bool {
  board.piece_on(mv.from) == Some(Piece::Pawn)
    && mv.from.file() != mv.to.file()
    && board.piece_on(mv.to).is_none()
}

#[cfg(test)]
mod tests {
  use super::{VisualState, after_move, semantic_fixture};
  use cozy_chess::{Board, Color, Move, Piece, Square};

  #[test]
  fn visual_state_inventory_and_semantic_fixtures_are_exhaustive() {
    assert_eq!(VisualState::ALL.len(), 17);
    assert_eq!(
      crate::DITTO_VISUAL_STATE_REGISTRY
        .matches("[[states]]")
        .count(),
      VisualState::ALL.len()
    );
    for state in VisualState::ALL {
      assert!(!state.registry_key().is_empty());
      assert!(
        crate::DITTO_VISUAL_STATE_REGISTRY.contains(&format!("key = \"{}\"", state.registry_key())),
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
