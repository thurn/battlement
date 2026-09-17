use std::{borrow::Cow, cell::RefCell, rc::Rc, time::Duration};

use reactant::{
  GameConsumer, GameHandle, GameOutput,
  app::App,
  prelude::*,
  rules::{ChoiceOwner, ChoicePolicy, ExecutionMode, Game as RulesGame, PromptData},
};
use trox::ls;

use crate::{CONTENT_SCENE, Game, ROOT_ID, model};

struct Counter;
struct Policy;
#[derive(Clone)]
struct Number;
enum Prompt<'a> {
  Number(Cow<'a, Number>),
}
struct Proof {
  game: GameHandle<Counter>,
  consumer: GameConsumer<Counter>,
  held: RefCell<Option<GameOutput<Counter>>>,
}
struct Screen(Rc<Proof>);

pub(crate) fn app() -> App<Game> {
  let mut app = App::with_model(CONTENT_SCENE, model::new());
  let game = app.start_game::<Counter>(0, |connection| ExecutionMode::Interactive {
    connection,
    policy: Policy,
  });
  let proof = Rc::new(Proof {
    game,
    consumer: app.game_consumer::<Counter>(),
    held: RefCell::new(None),
  });
  app
    .ui(
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
          GameRoot::new(Screen(proof)),
        )),
    )
    .document(|mut document| {
      document.root_id = ROOT_ID;
      document
    })
}
struct Menu;
impl Component for Menu {
  fn render(&self) -> impl Render {
    let (open, set_open) = reactant::hooks::use_state(false);
    View::new().child((
      Button::new(ls("Settings")).on_press(move |_: &mut Game| set_open.set(!open)),
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
    let submit = self.0.clone();
    let dispatch = self.0.clone();
    let fail = self.0.clone();
    let answer = self.0.clone();
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
      Button::new(ls("Submit output")).on_press(move |_: &mut Game| {
        let output = submit
          .held
          .borrow_mut()
          .take()
          .unwrap_or_else(|| submit.next());
        output.submitted();
      }),
      Button::new(ls("Choose number")).on_press(move |_: &mut Game| {
        if dispatch.game.dispatch(()) == reactant::DispatchResult::Started {
          dispatch.next().submitted();
        }
      }),
      prompt.map(|prompt| {
        Button::new(ls("Answer five"))
          .key(prompt.handle.clone())
          .on_press(move |_: &mut Game| {
            let Prompt::Number(number) = &prompt.prompt;
            prompt.handle.submit(number.as_ref(), 5);
            *answer.held.borrow_mut() = Some(answer.next());
          })
      }),
      Button::new(ls("Fail gameplay host"))
        .on_press(move |_: &mut Game| fail.consumer.fail("Native fixture host failure")),
      (status == reactant::GameStatus::Failed).then(|| {
        Label::new(ls("Recovery available: accepted state retained")).name("session-recovery")
      }),
    ))
  }
}
impl Proof {
  fn next(&self) -> GameOutput<Counter> {
    assert!(
      self.consumer.wait_for_output(Duration::from_secs(5)),
      "fixture publication timed out"
    );
    self.consumer.take_output().expect("fixture output")
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
  type Action = ();
  type StateAnimation = ();
  type Prompt<'a> = Prompt<'a>;
  type Context = ExecutionMode<Self, Policy>;
  fn logical_clone(state: &u32) -> u32 {
    *state
  }
  fn is_legal_action(_: &u32, _: &()) -> bool {
    true
  }
  fn execute(cx: &mut Self::Context, state: &mut u32, _: ()) {
    *state += cx.choose(state, Number);
  }
}
