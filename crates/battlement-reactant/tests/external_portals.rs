mod runtime_support;

use std::{
  any::Any,
  cell::RefCell,
  panic::{self, AssertUnwindSafe},
  rc::Rc,
  slice,
};

use battlement::{
  CameraState, ClickEvent, CommandBody, GameObject, GameObjectKind, ObjectId, PanelScaleMode,
  PanelSettings, ParentScene, PreparedAsset, Prop, ResponseMessage, Scene, SceneId, SessionId,
  Snapshot, UiDocument, UiDocumentState, UiEvent, UiEventKind, UiEventPhase, UiEventSubscription,
  UiLabel, UiNode, UiVisualElement, UiVisualElementProperties,
};
use battlement_fake::battlement_ui_fake::UiWorld;
use battlement_reactant::{
  component::Component,
  executor::{BoxFuture, SpawnedTask, Spawner},
  hooks::{self, StateSetter},
  portal::create_portal,
  render::Render,
  runtime::ReactantCommit,
};

struct IdleSpawner;

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

#[derive(Default)]
struct Game {
  log: Vec<&'static str>,
  show: bool,
}

struct StatefulLabel {
  setter: Rc<RefCell<Option<StateSetter<u8>>>>,
}

impl Component for StatefulLabel {
  fn render(&self) -> impl Render {
    let (value, setter) = hooks::use_state(0_u8);
    self.setter.replace(Some(setter));
    battlement_reactant::host::Label::new(trox::assert_localized(format!("state {value}")))
  }
}

#[test]
fn external_portals_append_after_the_prefix_and_enter_events_once() {
  let source = self::document();
  let (external, target_id, prefix_id) = self::external_document("caller prefix");
  let mut reactant = runtime_support::reactant(IdleSpawner);
  let external_target = reactant.register_external_container(target_id);
  let internal_target = reactant.create_portal_target();
  let portal_external = external_target.clone();
  let portal_internal = internal_target.clone();
  reactant.register_root(source.clone(), move |game: &Game| {
    game.show.then(|| {
      create_portal(
        battlement_reactant::host::View::new()
          .child(create_portal(
            battlement_reactant::host::Button::new(trox::assert_localized("action"))
              .on_click(|game: &mut Game| game.log.push("target")),
            portal_internal.clone(),
          ))
          .on_click_capture(|game: &mut Game| game.log.push("capture"))
          .on_click(|game: &mut Game| game.log.push("bubble"))
          .portal_target(internal_target.clone()),
        portal_external.clone(),
      )
    })
  });
  let mut game = Game {
    show: true,
    ..Game::default()
  };

  let response = reactant
    .begin_session(&mut game)
    .unwrap()
    .into_response(self::snapshot(
      &[source.clone(), external.clone()],
      vec![external.clone()],
    ));
  assert!(matches!(response.messages[0], ResponseMessage::Snapshot(_)));
  assert!(matches!(response.messages[1], ResponseMessage::Batch(_)));
  let ResponseMessage::Snapshot(initial) = &response.messages[0] else {
    unreachable!()
  };
  let target = self::node(initial, target_id);
  assert_eq!(target.children.len(), 1);
  assert_eq!(target.children[0].object_id, prefix_id);
  let ResponseMessage::Batch(batch) = &response.messages[1] else {
    unreachable!()
  };
  let CommandBody::VisualElementCreate(created) = &batch.groups[0].commands[0].body else {
    panic!("external portal session must create its top-level host")
  };
  assert_eq!(created.parent_id, target_id);
  assert_eq!(created.child_index, Some(1));
  assert_eq!(
    created.node.element.visual_element().event_subscriptions,
    Prop::Set(vec![
      UiEventSubscription::target(UiEventKind::Click),
      UiEventSubscription::new(UiEventKind::Click, UiEventPhase::Trickle),
    ])
  );
  assert_eq!(
    created.node.children[0]
      .element
      .visual_element()
      .event_subscriptions,
    Prop::Unset
  );

  let mut world = UiWorld::default();
  world.replace(initial.ui.clone()).unwrap();
  self::apply_groups(
    &mut world,
    batch
      .groups
      .iter()
      .map(|group| {
        group
          .commands
          .iter()
          .map(|command| command.body.clone())
          .collect()
      })
      .collect(),
  );
  let outer = world.element(target_id).unwrap().children()[1];
  let button = world.element(outer).unwrap().children()[0];
  assert!(
    reactant
      .dispatch(
        &mut game,
        UiEvent::click(button, ClickEvent::NavigationSubmit),
      )
      .unwrap()
      .is_empty()
  );
  assert_eq!(game.log, ["capture", "target", "bubble"]);

  game.show = false;
  self::apply(&mut world, reactant.refresh(&mut game).unwrap());
  assert_eq!(world.element(target_id).unwrap().children(), &[prefix_id]);
  game.show = true;
  self::apply(&mut world, reactant.refresh(&mut game).unwrap());
  assert_eq!(world.element(target_id).unwrap().children()[0], prefix_id);
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn reconnect_rebind_preserves_logical_portal_state() {
  let source = self::document();
  let (first_external, first_target_id, first_prefix_id) = self::external_document("first");
  let setter = Rc::new(RefCell::new(None));
  let mut reactant = runtime_support::reactant(IdleSpawner);
  let target = reactant.register_external_container(first_target_id);
  let portal_target = target.clone();
  let portal_setter = Rc::clone(&setter);
  reactant.register_root(source.clone(), move |_: &Game| {
    create_portal(
      StatefulLabel {
        setter: Rc::clone(&portal_setter),
      },
      portal_target.clone(),
    )
  });
  let mut game = Game::default();
  let (initial, initial_commit) =
    reactant
      .begin_session(&mut game)
      .unwrap()
      .into_parts(self::snapshot(
        &[source.clone(), first_external.clone()],
        vec![first_external],
      ));
  let mut world = UiWorld::default();
  world.replace(initial.ui).unwrap();
  self::apply(&mut world, initial_commit);
  let original_host = world.element(first_target_id).unwrap().children()[1];
  setter.borrow().clone().unwrap().set(7);
  self::apply(&mut world, reactant.poll(&mut game).unwrap());
  assert_eq!(
    world.element(original_host).unwrap().text(),
    Some("state 7")
  );

  let (second_external, second_target_id, second_prefix_id) = self::external_document("second");
  reactant.stage_external_container_rebind(&target, second_target_id);
  let (reconnected, reconnect_commit) =
    reactant
      .begin_session(&mut game)
      .unwrap()
      .into_parts(self::snapshot(
        &[source, second_external.clone()],
        vec![second_external],
      ));
  let mut reconnected_world = UiWorld::default();
  reconnected_world.replace(reconnected.ui).unwrap();
  self::apply(&mut reconnected_world, reconnect_commit);
  assert_eq!(
    reconnected_world
      .element(second_target_id)
      .unwrap()
      .children(),
    &[second_prefix_id, original_host]
  );
  assert_eq!(
    reconnected_world.element(original_host).unwrap().text(),
    Some("state 7")
  );
  assert_eq!(
    world.element(first_target_id).unwrap().children()[0],
    first_prefix_id
  );
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn external_target_validation_precedes_runtime_and_native_mutation() {
  self::duplicate_registration_is_a_guard_error();
  self::rebind_before_activation_is_a_guard_error();
  self::leaf_target_is_rejected_even_without_portal_content();
  self::missing_rebind_is_transactional();
  self::aliased_rebind_is_rejected();
}

fn duplicate_registration_is_a_guard_error() {
  let mut reactant = runtime_support::reactant::<Game>(IdleSpawner);
  let id = ObjectId::new_v4();
  reactant.register_external_container(id);
  let duplicate = panic::catch_unwind(AssertUnwindSafe(|| {
    reactant.register_external_container(id);
  }))
  .expect_err("duplicate external target must panic");
  assert_eq!(
    self::panic_message(duplicate),
    "two external Reactant portal targets cannot share a container"
  );
  reactant.register_external_container(ObjectId::new_v4());
}

fn rebind_before_activation_is_a_guard_error() {
  let mut reactant = runtime_support::reactant::<Game>(IdleSpawner);
  let target = reactant.register_external_container(ObjectId::new_v4());
  let guarded = panic::catch_unwind(AssertUnwindSafe(|| {
    reactant.stage_external_container_rebind(&target, ObjectId::new_v4());
  }))
  .expect_err("initial external bindings cannot be staged");
  assert_eq!(
    self::panic_message(guarded),
    "Reactant runtime is not active"
  );
  reactant.register_external_container(ObjectId::new_v4());
}

fn leaf_target_is_rejected_even_without_portal_content() {
  let external = self::document().child(UiNode::new(ObjectId::new_v4(), UiLabel::new("leaf")));
  let target_id = external.children[0].object_id;
  let mut reactant = runtime_support::reactant::<Game>(IdleSpawner);
  reactant.register_external_container(target_id);
  let session = reactant.begin_session(&mut Game::default()).unwrap();
  let rejected = panic::catch_unwind(AssertUnwindSafe(|| {
    let _ = session.into_parts(self::snapshot(
      slice::from_ref(&external),
      vec![external.clone()],
    ));
  }))
  .expect_err("leaf external target must fail conversion");
  assert_eq!(
    self::panic_message(rejected),
    "external Reactant portal target must be a container"
  );
}

fn missing_rebind_is_transactional() {
  let source = self::document();
  let (external, target_id, prefix_id) = self::external_document("stable");
  let mut reactant = runtime_support::reactant(IdleSpawner);
  let target = reactant.register_external_container(target_id);
  let portal_target = target.clone();
  reactant.register_root(source.clone(), move |_: &Game| {
    create_portal(
      battlement_reactant::host::Label::new(trox::assert_localized("portal")),
      portal_target.clone(),
    )
  });
  let mut game = Game::default();
  let (initial, commit) = reactant
    .begin_session(&mut game)
    .unwrap()
    .into_parts(self::snapshot(
      &[source.clone(), external.clone()],
      vec![external],
    ));
  let mut world = UiWorld::default();
  world.replace(initial.ui).unwrap();
  self::apply(&mut world, commit);
  let before = world.element(target_id).unwrap().children().to_vec();
  reactant.stage_external_container_rebind(&target, ObjectId::new_v4());
  let session = reactant.begin_session(&mut game).unwrap();
  assert!(
    panic::catch_unwind(AssertUnwindSafe(|| {
      let _ = session.into_parts(self::snapshot(&[source], Vec::new()));
    }))
    .is_err()
  );
  assert_eq!(world.element(target_id).unwrap().children(), before);
  assert_eq!(before[0], prefix_id);
}

fn aliased_rebind_is_rejected() {
  let mut reactant = runtime_support::reactant::<Game>(IdleSpawner);
  let first_id = ObjectId::new_v4();
  let second_id = ObjectId::new_v4();
  let external = self::document().children([
    UiNode::new(first_id, UiVisualElement::new()),
    UiNode::new(second_id, UiVisualElement::new()),
  ]);
  let first = reactant.register_external_container(first_id);
  let second = reactant.register_external_container(second_id);
  let (_, commit) = reactant
    .begin_session(&mut Game::default())
    .unwrap()
    .into_parts(self::snapshot(
      slice::from_ref(&external),
      vec![external.clone()],
    ));
  assert!(commit.is_empty());
  let alias = ObjectId::new_v4();
  reactant.stage_external_container_rebind(&first, alias);
  reactant.stage_external_container_rebind(&second, alias);
  let aliased = panic::catch_unwind(AssertUnwindSafe(|| {
    let _session = reactant.begin_session(&mut Game::default());
  }))
  .expect_err("aliased reconnect targets must panic");
  assert_eq!(
    self::panic_message(aliased),
    "two external Reactant portal targets cannot share a container"
  );
}

fn apply(world: &mut UiWorld, commit: ReactantCommit) {
  self::apply_groups(world, commit.into_groups());
}

fn apply_groups(world: &mut UiWorld, groups: Vec<Vec<CommandBody>>) {
  for body in groups.into_iter().flatten() {
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

fn external_document(text: &str) -> (UiDocument, ObjectId, ObjectId) {
  let target_id = ObjectId::new_v4();
  let prefix_id = ObjectId::new_v4();
  (
    self::document().child(
      UiNode::new(target_id, UiVisualElement::new())
        .child(UiNode::new(prefix_id, UiLabel::new(text))),
    ),
    target_id,
    prefix_id,
  )
}

fn snapshot(documents: &[UiDocument], ui: Vec<UiDocument>) -> Snapshot {
  let scene_id = SceneId::new_v4();
  let camera_id = ObjectId::new_v4();
  let mut objects = vec![GameObject::new(camera_id, CameraState::new())];
  objects.extend(documents.iter().map(|document| {
    GameObject::new(
      document.document_id,
      GameObjectKind::UiDocument(
        UiDocumentState::new(document.root_id).panel_settings(
          PanelSettings::new().scale_mode(PanelScaleMode::ConstantLogicalPixelSize),
        ),
      ),
    )
    .parent_scene(ParentScene::Persistent)
  }));
  let mut snapshot = Snapshot::new(
    SessionId::new_v4(),
    vec![PreparedAsset::Scene("test/scene".into())],
    vec![Scene::new(scene_id, "test/scene")],
    objects,
    camera_id,
  );
  snapshot.ui = ui;
  snapshot
}

fn node(snapshot: &Snapshot, id: ObjectId) -> &UiNode {
  snapshot
    .ui
    .iter()
    .flat_map(|document| &document.children)
    .find(|node| node.object_id == id)
    .expect("top-level test node exists")
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
