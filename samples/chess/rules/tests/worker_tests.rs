//! The small worker lane proves live prompt delivery and response routing. These
//! tests intentionally wait on worker notifications and are excluded from the
//! sub-millisecond gameplay benchmark. Keeping this boundary explicit prevents
//! fast scripted-choice tests from claiming coverage of an actual dialog.
mod support;
use crate::support::{catalog, fixtures, host};
use battlement::{ClickEvent, PhysicalKey, UiEventBody};
use battlement::{Connect, ScreenSize};
use chess_rules::{self, ChessGame, EngineDependencies, Opponent};
use cozy_chess::{Color, Piece, Square};
use reactant::rules::RulesWorker;
use reactant_testing::{Display, GameActionResult};
use std::{rc::Rc, time::Duration};

fn promotion_display() -> Display {
  Display::connect_with_clocked(
    |clock| {
      chess_rules::create_engine(EngineDependencies {
        position: Some(fixtures::promotion()),
        persistence: None,
        rules_worker: RulesWorker::default(),
        now: Rc::new(move || clock.now()),
        rng_seed: Some(43),
        opponent: Opponent::scripted(),
      })
    },
    catalog::assets(),
    Connect::new("test", "test", ScreenSize::new(1920, 1080)),
  )
}

#[test]
fn the_live_promotion_dialog_resumes_the_original_rules_call() {
  // The worker blocks inside choose(). A real displayed button supplies its typed
  // answer; its native event must resume that call and render the underpromotion.
  let mut display = promotion_display();
  assert_eq!(
    display.game_action::<ChessGame>(Duration::from_secs(5), |d| host::move_piece(
      d,
      Square::A7,
      Square::B8
    )),
    GameActionResult::AwaitingInput
  );
  // The pending drag can remain at its drop point; no promoted asset exists yet.
  assert!(
    !host::pieces(&display)
      .iter()
      .any(|(_, c, p)| *c == Color::White && *p == Piece::Knight)
  );
  let button = display
    .accessibility()
    .nodes
    .iter()
    .find(|n| n.label.as_deref() == Some("Knight"))
    .expect("visible Knight choice")
    .object_id;
  assert_eq!(
    display.game_action_presented::<ChessGame>(
      Duration::from_secs(5),
      |d| d.click_ui(button),
      |d| host::at(d, Square::B8) == vec![(Color::White, Piece::Knight)]
    ),
    GameActionResult::Completed
  );
  host::expect_empty(&display, Square::A7);
  host::expect_piece(&display, Square::B8, Color::White, Piece::Knight);
}

#[test]
fn resetting_a_waiting_dialog_discards_its_old_answer() {
  // Capture an event before replacing the session. Delivering that stale input
  // afterward must not promote a piece in the replacement board or revive a worker.
  let mut display = promotion_display();
  assert_eq!(
    display.game_action::<ChessGame>(Duration::from_secs(5), |d| host::move_piece(
      d,
      Square::A7,
      Square::B8
    )),
    GameActionResult::AwaitingInput
  );
  let button = display
    .accessibility()
    .nodes
    .iter()
    .find(|n| n.label.as_deref() == Some("Knight"))
    .unwrap()
    .object_id;
  for key in [
    PhysicalKey::ControlLeft,
    PhysicalKey::ShiftLeft,
    PhysicalKey::KeyR,
  ] {
    display.key_down(key);
  }
  for key in [
    PhysicalKey::KeyR,
    PhysicalKey::ShiftLeft,
    PhysicalKey::ControlLeft,
  ] {
    display.key_up(key);
  }
  assert_eq!(
    display.settle_game::<ChessGame>(Duration::from_secs(5)),
    GameActionResult::Completed
  );
  display.settle();
  display.deliver_ui_event(battlement::UiEvent {
    target_id: button,
    cancelable: true,
    default_prevented: false,
    body: UiEventBody::Click(ClickEvent::NavigationSubmit),
  });
  display.flush();
  host::expect_piece(&display, Square::A2, Color::White, Piece::Pawn);
  host::expect_piece(&display, Square::B8, Color::Black, Piece::Knight);
}

#[test]
fn the_real_zero_budget_opponent_has_a_deterministic_complete_board() {
  // Scripted replies cannot test the opponent itself. This separate worker test
  // exercises the production selector's zero-budget fallback twice, then compares
  // every displayed color and piece with an independently authored expected board.
  // Waiting is allowed in this integration lane and never enters ordinary helpers.
  for _ in 0..2 {
    let mut display = Display::connect_with_clocked(
      |clock| {
        chess_rules::create_engine(EngineDependencies {
          position: Some(fixtures::initial()),
          persistence: None,
          rules_worker: RulesWorker::default(),
          now: Rc::new(move || clock.now()),
          rng_seed: Some(43),
          opponent: Opponent::search(Duration::ZERO),
        })
      },
      catalog::assets(),
      Connect::new("test", "test", ScreenSize::new(1920, 1080)),
    );
    assert_eq!(
      display.game_action::<ChessGame>(Duration::from_secs(5), |d| {
        host::move_piece(d, Square::E2, Square::E4)
      }),
      GameActionResult::Completed
    );
    display.settle();
    display.flush();
    assert_eq!(
      display.settle_game::<ChessGame>(Duration::from_secs(5)),
      GameActionResult::Completed
    );
    display.settle();
    display.flush();
    let initial = fixtures::initial();
    for square in Square::ALL {
      let original = match square {
        Square::E4 => Square::E2,
        Square::A5 => Square::A7,
        _ => square,
      };
      if matches!(square, Square::E2 | Square::A7) {
        host::expect_empty(&display, square);
      } else if let Some(piece) = initial.piece_on(original) {
        host::expect_piece(&display, square, initial.color_on(original).unwrap(), piece);
      } else {
        host::expect_empty(&display, square);
      }
    }
    assert_eq!(host::pieces(&display).len(), 32);
  }
}
