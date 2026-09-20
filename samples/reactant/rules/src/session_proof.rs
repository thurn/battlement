use std::borrow::Cow;

use reactant::{
  GameHandle,
  prelude::*,
  rules::{ChoiceOwner, ChoicePolicy, ExecutionMode, Game as RulesGame, PromptData},
};
use trox::ls;

use crate::ROOT_ID;

struct Counter;
struct Policy;
#[derive(Clone)]
struct Number;
#[derive(Clone, Copy)]
enum Action {
  Choose,
  Fail,
}
enum Prompt<'a> {
  Number(Cow<'a, Number>),
}
struct Proof {
  game: GameHandle<Counter>,
}
struct Screen(Proof);
struct Root;

pub(crate) fn app() -> crate::ReactantApplication {
  reactant::Application::new(crate::CONTENT_SCENE)
    .child(Root)
    .document(|mut document| {
      document.root_id = ROOT_ID;
      document
    })
}

impl Component for Root {
  fn render(&self) -> impl Render {
    let game = reactant::use_game::<Counter, _>((), 0, |connection| ExecutionMode::Interactive {
      connection,
      policy: Policy,
    });
    View::new()
      .style(
        Style::new()
          .padding(32.px())
          .background_color(Color::rgb(0.05, 0.07, 0.11))
          .color(Color::rgb(0.95, 0.95, 1.0)),
      )
      .child((
        Label::new(ls("Rules session")).style(
          Style::new()
            .font_size(30.px())
            .color(Color::rgb(0.95, 0.95, 1.0)),
        ),
        Menu,
        GameRoot::new(Screen(Proof { game })),
      ))
  }
}
struct Menu;
impl Component for Menu {
  fn render(&self) -> impl Render {
    let (open, set_open) = reactant::hooks::use_state(false);
    View::new().child((
      Button::new(ls("Settings")).on_press(move || set_open.set(!open)),
      Label::new(ls(if open {
        "Settings open"
      } else {
        "Settings closed"
      }))
      .name("session-menu-status"),
    ))
  }
}
impl Component for Screen {
  fn render(&self) -> impl Render {
    let state = reactant::use_game_state::<Counter>();
    let status = reactant::use_game_status::<Counter>();
    let prompt = reactant::use_game_prompt::<Counter>();
    let dispatch = self.0.game.clone();
    View::new().child((
      Label::new(ls(format!(
        "Rendered {} / Accepted {} / {status:?}",
        *state,
        self.0.game.accepted_state()
      )))
      .name("session-status")
      .style(
        Style::new()
          .font_size(24.px())
          .color(Color::rgb(1.0, 1.0, 1.0)),
      ),
      Button::new(ls("Choose number")).on_press(move || {
        dispatch.dispatch(Action::Choose);
      }),
      prompt.map(|prompt| {
        Button::new(ls("Answer five"))
          .key(prompt.handle.clone())
          .on_press(move || {
            let Prompt::Number(number) = &prompt.prompt;
            prompt.handle.submit(number.as_ref(), 5);
          })
      }),
      Button::new(ls("Fail gameplay host")).on_press({
        let game = self.0.game.clone();
        move || {
          game.dispatch(Action::Fail);
        }
      }),
      (status == reactant::GameStatus::Failed).then(|| {
        Label::new(ls("Recovery available: accepted state retained")).name("session-recovery")
      }),
    ))
  }
}
impl PromptData<Counter> for Number {
  type ResponseType = u32;
  fn options(&self) -> impl Iterator<Item = u32> {
    [5].into_iter()
  }
  fn is_valid_response(&self, response: &u32) -> bool {
    *response == 5
  }
  fn as_prompt(&self) -> Prompt<'_> {
    Prompt::Number(Cow::Borrowed(self))
  }
  fn into_prompt(self) -> Prompt<'static> {
    Prompt::Number(Cow::Owned(self))
  }
}
impl ChoicePolicy<Counter> for Policy {
  fn owner(&self, _: &u32, _: &Prompt<'_>) -> ChoiceOwner {
    ChoiceOwner::Human
  }
  fn choose(&mut self, _: &u32, _: &Prompt<'_>) -> usize {
    0
  }
}
impl RulesGame for Counter {
  type State = u32;
  type Action = Action;
  type StateAnimation = ();
  type Prompt<'a> = Prompt<'a>;
  type Context = ExecutionMode<Self, Policy>;
  fn logical_clone(state: &u32) -> u32 {
    *state
  }
  fn is_legal_action(_: &u32, _: &Action) -> bool {
    true
  }
  fn execute(cx: &mut Self::Context, state: &mut u32, action: Action) {
    match action {
      Action::Choose => *state += cx.choose(state, Number),
      Action::Fail => panic!("Native fixture host failure"),
    }
  }
}
