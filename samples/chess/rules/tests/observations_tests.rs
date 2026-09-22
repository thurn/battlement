//! Tests of the teaching harness itself: representative broken renderers must not
//! make the game assertions pass. These scenes deliberately contain no chess rules;
//! deriving expected pieces from a rules board would make this check circular.
mod support;
use crate::support::{catalog, fixtures, game::ChessTest, host};
use battlement::{Connect, DragMode, ParentScene, PrefabAddress, Quaternion, ScreenSize, Vector3};
use chess_rules::assets;
use cozy_chess::{Color, Piece, Square};
use reactant::{
  Application, ApplicationEngine,
  rules::RulesWorker,
  world::{BoxHitRegion, Camera, Group, Prefab, SceneRoot},
};
use reactant_testing::{Display, InlineActionResult};
use std::panic::{self, AssertUnwindSafe};

fn rendered(addresses: Vec<PrefabAddress>, offset: Vector3, wrapped: bool) -> Display {
  Display::connect_inline(
    move |clock| {
      ApplicationEngine::with_clock(
        move || {
          let pieces = addresses
            .iter()
            .map(|address| {
              // Every mount allocates fresh IDs. The optional wrapper contributes to the
              // world transform, so the oracle must compose ancestors rather than read
              // a local position or demand that a hit region directly parent the prefab.
              Group::new()
                .position(if wrapped { Vector3::ZERO } else { offset })
                .child(Prefab::at(address.clone()))
            })
            .collect::<Vec<_>>();
          Application::new(assets::CONTENT)
            .rules_worker(RulesWorker::inline())
            .child(
              SceneRoot::new(ParentScene::PrimaryScene).child(
                Group::new()
                  .position(if wrapped { offset } else { Vector3::ZERO })
                  .child(pieces),
              ),
            )
        },
        move || clock.now(),
      )
    },
    catalog::assets(),
    Connect::new("test", "test", ScreenSize::new(1920, 1080)),
  )
}

#[test]
fn observations_ignore_identity_and_harmless_wrappers() {
  // Both physical trees display exactly the same pawn. A test tied to stable IDs
  // or direct ancestry would fail one, despite no player-visible difference.
  for wrapped in [false, true] {
    let display = rendered(vec![assets::white::PAWN], host::center(Square::E4), wrapped);
    host::expect_piece(&display, Square::E4, Color::White, Piece::Pawn);
  }
}

#[test]
fn observations_reject_wrong_color_surviving_victims_and_wrong_positions() {
  // Each deliberate defect must fail the *same* assertion. Catching its panic is
  // intentional here; ordinary scenarios never catch failures or change the oracle
  // to fit actual output. A surviving victim makes the destination ambiguous.
  for (addresses, position) in [
    (vec![assets::black::PAWN], host::center(Square::E4)),
    (
      vec![assets::white::PAWN, assets::black::BISHOP],
      host::center(Square::E4),
    ),
    (vec![assets::white::PAWN], host::center(Square::E3)),
    (vec![], host::center(Square::E4)),
  ] {
    let display = rendered(addresses, position, true);
    assert!(
      panic::catch_unwind(AssertUnwindSafe(|| {
        host::expect_piece(&display, Square::E4, Color::White, Piece::Pawn);
      }))
      .is_err()
    );
  }
}

#[test]
fn successful_move_helper_rejects_input_blocked_by_a_menu() {
  // The menu leaves a visible pawn behind it, but its input policy rejects moving.
  // A helper wired directly to rules dispatch would incorrectly pass this test.
  let mut game = ChessTest::from_position(fixtures::initial());
  game.pause();
  assert!(panic::catch_unwind(AssertUnwindSafe(|| game.play(Square::E2, Square::E4))).is_err());
  game.expect_piece(Square::E2, Color::White, Piece::Pawn);
}

#[test]
fn unused_promotion_answers_and_unadmitted_opponent_replies_fail_immediately() {
  // A script is an expectation, not optional configuration. Supplying a promotion
  // answer for an ordinary pawn move must fail even though the board move succeeds.
  // Likewise a reply on White's turn must fail without waiting for the coordinator.
  let mut game = ChessTest::from_position(fixtures::initial());
  assert!(
    panic::catch_unwind(AssertUnwindSafe(|| {
      game.promote(Square::E2, Square::E4, Piece::Knight);
    }))
    .is_err()
  );
  let mut game = ChessTest::from_position(fixtures::initial());
  assert!(
    panic::catch_unwind(AssertUnwindSafe(|| {
      game.reply(Square::E7, Square::E5);
    }))
    .is_err()
  );
  game.expect_piece(Square::E7, Color::Black, Piece::Pawn);
}

#[test]
fn a_disconnected_drop_callback_cannot_masquerade_as_a_completed_move() {
  // Native dragging updates a transform before rules run. This deliberately broken
  // component has a real draggable piece but no drop callback: the pawn reaches e4
  // visually, yet the driver must return NoAction. Admission is orchestration
  // evidence, never a replacement for the independent rendered-piece assertions.
  let mut display = Display::connect_inline(
    |clock| {
      ApplicationEngine::with_clock(
        || {
          Application::new(assets::CONTENT)
            .rules_worker(RulesWorker::inline())
            .camera(|camera| {
              Camera::new()
                .perspective(60.0)
                .position(Vector3::new(0.0, 8.0, -3.75))
                .rotation(Quaternion::new(
                  0.58184814,
                  -0.001219943,
                  0.0008727778,
                  0.813296,
                ))
                .into_object(camera.object_id)
            })
            .child(
              SceneRoot::new(ParentScene::PrimaryScene).child(
                BoxHitRegion::new()
                  .size(Vector3::new(0.9, 1.5, 0.9))
                  .position(host::center(Square::E2))
                  .draggable(DragMode::SnapToPointer)
                  .child(Prefab::at(assets::white::PAWN)),
              ),
            )
        },
        move || clock.now(),
      )
    },
    catalog::assets(),
    Connect::new("test", "test", ScreenSize::new(1920, 1080)),
  );
  assert_eq!(
    display.action_inline(|d| host::move_piece(d, Square::E2, Square::E4)),
    InlineActionResult::NoAction
  );
  host::expect_piece(&display, Square::E4, Color::White, Piece::Pawn);
}
