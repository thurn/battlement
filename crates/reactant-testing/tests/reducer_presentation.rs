#[path = "support/receipt_gate.rs"]
mod receipt_gate;

use std::{
  cell::{Cell, RefCell},
  rc::Rc,
  time::Duration,
};

use battlement::{
  CommandBody, MotionEventBatch, MotionPlaybackEvent, MotionPlaybackOutcome, MotionScopeCommand,
  MotionSequence, ObjectId, SessionId, object_id,
};
use battlement_fake::assets::FakeAssetCatalog;
use battlement_native::CoreClientMessageView;
use reactant::{
  ApplicationEngine, GameConsumer, GamePresentation, ReducerDispatch, ReducerHandle,
  animation_controls::{AnimationSequence, MotionSelector},
  host::ButtonHost,
  prelude::*,
};
use reactant_rules::{GameReducer, ReducerGame, ReducerOutput};
use reactant_testing::Display;
use receipt_gate::ReceiptGate;
use trox::ls;

const ROOT: ObjectId = object_id!("095c61bb-fbbf-4701-b96a-ec7e87c034b3");
const TIMEOUT: Duration = Duration::from_secs(10);

type TestDisplay = Display<ReceiptGate>;

type Consumer = GameConsumer<ReducerGame<Counter>>;

struct Counter;
#[derive(Clone, Copy)]
enum Event {
  Hold,
  Move,
  Ambient,
}

impl GameReducer for Counter {
  type State = usize;
  type Action = Event;
  type Event = Event;
  type Rejection = ();
  fn validate(_: &usize, _: &Event) -> Result<(), ()> {
    Ok(())
  }
  fn reduce(&mut self, state: &mut usize, event: Event, output: &mut ReducerOutput<Self>) {
    *state += 1;
    output.publish(state, || event);
  }
}

#[derive(Clone, Default)]
struct Mount {
  handle: Rc<RefCell<Option<ReducerHandle<Counter>>>>,
  controls: Rc<RefCell<Option<GamePresentation>>>,
  initialized: Rc<Cell<usize>>,
}

struct Board(Mount);
impl Component for Board {
  fn render(&self) -> impl Render {
    *self.0.controls.borrow_mut() = Some(reactant::use_game_presentation());
    let scope = reactant::animation_controls::use_animation_scope();
    let animation_scope = scope.clone();
    reactant::use_animate::<ReducerGame<Counter>>(move |event| {
      Some(match event {
        Event::Hold => SnapshotAnimation::wait(Duration::from_millis(200)),
        Event::Move | Event::Ambient => {
          let animation = SnapshotAnimation::sequence(
            animation_scope,
            AnimationSequence::new().animate(
              MotionSelector::name("card"),
              StyleTarget::new().x(20.0),
              Transition::tween()
                .duration_secs(0.2)
                .ease(Easing::Linear)
                .repeat(if matches!(event, Event::Ambient) {
                  Repeat::Forever
                } else {
                  Repeat::Count(0)
                }),
            ),
          );
          if matches!(event, Event::Ambient) {
            animation.nonblocking()
          } else {
            animation
          }
        }
      })
    });
    let receipt = reactant::use_game_presentation_receipt::<ReducerGame<Counter>>();
    View::new()
      .motion(MotionProps::new().animation_scope(scope))
      .child((
        View::new()
          .name("card")
          .style(Style::new().width(10.px()).height(10.px()))
          .motion(MotionProps::new().motion_name("card")),
        Label::new(ls(format!("{:?}", receipt.status))).name("receipt"),
      ))
  }
}
impl Component for Mount {
  fn render(&self) -> impl Render {
    let (key, replace) = reactant::hooks::use_state(0_u64);
    let (menu, update_menu) = reactant::hooks::use_state(0_usize);
    let initialized = self.initialized.clone();
    let game = reactant::use_game_reducer(
      key,
      move || {
        initialized.set(initialized.get() + 1);
        0
      },
      Counter,
    );
    *self.handle.borrow_mut() = Some(game.clone());
    (
      GameRoot::new(Board(self.clone())),
      Label::new(ls(menu.to_string())).name("menu-count"),
      ButtonHost::new(ls("Menu"))
        .name("menu")
        .on_click(move || update_menu.set(menu + 1)),
      ButtonHost::new(ls("Recover"))
        .name("recover")
        .on_click(move || {
          game.recover();
        }),
      ButtonHost::new(ls("Replace"))
        .name("replace")
        .on_click(move || replace.set(key + 1)),
    )
  }
}

fn setup() -> (TestDisplay, Mount, Consumer) {
  let mount = Mount::default();
  let component = mount.clone();
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("reducer/presentation");
  let mut display = Display::connect(
    ReceiptGate::new(ApplicationEngine::new(move || {
      Application::new("reducer/presentation")
        .child(component.clone())
        .document(|mut document| {
          document.root_id = ROOT;
          document
        })
    })),
    assets,
  );
  display.flush();
  let consumer = display.with_engine(|engine| engine.game_consumer::<ReducerGame<Counter>>());
  consumer.resume_automatic_submission();
  display.poll();
  display.flush();
  assert!(self::game(&mount).presentation().is_settled());
  (display, mount, consumer)
}
fn game(mount: &Mount) -> ReducerHandle<Counter> {
  mount.handle.borrow().as_ref().unwrap().clone()
}
fn submit_all(display: &mut TestDisplay, game: &ReducerHandle<Counter>, consumer: &Consumer) {
  for _ in 0..10 {
    if game.status() == GameStatus::Ready {
      return;
    }
    assert!(consumer.wait_for_output(TIMEOUT));
    display.poll();
  }
  panic!("final output was not submitted");
}

#[test]
fn native_wait_and_card_motion_settle_after_completion_and_resume_the_same_occurrence() {
  for event in [Event::Hold, Event::Move] {
    let (mut display, mount, consumer) = self::setup();
    let game = self::game(&mount);
    assert_eq!(
      game.dispatch(game.accepted().version, event),
      ReducerDispatch::Started
    );
    self::submit_all(&mut display, &game, &consumer);
    assert_eq!(*game.accepted().state, 1);
    assert_eq!(game.accepted().version.revision, 1);
    assert_eq!(game.presented().version, game.accepted().version);
    assert_eq!(game.presentation().status, PresentationStatus::Pending);
    assert_eq!(
      game.dispatch(game.accepted().version, Event::Hold),
      ReducerDispatch::Busy
    );
    let controls = mount.controls.borrow().as_ref().unwrap().clone();
    controls.pause();
    display.flush();
    display.click_ui(display.find_ui(ROOT, "menu"));
    display.flush();
    assert_eq!(
      display
        .ui_element(display.find_ui(ROOT, "menu-count"))
        .text(),
      Some("1")
    );
    display.advance(Duration::from_secs(2));
    assert_eq!(game.presentation().status, PresentationStatus::Pending);
    assert_eq!(game.accepted().version.revision, 1);
    controls.resume();
    display.flush();
    display.advance(Duration::from_millis(199));
    assert_eq!(game.presentation().status, PresentationStatus::Pending);
    display.advance(Duration::from_millis(1));
    display.flush();
    assert!(game.presentation().is_settled());
    assert_eq!(game.presentation().version, game.accepted().version);
    assert_eq!(
      display.ui_element(display.find_ui(ROOT, "receipt")).text(),
      Some("Settled")
    );
    assert_eq!(
      game.dispatch(game.accepted().version, Event::Hold),
      ReducerDispatch::Started
    );
  }
}

#[test]
fn nonblocking_ambience_does_not_hold_gameplay_and_replacement_cancels_old_receipts() {
  let (mut display, mount, consumer) = self::setup();
  let game = self::game(&mount);
  assert_eq!(
    game.dispatch(game.accepted().version, Event::Ambient),
    ReducerDispatch::Started
  );
  self::submit_all(&mut display, &game, &consumer);
  assert!(game.presentation().is_settled());
  assert_eq!(display.presentation_time(), Duration::ZERO);
  assert_eq!(
    game.dispatch(game.accepted().version, Event::Hold),
    ReducerDispatch::Started
  );
  self::submit_all(&mut display, &game, &consumer);
  assert_eq!(game.presentation().status, PresentationStatus::Pending);
  display.click_ui(display.find_ui(ROOT, "replace"));
  display.flush();
  let replacement = self::game(&mount);
  assert_ne!(
    replacement.accepted().version.session,
    game.accepted().version.session
  );
  assert_eq!(game.presentation().status, PresentationStatus::Cancelled);
  display.advance(Duration::from_secs(1));
  display.flush();
  assert!(replacement.presentation().is_settled());
  assert_eq!(*replacement.accepted().state, 0);
  assert_eq!(
    game.dispatch(game.accepted().version, Event::Hold),
    ReducerDispatch::Stale
  );
}

#[test]
fn recovery_keeps_the_accepted_revision_and_does_not_replay_prior_effects() {
  for accept_final in [false, true] {
    let (mut display, mount, _consumer) = self::setup();
    let game = self::game(&mount);
    let consumer = display.with_engine(|engine| engine.game_consumer::<ReducerGame<Counter>>());
    assert_eq!(
      game.dispatch(game.accepted().version, Event::Hold),
      ReducerDispatch::Started
    );
    assert!(consumer.wait_for_output(TIMEOUT));
    let intermediate = consumer.take_output().unwrap();
    display.poll();
    intermediate.submitted();
    display.poll();
    assert!(consumer.wait_for_output(TIMEOUT));
    let final_output = consumer.take_output().unwrap();
    assert!(final_output.is_final());
    assert!(consumer.wait_for_worker_stopped(TIMEOUT));
    if accept_final {
      final_output.submitted();
    }
    display.with_engine(|engine| engine.hold = true);
    display.advance(Duration::from_millis(200));
    display.with_engine(|engine| engine.fail_last());
    display.flush();
    final_output.submitted();
    assert_eq!(game.presentation().status, PresentationStatus::Failed);
    let accepted = game.accepted();
    assert_eq!(*accepted.state, usize::from(accept_final));
    assert_eq!(accepted.version.revision, u64::from(accept_final));
    display.with_engine(|engine| engine.hold = false);
    assert!(game.recover());
    assert!(!game.recover());
    display.flush();
    let replacement = self::game(&mount);
    assert_ne!(
      replacement.accepted().version.session,
      accepted.version.session
    );
    assert_eq!(
      replacement.accepted().version.revision,
      accepted.version.revision
    );
    assert_eq!(replacement.accepted().state, accepted.state);
    assert_eq!(mount.initialized.get(), 1);
    assert!(replacement.presentation().is_settled());
    assert!(replacement.diagnostic().is_none());
    display.advance(Duration::from_secs(1));
    display.flush();
    final_output.submitted();
    assert!(replacement.presentation().is_settled());
    assert!(!game.recover());
    assert_eq!(
      replacement.dispatch(accepted.version, Event::Hold),
      ReducerDispatch::Stale
    );
  }
}

#[test]
fn duplicate_wrong_session_and_ended_owner_receipts_cannot_settle_new_work() {
  let (mut display, mount, consumer) = self::setup();
  let game = self::game(&mount);
  display.with_engine(|engine| engine.hold = true);
  assert_eq!(
    game.dispatch(game.accepted().version, Event::Hold),
    ReducerDispatch::Started
  );
  self::submit_all(&mut display, &game, &consumer);
  display.advance(Duration::from_millis(200));
  assert_eq!(game.presentation().status, PresentationStatus::Pending);
  let old = display.with_engine(|engine| engine.receipts.clone());
  display.with_engine(|engine| engine.release_all());
  display.flush();
  assert!(game.presentation().is_settled());
  assert_eq!(
    game.dispatch(game.accepted().version, Event::Hold),
    ReducerDispatch::Started
  );
  self::submit_all(&mut display, &game, &consumer);
  display.advance(Duration::from_millis(200));
  display.with_engine(|engine| {
    for bytes in old {
      engine.replay(bytes);
    }
    let CoreClientMessageView::BatchCompleted(mut receipt) =
      CoreClientMessageView::read(engine.receipts.last().unwrap()).unwrap()
    else {
      unreachable!()
    };
    receipt.session_id = SessionId::new_v4();
    engine.replay(
      battlement_flatbuffers::write_core_batch_completed(&receipt)
        .unwrap()
        .as_bytes()
        .to_vec(),
    );
  });
  display.flush();
  assert_eq!(game.presentation().status, PresentationStatus::Pending);
  display.click_ui(display.find_ui(ROOT, "replace"));
  display.flush();
  let replacement = self::game(&mount);
  assert_eq!(game.presentation().status, PresentationStatus::Cancelled);
  assert_eq!(
    replacement.presentation().status,
    PresentationStatus::Pending
  );
  display.with_engine(|engine| engine.release_all());
  display.flush();
  assert!(replacement.presentation().is_settled());
  assert_eq!(*replacement.accepted().state, 0);
}

#[test]
fn interrupted_blocking_motion_fails_and_recovers_without_replaying_the_motion() {
  let (mut display, mount, consumer) = self::setup();
  let game = self::game(&mount);
  assert_eq!(
    game.dispatch(game.accepted().version, Event::Move),
    ReducerDispatch::Started
  );
  self::submit_all(&mut display, &game, &consumer);
  assert_eq!(game.presentation().status, PresentationStatus::Pending);
  let (playback_id, generation) = display
    .commands()
    .iter()
    .find_map(|entry| {
      let CommandBody::MotionScope(operation) = &entry.command.body else {
        return None;
      };
      let MotionScopeCommand::Start {
        playback_id,
        generation,
        ..
      } = operation.command
      else {
        return None;
      };
      Some((playback_id, generation))
    })
    .expect("recorded blocking card playback");
  display.deliver_motion_events(MotionEventBatch {
    first_sequence: MotionSequence(0),
    last_sequence: MotionSequence(0),
    events: Vec::new(),
    samples: Vec::new(),
    value_samples: Vec::new(),
    playback_events: vec![MotionPlaybackEvent {
      playback_id,
      generation,
      outcome: MotionPlaybackOutcome::Cancelled,
    }],
    label_events: Vec::new(),
    gesture_events: Vec::new(),
  });
  display.flush();
  assert_eq!(game.presentation().status, PresentationStatus::Failed);
  assert_eq!(*game.accepted().state, 1);
  assert_eq!(
    game.dispatch(game.accepted().version, Event::Hold),
    ReducerDispatch::Failed
  );
  assert!(game.recover());
  display.flush();
  let replacement = self::game(&mount);
  assert_eq!(*replacement.accepted().state, 1);
  assert_eq!(replacement.accepted().version.revision, 1);
  assert!(replacement.presentation().is_settled());
  assert_eq!(display.presentation_time(), Duration::ZERO);
}
