use std::{
  any::Any,
  cell::RefCell,
  panic::{self, AssertUnwindSafe},
  rc::Rc,
};

use battlement::{
  CameraState, ClickEvent, CommandBody, GameObject, GameObjectKind, ObjectId, PanelScaleMode,
  PanelSettings, ParentScene, PreparedAsset, Prop, Scene, SceneId, SessionId, Snapshot, UiDocument,
  UiDocumentState, UiEvent, UiEventKind, UiEventPhase, UiEventSubscription,
};
use battlement_fake::battlement_ui_fake::UiWorld;
use battlement_reactant::{
  component::Component,
  context::Context,
  executor::{BoxFuture, SpawnedTask, Spawner},
  hooks::{self, StateSetter},
  key::KeyRenderExt,
  portal::create_portal,
  render::Render,
  runtime::{Reactant, ReactantCommit},
};

static THEME: Context<&'static str> = Context::new(|| "default");

struct IdleSpawner;

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

#[derive(Default)]
struct Game {
  key: u8,
  log: Vec<&'static str>,
  show_target: bool,
  target_key: u8,
  use_second: bool,
}

struct PortaledButton {
  name: &'static str,
}

impl Component for PortaledButton {
  fn render(&self) -> impl Render {
    battlement_reactant::host::Button::new(format!("{} {}", self.name, hooks::use_context(&THEME)))
      .on_click(|game: &mut Game| {
        game.log.push("target");
      })
  }
}

struct StatefulPortal {
  setter: Rc<RefCell<Option<StateSetter<u8>>>>,
}

impl Component for StatefulPortal {
  fn render(&self) -> impl Render {
    let (value, setter) = hooks::use_state(0_u8);
    self.setter.replace(Some(setter));
    battlement_reactant::host::Label::new(format!("state {value}"))
  }
}

#[test]
fn internal_portals_preserve_logical_ancestry_and_global_source_order() {
  let target_document = self::document();
  let first_document = self::document();
  let second_document = self::document();
  let mut reactant = Reactant::new(IdleSpawner);
  let target = reactant.create_portal_target();
  let first_target = target.clone();
  reactant.register_root(first_document.clone(), move |_: &Game| {
    THEME.provider("dark-a").child(
      battlement_reactant::host::View::new()
        .child(create_portal(
          PortaledButton { name: "A" },
          first_target.clone(),
        ))
        .on_click_capture(|game: &mut Game| game.log.push("capture"))
        .on_click(|game: &mut Game| game.log.push("bubble")),
    )
  });
  let second_target = target.clone();
  reactant.register_root(second_document.clone(), move |_: &Game| {
    THEME.provider("dark-b").child(create_portal(
      battlement_reactant::host::Button::new("B dark-b"),
      second_target.clone(),
    ))
  });
  reactant.register_root(target_document.clone(), move |_: &Game| {
    battlement_reactant::host::View::new()
      .child(battlement_reactant::host::Label::new("ordinary"))
      .portal_target(target.clone())
  });
  let mut game = Game::default();

  let initial = reactant
    .begin_session(&mut game)
    .unwrap()
    .into_parts(self::snapshot(&[
      first_document.clone(),
      second_document.clone(),
      target_document.clone(),
    ]))
    .0;
  let first_source = initial.ui[0].children[0].object_id;
  let target_host = initial.ui[2].children[0].object_id;
  let target_children = initial.ui[2].children[0]
    .children
    .iter()
    .map(|child| child.object_id)
    .collect::<Vec<_>>();
  let portaled_button = target_children[1];
  let mut world = UiWorld::default();
  world.replace(initial.ui.clone()).unwrap();

  assert!(world.element(first_source).unwrap().children().is_empty());
  assert_eq!(
    self::texts(&world, target_host),
    ["ordinary", "A dark-a", "B dark-b"]
  );
  assert_eq!(
    initial.ui[2].element.event_subscriptions,
    Prop::Set(vec![
      UiEventSubscription::target(UiEventKind::Click),
      UiEventSubscription::new(UiEventKind::Click, UiEventPhase::Trickle),
    ])
  );

  assert!(
    reactant
      .dispatch(
        &mut game,
        UiEvent::click(portaled_button, ClickEvent::NavigationSubmit),
      )
      .unwrap()
      .is_empty()
  );
  assert_eq!(game.log, ["capture", "target", "bubble"]);
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn changing_a_portal_target_or_key_remounts_its_stateful_range() {
  let source_document = self::document();
  let target_document = self::document();
  let setter = Rc::new(RefCell::new(None));
  let mut reactant = Reactant::new(IdleSpawner);
  let first = reactant.create_portal_target();
  let second = reactant.create_portal_target();
  let source_first = first.clone();
  let source_second = second.clone();
  let source_setter = Rc::clone(&setter);
  reactant.register_root(source_document.clone(), move |game: &Game| {
    create_portal(
      StatefulPortal {
        setter: Rc::clone(&source_setter),
      },
      if game.use_second {
        source_second.clone()
      } else {
        source_first.clone()
      },
    )
    .key(game.key)
  });
  reactant.register_root(target_document.clone(), move |game: &Game| {
    (
      battlement_reactant::host::View::new().portal_target(first.clone()),
      battlement_reactant::host::View::new()
        .key(game.target_key)
        .portal_target(second.clone()),
    )
  });
  let mut game = Game::default();
  let initial = self::begin(
    &mut reactant,
    &mut game,
    &[source_document.clone(), target_document.clone()],
  );
  let first_target = initial.ui[1].children[0].object_id;
  let second_target = initial.ui[1].children[1].object_id;
  let original = initial.ui[1].children[0].children[0].object_id;
  let mut world = UiWorld::default();
  world.replace(initial.ui).unwrap();
  setter.borrow().clone().unwrap().set(7);
  self::apply(&mut world, reactant.poll(&mut game).unwrap());
  assert_eq!(world.element(original).unwrap().text(), Some("state 7"));

  game.use_second = true;
  self::apply(&mut world, reactant.refresh(&mut game).unwrap());
  let moved = world.element(second_target).unwrap().children()[0];
  assert_ne!(moved, original);
  assert!(world.element(first_target).unwrap().children().is_empty());
  assert_eq!(world.element(moved).unwrap().text(), Some("state 0"));

  setter.borrow().clone().unwrap().set(8);
  self::apply(&mut world, reactant.poll(&mut game).unwrap());
  assert_eq!(world.element(moved).unwrap().text(), Some("state 8"));
  game.key = 1;
  self::apply(&mut world, reactant.refresh(&mut game).unwrap());
  let rekeyed = world.element(second_target).unwrap().children()[0];
  assert_ne!(rekeyed, moved);
  assert_eq!(world.element(rekeyed).unwrap().text(), Some("state 0"));

  setter.borrow().clone().unwrap().set(9);
  self::apply(&mut world, reactant.poll(&mut game).unwrap());
  game.target_key = 1;
  self::apply(&mut world, reactant.refresh(&mut game).unwrap());
  let remounted_target = world.element(target_document.root_id).unwrap().children()[1];
  let remounted_portal = world.element(remounted_target).unwrap().children()[0];
  assert_ne!(remounted_target, second_target);
  assert_ne!(remounted_portal, rekeyed);
  assert_eq!(
    world.element(remounted_portal).unwrap().text(),
    Some("state 9")
  );
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn invalid_internal_targets_fail_before_native_mutation() {
  self::missing_target_is_transactional();
  self::duplicate_target_is_rejected();
  self::cross_runtime_target_is_rejected();
}

fn missing_target_is_transactional() {
  let source_document = self::document();
  let target_document = self::document();
  let mut reactant = Reactant::new(IdleSpawner);
  let target = reactant.create_portal_target();
  let source_target = target.clone();
  reactant.register_root(source_document.clone(), move |_: &Game| {
    create_portal(
      battlement_reactant::host::Label::new("portal"),
      source_target.clone(),
    )
  });
  reactant.register_root(target_document.clone(), move |game: &Game| {
    game
      .show_target
      .then(|| battlement_reactant::host::View::new().portal_target(target.clone()))
  });
  let mut game = Game {
    show_target: true,
    ..Game::default()
  };
  let initial = self::begin(
    &mut reactant,
    &mut game,
    &[source_document, target_document],
  );
  let target_host = initial.ui[1].children[0].object_id;
  let portaled = initial.ui[1].children[0].children[0].object_id;
  let mut world = UiWorld::default();
  world.replace(initial.ui).unwrap();
  game.show_target = false;

  assert!(panic::catch_unwind(AssertUnwindSafe(|| reactant.refresh(&mut game))).is_err());
  assert_eq!(world.element(target_host).unwrap().children(), &[portaled]);
  assert_eq!(world.element(portaled).unwrap().text(), Some("portal"));
  self::assert_poisoned(&mut reactant);
}

fn duplicate_target_is_rejected() {
  let document = self::document();
  let mut reactant = Reactant::new(IdleSpawner);
  let target = reactant.create_portal_target();
  reactant.register_root(document, move |_: &Game| {
    (
      battlement_reactant::host::View::new().portal_target(target.clone()),
      battlement_reactant::host::View::new().portal_target(target.clone()),
    )
  });
  assert!(
    panic::catch_unwind(AssertUnwindSafe(|| {
      let _session = reactant.begin_session(&mut Game::default());
    }))
    .is_err()
  );
  self::assert_poisoned(&mut reactant);
}

fn cross_runtime_target_is_rejected() {
  let foreign = Reactant::<Game>::new(IdleSpawner).create_portal_target();
  let document = self::document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document, move |_: &Game| {
    battlement_reactant::host::View::new().portal_target(foreign.clone())
  });
  assert!(
    panic::catch_unwind(AssertUnwindSafe(|| {
      let _session = reactant.begin_session(&mut Game::default());
    }))
    .is_err()
  );
  self::assert_poisoned(&mut reactant);
}

fn assert_poisoned(reactant: &mut Reactant<Game>) {
  let poisoned = panic::catch_unwind(AssertUnwindSafe(|| {
    let _session = reactant.begin_session(&mut Game::default());
  }))
  .expect_err("portal validation should poison the runtime");
  assert_eq!(
    self::panic_message(poisoned),
    "Reactant runtime is poisoned"
  );
}

fn begin(reactant: &mut Reactant<Game>, game: &mut Game, documents: &[UiDocument]) -> Snapshot {
  reactant
    .begin_session(game)
    .unwrap()
    .into_parts(self::snapshot(documents))
    .0
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

fn texts(world: &UiWorld, root: ObjectId) -> Vec<&str> {
  world
    .element(root)
    .unwrap()
    .children()
    .iter()
    .map(|id| world.element(*id).unwrap().text().unwrap())
    .collect()
}

fn document() -> UiDocument {
  UiDocument::with_root_id(ObjectId::new_v4(), ObjectId::new_v4())
}

fn snapshot(documents: &[UiDocument]) -> Snapshot {
  let scene_id = SceneId::new_v4();
  let camera_id = ObjectId::new_v4();
  let mut game_objects = vec![GameObject::new(camera_id, CameraState::new())];
  game_objects.extend(documents.iter().map(|document| {
    GameObject::new(
      document.document_id,
      GameObjectKind::UiDocument(
        UiDocumentState::new(document.root_id)
          .panel_settings(PanelSettings::new().scale_mode(PanelScaleMode::ConstantPixelSize)),
      ),
    )
    .parent_scene(ParentScene::Persistent)
  }));
  Snapshot::new(
    SessionId::new_v4(),
    vec![PreparedAsset::Scene("test/scene".into())],
    vec![Scene::new(scene_id, "test/scene")],
    game_objects,
    camera_id,
  )
}

fn panic_message(payload: Box<dyn Any + Send>) -> String {
  match payload.downcast::<String>() {
    Ok(message) => *message,
    Err(payload) => payload
      .downcast::<&'static str>()
      .map(|message| message.to_string())
      .unwrap_or_else(|_| "non-string panic".to_string()),
  }
}
