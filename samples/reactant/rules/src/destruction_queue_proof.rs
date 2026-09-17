use std::{rc::Rc, time::Duration};

use battlement::{
  Command, CommandBody, ParentScene, PropertyCommand, Tween, TweenPositionPayload, Vector3,
};
use reactant::{
  GameConsumer, GameHandle,
  app::App,
  hooks, native_host,
  prelude::*,
  rules::{ChoiceOwner, ChoicePolicy, ExecutionMode, Game as RulesGame},
};
use trox::ls;

use crate::{CONTENT_SCENE, Game, ROOT_ID, model};

struct Queue;
struct Policy;
struct Proof {
  game: GameHandle<Queue>,
  consumer: GameConsumer<Queue>,
}
struct Menu(Rc<Proof>);
struct Board;

pub(crate) fn app() -> App<Game> {
  let mut app = App::with_model(CONTENT_SCENE, model::new());
  let game = app.start_game::<Queue>(0, |connection| ExecutionMode::Interactive {
    connection,
    policy: Policy,
  });
  let consumer = app.game_consumer::<Queue>();
  consumer.resume_automatic_submission();
  app
    .ui(
      View::new()
        .style(
          Style::new()
            .padding(32.px())
            .color(Color::WHITE)
            .background_color(Color::rgb(0.05, 0.08, 0.14)),
        )
        .child((
          Heading::new(ls("Move before removal"), 1),
          Menu(Rc::new(Proof { game, consumer })),
          GameRoot::new(Board),
        )),
    )
    .document(|mut document| {
      document.root_id = ROOT_ID;
      document
    })
    .camera(|camera| camera.position(Vector3::new(0.0, 0.0, -10.0)))
}

impl Component for Menu {
  fn render(&self) -> impl Render {
    let status = reactant::use_game_status::<Queue>();
    let start = self.0.clone();
    (
      Heading::new(
        ls(format!(
          "Accepted {} / {status:?}",
          self.0.game.accepted_state()
        )),
        2,
      ),
      Button::new(ls("Move then destroy")).on_press(move |_: &mut Game| {
        if start.game.dispatch(()) == reactant::DispatchResult::Started {
          assert!(
            start
              .consumer
              .wait_for_worker_stopped(Duration::from_secs(5))
          );
        }
      }),
    )
  }
}

impl Component for Board {
  fn render(&self) -> impl Render {
    let stage = *reactant::use_game_state::<Queue>();
    let reference = native_host::use_object_ref();
    let movement_reference = reference.clone();
    let app = reactant::app_context::use_app();
    hooks::use_effect(
      move || {
        if stage == 1 || stage == 2 {
          app.send(Command::new_v4(CommandBody::TransformTweenLocalPosition(
            PropertyCommand::canceling(TweenPositionPayload {
              object_id: movement_reference
                .object_id()
                .expect("committed queue visual"),
              position: Vector3::new(f64::from(stage) * 2.0 - 3.0, -2.0, 0.0),
              tween: Tween::new().duration_ms(1000),
            }),
          )));
        }
      },
      stage,
    );
    (
      Heading::new(ls(format!("Presented stage {stage}")), 2),
      SceneRoot::new(ParentScene::PrimaryScene).child((stage < 3).then(|| {
        Prefab::at("reactant/mixed-visual")
          .reference(reference)
          .position(Vector3::new(-3.0, -2.0, 0.0))
          .scale(Vector3::new(1.4, 1.4, 1.4))
      })),
    )
  }
}

impl ChoicePolicy<Queue> for Policy {
  fn owner(&self, _: &u32, _: &()) -> ChoiceOwner {
    ChoiceOwner::Policy
  }
  fn choose(&mut self, _: &u32, _: &()) -> usize {
    unreachable!()
  }
}
impl RulesGame for Queue {
  type State = u32;
  type Action = ();
  type StateAnimation = ();
  type Prompt<'a> = ();
  type Context = ExecutionMode<Self, Policy>;
  fn logical_clone(state: &u32) -> u32 {
    *state
  }
  fn is_legal_action(state: &u32, _: &()) -> bool {
    *state == 0
  }
  fn execute(context: &mut Self::Context, state: &mut u32, _: ()) {
    for stage in 1..=3 {
      *state = stage;
      context.present(state, || ());
    }
  }
}
