//! End-to-end latency measurement, deliberately separate from ordinary CI tests.
//! Run with: cargo test --release --test performance -- --ignored --nocapture
//! Every sample includes fixture construction, fresh engine/host, real gestures,
//! finite presentation, behavioral assertions, and destruction. Only the immutable
//! asset catalog and executable are warmed. No AI search, JSON, or live dialog is
//! hidden in the numbers. Shared CI load is unsuitable for a strict timing gate.
mod support;
use crate::support::{catalog, fixtures, game::ChessTest, host};
use cozy_chess::{Color, Piece, Square};
use std::time::{Duration, Instant};

#[test]
#[ignore = "run optimized and without competing builds to measure complete scenarios"]
fn complete_scenario_latency() {
  // The ignored benchmark must compile in debug CI, but reject a debug run.
  #[allow(clippy::assertions_on_constants)]
  {
    assert!(!cfg!(debug_assertions), "measure optimized code");
  }
  let first_use = Instant::now();
  let _assets = catalog::assets();
  eprintln!(
    "immutable_catalog_first_use={}ns",
    first_use.elapsed().as_nanos()
  );
  let mut missed = Vec::new();
  for (name, scenario) in [
    ("initial_board", initial_board as fn()),
    ("initial_move", initial_move),
    ("capture", capture),
    ("castle", castle),
    ("en_passant", en_passant),
    ("promotion", promotion),
    ("rejected_move", rejected_move),
    ("title_start", title_start),
    ("reset", reset),
    ("opponent_reply", opponent_reply),
  ] {
    for _ in 0..20 {
      scenario();
    }
    let mut samples = Vec::with_capacity(1_000);
    for _ in 0..1_000 {
      let start = Instant::now();
      scenario();
      samples.push(start.elapsed());
    }
    samples.sort_unstable();
    if samples[950] >= Duration::from_millis(1) {
      missed.push(name);
    }
    eprintln!(
      "{name}: samples={} median={}ns p95={}ns max={}ns target_p95_lt_1ms={}",
      samples.len(),
      samples[500].as_nanos(),
      samples[950].as_nanos(),
      samples[999].as_nanos(),
      samples[950] < Duration::from_millis(1)
    );
  }
  assert!(
    missed.is_empty(),
    "sub-millisecond acceptance not met: {missed:?}"
  );
}

// Each benchmark retains the same independent observations as its ordinary test.
// Setup is fresh on every call, so no previous scenario can warm mutable state.
fn initial_board() {
  let g = ChessTest::from_position(fixtures::initial());
  assert_eq!(host::pieces(&g.display).len(), 32);
  g.expect_piece(Square::E2, Color::White, Piece::Pawn);
}

fn rejected_move() {
  let mut g = ChessTest::from_position(fixtures::initial());
  host::move_piece(&mut g.display, Square::E2, Square::E5);
  g.display.finish_inline();
  g.expect_piece(Square::E2, Color::White, Piece::Pawn);
  g.expect_empty(Square::E5);
}

fn title_start() {
  let mut g = ChessTest::title();
  g.start();
  assert_eq!(host::pieces(&g.display).len(), 32);
  g.expect_piece(Square::E2, Color::White, Piece::Pawn);
}

fn reset() {
  let mut g = ChessTest::from_position(fixtures::capture());
  g.new_game();
  assert_eq!(host::pieces(&g.display).len(), 32);
  g.expect_piece(Square::E2, Color::White, Piece::Pawn);
}

fn opponent_reply() {
  let mut g = ChessTest::from_position(fixtures::initial());
  g.play(Square::E2, Square::E4);
  g.reply(Square::E7, Square::E5);
  g.expect_piece(Square::E4, Color::White, Piece::Pawn);
  g.expect_piece(Square::E5, Color::Black, Piece::Pawn);
}

fn initial_move() {
  let mut g = ChessTest::from_position(fixtures::initial());
  g.play(Square::E2, Square::E4);
  g.expect_piece(Square::E4, Color::White, Piece::Pawn);
}

fn capture() {
  let mut g = ChessTest::from_position(fixtures::capture());
  g.play(Square::D4, Square::E5);
  g.expect_piece(Square::E5, Color::White, Piece::Bishop);
}

fn castle() {
  let mut g = ChessTest::from_position(fixtures::castle());
  g.play(Square::E1, Square::G1);
  g.expect_piece(Square::G1, Color::White, Piece::King);
  g.expect_piece(Square::F1, Color::White, Piece::Rook);
}

fn en_passant() {
  let mut g = ChessTest::from_position(fixtures::en_passant());
  g.play(Square::E5, Square::D6);
  g.expect_piece(Square::D6, Color::White, Piece::Pawn);
  g.expect_empty(Square::D5);
}

fn promotion() {
  let mut g = ChessTest::from_position(fixtures::promotion());
  g.promote(Square::A7, Square::B8, Piece::Knight);
  g.expect_piece(Square::B8, Color::White, Piece::Knight);
}

#[test]
#[ignore = "isolated latency diagnostic"]
fn phase_latency() {
  let _assets = catalog::assets();
  for _ in 0..5 {
    let start = Instant::now();
    let board = fixtures::capture();
    let fixture = start.elapsed();
    let start = Instant::now();
    let mut game = ChessTest::from_position(board);
    let setup = start.elapsed();
    let start = Instant::now();
    game.play(Square::D4, Square::E5);
    let action = start.elapsed();
    let start = Instant::now();
    game.expect_piece(Square::E5, Color::White, Piece::Bishop);
    let assertion = start.elapsed();
    let start = Instant::now();
    drop(game);
    let teardown = start.elapsed();
    eprintln!(
      "fixture={fixture:?} setup={setup:?} action={action:?} assertion={assertion:?} teardown={teardown:?}"
    );
  }
}
