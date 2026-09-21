use crate::visual_state::{DITTO_VISUAL_STATE_REGISTRY, VisualState, after_move, semantic_fixture};
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
