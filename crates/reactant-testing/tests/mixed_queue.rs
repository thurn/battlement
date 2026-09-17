use std::{cell::RefCell, rc::Rc, time::Duration};

use battlement::{Command, CommandBody, ParentScene, Vector3, WaitPayload};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{
  app::App,
  app_context, hooks,
  host::ButtonHost,
  native_host,
  portal::{self, PortalTarget},
  prelude::*,
};
use reactant_rules::{ChoiceOwner, ChoicePolicy, DisplayConnection, ExecutionMode, Game};
use reactant_testing::Display;
use trox::ls;

const TIMEOUT: Duration = Duration::from_secs(5);

struct Queue;
struct Policy;
struct Context(ExecutionMode<Queue, Policy>);
struct Board {
  target: PortalTarget,
  reference: Rc<RefCell<Option<ObjectRef>>>,
}

impl ChoicePolicy<Queue> for Policy {
  fn owner(&self, _: &u32, _: &()) -> ChoiceOwner {
    ChoiceOwner::Policy
  }
  fn choose(&mut self, _: &u32, _: &()) -> usize {
    unreachable!()
  }
}

impl Game for Queue {
  type State = u32;
  type Action = u32;
  type StateAnimation = ();
  type Prompt<'a> = ();
  type Context = Context;
  fn logical_clone(state: &u32) -> u32 {
    *state
  }
  fn is_legal_action(_: &u32, _: &u32) -> bool {
    true
  }
  fn execute(context: &mut Context, state: &mut u32, count: u32) {
    for _ in 0..count {
      *state += 1;
      context.0.present(state, || ());
    }
  }
}

fn context(connection: DisplayConnection<Queue>) -> Context {
  Context(ExecutionMode::Interactive {
    connection,
    policy: Policy,
  })
}

impl Component for Board {
  fn render(&self) -> impl Render {
    let state = *reactant::use_game_state::<Queue>();
    let reference = native_host::use_object_ref();
    self.reference.replace(Some(reference.clone()));
    let app = app_context::use_app();
    hooks::use_effect(
      move || {
        if state != 0 {
          app.send(Command::new_v4(CommandBody::TimeWait(WaitPayload {
            duration_ms: 200,
          })));
        }
      },
      state,
    );
    SceneRoot::new(ParentScene::PrimaryScene).child(
      WorldGroup::new()
        .position(Vector3::new(f64::from(state), 0.0, 0.0))
        .reference(reference)
        .child(portal::create_portal(
          Label::new(ls(state.to_string())).name("stage"),
          self.target.clone(),
        )),
    )
  }
}

#[test]
fn mixed_batches_render_ahead_without_menu_overtaking_and_cancel_owned_objects() {
  let mut app = App::with_model("mixed/scene", 0_u32);
  let target = app.create_portal_target();
  let reference = Rc::new(RefCell::new(None));
  let board_reference = reference.clone();
  app = app.root(move |menu| {
    (
      ButtonHost::new(ls("Menu"))
        .name("menu")
        .on_click(|menu: &mut u32| *menu += 1),
      Label::new(ls(menu.to_string())).name("menu-count"),
      View::new().portal_target(target.clone()),
      GameRoot::new(Board {
        target: target.clone(),
        reference: board_reference.clone(),
      }),
    )
  });
  let game = app.start_game::<Queue>(0, self::context);
  let consumer = app.game_consumer::<Queue>();
  consumer.resume_automatic_submission();
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("mixed/scene");
  let mut display = Display::connect(app, assets);
  display.poll();
  let native = reference.borrow().as_ref().unwrap().object_id().unwrap();
  assert_eq!(
    display.object(native).unwrap().local_transform().position.x,
    0.0
  );
  game.dispatch(3);
  assert!(consumer.wait_for_worker_stopped(TIMEOUT));
  for _ in 0..12 {
    if game.status() == GameStatus::Ready {
      break;
    }
    assert!(consumer.wait_for_output(TIMEOUT));
    display.poll();
  }
  assert_eq!(game.accepted_state(), 3);
  let stage = display.find_ui(root, "stage");
  assert_eq!(display.ui_element(stage).text(), Some("1"));
  assert_eq!(
    display.object(native).unwrap().local_transform().position.x,
    1.0
  );
  let menu = display.find_ui(root, "menu");
  display.click_ui(menu);
  assert_eq!(
    display
      .ui_element(display.find_ui(root, "menu-count"))
      .text(),
    Some("1")
  );
  assert_eq!(
    display.object(native).unwrap().local_transform().position.x,
    1.0
  );
  display.advance_time(Duration::from_millis(200));
  assert_eq!(display.ui_element(stage).text(), Some("2"));
  assert_eq!(
    display.object(native).unwrap().local_transform().position.x,
    2.0
  );
  game.stop();
  display.poll();
  assert!(display.object(native).is_none());
  assert!(!reference.borrow().as_ref().unwrap().is_attached());
  assert_eq!(display.find_ui(root, "menu"), menu);
  assert_eq!(display.with_engine(|app| app.retained_gameplay_bytes()), 0);
  let replacement = display.with_engine(|app| app.start_game::<Queue>(9, self::context));
  display.poll();
  display.poll();
  assert_eq!(replacement.status(), GameStatus::Ready);
  let next = reference.borrow().as_ref().unwrap().object_id().unwrap();
  assert_ne!(native, next);
  assert_eq!(
    display.object(next).unwrap().local_transform().position.x,
    9.0
  );
  assert_eq!(
    display.ui_element(display.find_ui(root, "stage")).text(),
    Some("9")
  );
  assert_eq!(display.frame(), 0);
  assert_eq!(display.presentation_time(), Duration::from_millis(200));
}

struct DestroyingBoard(Rc<RefCell<Option<ObjectRef>>>);
impl Component for DestroyingBoard {
  fn render(&self) -> impl Render {
    let stage = *reactant::use_game_state::<Queue>();
    let reference = native_host::use_object_ref();
    self.0.replace(Some(reference.clone()));
    let app = app_context::use_app();
    hooks::use_effect(
      move || {
        if stage == 1 || stage == 2 {
          app.send(Command::new_v4(CommandBody::TimeWait(WaitPayload {
            duration_ms: 200,
          })));
        }
      },
      stage,
    );
    SceneRoot::new(ParentScene::PrimaryScene).child((stage < 3).then(|| {
      WorldGroup::new()
        .position(Vector3::new(f64::from(stage), 0.0, 0.0))
        .reference(reference)
    }))
  }
}

#[test]
fn rendered_ahead_absence_detaches_logical_ref_but_preserves_queued_movement() {
  let reference = Rc::new(RefCell::new(None));
  let mut app = App::new("mixed/scene").ui(GameRoot::new(DestroyingBoard(reference.clone())));
  let game = app.start_game::<Queue>(0, self::context);
  let consumer = app.game_consumer::<Queue>();
  consumer.resume_automatic_submission();
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("mixed/scene");
  let mut display = Display::connect(app, assets);
  display.poll();
  let reference = reference.borrow().as_ref().unwrap().clone();
  let native = reference.object_id().unwrap();
  let anchor = reference
    .local_point(Vector3::new(0.5, 0.0, 0.0))
    .follow()
    .resolve();
  game.dispatch(3);
  assert!(consumer.wait_for_worker_stopped(TIMEOUT));
  for _ in 0..12 {
    if game.status() == GameStatus::Ready {
      break;
    }
    assert!(consumer.wait_for_output(TIMEOUT));
    display.poll();
  }
  assert_eq!(game.accepted_state(), 3);
  assert!(!reference.is_attached());
  assert_eq!(
    display.object(native).unwrap().local_transform().position.x,
    1.0
  );
  assert_eq!(
    display.world_point(anchor.object_id(), anchor.offset()).x,
    1.5
  );
  assert_eq!(display.presentation_time(), Duration::ZERO);
  display.advance_time(Duration::from_millis(200));
  assert_eq!(
    display.object(native).unwrap().local_transform().position.x,
    2.0
  );
  assert_eq!(
    display.world_point(anchor.object_id(), anchor.offset()).x,
    2.5
  );
  display.advance_time(Duration::from_millis(200));
  assert!(display.object(native).is_none());
  assert_eq!(anchor.object_id(), native);
  let replacement = display.with_engine(|app| app.start_game::<Queue>(0, self::context));
  display.poll();
  display.poll();
  assert_eq!(replacement.status(), GameStatus::Ready);
  assert!(display.object(anchor.object_id()).is_none());
  assert_eq!(display.frame(), 0);
}
