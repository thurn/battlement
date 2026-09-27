use std::{boxed::Box as HeapBox, cell::RefCell, rc::Rc, time::Duration};

use battlement_fake::assets::FakeAssetCatalog;
use battlement_hearts_rules::{
  HeartsReducer,
  ai::{decision::Decision, search},
  domain::{HeartsState, Intention, RandomStream, Rejection, Seat},
  reducer::HeartsAction,
};
use reactant::{GameConsumer, GameOutput, GameStatus, ReducerDispatch, ReducerHandle, prelude::*};
use reactant_rules::ReducerGame;
use reactant_testing::Display;

const TIMEOUT: Duration = Duration::from_secs(10);
type Game = ReducerGame<HeartsReducer>;

#[derive(Clone, Default)]
struct Mount(Rc<RefCell<Option<ReducerHandle<HeartsReducer>>>>);

impl Component for Mount {
  fn render(&self) -> impl Render {
    let game = reactant::use_game_reducer((), || HeartsState::new(27), HeartsReducer);
    *self.0.borrow_mut() = Some(game);
    View::new()
  }
}

fn setup() -> (Display, ReducerHandle<HeartsReducer>, GameConsumer<Game>) {
  let mount = Mount::default();
  let component = mount.clone();
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("decision/scene");
  let mut display = Display::mount(
    move || Application::new("decision/scene").child(component.clone()),
    assets,
  );
  display.flush();
  let game = mount.0.borrow().as_ref().unwrap().clone();
  let consumer = display.with_engine(|engine| engine.game_consumer::<Game>());
  self::output(&consumer).submitted();
  assert_eq!(game.status(), GameStatus::Ready);
  (display, game, consumer)
}

fn output(consumer: &GameConsumer<Game>) -> GameOutput<Game> {
  assert!(consumer.wait_for_output(TIMEOUT));
  consumer.take_output().expect("output")
}

fn decision(game: &ReducerHandle<HeartsReducer>) -> Decision {
  let accepted = game.accepted();
  search::evaluate(
    accepted.state.observe(Seat::South),
    accepted.state.random().ai[0],
    || {},
  )
}

#[test]
fn stale_observation_stream_and_wrong_actor_reject_without_random_advancement() {
  let (_display, game, _consumer) = self::setup();
  let before = game.accepted();
  let valid = self::decision(&game);
  for kind in 0..3 {
    let mut decision = valid.clone();
    let expected = match kind {
      0 => {
        decision.observation.table.hand_index += 1;
        Rejection::StaleDecision
      }
      1 => {
        decision.previous_stream = RandomStream::from_seed(999);
        Rejection::StaleDecision
      }
      _ => {
        decision.intention = Intention::NextHand;
        Rejection::NotYourTurn
      }
    };
    assert_eq!(
      game.dispatch(
        before.version,
        HeartsAction::Computer(HeapBox::new(decision))
      ),
      ReducerDispatch::Rejected(expected)
    );
    assert_eq!(game.accepted().state, before.state);
    assert_eq!(game.accepted().version, before.version);
  }
}

#[test]
fn output_and_pre_acceptance_failure_preserve_rng_but_late_failure_keeps_accepted_rng() {
  for failure in 0..3 {
    let (_display, game, consumer) = self::setup();
    let before = game.accepted();
    let decision = self::decision(&game);
    let next = decision.next_stream;
    assert_eq!(
      game.dispatch(
        before.version,
        HeartsAction::Computer(HeapBox::new(decision))
      ),
      ReducerDispatch::Started
    );
    let event = self::output(&consumer);
    assert_eq!(game.accepted().state, before.state);
    assert_eq!(game.presented().state.random().ai[0], next);
    if failure == 0 {
      consumer.fail("output submission failure");
      event.submitted();
    } else {
      event.submitted();
      let final_output = self::output(&consumer);
      assert!(final_output.is_final());
      assert_eq!(game.accepted().state, before.state);
      if failure == 2 {
        final_output.submitted();
      }
      consumer.fail(if failure == 2 {
        "late playback failure"
      } else {
        "pre-acceptance failure"
      });
      final_output.submitted();
    }
    assert_eq!(game.status(), GameStatus::Failed);
    let accepted = game.accepted();
    if failure == 2 {
      assert_eq!(accepted.version.revision, before.version.revision + 1);
      assert_eq!(accepted.state.random().ai[0], next);
      assert!(accepted.state.observe(Seat::South).pending_pass.is_some());
      assert_eq!(accepted.state.random().deck, before.state.random().deck);
      assert_eq!(
        accepted.state.random().ai[1..],
        before.state.random().ai[1..]
      );
    } else {
      assert_eq!(accepted.state, before.state);
      assert_eq!(accepted.version, before.version);
    }
  }
}

#[test]
fn unmount_before_final_submission_discards_intention_and_rng_together() {
  let (display, game, consumer) = self::setup();
  let before = game.accepted();
  assert_eq!(
    game.dispatch(
      before.version,
      HeartsAction::Computer(HeapBox::new(self::decision(&game)))
    ),
    ReducerDispatch::Started
  );
  self::output(&consumer).submitted();
  let final_output = self::output(&consumer);
  assert!(final_output.is_final());
  drop(display);
  final_output.submitted();
  assert_eq!(game.accepted().state, before.state);
  assert_eq!(game.accepted().version, before.version);
}
