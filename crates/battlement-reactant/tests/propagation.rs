use std::{io, ptr};

use battlement::{
  CameraState, ClickEvent, FocusEvent, GameObject, GameObjectKind, ObjectId, PanelPoint,
  PanelScaleMode, PanelSettings, ParentScene, PointerCrossingEvent, PointerType, PreparedAsset,
  Scene, SceneId, SessionId, Snapshot, TextField, UiDocument, UiDocumentState, UiEvent,
  UiEventBody, VisualElement, VisualElementProperties,
};
use battlement_reactant::{
  event::{EventPhase, EventRenderExt, ReactantEvent},
  executor::{BoxFuture, SpawnedTask, Spawner},
  primitive::ContainerRenderExt,
  runtime::Reactant,
};

struct IdleSpawner;

#[derive(Default)]
struct Ledger {
  entries: Vec<Entry>,
  click_events: Vec<ReactantEvent<ClickEvent>>,
  fail_render: bool,
  stop_at: Option<&'static str>,
}

#[derive(Debug, Eq, PartialEq)]
struct Entry {
  label: &'static str,
  phase: EventPhase,
  target: ObjectId,
  current: ObjectId,
  related: Option<ObjectId>,
}

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

#[test]
fn capture_target_bubble_and_focus_use_the_logical_host_path() {
  let document = self::document();
  let mut ledger = Ledger::default();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), self::propagation_view);
  let snapshot = reactant
    .begin_session(&mut ledger)
    .expect("logical tree renders")
    .into_parts(self::snapshot(&document))
    .0;
  let outer = self::find_named(&snapshot.ui[0].children, "outer");
  let middle = self::find_named(&snapshot.ui[0].children, "middle");
  let target = self::find_named(&snapshot.ui[0].children, "target");

  self::dispatch(
    &mut reactant,
    &mut ledger,
    UiEvent::click(target, ClickEvent::NavigationSubmit),
  );
  assert_eq!(
    ledger.entries,
    vec![
      self::entry("outer-capture", EventPhase::Capture, target, outer, None),
      self::entry("middle-capture", EventPhase::Capture, target, middle, None),
      self::entry("target-capture", EventPhase::Target, target, target, None),
      self::entry("target", EventPhase::Target, target, target, None),
      self::entry("middle", EventPhase::Bubble, target, middle, None),
      self::entry("outer", EventPhase::Bubble, target, outer, None),
    ]
  );
  let first_payload = ledger.click_events[0].payload();
  assert!(
    ledger
      .click_events
      .iter()
      .skip(1)
      .all(|event| ptr::eq(first_payload, event.payload()))
  );

  ledger.entries.clear();
  ledger.stop_at = Some("middle-capture");
  self::dispatch(
    &mut reactant,
    &mut ledger,
    UiEvent::click(target, ClickEvent::NavigationSubmit),
  );
  assert_eq!(
    ledger.entries,
    vec![
      self::entry("outer-capture", EventPhase::Capture, target, outer, None),
      self::entry("middle-capture", EventPhase::Capture, target, middle, None),
    ]
  );

  ledger.entries.clear();
  ledger.stop_at = None;
  self::dispatch(
    &mut reactant,
    &mut ledger,
    UiEvent {
      target_id: target,
      body: UiEventBody::FocusIn(FocusEvent::default()),
    },
  );
  assert_eq!(
    ledger.entries,
    vec![
      self::entry("target-focus", EventPhase::Target, target, target, None),
      self::entry("middle-focus", EventPhase::Bubble, target, middle, None),
      self::entry("outer-focus", EventPhase::Bubble, target, outer, None),
    ]
  );
  let _ = reactant.shutdown(&mut ledger).into_groups();
}

#[test]
fn pointer_crossings_follow_logical_paths_and_deduplicate_only_complements() {
  let document = self::document();
  let mut ledger = Ledger::default();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), self::crossing_view);
  let snapshot = reactant
    .begin_session(&mut ledger)
    .expect("crossing tree renders")
    .into_parts(self::snapshot(&document))
    .0;
  let outer = self::find_named(&snapshot.ui[0].children, "crossing-outer");
  let a_parent = self::find_named(&snapshot.ui[0].children, "a-parent");
  let a_leaf = self::find_named(&snapshot.ui[0].children, "a-leaf");
  let _b_parent = self::find_named(&snapshot.ui[0].children, "b-parent");
  let b_leaf = self::find_named(&snapshot.ui[0].children, "b-leaf");

  self::cross(&mut reactant, &mut ledger, a_leaf, Some(b_leaf), false, 7);
  self::cross(&mut reactant, &mut ledger, b_leaf, Some(a_leaf), true, 7);
  assert_eq!(
    self::labels(&ledger),
    [
      "a-leaf-leave",
      "a-parent-leave",
      "b-parent-enter",
      "b-leaf-enter"
    ]
  );
  assert_eq!(ledger.entries[0].related, Some(b_leaf));
  assert_eq!(ledger.entries[2].related, Some(a_leaf));
  for entry in &ledger.entries {
    assert_eq!(entry.phase, EventPhase::Target);
    assert_eq!(entry.target, entry.current);
  }

  ledger.entries.clear();
  self::cross(&mut reactant, &mut ledger, a_leaf, Some(a_parent), false, 7);
  self::cross(&mut reactant, &mut ledger, a_parent, Some(a_leaf), true, 7);
  assert_eq!(self::labels(&ledger), ["a-leaf-leave"]);

  ledger.entries.clear();
  self::cross(&mut reactant, &mut ledger, b_leaf, None, false, 7);
  assert_eq!(
    self::labels(&ledger),
    ["b-leaf-leave", "b-parent-leave", "outer-leave"]
  );
  assert_eq!(ledger.entries[2].current, outer);

  ledger.entries.clear();
  ledger.stop_at = Some("a-parent-leave");
  self::cross(&mut reactant, &mut ledger, a_leaf, Some(b_leaf), false, 7);
  assert_eq!(self::labels(&ledger), ["a-leaf-leave", "a-parent-leave"]);

  ledger.entries.clear();
  ledger.stop_at = None;
  self::cross(&mut reactant, &mut ledger, a_leaf, Some(b_leaf), false, 7);
  self::dispatch(
    &mut reactant,
    &mut ledger,
    UiEvent::click(ObjectId::new_v4(), ClickEvent::NavigationSubmit),
  );
  self::cross(&mut reactant, &mut ledger, b_leaf, Some(a_leaf), true, 7);
  assert_eq!(ledger.entries.len(), 8);

  ledger.entries.clear();
  self::dispatch(
    &mut reactant,
    &mut ledger,
    UiEvent::click(ObjectId::new_v4(), ClickEvent::NavigationSubmit),
  );
  self::cross(&mut reactant, &mut ledger, a_leaf, Some(b_leaf), false, 7);
  self::cross(&mut reactant, &mut ledger, b_leaf, Some(a_leaf), true, 8);
  assert_eq!(ledger.entries.len(), 8);
  let _ = reactant.shutdown(&mut ledger).into_groups();
}

#[test]
fn failed_reconnect_preserves_complementary_pointer_deduplication() {
  let document = self::document();
  let mut ledger = Ledger::default();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), self::crossing_view);
  let snapshot = reactant
    .begin_session(&mut ledger)
    .unwrap()
    .into_parts(self::snapshot(&document))
    .0;
  let a_leaf = self::find_named(&snapshot.ui[0].children, "a-leaf");
  let b_leaf = self::find_named(&snapshot.ui[0].children, "b-leaf");

  self::cross(&mut reactant, &mut ledger, a_leaf, Some(b_leaf), false, 7);
  ledger.entries.clear();
  ledger.fail_render = true;
  assert!(reactant.begin_session(&mut ledger).is_err());
  ledger.fail_render = false;
  self::cross(&mut reactant, &mut ledger, b_leaf, Some(a_leaf), true, 7);

  assert!(ledger.entries.is_empty());
  let _ = reactant.shutdown(&mut ledger).into_groups();
}

#[test]
fn complementary_pointer_events_keep_their_raw_capture_and_bubble_paths() {
  let document = self::document();
  let mut ledger = Ledger::default();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), self::raw_crossing_view);
  let snapshot = reactant
    .begin_session(&mut ledger)
    .expect("raw crossing tree renders")
    .into_parts(self::snapshot(&document))
    .0;
  let target = self::find_named(&snapshot.ui[0].children, "raw-a");
  let related = self::find_named(&snapshot.ui[0].children, "raw-b");

  self::cross(&mut reactant, &mut ledger, target, Some(related), false, 3);
  self::cross(&mut reactant, &mut ledger, related, Some(target), true, 3);

  assert_eq!(
    self::labels(&ledger),
    [
      "root-out-capture",
      "target-out-capture",
      "target-out",
      "root-out",
      "root-over-capture",
      "target-over-capture",
      "target-over",
      "root-over",
    ]
  );
  let _ = reactant.shutdown(&mut ledger).into_groups();
}

fn propagation_view(_ledger: &Ledger) -> impl battlement_reactant::render::Render + use<> {
  VisualElement::new()
    .name("outer")
    .child(
      VisualElement::new()
        .name("middle")
        .child(
          TextField::new()
            .name("target")
            .on_click_capture_event(self::record_click("target-capture"))
            .on_click_event(self::record_click("target"))
            .on_focus_event(self::record_focus("target-focus")),
        )
        .on_click_capture_event(self::record_click("middle-capture"))
        .on_click_event(self::record_click("middle"))
        .on_focus_event(self::record_focus("middle-focus")),
    )
    .on_click_capture_event(self::record_click("outer-capture"))
    .on_click_event(self::record_click("outer"))
    .on_focus_event(self::record_focus("outer-focus"))
}

fn crossing_view(
  ledger: &Ledger,
) -> Result<impl battlement_reactant::render::Render + use<>, io::Error> {
  if ledger.fail_render {
    return Err(io::Error::other("render failed"));
  }
  let a = VisualElement::new()
    .name("a-parent")
    .child(
      VisualElement::new()
        .name("a-leaf")
        .on_pointer_enter_event(self::record_crossing("a-leaf-enter"))
        .on_pointer_leave_event(self::record_crossing("a-leaf-leave")),
    )
    .on_pointer_enter_event(self::record_crossing("a-parent-enter"))
    .on_pointer_leave_event(self::record_crossing("a-parent-leave"));
  let b = VisualElement::new()
    .name("b-parent")
    .child(
      VisualElement::new()
        .name("b-leaf")
        .on_pointer_enter_event(self::record_crossing("b-leaf-enter"))
        .on_pointer_leave_event(self::record_crossing("b-leaf-leave")),
    )
    .on_pointer_enter_event(self::record_crossing("b-parent-enter"))
    .on_pointer_leave_event(self::record_crossing("b-parent-leave"));
  Ok(
    VisualElement::new()
      .name("crossing-outer")
      .child(a)
      .child(b)
      .on_pointer_enter_event(self::record_crossing("outer-enter"))
      .on_pointer_leave_event(self::record_crossing("outer-leave")),
  )
}

fn raw_crossing_view(_ledger: &Ledger) -> impl battlement_reactant::render::Render + use<> {
  VisualElement::new()
    .child(
      VisualElement::new()
        .name("raw-a")
        .on_pointer_out_capture_event(self::record_crossing("target-out-capture"))
        .on_pointer_out_event(self::record_crossing("target-out")),
    )
    .child(
      VisualElement::new()
        .name("raw-b")
        .on_pointer_over_capture_event(self::record_crossing("target-over-capture"))
        .on_pointer_over_event(self::record_crossing("target-over")),
    )
    .on_pointer_out_capture_event(self::record_crossing("root-out-capture"))
    .on_pointer_out_event(self::record_crossing("root-out"))
    .on_pointer_over_capture_event(self::record_crossing("root-over-capture"))
    .on_pointer_over_event(self::record_crossing("root-over"))
}

fn record_click(label: &'static str) -> impl Fn(&mut Ledger, ReactantEvent<ClickEvent>) + 'static {
  move |ledger, event| {
    ledger.click_events.push(event.clone());
    self::record(ledger, label, event, None);
  }
}

fn record_focus(label: &'static str) -> impl Fn(&mut Ledger, ReactantEvent<FocusEvent>) + 'static {
  move |ledger, event| {
    let related = event.payload().related_target_id;
    self::record(ledger, label, event, related);
  }
}

fn record_crossing(
  label: &'static str,
) -> impl Fn(&mut Ledger, ReactantEvent<PointerCrossingEvent>) + 'static {
  move |ledger, event| {
    let related = event.payload().related_target_id;
    self::record(ledger, label, event, related);
  }
}

fn record<E>(
  ledger: &mut Ledger,
  label: &'static str,
  event: ReactantEvent<E>,
  related: Option<ObjectId>,
) {
  ledger.entries.push(Entry {
    label,
    phase: event.phase(),
    target: event.target().object_id(),
    current: event.current_target().object_id(),
    related,
  });
  if ledger.stop_at == Some(label) {
    event.stop_propagation();
  }
}

fn cross(
  reactant: &mut Reactant<Ledger>,
  ledger: &mut Ledger,
  target_id: ObjectId,
  related_target_id: Option<ObjectId>,
  over: bool,
  pointer_id: i32,
) {
  let payload = PointerCrossingEvent {
    related_target_id,
    pointer_id,
    position: PanelPoint::default(),
    pointer_type: PointerType::Mouse,
  };
  self::dispatch(
    reactant,
    ledger,
    UiEvent {
      target_id,
      body: if over {
        UiEventBody::PointerOver(payload)
      } else {
        UiEventBody::PointerOut(payload)
      },
    },
  );
}

fn dispatch(reactant: &mut Reactant<Ledger>, ledger: &mut Ledger, event: UiEvent) {
  assert!(
    reactant
      .dispatch(ledger, event)
      .expect("event dispatch succeeds")
      .into_groups()
      .is_empty()
  );
}

fn entry(
  label: &'static str,
  phase: EventPhase,
  target: ObjectId,
  current: ObjectId,
  related: Option<ObjectId>,
) -> Entry {
  Entry {
    label,
    phase,
    target,
    current,
    related,
  }
}

fn labels(ledger: &Ledger) -> Vec<&'static str> {
  ledger.entries.iter().map(|entry| entry.label).collect()
}

fn find_named(nodes: &[battlement::UiNode], name: &str) -> ObjectId {
  nodes
    .iter()
    .find_map(|node| {
      (node.element.visual_element().name == battlement::Prop::Set(name.to_owned()))
        .then_some(node.object_id)
        .or_else(|| self::find_named_optional(&node.children, name))
    })
    .unwrap_or_else(|| panic!("missing node named {name}"))
}

fn find_named_optional(nodes: &[battlement::UiNode], name: &str) -> Option<ObjectId> {
  nodes.iter().find_map(|node| {
    (node.element.visual_element().name == battlement::Prop::Set(name.to_owned()))
      .then_some(node.object_id)
      .or_else(|| self::find_named_optional(&node.children, name))
  })
}

fn document() -> UiDocument {
  UiDocument::with_root_id(ObjectId::new_v4(), ObjectId::new_v4())
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
