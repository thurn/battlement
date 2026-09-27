//! Live dialogs need the real blocking rules worker; ordinary tests script choices.
//! Both use the same Display API. Only this configuration waits on notifications.
mod support;
use crate::support::{fixtures, game::ChessTest};
use battlement::application::ApplicationState;
use chess_rules::{ChessGame, Opponent, assets};
use cozy_chess::{Color, Piece, Square};
use reactant::DispatchResult;
use reactant::rules::Game;
use reactant_testing::ActionResult;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn promotion_dialog_resumes_the_move() {
  // Choosing an actual displayed button resumes the original synchronous choose call.
  let mut game = ChessTest::live_dialog(fixtures::promotion());
  assert_eq!(
    game.attempt_move(Square::A7, Square::B8),
    ActionResult::AwaitingInput
  );
  game.display.expect_button("Knight");
  game.display.expect_prefab_count(assets::white::KNIGHT, 0);
  game.display.click_button("Knight");
  game.expect_empty(Square::A7);
  game.expect_piece(Square::B8, Color::White, Piece::Knight);
  assert_eq!(game.pieces().len(), 3);
}

#[test]
fn restart_discards_a_pending_promotion_answer() {
  // A captured event belongs to the old session and cannot mutate the replacement.
  let mut game = ChessTest::live_dialog(fixtures::promotion());
  assert_eq!(
    game.attempt_move(Square::A7, Square::B8),
    ActionResult::AwaitingInput
  );
  let stale_answer = game.display.button_event("Knight");
  game.restart();
  game
    .display
    .action(|display| display.deliver_ui_event(stale_answer));
  game.expect_piece(Square::A2, Color::White, Piece::Pawn);
  game.expect_piece(Square::B8, Color::Black, Piece::Knight);
  assert_eq!(game.pieces().len(), 32);
}

#[test]
fn zero_budget_computer_plays_a_deterministic_reply() {
  // This tests the production selector, not the scripted opponent. Independent
  // games must produce the same complete visible board, including unmoved pieces.
  let mut first = ChessTest::with_computer(fixtures::initial());
  let mut second = ChessTest::with_computer(fixtures::initial());
  first.play(Square::E2, Square::E4);
  second.play(Square::E2, Square::E4);
  first.advance(Duration::from_secs(2));
  second.advance(Duration::from_secs(2));
  wait_for_reply(&mut first);
  wait_for_reply(&mut second);
  first.expect_piece(Square::E4, Color::White, Piece::Pawn);
  first.expect_piece(Square::A5, Color::Black, Piece::Pawn);
  first.expect_empty(Square::A7);
  first.expect_board(&fixtures::opening_reply());
  assert_eq!(first.pieces(), second.pieces());
}

fn wait_for_reply(game: &mut ChessTest) {
  let deadline = Instant::now() + Duration::from_secs(5);
  loop {
    game.display.settle();
    let white = game.display.with_engine(|engine| {
      engine
        .game::<ChessGame>()
        .unwrap()
        .accepted_state()
        .board()
        .side_to_move()
        == Color::White
    });
    if white {
      return;
    }
    assert!(Instant::now() < deadline, "computer reply did not complete");
    thread::yield_now();
  }
}

#[test]
fn running_search_is_off_thread_and_cancelled_by_pause_menu_restart_and_unmount() {
  for action in ["pause", "background", "menu", "restart", "unmount"] {
    let (started, start) = mpsc::channel();
    let (finished, finish) = mpsc::channel();
    let owner = thread::current().id();
    let opponent = Opponent::with_search(move |board, token| {
      assert_ne!(thread::current().id(), owner);
      started.send(()).unwrap();
      while !token.is_cancelled() {
        thread::sleep(Duration::from_millis(1));
      }
      // Deliberately return a legal result after cancellation: ownership must reject it.
      let reply = "e7e5".parse().unwrap();
      assert!(board.is_legal(reply));
      finished.send(()).unwrap();
      reply
    });
    let mut game = ChessTest::with_opponent(fixtures::initial(), opponent);
    game.play(Square::E2, Square::E4);
    game.advance(Duration::from_secs(2));
    start
      .recv_timeout(Duration::from_secs(5))
      .expect("search never started");
    let before = Instant::now();
    match action {
      "pause" => game.pause(),
      "background" => {
        game.display.set_application_state(ApplicationState {
          focused: false,
          paused: true,
        });
        game.display.settle();
      }
      "menu" => game.display.click_button("Main menu"),
      "restart" => game.restart(),
      "unmount" => {
        drop(game);
        finish
          .recv_timeout(Duration::from_secs(1))
          .expect("unmount did not cancel search");
        continue;
      }
      _ => unreachable!(),
    }
    assert!(
      before.elapsed() < Duration::from_secs(1),
      "UI waited for search"
    );
    finish
      .recv_timeout(Duration::from_secs(1))
      .expect("search was not cancelled");
    game.display.settle();
    game.expect_piece(Square::E7, Color::Black, Piece::Pawn);
    game.expect_empty(Square::E5);
    if action == "restart" {
      game.expect_board(&fixtures::initial());
    }
  }
}

#[test]
fn computer_result_admission_requires_the_exact_position_and_legal_move() {
  type ChessAction = <ChessGame as Game>::Action;
  let mut game = ChessTest::from_position(fixtures::initial());
  game.play(Square::E2, Square::E4);
  let handle = game
    .display
    .with_engine(|engine| engine.game::<ChessGame>().unwrap());
  let position = handle.accepted_state().board().clone();
  let mut stale = position.clone();
  stale.play("e7e5".parse().unwrap());
  for (position, reply) in [(stale, "e7e5"), (position.clone(), "a1a8")] {
    assert!(!<ChessGame as Game>::is_legal_action(
      &handle.accepted_state(),
      &ChessAction::ComputerMove {
        position,
        reply: reply.parse().unwrap()
      },
    ));
  }
  assert_eq!(
    handle.dispatch(ChessAction::ComputerMove {
      position,
      reply: "e7e5".parse().unwrap()
    }),
    DispatchResult::Started
  );
  game.display.settle();
  game.expect_piece(Square::E5, Color::Black, Piece::Pawn);
}

#[test]
fn resume_searches_again_and_accepts_only_the_new_reply() {
  let (started, start) = mpsc::channel();
  let attempts = AtomicUsize::new(0);
  let opponent = Opponent::with_search(move |board, token| {
    let attempt = attempts.fetch_add(1, Ordering::SeqCst);
    let reply = if attempt == 0 {
      started.send(()).unwrap();
      while !token.is_cancelled() {
        thread::sleep(Duration::from_millis(1));
      }
      "e7e5"
    } else {
      "d7d5"
    }
    .parse()
    .unwrap();
    assert!(board.is_legal(reply));
    reply
  });
  let mut game = ChessTest::with_opponent(fixtures::initial(), opponent);
  game.play(Square::E2, Square::E4);
  game.advance(Duration::from_secs(2));
  start.recv_timeout(Duration::from_secs(5)).unwrap();
  game.pause();
  game.pause();
  wait_for_reply(&mut game);
  game.expect_piece(Square::D5, Color::Black, Piece::Pawn);
  game.expect_piece(Square::E7, Color::Black, Piece::Pawn);
}
