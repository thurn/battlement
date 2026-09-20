use std::{rc::Rc, time::Duration};

use battlement::{ObjectId, ParentScene, Vector3, object_id};
use reactant::{
  GameHandle,
  animation_controls::{AnimationSequence, MotionSelector},
  prelude::*,
  rules::{ChoiceOwner, ChoicePolicy, ExecutionMode, Game as RulesGame},
};
use trox::ls;

use crate::ROOT_ID;

const CARD: ObjectId = object_id!("25300000-0000-4000-8000-000000000093");

struct PausableGame;
struct Policy;
struct Context(ExecutionMode<PausableGame, Policy>);
struct Proof {
  game: GameHandle<PausableGame>,
}
struct Root;
struct Menu(Rc<Proof>);
struct Board;

#[derive(Clone, Copy)]
enum Animation {
  Move,
  Hold,
}

pub(crate) fn app() -> crate::ReactantApplication {
  reactant::Application::new(crate::CONTENT_SCENE)
    .child(Root)
    .document(|mut document| {
      document.root_id = ROOT_ID;
      document
    })
    .camera(|camera| {
      reactant::world::Camera::new()
        .orthographic(4.5)
        .clipping(0.1, 50.0)
        .background(Color::rgb(0.015, 0.03, 0.06))
        .position(Vector3::new(0.0, 0.0, -10.0))
        .into_object(camera.object_id)
    })
}

impl Component for Root {
  fn render(&self) -> impl Render {
    let game = reactant::use_game::<PausableGame, _>((), 0, |connection| {
      Context(ExecutionMode::Interactive {
        connection,
        policy: Policy,
      })
    });
    let proof = Rc::new(Proof { game });
    View::new()
      .style(
        Style::new()
          .padding(32.px())
          .background_color(Color::rgb(0.04, 0.07, 0.12))
          .color(Color::WHITE),
      )
      .child((
        Heading::new(ls("Gameplay presentation pause"), 1),
        Menu(proof),
        GameRoot::new(Board),
      ))
  }
}

impl Component for Menu {
  fn render(&self) -> impl Render {
    let (open, set_open) = reactant::hooks::use_state(false);
    let begin = self.0.clone();
    View::new().child((
      Button::new(ls("Begin paused game")).on_press(move || {
        begin.game.dispatch(());
      }),
      Button::new(ls("Settings")).on_press(move || set_open.set(!open)),
      Heading::new(
        ls(if open {
          "Settings open"
        } else {
          "Settings closed"
        }),
        2,
      ),
    ))
  }
}

impl Component for Board {
  fn render(&self) -> impl Render {
    let stage = *reactant::use_game_state::<PausableGame>();
    let scope = reactant::animation_controls::use_animation_scope();
    let event_scope = scope.clone();
    reactant::use_animate::<PausableGame>(move |animation| {
      Some(match animation {
        Animation::Move => SnapshotAnimation::sequence(
          event_scope.clone(),
          AnimationSequence::new().animate(
            MotionSelector::name("paused-card"),
            StyleTarget::new().local_position_x(2.4),
            Transition::tween().duration_secs(1.0).ease(Easing::Linear),
          ),
        ),
        Animation::Hold => SnapshotAnimation::wait(Duration::from_secs(1)),
      })
    });
    let presentation = reactant::use_game_presentation();
    let pause = presentation.clone();
    (
      View::new().child((
        Heading::new(ls(format!("Gameplay stage: {stage}")), 2),
        Button::new(ls("Pause gameplay")).on_press(move || pause.pause()),
        Button::new(ls("Resume gameplay")).on_press(move || presentation.resume()),
      )),
      reactant::world::SceneRoot::new(ParentScene::PrimaryScene).child(
        reactant::world::Group::new()
          .child(
            reactant::world::Group::new()
              .id(*CARD.as_uuid())
              .position(Vector3::new(-2.4, -0.7, 0.0))
              .child(
                reactant::world::Sprite::new()
                  .texture("reactant/assets/texture")
                  .size(1.3, 1.8),
              )
              .motion(MotionProps::new().motion_name("paused-card")),
          )
          .motion(MotionProps::new().animation_scope(scope)),
      ),
    )
  }
}

impl ChoicePolicy<PausableGame> for Policy {
  fn owner(&self, _: &u32, _: &()) -> ChoiceOwner {
    ChoiceOwner::Policy
  }

  fn choose(&mut self, _: &u32, _: &()) -> usize {
    unreachable!()
  }
}

impl RulesGame for PausableGame {
  type State = u32;
  type Action = ();
  type StateAnimation = Animation;
  type Prompt<'a> = ();
  type Context = Context;

  fn logical_clone(state: &u32) -> u32 {
    *state
  }

  fn is_legal_action(state: &u32, _: &()) -> bool {
    *state == 0
  }

  fn execute(context: &mut Context, state: &mut u32, _: ()) {
    *state = 1;
    context.0.present(state, || Animation::Move);
    *state = 2;
    context.0.present(state, || Animation::Hold);
    *state = 3;
  }
}
