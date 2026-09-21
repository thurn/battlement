use std::time::Duration;

use cozy_chess::Board;

#[test]
/// Verifies scenarios remain deterministic when they disable timed search.
fn zero_budget_always_chooses_the_same_legal_move() {
  let board = Board::default();
  let first = super::choose_move(&board, Duration::ZERO).unwrap();

  assert!(board.is_legal(first));
  assert_eq!(super::choose_move(&board, Duration::ZERO), Some(first));
}

#[test]
/// Verifies the zero-budget fallback still takes an immediately winning move.
fn zero_budget_prefers_an_immediate_checkmate() {
  let board: Board = "8/8/8/8/8/5kq1/8/7K b - - 0 1".parse().unwrap();
  let selected = super::choose_move(&board, Duration::ZERO).unwrap();
  let mut finished = board.clone();

  finished.play_unchecked(selected);

  assert_eq!(finished.status(), cozy_chess::GameStatus::Won);
}
