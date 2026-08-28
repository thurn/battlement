use std::{any::Any, panic, panic::AssertUnwindSafe};

use battlement::{
  Button, CameraState, ClickEvent, CommandBody, GameObject, GameObjectKind, Label, ObjectId,
  PanelScaleMode, PanelSettings, ParentScene, PreparedAsset, Prop, Scene, SceneId, SessionId,
  Snapshot, UiDocument, UiDocumentState, UiEvent, UiEventKind, VisualElementProperties,
};
use battlement_fake::battlement_ui_fake::UiWorld;
use battlement_reactant::{
  event::{EventPhase, EventRenderExt, ReactantEvent},
  executor::{BoxFuture, SpawnedTask, Spawner},
  render::{Node, Render},
  runtime::{Reactant, ReactantCommit},
};

struct IdleSpawner;

#[derive(Clone, Copy)]
enum Form {
  BriefLast,
  EventLast,
  Hidden,
}

struct Game {
  form: Form,
  status: String,
}

struct NonClonePayload;

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

#[test]
fn click_dispatch_uses_the_last_slot_callback_without_resubscribing() {
  let document = self::document();
  let mut game = Game {
    form: Form::BriefLast,
    status: "ready".to_owned(),
  };
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), self::view);
  let initial = reactant
    .begin_session(&mut game)
    .expect("initial render succeeds")
    .into_parts(self::snapshot(&document))
    .0;
  let button_id = initial.ui[0].children[0].object_id;
  let label_id = initial.ui[0].children[1].object_id;
  assert!(matches!(
    initial.ui[0].children[0]
      .element
      .visual_element()
      .events,
    Prop::Set(ref events) if events == &[UiEventKind::Click]
  ));
  let mut world = UiWorld::default();
  world.replace(initial.ui).expect("initial tree is valid");

  let brief = reactant
    .dispatch(
      &mut game,
      UiEvent::click(button_id, ClickEvent::NavigationSubmit),
    )
    .expect("brief click dispatches");
  self::apply(&mut world, brief);
  assert_eq!(game.status, "brief-last");
  assert_eq!(world.element(label_id).unwrap().text(), Some("brief-last"));

  game.form = Form::EventLast;
  let replacement = reactant
    .refresh(&mut game)
    .expect("callback replacement renders");
  assert!(replacement.is_empty());
  assert!(replacement.into_groups().is_empty());

  let aware = reactant
    .dispatch(
      &mut game,
      UiEvent::click(button_id, ClickEvent::NavigationSubmit),
    )
    .expect("event-aware click dispatches");
  self::apply(&mut world, aware);
  assert_eq!(game.status, "event-last");

  game.status = "dirty but unrecognized".to_owned();
  assert!(
    reactant
      .dispatch(
        &mut game,
        UiEvent::click(ObjectId::new_v4(), ClickEvent::NavigationSubmit),
      )
      .expect("unknown target is ignored")
      .is_empty()
  );
  assert!(
    reactant
      .dispatch(
        &mut game,
        UiEvent::click(label_id, ClickEvent::NavigationSubmit),
      )
      .expect("unsubscribed target is ignored")
      .is_empty()
  );
  assert_eq!(world.element(label_id).unwrap().text(), Some("event-last"));

  game.form = Form::Hidden;
  self::apply(
    &mut world,
    reactant.refresh(&mut game).expect("unmount renders"),
  );
  assert!(world.element(button_id).is_none());
  assert!(
    reactant
      .dispatch(
        &mut game,
        UiEvent::click(button_id, ClickEvent::NavigationSubmit),
      )
      .expect("unmounted target is ignored")
      .is_empty()
  );
}

#[test]
fn handler_model_type_is_validated_before_session_commit() {
  let document = self::document();
  let mut reactant = Reactant::<Game>::new(IdleSpawner);
  reactant.register_root(document.clone(), |_game| {
    Button::new("wrong").on_click(|_wrong: &mut String| {})
  });
  let mut game = Game {
    form: Form::BriefLast,
    status: String::new(),
  };
  let mismatch = panic::catch_unwind(AssertUnwindSafe(|| {
    reactant.begin_session(&mut game).map(|session| {
      let _response = session.into_response(self::snapshot(&document));
    })
  }))
  .expect_err("a mismatched handler model should panic");
  assert_eq!(
    self::panic_message(mismatch),
    "Reactant handler model type does not match its runtime"
  );

  let poisoned = panic::catch_unwind(AssertUnwindSafe(|| {
    reactant.begin_session(&mut game).map(|session| {
      let _response = session.into_response(self::snapshot(&document));
    })
  }))
  .expect_err("a model mismatch should poison the runtime");
  assert_eq!(
    self::panic_message(poisoned),
    "Reactant runtime is poisoned"
  );
}

#[test]
fn event_views_clone_without_requiring_clone_payloads() {
  fn assert_clone<T: Clone>() {}

  assert_clone::<ReactantEvent<NonClonePayload>>();
}

fn view(game: &Game) -> impl Render + use<> {
  let button = match game.form {
    Form::BriefLast => Some(Node::new(
      Button::new("Activate")
        .on_click_event(|game: &mut Game, _event| game.status = "event-first".to_owned())
        .on_click(|game: &mut Game| game.status = "brief-last".to_owned()),
    )),
    Form::EventLast => Some(Node::new(
      Button::new("Activate")
        .on_click(|game: &mut Game| game.status = "brief-first".to_owned())
        .on_click_event(|game: &mut Game, event| {
          assert_eq!(event.phase(), EventPhase::Target);
          assert_eq!(event.target(), event.current_target());
          assert!(matches!(event.payload(), ClickEvent::NavigationSubmit));
          game.status = "event-last".to_owned();
        }),
    )),
    Form::Hidden => None,
  };
  (button, Label::new(game.status.clone()))
}

fn apply(world: &mut UiWorld, commit: ReactantCommit) {
  for body in commit.into_groups().into_iter().flatten() {
    match body {
      CommandBody::VisualElementCreate(value) => world.create(*value).unwrap(),
      CommandBody::VisualElementUpdate(value) => world.update(*value).unwrap(),
      CommandBody::VisualElementDestroy(value) => world.destroy(value.object_id).unwrap(),
      _ => panic!("Reactant emitted a non-UI command"),
    }
  }
}

fn document() -> UiDocument {
  UiDocument::with_root_id(ObjectId::new_v4(), ObjectId::new_v4())
}

fn panic_message(payload: Box<dyn Any + Send>) -> String {
  payload
    .downcast_ref::<&str>()
    .map(|message| (*message).to_owned())
    .or_else(|| payload.downcast_ref::<String>().cloned())
    .unwrap_or_else(|| "non-string panic payload".to_owned())
}

fn snapshot(document: &UiDocument) -> Snapshot {
  let camera_id = ObjectId::new_v4();
  Snapshot::new(
    SessionId::new_v4(),
    vec![PreparedAsset::Scene("test/scene".into())],
    vec![Scene::new(SceneId::new_v4(), "test/scene")],
    vec![
      GameObject::new(camera_id, CameraState::new()),
      GameObject::new(
        document.document_id,
        GameObjectKind::UiDocument(
          UiDocumentState::new(document.root_id)
            .panel_settings(PanelSettings::new().scale_mode(PanelScaleMode::ConstantPixelSize)),
        ),
      )
      .parent_scene(ParentScene::Persistent),
    ],
    camera_id,
  )
}
