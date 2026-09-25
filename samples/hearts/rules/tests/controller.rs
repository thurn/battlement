use std::{
  cell::RefCell,
  rc::Rc,
  sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
    mpsc::{self, Receiver, Sender},
  },
  time::Duration,
};

use battlement::{ObjectId, object_id};
use battlement_fake::assets::FakeAssetCatalog;
use battlement_hearts_rules::{
  HeartsController, HeartsReducer, controller,
  domain::{AiObservation, Event, HeartsState, Phase, Seat},
};
use reactant::{GameStatus, ReducerDispatch, TaskState, host::ButtonHost, prelude::*};
use reactant_rules::ReducerGame;
use reactant_testing::Display;
use trox::ls;

const ROOT: ObjectId = object_id!("ae78d34b-e946-4b6d-9402-8df86b5d16ec");
const TIMEOUT: Duration = Duration::from_secs(10);

type Game = ReducerGame<HeartsReducer>;

struct Probe {
  calls: AtomicUsize,
  started: Sender<AiObservation>,
  first: Mutex<Receiver<()>>,
  hold_first: bool,
}

#[derive(Clone)]
struct Fixture {
  controller: Rc<RefCell<Option<HeartsController>>>,
  probe: Arc<Probe>,
  delay_pass: bool,
}
struct Board(bool);

impl Component for Board {
  fn render(&self) -> impl Render {
    reactant::use_animate::<Game>(|event| {
      (self.0 && matches!(event, Event::PassSubmitted { seat: Seat::South }))
        .then(|| SnapshotAnimation::wait(Duration::from_millis(200)))
    });
    Label::new(ls("Hearts"))
  }
}

impl Component for Fixture {
  fn render(&self) -> impl Render {
    let (key, replace) = reactant::hooks::use_state(0_u64);
    let (paused, pause) = reactant::hooks::use_state(false);
    let (menu, count) = reactant::hooks::use_state(0_usize);
    let probe = Arc::clone(&self.probe);
    let controller = controller::use_hearts_with_policy(
      key,
      move || HeartsState::new(71 + key),
      Seat::South,
      paused,
      move |observation, token| {
        let call = probe.calls.fetch_add(1, Ordering::SeqCst);
        probe.started.send(observation.clone()).unwrap();
        if probe.hold_first && call == 0 {
          probe.first.lock().unwrap().recv_timeout(TIMEOUT).unwrap();
        }
        token.checkpoint();
        controller::choose_legal(observation, token)
      },
    );
    let phase = format!("{:?}", controller.view.table.phase);
    *self.controller.borrow_mut() = Some(controller);
    (
      GameRoot::new(Board(self.delay_pass)),
      Label::new(ls(phase)).name("phase"),
      Label::new(ls(menu.to_string())).name("menu-count"),
      ButtonHost::new(ls("Menu"))
        .name("menu")
        .on_click(move || count.set(menu + 1)),
      ButtonHost::new(ls("Pause"))
        .name("pause")
        .on_click(move || pause.set(!paused)),
      ButtonHost::new(ls("Replace"))
        .name("replace")
        .on_click(move || replace.set(key + 1)),
    )
  }
}

fn setup(
  hold_first: bool,
  delay_pass: bool,
) -> (Display, Fixture, Receiver<AiObservation>, Sender<()>) {
  let (started, starts) = mpsc::channel();
  let (release, first) = mpsc::channel();
  let fixture = Fixture {
    controller: Rc::default(),
    probe: Arc::new(Probe {
      calls: AtomicUsize::new(0),
      started,
      first: Mutex::new(first),
      hold_first,
    }),
    delay_pass,
  };
  let component = fixture.clone();
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("hearts/controller");
  let mut display = Display::mount(
    move || {
      Application::new("hearts/controller")
        .child(component.clone())
        .document(|mut document| {
          document.root_id = ROOT;
          document
        })
    },
    assets,
  );
  display.flush();
  self::rules(&mut display, &self::current(&fixture));
  (display, fixture, starts, release)
}
fn current(fixture: &Fixture) -> HeartsController {
  fixture.controller.borrow().as_ref().unwrap().clone()
}
fn click(display: &mut Display, name: &str) {
  display.click_ui(display.find_ui(ROOT, name));
  display.flush();
}
fn rules(display: &mut Display, game: &HeartsController) {
  for _ in 0..20 {
    display.flush();
    if display.with_engine(|engine| engine.has_ready_changes()) {
      continue;
    }
    if game.game.status() != GameStatus::Busy {
      return;
    }
    assert!(display.wait_for_game_output::<Game>(TIMEOUT));
    display.poll();
  }
  panic!("bounded action failed to complete");
}

#[test]
fn scripted_hand_uses_shared_rules_and_observation_only_opponents() {
  let (mut display, fixture, _starts, _release) = self::setup(false, false);
  let mut human_actions = 0;
  for _ in 0..300 {
    display.flush();
    let game = self::current(&fixture);
    if game.game.status() == GameStatus::Busy {
      assert!(display.wait_for_game_output::<Game>(TIMEOUT));
      display.poll();
      continue;
    }
    match game.view.table.phase {
      Phase::HandOver => {
        assert_eq!(game.view.table.history.len(), 52);
        assert_eq!(game.view.captured.iter().map(Vec::len).sum::<usize>(), 52);
        assert_eq!(human_actions, 14);
        assert!(game.game.presentation().is_settled());
        assert_eq!(game.game.accepted().version.revision, 56);
        assert_eq!(display.presentation_time(), Duration::ZERO);
        return;
      }
      Phase::Passing if game.view.pending_pass.is_none() => {
        let pass = game.view.hands[Seat::South.index()]
          .iter()
          .take(3)
          .map(|card| card.token)
          .collect::<Vec<_>>();
        assert_eq!(game.pass(&pass), ReducerDispatch::Started);
        human_actions += 1;
      }
      Phase::Playing { turn: Seat::South } => {
        assert_eq!(
          game.play(game.view.legal_plays[0]),
          ReducerDispatch::Started
        );
        human_actions += 1;
      }
      _ => {
        assert!(reactant::testing::wait_for_task(&game.computer, TIMEOUT));
        display.flush();
      }
    }
  }
  panic!("scripted hand did not finish");
}

#[test]
fn secret_pass_keeps_same_ai_work_and_ready_result_waits_for_native_presentation() {
  let (mut display, fixture, starts, release) = self::setup(true, true);
  let observation = starts.recv_timeout(TIMEOUT).unwrap();
  assert_eq!(observation.seat, Seat::West);
  let initial = self::current(&fixture);
  let pass = initial.view.hands[Seat::South.index()]
    .iter()
    .take(3)
    .map(|card| card.token)
    .collect::<Vec<_>>();
  assert_eq!(initial.pass(&pass), ReducerDispatch::Started);
  self::rules(&mut display, &initial);
  let accepted = initial.game.accepted();
  assert_eq!(accepted.version.revision, 1);
  assert_eq!(accepted.state.observe(Seat::West), observation);
  assert!(!initial.game.presentation().is_settled());
  assert_eq!(fixture.probe.calls.load(Ordering::SeqCst), 1);
  assert!(matches!(initial.computer.state(), TaskState::Pending));
  self::click(&mut display, "menu");
  assert_eq!(
    display
      .ui_element(display.find_ui(ROOT, "menu-count"))
      .text(),
    Some("1")
  );
  assert_eq!(fixture.probe.calls.load(Ordering::SeqCst), 1);
  release.send(()).unwrap();
  assert!(reactant::testing::wait_for_task(&initial.computer, TIMEOUT));
  display.flush();
  assert!(matches!(initial.computer.state(), TaskState::Ready(_)));
  assert_eq!(initial.game.accepted().version.revision, 1);
  display.advance(Duration::from_millis(199));
  assert_eq!(initial.game.accepted().version.revision, 1);
  display.advance(Duration::from_millis(1));
  self::rules(&mut display, &initial);
  assert!(
    initial
      .game
      .accepted()
      .state
      .observe(Seat::West)
      .pending_pass
      .is_some()
  );
  assert!(initial.game.accepted().version.revision >= 2);
  assert_eq!(initial.pass(&pass), ReducerDispatch::Stale);
}

#[test]
fn pause_and_session_replacement_discard_in_flight_opponent_results() {
  let (mut display, fixture, starts, release) = self::setup(true, false);
  starts.recv_timeout(TIMEOUT).unwrap();
  let old = self::current(&fixture);
  self::click(&mut display, "pause");
  assert!(matches!(old.computer.state(), TaskState::Idle));
  let paused = self::current(&fixture);
  assert!(matches!(paused.computer.state(), TaskState::Idle));
  self::click(&mut display, "replace");
  let replacement = self::current(&fixture);
  assert_ne!(
    replacement.game.accepted().version.session,
    old.game.accepted().version.session
  );
  release.send(()).unwrap();
  display.flush();
  assert_eq!(replacement.game.accepted().version.revision, 0);
  assert_eq!(fixture.probe.calls.load(Ordering::SeqCst), 1);
  self::click(&mut display, "pause");
  assert_eq!(starts.recv_timeout(TIMEOUT).unwrap().seat, Seat::West);
  let resumed = self::current(&fixture);
  assert!(reactant::testing::wait_for_task(&resumed.computer, TIMEOUT));
  self::rules(&mut display, &resumed);
  assert!(matches!(old.computer.state(), TaskState::Idle));
  assert!(
    resumed
      .game
      .accepted()
      .state
      .observe(Seat::West)
      .pending_pass
      .is_some(),
    "accepted={:?}, status={:?}, receipt={:?}, task={:?}",
    resumed.game.accepted().version,
    resumed.game.status(),
    resumed.game.presentation(),
    resumed.computer.state()
  );
}
