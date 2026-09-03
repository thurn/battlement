use std::{io, ptr};

use battlement::{
  CameraState, ClickEvent, FocusEvent, GameObject, GameObjectKind, ObjectId, PanelPoint,
  PanelScaleMode, PanelSettings, ParentScene, PointerBoundaryEvent, PointerCrossingEvent,
  PointerType, PreparedAsset, Scene, SceneId, SessionId, Snapshot, UiDocument, UiDocumentState,
  UiEvent, UiEventBody, UiEventDisposition, UiVisualElementProperties,
};
use battlement_reactant::{
  event::{EventPhase, ReactantEvent},
  executor::{BoxFuture, SpawnedTask, Spawner},
  runtime::Reactant,
};

struct IdleSpawner;

#[derive(Default)]
struct Ledger {
  entries: Vec<Entry>,
  click_events: Vec<ReactantEvent<ClickEvent>>,
  cancelable_observations: Vec<bool>,
  prevented_observations: Vec<bool>,
  fail_render: bool,
  prevent_at: Option<&'static str>,
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
      cancelable: false,
      default_prevented: false,
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
fn default_prevention_is_shared_and_independent_of_logical_phase() {
  for prevent_at in ["outer-capture", "target", "outer"] {
    let document = self::document();
    let mut ledger = Ledger {
      prevent_at: Some(prevent_at),
      ..Ledger::default()
    };
    let mut reactant = Reactant::new(IdleSpawner);
    reactant.register_root(document.clone(), self::propagation_view);
    let snapshot = reactant
      .begin_session(&mut ledger)
      .expect("prevention tree renders")
      .into_parts(self::snapshot(&document))
      .0;
    let target = self::find_named(&snapshot.ui[0].children, "target");

    let result = reactant
      .dispatch(
        &mut ledger,
        UiEvent::click(target, ClickEvent::NavigationSubmit),
      )
      .expect("cancelable click dispatches");

    assert_eq!(result.disposition(), UiEventDisposition::PreventDefault);
    assert!(result.prevented_by_reactant());
    assert!(ledger.cancelable_observations.iter().all(|value| *value));
    assert!(
      ledger
        .click_events
        .iter()
        .all(ReactantEvent::default_prevented)
    );
    let _ = result.into_groups();
    let _ = reactant.shutdown(&mut ledger).into_groups();
  }
}

#[test]
fn incoming_prevention_survives_and_noncancelable_prevention_is_ignored() {
  let document = self::document();
  let mut ledger = Ledger::default();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), self::propagation_view);
  let snapshot = reactant
    .begin_session(&mut ledger)
    .expect("prevention tree renders")
    .into_parts(self::snapshot(&document))
    .0;
  let target = self::find_named(&snapshot.ui[0].children, "target");

  let incoming = reactant
    .dispatch(
      &mut ledger,
      UiEvent::new(
        target,
        true,
        true,
        UiEventBody::Click(ClickEvent::NavigationSubmit),
      ),
    )
    .expect("already-prevented click dispatches");
  assert_eq!(incoming.disposition(), UiEventDisposition::PreventDefault);
  assert!(!incoming.prevented_by_reactant());
  assert!(ledger.prevented_observations.iter().all(|value| *value));
  let _ = incoming.into_groups();

  ledger.click_events.clear();
  ledger.cancelable_observations.clear();
  ledger.prevented_observations.clear();
  ledger.prevent_at = Some("target");
  let noncancelable = reactant
    .dispatch(
      &mut ledger,
      UiEvent::new(
        target,
        false,
        false,
        UiEventBody::Click(ClickEvent::NavigationSubmit),
      ),
    )
    .expect("noncancelable click dispatches");
  assert_eq!(noncancelable.disposition(), UiEventDisposition::Continue);
  assert!(!noncancelable.prevented_by_reactant());
  assert!(ledger.cancelable_observations.iter().all(|value| !value));
  assert!(
    ledger
      .click_events
      .iter()
      .all(|event| !event.default_prevented())
  );
  let _ = noncancelable.into_groups();
  let _ = reactant.shutdown(&mut ledger).into_groups();
}

#[test]
fn pointer_boundaries_dispatch_only_the_reported_native_event() {
  let document = self::document();
  let mut ledger = Ledger::default();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), self::crossing_view);
  let snapshot = reactant
    .begin_session(&mut ledger)
    .expect("crossing tree renders")
    .into_parts(self::snapshot(&document))
    .0;
  let a_parent = self::find_named(&snapshot.ui[0].children, "a-parent");
  let a_leaf = self::find_named(&snapshot.ui[0].children, "a-leaf");
  let b_leaf = self::find_named(&snapshot.ui[0].children, "b-leaf");

  self::boundary(&mut reactant, &mut ledger, a_leaf, false, 7);
  assert_eq!(self::labels(&ledger), ["a-leaf-leave"]);
  assert_eq!(ledger.entries[0].phase, EventPhase::Target);
  assert_eq!(ledger.entries[0].target, a_leaf);
  assert_eq!(ledger.entries[0].current, a_leaf);

  ledger.entries.clear();
  self::boundary(&mut reactant, &mut ledger, a_parent, false, 7);
  self::boundary(&mut reactant, &mut ledger, b_leaf, true, 7);
  assert_eq!(self::labels(&ledger), ["a-parent-leave", "b-leaf-enter"]);

  ledger.entries.clear();
  self::cross(&mut reactant, &mut ledger, a_leaf, Some(b_leaf), false, 7);
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
  battlement_reactant::host::View::new()
    .name("outer")
    .child(
      battlement_reactant::host::View::new()
        .name("middle")
        .child(
          battlement_reactant::host::TextField::new()
            .name("target")
            .on_click_capture_event_with_model(self::record_click("target-capture"))
            .on_click_event_with_model(self::record_click("target"))
            .on_focus_in_event_with_model(self::record_focus("target-focus")),
        )
        .on_click_capture_event_with_model(self::record_click("middle-capture"))
        .on_click_event_with_model(self::record_click("middle"))
        .on_focus_in_event_with_model(self::record_focus("middle-focus")),
    )
    .on_click_capture_event_with_model(self::record_click("outer-capture"))
    .on_click_event_with_model(self::record_click("outer"))
    .on_focus_in_event_with_model(self::record_focus("outer-focus"))
}

fn crossing_view(
  ledger: &Ledger,
) -> Result<impl battlement_reactant::render::Render + use<>, io::Error> {
  if ledger.fail_render {
    return Err(io::Error::other("render failed"));
  }
  let a = battlement_reactant::host::View::new()
    .name("a-parent")
    .child(
      battlement_reactant::host::View::new()
        .name("a-leaf")
        .on_pointer_enter_event_with_model(self::record_boundary("a-leaf-enter"))
        .on_pointer_leave_event_with_model(self::record_boundary("a-leaf-leave")),
    )
    .on_pointer_enter_event_with_model(self::record_boundary("a-parent-enter"))
    .on_pointer_leave_event_with_model(self::record_boundary("a-parent-leave"));
  let b = battlement_reactant::host::View::new()
    .name("b-parent")
    .child(
      battlement_reactant::host::View::new()
        .name("b-leaf")
        .on_pointer_enter_event_with_model(self::record_boundary("b-leaf-enter"))
        .on_pointer_leave_event_with_model(self::record_boundary("b-leaf-leave")),
    )
    .on_pointer_enter_event_with_model(self::record_boundary("b-parent-enter"))
    .on_pointer_leave_event_with_model(self::record_boundary("b-parent-leave"));
  Ok(
    battlement_reactant::host::View::new()
      .name("crossing-outer")
      .child(a)
      .child(b)
      .on_pointer_enter_event_with_model(self::record_boundary("outer-enter"))
      .on_pointer_leave_event_with_model(self::record_boundary("outer-leave")),
  )
}

fn raw_crossing_view(_ledger: &Ledger) -> impl battlement_reactant::render::Render + use<> {
  battlement_reactant::host::View::new()
    .child(
      battlement_reactant::host::View::new()
        .name("raw-a")
        .on_pointer_out_capture_event_with_model(self::record_crossing("target-out-capture"))
        .on_pointer_out_event_with_model(self::record_crossing("target-out")),
    )
    .child(
      battlement_reactant::host::View::new()
        .name("raw-b")
        .on_pointer_over_capture_event_with_model(self::record_crossing("target-over-capture"))
        .on_pointer_over_event_with_model(self::record_crossing("target-over")),
    )
    .on_pointer_out_capture_event_with_model(self::record_crossing("root-out-capture"))
    .on_pointer_out_event_with_model(self::record_crossing("root-out"))
    .on_pointer_over_capture_event_with_model(self::record_crossing("root-over-capture"))
    .on_pointer_over_event_with_model(self::record_crossing("root-over"))
}

fn record_click(label: &'static str) -> impl Fn(&mut Ledger, ReactantEvent<ClickEvent>) + 'static {
  move |ledger, event| {
    ledger.cancelable_observations.push(event.cancelable());
    ledger
      .prevented_observations
      .push(event.default_prevented());
    if ledger.prevent_at == Some(label) {
      event.prevent_default();
    }
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

fn record_boundary(
  label: &'static str,
) -> impl Fn(&mut Ledger, ReactantEvent<PointerBoundaryEvent>) + 'static {
  move |ledger, event| self::record(ledger, label, event, None)
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
      cancelable: false,
      default_prevented: false,
      body: if over {
        UiEventBody::PointerOver(payload)
      } else {
        UiEventBody::PointerOut(payload)
      },
    },
  );
}

fn boundary(
  reactant: &mut Reactant<Ledger>,
  ledger: &mut Ledger,
  target_id: ObjectId,
  enter: bool,
  pointer_id: i32,
) {
  let payload = PointerBoundaryEvent {
    pointer_id,
    position: PanelPoint::default(),
    pointer_type: PointerType::Mouse,
  };
  self::dispatch(
    reactant,
    ledger,
    UiEvent::new(
      target_id,
      false,
      false,
      if enter {
        UiEventBody::PointerEnter(payload)
      } else {
        UiEventBody::PointerLeave(payload)
      },
    ),
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
        GameObjectKind::UiDocument(UiDocumentState::new(document.root_id).panel_settings(
          PanelSettings::new().scale_mode(PanelScaleMode::ConstantLogicalPixelSize),
        )),
      )
      .parent_scene(ParentScene::Persistent),
    ],
    camera_id,
  )
}
