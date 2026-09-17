use std::{rc::Rc, time::Duration};

use battlement::{
  Command, CommandBody, GameObject, GameObjectKind, MaterialAssignment, ObjectId, ParentScene,
  PropertyCommand, Tween, TweenPositionPayload, Vector3, object_id,
};
use reactant::{
  GameConsumer, GameHandle,
  app::App,
  prelude::*,
  rules::{ChoiceOwner, ChoicePolicy, ExecutionMode, Game as RulesGame},
};
use trox::ls;

use crate::{CONTENT_SCENE, Game, MOTION_MATERIAL, ROOT_ID, model};

const GAME_CUBE: ObjectId = object_id!("25300000-0000-4000-8000-000000000091");
const MENU_CUBE: ObjectId = object_id!("25300000-0000-4000-8000-000000000092");

struct QueueGame;
struct Policy;
struct Proof {
  game: GameHandle<QueueGame>,
  consumer: GameConsumer<QueueGame>,
}
struct Menu(Rc<Proof>);
struct Board;

pub(crate) fn app() -> App<Game> {
  let mut app = App::with_model(CONTENT_SCENE, model::new());
  let game = app.start_game::<QueueGame>(0, |connection| ExecutionMode::Interactive {
    connection,
    policy: Policy,
  });
  let consumer = app.game_consumer::<QueueGame>();
  consumer.resume_automatic_submission();
  let proof = Rc::new(Proof { game, consumer });
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
          Label::new(ls("Ordered game output")).style(Style::new().font_size(30.px())),
          Menu(proof),
          GameRoot::new(Board),
        )),
    )
    .document(|mut document| {
      document.root_id = ROOT_ID;
      document
    })
    .camera(|camera| camera.position(Vector3::new(0.0, 0.0, -10.0)))
    .object(
      GameObject::new(
        GAME_CUBE,
        GameObjectKind::Cube {
          materials: vec![MaterialAssignment::new(0, MOTION_MATERIAL)],
        },
      )
      .parent_scene(ParentScene::Persistent)
      .position(Vector3::new(-3.0, -2.0, 0.0)),
    )
    .object(
      GameObject::new(
        MENU_CUBE,
        GameObjectKind::Cube {
          materials: vec![MaterialAssignment::new(0, MOTION_MATERIAL)],
        },
      )
      .parent_scene(ParentScene::Persistent)
      .position(Vector3::new(3.0, -2.0, 0.0)),
    )
}

impl Component for Menu {
  fn render(&self) -> impl Render {
    let status = reactant::use_game_status::<QueueGame>();
    let (open, set_open) = reactant::hooks::use_state(false);
    let start = self.0.clone();
    let stop = self.0.clone();
    let app = reactant::app_context::use_app();
    View::new().child((
      Label::new(ls(format!(
        "Rules {status:?} / accepted {}",
        self.0.game.accepted_state()
      )))
      .name("queue-acceptance"),
      Button::new(ls("Begin ordered game")).on_press(move |_: &mut Game| {
        if start.game.dispatch(()) == reactant::DispatchResult::Started {
          assert!(
            start
              .consumer
              .wait_for_worker_stopped(Duration::from_secs(5)),
            "fixture worker timed out"
          );
          app.send(self::movement(MENU_CUBE, -3.0, 4000).nonblocking());
        }
      }),
      Button::new(ls("Stop queued game")).on_press(move |_: &mut Game| stop.game.stop()),
      Button::new(ls("Settings")).on_press(move |_: &mut Game| set_open.set(!open)),
      Label::new(ls(if open {
        "Settings open"
      } else {
        "Settings closed"
      }))
      .name("queue-menu"),
    ))
  }
}

impl Component for Board {
  fn render(&self) -> impl Render {
    let stage = *reactant::use_game_state::<QueueGame>();
    let offset = use_motion_value(0.0_f32);
    let animate_offset = offset.clone();
    let app = reactant::app_context::use_app();
    reactant::hooks::use_effect(
      move || {
        if stage > 0 {
          app.send(self::movement(GAME_CUBE, f64::from(stage) - 2.0, 1000));
          animate_offset.animate(stage as f32 * 70.0, Transition::tween().duration_secs(1.0));
        }
      },
      stage,
    );
    Label::new(ls(match stage {
      0 => "Initial hand",
      1 => "Card played",
      2 => "Card drawn",
      _ => "Energy +1",
    }))
    .name("queue-stage")
    .animate(StyleTarget::new().x_value(offset))
    .style(Style::new().font_size(26.px()))
  }
}

fn movement(object_id: ObjectId, x: f64, duration: u64) -> Command {
  Command::new_v4(CommandBody::TransformTweenLocalPosition(
    PropertyCommand::canceling(TweenPositionPayload {
      object_id,
      position: Vector3::new(x, -2.0, 0.0),
      tween: Tween::new().duration_ms(duration),
    }),
  ))
}

impl ChoicePolicy<QueueGame> for Policy {
  fn owner(&self, _: &u32, _: &()) -> ChoiceOwner {
    ChoiceOwner::Policy
  }
  fn choose(&mut self, _: &u32, _: &()) -> usize {
    unreachable!()
  }
}
impl RulesGame for QueueGame {
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
