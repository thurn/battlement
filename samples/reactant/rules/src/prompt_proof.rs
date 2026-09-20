use std::{borrow::Cow, rc::Rc};

use battlement::{
  Command, CommandBody, GameObject, GameObjectKind, MaterialAssignment, ObjectId, ParentScene,
  PropertyCommand, Tween, TweenPositionPayload, Vector3, object_id,
};
use reactant::{
  GameHandle,
  prelude::*,
  rules::{ChoiceOwner, ChoicePolicy, ExecutionMode, Game as RulesGame, PromptData},
};
use trox::ls;

use crate::{MOTION_MATERIAL, ROOT_ID};

const PROMPT_CUBE: ObjectId = object_id!("25300000-0000-4000-8000-000000000093");

struct Decisions;
struct Policy;
#[derive(Clone)]
struct Confirm;
enum Prompt<'a> {
  Confirm(Cow<'a, Confirm>),
}
struct Proof {
  game: GameHandle<Decisions>,
}
struct Root;
struct Menu(Rc<Proof>);
struct AnswerFromApp;
struct Board;

pub(crate) fn app() -> crate::ReactantApplication {
  reactant::Application::new(crate::CONTENT_SCENE)
    .child(Root)
    .document(|mut document| {
      document.root_id = ROOT_ID;
      document
    })
    .camera(|camera| camera.position(Vector3::new(0.0, 0.0, -10.0)))
    .object(
      GameObject::new(
        PROMPT_CUBE,
        GameObjectKind::Cube {
          materials: vec![MaterialAssignment::new(0, MOTION_MATERIAL)],
        },
      )
      .parent_scene(ParentScene::Persistent)
      .position(Vector3::new(-3.0, -2.0, 0.0)),
    )
}

impl Component for Root {
  fn render(&self) -> impl Render {
    let game = reactant::use_game::<Decisions, _>((), 0, |connection| ExecutionMode::Interactive {
      connection,
      policy: Policy,
    });
    let proof = Rc::new(Proof { game });
    View::new()
      .style(
        Style::new()
          .padding(32.px())
          .background_color(Color::rgb(0.05, 0.07, 0.11))
          .color(Color::rgb(0.95, 0.95, 1.0)),
      )
      .child((
        Label::new(ls("Queued prompt input")).style(Style::new().font_size(30.px())),
        Menu(proof.clone()),
        GameRoot::new(Board),
      ))
  }
}

impl Component for Menu {
  fn render(&self) -> impl Render {
    let status = self.0.game.status();
    let (open, set_open) = reactant::hooks::use_state(false);
    let start = self.0.clone();
    let stop = self.0.clone();
    View::new().child((
      Label::new(ls(format!(
        "Rules {status:?} / accepted {}",
        self.0.game.accepted_state()
      ))),
      Button::new(ls("Begin prompts")).on_press(move || {
        start.game.dispatch(());
      }),
      GameRoot::new(AnswerFromApp),
      Button::new(ls("Settings")).on_press(move || set_open.set(!open)),
      Button::new(ls("Stop prompts")).on_press(move || stop.game.stop()),
      Label::new(ls(if open {
        "Settings open"
      } else {
        "Settings closed"
      })),
    ))
  }
}

impl Component for AnswerFromApp {
  fn render(&self) -> impl Render {
    let prompt = reactant::use_game_prompt::<Decisions>();
    Button::new(ls("Answer from app")).on_press(move || {
      if let Some(prompt) = &prompt {
        let Prompt::Confirm(value) = &prompt.prompt;
        prompt.handle.submit(value.as_ref(), 1);
      }
    })
  }
}

impl Component for Board {
  fn render(&self) -> impl Render {
    let state = *reactant::use_game_state::<Decisions>();
    let prompt = reactant::use_game_prompt::<Decisions>();
    let app = reactant::app_context::use_app();
    let offset = use_motion_value(0.0_f32);
    let animate_offset = offset.clone();
    let waiting = prompt.is_some();
    reactant::hooks::use_effect(
      move || {
        if waiting {
          app.send(Command::new_v4(CommandBody::TransformTweenLocalPosition(
            PropertyCommand::canceling(TweenPositionPayload {
              object_id: PROMPT_CUBE,
              position: Vector3::new(state as f64 - 2.0, -2.0, 0.0),
              tween: Tween::new().duration_ms(2000),
            }),
          )));
          animate_offset.animate(
            (state + 1) as f32 * 70.0,
            Transition::tween().duration_secs(2.0),
          );
        }
      },
      prompt.as_ref().map(|prompt| prompt.handle.clone()),
    );
    View::new().child((
      Label::new(ls(format!("Rendered choice {state}")))
        .animate(StyleTarget::new().x_value(offset))
        .style(Style::new().font_size(26.px())),
      prompt.map(|prompt| {
        Button::new(ls("Choose one"))
          .key(prompt.handle.clone())
          .on_press(move || {
            let Prompt::Confirm(value) = &prompt.prompt;
            prompt.handle.submit(value.as_ref(), 1);
          })
      }),
    ))
  }
}

impl PromptData<Decisions> for Confirm {
  type ResponseType = usize;
  fn options(&self) -> impl Iterator<Item = usize> {
    [1].into_iter()
  }
  fn is_valid_response(&self, response: &usize) -> bool {
    *response == 1
  }
  fn as_prompt(&self) -> Prompt<'_> {
    Prompt::Confirm(Cow::Borrowed(self))
  }
  fn into_prompt(self) -> Prompt<'static> {
    Prompt::Confirm(Cow::Owned(self))
  }
}
impl ChoicePolicy<Decisions> for Policy {
  fn owner(&self, _: &usize, _: &Prompt<'_>) -> ChoiceOwner {
    ChoiceOwner::Human
  }
  fn choose(&mut self, _: &usize, _: &Prompt<'_>) -> usize {
    unreachable!()
  }
}
impl RulesGame for Decisions {
  type State = usize;
  type Action = ();
  type StateAnimation = ();
  type Prompt<'a> = Prompt<'a>;
  type Context = ExecutionMode<Self, Policy>;
  fn logical_clone(state: &usize) -> usize {
    *state
  }
  fn is_legal_action(state: &usize, _: &()) -> bool {
    *state == 0
  }
  fn execute(context: &mut Self::Context, state: &mut usize, _: ()) {
    for _ in 0..2 {
      *state += context.choose(state, Confirm);
    }
  }
}
