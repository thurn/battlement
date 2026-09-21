use std::{cell::Cell, rc::Rc, time::Duration};

use battlement::{Command, CommandBody, WaitPayload};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{app_context, hooks, host::ButtonHost, prelude::*, testing::App, testing::GameApp};
use reactant_rules::{ChoiceOwner, ChoicePolicy, DisplayConnection, ExecutionMode, Game};
use reactant_testing::Display;
use trox::ls;

#[derive(Clone, Default)]
struct State {
  score: u32,
  hand: u32,
}
struct Scores;
struct Policy;
struct Context(ExecutionMode<Scores, Policy>);
impl ChoicePolicy<Scores> for Policy {
  fn owner(&self, _: &State, _: &()) -> ChoiceOwner {
    ChoiceOwner::Policy
  }
  fn choose(&mut self, _: &State, _: &()) -> usize {
    unreachable!()
  }
}
impl Game for Scores {
  type State = State;
  type Action = bool;
  type StateAnimation = ();
  type Prompt<'a> = ();
  type Context = Context;
  fn logical_clone(state: &State) -> State {
    state.clone()
  }
  fn is_legal_action(_: &State, _: &bool) -> bool {
    true
  }
  fn execute(context: &mut Context, state: &mut State, score: bool) {
    if score {
      state.score += 1;
    } else {
      state.hand += 1;
    }
    context.0.present(state, || ());
  }
}
fn context(connection: DisplayConnection<Scores>) -> Context {
  Context(ExecutionMode::Interactive {
    connection,
    policy: Policy,
  })
}
#[derive(PartialEq)]
struct Score(Rc<Cell<u32>>);
impl Component for Score {
  fn render(&self) -> impl Render {
    self.0.set(self.0.get() + 1);
    let score = reactant::use_game_selector::<Scores, _>(|state| state.score);
    score_label(score)
  }
}
fn score_label(score: u32) -> impl Render {
  Label::new(ls(score.to_string())).name("score")
}
struct Props;
impl Component for Props {
  fn render(&self) -> impl Render {
    score_label(reactant::use_game_state::<Scores>().score)
  }
}
struct Hand;
impl Component for Hand {
  fn render(&self) -> impl Render {
    let hand = reactant::use_game_selector_with::<Scores, _>(|state| state.hand, |a, b| a == b);
    let app = app_context::use_app();
    hooks::use_effect(
      move || {
        if hand > 0 {
          app.send(Command::new_v4(CommandBody::TimeWait(WaitPayload {
            duration_ms: 200,
          })));
        }
      },
      hand,
    );
    Label::new(ls(hand.to_string())).name("hand")
  }
}
struct Gameplay(DisplayStore<u32>);
impl Component for Gameplay {
  fn render(&self) -> impl Render {
    let local = use_external_store(self.0.clone());
    Label::new(ls(local.to_string())).name("local")
  }
}
struct Menu(DisplayStore<u32>);
impl Component for Menu {
  fn render(&self) -> impl Render {
    let count = use_external_store(self.0.clone());
    let store = self.0.clone();
    (
      ButtonHost::new(ls("Menu"))
        .name("menu")
        .on_click(move || store.update(|value| *value += 1)),
      Label::new(ls(count.to_string())).name("menu-count"),
    )
  }
}

#[test]
fn selectors_and_props_show_the_same_state_without_overtaking_queued_gameplay() {
  for props in [false, true] {
    let renders = Rc::new(Cell::new(0));
    let local = DisplayStore::new(0);
    let score = if props {
      Node::new(Props)
    } else {
      Node::new(reactant::component::memo(Score(renders.clone())))
    };
    let mut app = App::new("selectors/scene").ui((
      Menu(DisplayStore::new(0)),
      GameRoot::new((Hand, score, Gameplay(local.clone()))),
    ));
    let game = app.start_game::<Scores>(State::default(), context);
    let consumer = app.game_consumer::<Scores>();
    consumer.resume_automatic_submission();
    let root = app.root_document().root_id;
    let mut assets = FakeAssetCatalog::new();
    assets.add_scene("selectors/scene");
    let mut display = Display::connect(app, assets);
    display.poll();
    let initial = renders.get();
    for score in [false, true] {
      game.dispatch(score);
      assert!(consumer.wait_for_worker_stopped(Duration::from_secs(5)));
      for _ in 0..12 {
        if game.status() == GameStatus::Ready {
          break;
        }
        assert!(consumer.wait_for_output(Duration::from_secs(5)));
        display.poll();
      }
      assert_eq!(game.status(), GameStatus::Ready);
      if !score {
        assert_eq!(renders.get(), initial);
      }
    }
    assert_eq!(game.accepted_state().score, 1);
    let score = display.find_ui(root, "score");
    assert_eq!(display.ui_element(score).text(), Some("0"));
    local.set(1);
    display.poll();
    display.click_ui(display.find_ui(root, "menu"));
    display.poll();
    assert_eq!(
      display
        .ui_element(display.find_ui(root, "menu-count"))
        .text(),
      Some("1")
    );
    assert_eq!(
      display.ui_element(display.find_ui(root, "local")).text(),
      Some("0")
    );
    assert_eq!(display.ui_element(score).text(), Some("0"));
    display.settle();
    assert_eq!(display.ui_element(score).text(), Some("1"));
    assert_eq!(
      display.ui_element(display.find_ui(root, "local")).text(),
      Some("1")
    );
    if !props {
      assert_eq!(renders.get(), initial + 1);
    }
    assert_eq!(display.frame(), 0);
  }
}
