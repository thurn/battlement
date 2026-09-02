use std::{
  cell::{Cell, RefCell},
  collections::VecDeque,
  fmt,
  num::NonZeroU64,
  panic::{self, AssertUnwindSafe},
  rc::Rc,
  sync::Arc,
};

use battlement::{
  AnchorName, CameraState, CameraTarget, ClientMessage, Command, CommandBody, Connect, DisplayId,
  DisplayOrientation, ElementGeometry, GameObject, GameObjectKind, GeometryGeneration,
  GeometryObservationBatch, GeometryObservationId, GeometryObservationResult,
  GeometryObservationTarget, GeometryObservationUpdate, GeometryObservationValue, GeometryRegistry,
  GeometryUnavailable, GeometryValue, ObjectId, PanelScaleMode, PanelSettings, ParentScene,
  PreparedAsset, Projective2, Prop, Rect, Response, ResponseMessage, Scene, SceneId, SessionId,
  Snapshot, UiDocument, UiDocumentState, UiElement, UiEventAction, UiEventResponse, UiNode,
  UiVisualElementProperties, ViewportGeometry, ViewportRect,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_native::{Engine, EngineError};
use battlement_reactant::{
  component::{self, Component},
  element_ref::{self, ElementRef},
  error_boundary::ErrorBoundary,
  executor::{BoxFuture, SpawnedTask, Spawner},
  external_store::{ExternalStore, StoreNotify, Subscription},
  geometry::{
    self, GeometrySnapshot, Measurement, MeasurementStatus, ViewportRef, WorldGeometry, WorldRef,
  },
  hooks,
  render::Render,
  runtime::{Reactant, RenderError, ResponseReactantExt},
};

type ViewportMeasurements = (
  Measurement<ViewportGeometry>,
  Vec<Measurement<ViewportGeometry>>,
);
type ViewportSnapshots = Rc<RefCell<Vec<GeometrySnapshot<ViewportMeasurements>>>>;
type ElementVectorSnapshots = Rc<RefCell<Vec<GeometrySnapshot<Vec<Measurement<ElementGeometry>>>>>>;
type ShapeMeasurements = (
  Measurement<ViewportGeometry>,
  [Measurement<ViewportGeometry>; 2],
  Vec<Measurement<ViewportGeometry>>,
  Measurement<WorldGeometry>,
);

struct IdleSpawner;

struct ScriptedEngine {
  connect: Option<Response>,
  polls: VecDeque<Response>,
}

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

impl Engine for ScriptedEngine {
  type ActionPayload = ();
  type ErrorCode = ();
  type Command = Command;

  fn connect(&mut self, _message: Connect) -> Result<Response, EngineError> {
    self
      .connect
      .take()
      .ok_or_else(|| EngineError::new("unexpected reconnect"))
  }

  fn submit(&mut self, _message: ClientMessage<(), ()>) -> Result<Response, EngineError> {
    Err(EngineError::new("unexpected submission"))
  }

  fn submit_ui_event(
    &mut self,
    message: UiEventAction,
  ) -> Result<UiEventResponse<Self::Command>, EngineError> {
    Ok(UiEventResponse::from_event(
      &message.event,
      Response::empty(message.session_id),
    ))
  }

  fn poll(&mut self) -> Result<Option<Response>, EngineError> {
    Ok(self.polls.pop_front())
  }
}

#[derive(Default)]
struct ViewportGame {
  displays: Vec<DisplayId>,
}

#[derive(Clone)]
struct ViewportFixture {
  displays: Vec<DisplayId>,
  snapshots: ViewportSnapshots,
}

#[derive(Default)]
struct ElementGame {
  key: u8,
}

#[derive(Clone)]
struct ElementFixture {
  key: u8,
  snapshots: Rc<RefCell<Vec<GeometrySnapshot<Measurement<ElementGeometry>>>>>,
}

#[derive(Clone)]
struct ShapeFixture {
  object_id: ObjectId,
}

#[derive(Clone)]
struct MemoGeometryFixture {
  renders: Rc<Cell<usize>>,
  snapshots: Rc<RefCell<Vec<GeometrySnapshot<Measurement<ElementGeometry>>>>>,
  element_ref: Rc<RefCell<Option<ElementRef>>>,
}

struct InvalidGeometryRead;

#[derive(Clone, Copy, PartialEq)]
struct StaticStore(u8);

#[derive(Default)]
struct RetryGame {
  fail: bool,
  host_key: u8,
  reports: usize,
  store: u8,
}

#[derive(Clone)]
struct RetryFixture {
  effects: Rc<Cell<usize>>,
  fail: bool,
  host_key: u8,
  snapshots: Rc<RefCell<Vec<GeometrySnapshot<Measurement<ElementGeometry>>>>>,
  store: u8,
}

#[derive(Default)]
struct TransitionGame {
  attach: bool,
  observe: bool,
}

#[derive(Clone)]
struct TransitionFixture {
  attach: bool,
  observe: bool,
  snapshots: ElementVectorSnapshots,
}

#[derive(Debug)]
struct RetryError;

impl fmt::Display for RetryError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("retry fixture error")
  }
}

impl std::error::Error for RetryError {}

impl ExternalStore for StaticStore {
  type Snapshot = u8;

  fn snapshot(&self) -> Self::Snapshot {
    self.0
  }

  fn subscribe(&self, _notify: StoreNotify) -> Subscription {
    Subscription::new(|| {})
  }
}

impl PartialEq for MemoGeometryFixture {
  fn eq(&self, other: &Self) -> bool {
    Rc::ptr_eq(&self.renders, &other.renders)
      && Rc::ptr_eq(&self.snapshots, &other.snapshots)
      && Rc::ptr_eq(&self.element_ref, &other.element_ref)
  }
}

impl Component for ViewportFixture {
  fn render(&self) -> impl Render {
    let (first, remaining) = self.displays.split_first().expect("fixture has a display");
    let snapshot = geometry::use_geometry((
      ViewportRef::display(*first),
      remaining
        .iter()
        .copied()
        .map(ViewportRef::display)
        .collect::<Vec<_>>(),
    ));
    self.snapshots.borrow_mut().push(snapshot.clone());
    battlement_reactant::host::Label::new(format!("generation {:?}", snapshot.generation))
  }
}

impl Component for ElementFixture {
  fn render(&self) -> impl Render {
    let element_ref = element_ref::use_element_ref();
    let snapshot = geometry::use_geometry(element_ref.clone());
    self.snapshots.borrow_mut().push(snapshot.clone());
    battlement_reactant::host::Label::new(format!("status {:?}", snapshot.measurements.status))
      .name("target")
      .key(self.key)
      .element_ref(element_ref)
  }
}

impl Component for ShapeFixture {
  fn render(&self) -> impl Render {
    let display = ViewportRef::display(DisplayId(0));
    let snapshot = geometry::use_geometry((
      display,
      [display, ViewportRef::display(DisplayId(1))],
      vec![display, display],
      WorldRef::named_anchor(
        self.object_id,
        AnchorName("head".to_owned()),
        CameraTarget::Input,
      ),
    ));
    let _: ShapeMeasurements = snapshot.measurements;
    battlement_reactant::host::Label::new("shape")
  }
}

impl Component for MemoGeometryFixture {
  fn render(&self) -> impl Render {
    self.renders.set(self.renders.get() + 1);
    let element_ref = element_ref::use_element_ref();
    let snapshot = geometry::use_geometry(element_ref.clone());
    self.snapshots.borrow_mut().push(snapshot.clone());
    self.element_ref.replace(Some(element_ref.clone()));
    battlement_reactant::host::Label::new(format!("status {:?}", snapshot.measurements.status))
      .name("memo-target")
      .element_ref(element_ref)
  }
}

impl Component for InvalidGeometryRead {
  fn render(&self) -> impl Render {
    let element_ref = element_ref::use_element_ref();
    let _ = element_ref.geometry();
    battlement_reactant::host::Label::new("invalid").element_ref(element_ref)
  }
}

impl Component for RetryFixture {
  fn render(&self) -> impl Render {
    let element_ref = element_ref::use_element_ref();
    let snapshot = geometry::use_geometry(element_ref.clone());
    self.snapshots.borrow_mut().push(snapshot);
    let effects = Rc::clone(&self.effects);
    hooks::use_effect_always(move || effects.set(effects.get() + 1));
    let _ = hooks::use_external_store(StaticStore(self.store));
    (
      battlement_reactant::host::Label::new("retry target")
        .key(self.host_key)
        .element_ref(element_ref),
      ErrorBoundary::new(|_: &RenderError| battlement_reactant::host::Label::new("fallback"))
        .on_error(|game: &mut RetryGame, _| game.reports += 1)
        .child(if self.fail {
          Err(RetryError)
        } else {
          Ok(battlement_reactant::host::Label::new("primary"))
        }),
    )
  }
}

impl Component for TransitionFixture {
  fn render(&self) -> impl Render {
    let element_ref = element_ref::use_element_ref();
    let targets = if self.observe {
      vec![element_ref.clone()]
    } else {
      Vec::new()
    };
    self
      .snapshots
      .borrow_mut()
      .push(geometry::use_geometry(targets));
    (
      self
        .attach
        .then(|| battlement_reactant::host::Label::new("attached").element_ref(element_ref)),
      (!self.attach).then(|| battlement_reactant::host::Label::new("detached")),
    )
  }
}

#[test]
fn target_shapes_deduplicate_equal_values_and_diff_in_registry_order() {
  let document = self::document();
  let mut shape = Reactant::new(IdleSpawner);
  let object_id = ObjectId::new_v4();
  shape.register_root(document.clone(), move |_| ShapeFixture { object_id });
  let (_, groups) = self::begin(&mut shape, &mut (), &document);
  let updates = self::updates(&groups);
  assert_eq!(updates.len(), 1);
  assert_eq!(updates[0].added.len(), 3);
  assert!(matches!(
    updates[0].added[2].target,
    GeometryObservationTarget::WorldAnchor { .. }
  ));

  let snapshots = Rc::new(RefCell::new(Vec::new()));
  let view_snapshots = Rc::clone(&snapshots);
  let mut game = ViewportGame {
    displays: vec![DisplayId(0), DisplayId(1)],
  };
  let mut runtime = Reactant::new(IdleSpawner);
  runtime.register_root(document.clone(), move |game: &ViewportGame| {
    ViewportFixture {
      displays: game.displays.clone(),
      snapshots: Rc::clone(&view_snapshots),
    }
  });
  let (_, groups) = self::begin(&mut runtime, &mut game, &document);
  let initial = self::updates(&groups)[0].clone();
  let mut registry = GeometryRegistry::default();
  registry.apply_update(&initial).unwrap();
  let _ = runtime.poll(&mut game).unwrap().into_groups();

  game.displays = vec![DisplayId(1), DisplayId(2)];
  let groups = runtime.refresh(&mut game).unwrap().into_groups();
  let updates = self::updates(&groups);
  assert_eq!(updates.len(), 2);
  assert_eq!(
    groups.first().and_then(|group| group.first()),
    Some(&CommandBody::GeometryObservationUpdate(updates[0].clone()))
  );
  assert_eq!(
    groups.last().and_then(|group| group.first()),
    Some(&CommandBody::GeometryObservationUpdate(updates[1].clone()))
  );
  assert_eq!(updates[0].removed, [initial.added[0].observation_id]);
  assert_eq!(updates[1].added.len(), 1);
  registry.apply_update(&updates[0]).unwrap();
  registry.apply_update(&updates[1]).unwrap();
  assert!(registry.get(initial.added[0].observation_id).is_none());
  assert!(registry.get(initial.added[1].observation_id).is_some());
  assert!(registry.get(updates[1].added[0].observation_id).is_some());

  let _ = runtime.poll(&mut game).unwrap().into_groups();
  game.displays = vec![DisplayId(2), DisplayId(1), DisplayId(1)];
  let groups = runtime.refresh(&mut game).unwrap().into_groups();
  assert!(self::updates(&groups).is_empty());
  let groups = runtime.refresh(&mut game).unwrap().into_groups();
  assert!(self::updates(&groups).is_empty());
  assert!(snapshots.borrow().iter().all(|snapshot| {
    snapshot.generation.is_none()
      && snapshot.measurements.0.status == MeasurementStatus::Waiting
      && snapshot
        .measurements
        .1
        .iter()
        .all(|measurement| measurement.status == MeasurementStatus::Waiting)
  }));
  let _ = shape.shutdown(&mut ()).into_groups();
  let _ = runtime.shutdown(&mut game).into_groups();
}

#[test]
fn snapshots_publish_only_complete_generations_and_status_changes() {
  let snapshots = Rc::new(RefCell::new(Vec::new()));
  let view_snapshots = Rc::clone(&snapshots);
  let document = self::document();
  let mut game = ViewportGame {
    displays: vec![DisplayId(0), DisplayId(1)],
  };
  let mut runtime = Reactant::new(IdleSpawner);
  runtime.register_root(document.clone(), move |game: &ViewportGame| {
    ViewportFixture {
      displays: game.displays.clone(),
      snapshots: Rc::clone(&view_snapshots),
    }
  });
  let (_, groups) = self::begin(&mut runtime, &mut game, &document);
  let added = self::updates(&groups)[0].added.clone();
  let _ = runtime.poll(&mut game).unwrap().into_groups();
  let waiting_renders = snapshots.borrow().len();

  assert!(
    runtime
      .observe_geometry(
        &mut game,
        GeometryObservationBatch {
          generation: self::generation(1),
          changed: vec![self::viewport_value(added[0].observation_id, 0, 10.0)],
        },
      )
      .unwrap()
      .is_empty()
  );
  assert_eq!(snapshots.borrow().len(), waiting_renders);

  let _ = runtime
    .observe_geometry(
      &mut game,
      GeometryObservationBatch {
        generation: self::generation(2),
        changed: vec![self::viewport_value(added[1].observation_id, 1, 20.0)],
      },
    )
    .unwrap()
    .into_groups();
  let recorded = snapshots.borrow();
  let complete = recorded.last().unwrap();
  assert_eq!(complete.generation, Some(self::generation(2)));
  assert_eq!(complete.measurements.0.latest.unwrap().viewport.x, 10.0);
  assert_eq!(complete.measurements.1[0].latest.unwrap().viewport.x, 20.0);
  assert!(
    [
      complete.measurements.0.status,
      complete.measurements.1[0].status
    ]
    .iter()
    .all(|status| *status == MeasurementStatus::Current)
  );
  let first_current = complete.measurements.0;
  drop(recorded);

  let _ = runtime
    .observe_geometry(
      &mut game,
      GeometryObservationBatch {
        generation: self::generation(3),
        changed: vec![GeometryObservationValue {
          observation_id: added[1].observation_id,
          result: GeometryObservationResult::Unavailable(GeometryUnavailable::DisplayUnavailable),
        }],
      },
    )
    .unwrap()
    .into_groups();
  let recorded = snapshots.borrow();
  let unavailable = recorded.last().unwrap();
  assert_eq!(unavailable.generation, Some(self::generation(3)));
  assert_eq!(
    unavailable.measurements.0.status,
    MeasurementStatus::Current
  );
  assert_eq!(
    unavailable.measurements.1[0].status,
    MeasurementStatus::Unavailable(GeometryUnavailable::DisplayUnavailable)
  );
  assert_eq!(
    unavailable.measurements.1[0].latest.unwrap().viewport.x,
    20.0
  );
  let second_unavailable = unavailable.measurements.1[0];
  drop(recorded);

  let _ = runtime
    .observe_geometry(
      &mut game,
      GeometryObservationBatch {
        generation: self::generation(4),
        changed: Vec::new(),
      },
    )
    .unwrap()
    .into_groups();
  let recorded = snapshots.borrow();
  let unchanged = recorded.last().unwrap();
  assert_eq!(unchanged.generation, Some(self::generation(4)));
  assert_eq!(unchanged.measurements.0.latest, first_current.latest);
  assert_eq!(unchanged.measurements.1[0], second_unavailable);
  let _ = runtime.shutdown(&mut game).into_groups();
}

#[test]
fn element_reattachment_retires_before_destroy_and_registers_after_create() {
  let snapshots = Rc::new(RefCell::new(Vec::new()));
  let view_snapshots = Rc::clone(&snapshots);
  let document = self::document();
  let mut game = ElementGame::default();
  let mut runtime = Reactant::new(IdleSpawner);
  runtime.register_root(document.clone(), move |game: &ElementGame| ElementFixture {
    key: game.key,
    snapshots: Rc::clone(&view_snapshots),
  });
  let session_id = SessionId::new_v4();
  let initial_response = runtime
    .begin_session(&mut game)
    .unwrap()
    .into_response(self::snapshot_for(&document, session_id));
  let snapshot = self::response_snapshot(&initial_response);
  let groups = self::response_groups(&initial_response);
  let initial = self::updates(&groups)[0].clone();
  let first_host = self::named_id(snapshot, "target");
  assert!(matches!(
    initial.added[0].target,
    GeometryObservationTarget::UiElement { object_id } if object_id == first_host
  ));
  let _ = runtime.poll(&mut game).unwrap().into_groups();
  let _ = runtime
    .observe_geometry(
      &mut game,
      GeometryObservationBatch {
        generation: self::generation(1),
        changed: vec![GeometryObservationValue {
          observation_id: initial.added[0].observation_id,
          result: GeometryObservationResult::Current(GeometryValue::Element(
            self::element_geometry(),
          )),
        }],
      },
    )
    .unwrap()
    .into_groups();
  assert_eq!(
    snapshots.borrow().last().unwrap().measurements.status,
    MeasurementStatus::Current
  );

  game.key = 1;
  let refresh_response =
    Response::empty(session_id).append_reactant(runtime.refresh(&mut game).unwrap());
  let groups = self::response_groups(&refresh_response);
  let updates = self::updates(&groups);
  assert_eq!(updates.len(), 2);
  assert!(matches!(
    groups[0][0],
    CommandBody::GeometryObservationUpdate(_)
  ));
  assert!(
    groups[1..groups.len() - 1]
      .iter()
      .flatten()
      .any(|body| matches!(body, CommandBody::VisualElementDestroy(_)))
  );
  assert!(
    groups[1..groups.len() - 1]
      .iter()
      .flatten()
      .any(|body| matches!(body, CommandBody::VisualElementCreate(_)))
  );
  assert!(matches!(
    groups.last().unwrap()[0],
    CommandBody::GeometryObservationUpdate(_)
  ));
  assert_eq!(updates[0].removed, [initial.added[0].observation_id]);
  assert_ne!(
    updates[1].added[0].observation_id,
    initial.added[0].observation_id
  );
  let waiting = snapshots.borrow().last().unwrap().clone();
  assert_eq!(waiting.generation, None);
  assert_eq!(waiting.measurements.status, MeasurementStatus::Waiting);
  assert_eq!(waiting.measurements.latest, None);

  let mut client = FakeClient::connect(
    ScriptedEngine {
      connect: Some(initial_response),
      polls: VecDeque::from([refresh_response]),
    },
    self::catalog(),
  );
  assert!(
    client
      .geometry_registry()
      .get(initial.added[0].observation_id)
      .is_some()
  );
  let command_start = client.commands().len();
  client.poll();
  assert!(
    client
      .geometry_registry()
      .get(initial.added[0].observation_id)
      .is_none()
  );
  assert!(
    client
      .geometry_registry()
      .get(updates[1].added[0].observation_id)
      .is_some()
  );
  let commands = &client.commands()[command_start..];
  let removed = commands
    .iter()
    .position(|entry| {
      matches!(
        &entry.command.body,
        CommandBody::GeometryObservationUpdate(update) if !update.removed.is_empty()
      )
    })
    .unwrap();
  let destroyed = commands
    .iter()
    .position(|entry| matches!(entry.command.body, CommandBody::VisualElementDestroy(_)))
    .unwrap();
  let created = commands
    .iter()
    .position(|entry| matches!(entry.command.body, CommandBody::VisualElementCreate(_)))
    .unwrap();
  let added = commands
    .iter()
    .position(|entry| {
      matches!(
        &entry.command.body,
        CommandBody::GeometryObservationUpdate(update) if !update.added.is_empty()
      )
    })
    .unwrap();
  assert!(removed < destroyed && destroyed < created && created < added);
  let created_id = commands
    .iter()
    .find_map(|entry| match &entry.command.body {
      CommandBody::VisualElementCreate(value) => Some(value.node.object_id),
      _ => None,
    })
    .unwrap();
  assert_eq!(
    client.ui().element(created_id).text(),
    Some("status Waiting")
  );

  let current = self::element_geometry();
  let _ = runtime
    .observe_geometry(
      &mut game,
      GeometryObservationBatch {
        generation: self::generation(2),
        changed: vec![GeometryObservationValue {
          observation_id: updates[1].added[0].observation_id,
          result: GeometryObservationResult::Current(GeometryValue::Element(current)),
        }],
      },
    )
    .unwrap()
    .into_groups();

  game.key = 2;
  let reconnect_session = SessionId::new_v4();
  let reconnect_response = runtime
    .begin_session(&mut game)
    .unwrap()
    .into_response(self::snapshot_for(&document, reconnect_session));
  let reconnect_updates = self::updates(&self::response_groups(&reconnect_response));
  assert_ne!(
    reconnect_updates[0].added[0].observation_id,
    updates[1].added[0].observation_id
  );
  let reconnect_snapshot = self::response_snapshot(&reconnect_response);
  let reconnect_host = self::named_id(reconnect_snapshot, "target");
  assert_eq!(
    reconnect_snapshot
      .ui
      .iter()
      .flat_map(|document| &document.children)
      .find_map(|node| self::node_text(node, reconnect_host)),
    Some("status Waiting")
  );
  let recorded = snapshots.borrow();
  let waiting = recorded.last().unwrap();
  assert_eq!(waiting.measurements.status, MeasurementStatus::Waiting);
  assert_eq!(waiting.measurements.latest, None);
  let _ = runtime.shutdown(&mut game).into_groups();
}

#[test]
fn element_cache_survives_reconnect_and_geometry_defeats_memo_bailout() {
  let renders = Rc::new(Cell::new(0));
  let snapshots = Rc::new(RefCell::new(Vec::new()));
  let captured_ref = Rc::new(RefCell::new(None));
  let document = self::document();
  let mut runtime = Reactant::new(IdleSpawner);
  let view_renders = Rc::clone(&renders);
  let view_snapshots = Rc::clone(&snapshots);
  let view_ref = Rc::clone(&captured_ref);
  runtime.register_root(document.clone(), move |_| {
    component::memo(MemoGeometryFixture {
      renders: Rc::clone(&view_renders),
      snapshots: Rc::clone(&view_snapshots),
      element_ref: Rc::clone(&view_ref),
    })
  });
  let (_, groups) = self::begin(&mut runtime, &mut (), &document);
  let initial_id = self::updates(&groups)[0].added[0].observation_id;
  let _ = runtime.poll(&mut ()).unwrap().into_groups();
  let before_sample = renders.get();
  let current_value = self::element_geometry();
  let _ = runtime
    .observe_geometry(
      &mut (),
      GeometryObservationBatch {
        generation: self::generation(1),
        changed: vec![GeometryObservationValue {
          observation_id: initial_id,
          result: GeometryObservationResult::Current(GeometryValue::Element(current_value)),
        }],
      },
    )
    .unwrap()
    .into_groups();
  assert_eq!(renders.get(), before_sample + 1);
  let element_ref = captured_ref.borrow().clone().unwrap();
  assert_eq!(
    element_ref.geometry(),
    Measurement {
      latest: Some(current_value),
      status: MeasurementStatus::Current,
    }
  );

  let (_, groups) = self::begin(&mut runtime, &mut (), &document);
  let reconnect_id = self::updates(&groups)[0].added[0].observation_id;
  assert_ne!(reconnect_id, initial_id);
  let waiting = snapshots.borrow().last().unwrap().measurements;
  assert_eq!(waiting.latest, Some(current_value));
  assert_eq!(waiting.status, MeasurementStatus::Waiting);
  assert_eq!(element_ref.geometry(), waiting);

  let _ = runtime
    .observe_geometry(
      &mut (),
      GeometryObservationBatch {
        generation: self::generation(2),
        changed: vec![GeometryObservationValue {
          observation_id: initial_id,
          result: GeometryObservationResult::Unavailable(GeometryUnavailable::Detached),
        }],
      },
    )
    .unwrap()
    .into_groups();
  assert_eq!(element_ref.geometry(), waiting);

  let mut moved_value = current_value;
  moved_value.viewport_bound.x = 50.0;
  let _ = runtime
    .observe_geometry(
      &mut (),
      GeometryObservationBatch {
        generation: self::generation(1),
        changed: vec![GeometryObservationValue {
          observation_id: reconnect_id,
          result: GeometryObservationResult::Current(GeometryValue::Element(moved_value)),
        }],
      },
    )
    .unwrap()
    .into_groups();
  assert_eq!(element_ref.geometry().latest, Some(moved_value));
  assert_eq!(element_ref.geometry().status, MeasurementStatus::Current);

  let invalid_document = self::document();
  let mut invalid = Reactant::new(IdleSpawner);
  invalid.register_root(invalid_document.clone(), |_| InvalidGeometryRead);
  let panic = panic::catch_unwind(AssertUnwindSafe(|| {
    let _ = invalid.begin_session(&mut ());
  }));
  assert!(panic.is_err());
  let _ = runtime.shutdown(&mut ()).into_groups();
}

#[test]
fn reconnect_preview_is_transactional_for_effect_store_and_boundary_hooks() {
  let effects = Rc::new(Cell::new(0));
  let snapshots = Rc::new(RefCell::new(Vec::new()));
  let document = self::document();
  let mut game = RetryGame::default();
  let view_effects = Rc::clone(&effects);
  let view_snapshots = Rc::clone(&snapshots);
  let mut runtime = Reactant::new(IdleSpawner);
  runtime.register_root(document.clone(), move |game: &RetryGame| RetryFixture {
    effects: Rc::clone(&view_effects),
    fail: game.fail,
    host_key: game.host_key,
    snapshots: Rc::clone(&view_snapshots),
    store: game.store,
  });
  let (_, groups) = self::begin(&mut runtime, &mut game, &document);
  let observation_id = self::updates(&groups)[0].added[0].observation_id;
  let _ = runtime.poll(&mut game).unwrap().into_groups();
  let _ = runtime
    .observe_geometry(
      &mut game,
      GeometryObservationBatch {
        generation: self::generation(1),
        changed: vec![GeometryObservationValue {
          observation_id,
          result: GeometryObservationResult::Current(GeometryValue::Element(
            self::element_geometry(),
          )),
        }],
      },
    )
    .unwrap()
    .into_groups();

  game.fail = true;
  game.host_key = 1;
  game.store = 1;
  let response = runtime
    .begin_session(&mut game)
    .unwrap()
    .into_response(self::snapshot_for(&document, SessionId::new_v4()));
  assert_eq!(
    snapshots.borrow().last().unwrap().measurements,
    Measurement {
      latest: None,
      status: MeasurementStatus::Waiting,
    }
  );
  assert_eq!(self::updates(&self::response_groups(&response)).len(), 1);
  let _ = runtime.poll(&mut game).unwrap().into_groups();
  assert_eq!(game.reports, 1);
  assert!(effects.get() > 0);
  let _ = runtime.shutdown(&mut game).into_groups();
}

#[test]
fn reconnect_preview_tracks_cached_target_readdition_and_detachment() {
  let document = self::document();
  let snapshots = Rc::new(RefCell::new(Vec::new()));
  let view_snapshots = Rc::clone(&snapshots);
  let mut game = TransitionGame {
    attach: true,
    observe: true,
  };
  let mut runtime = Reactant::new(IdleSpawner);
  runtime.register_root(document.clone(), move |game: &TransitionGame| {
    TransitionFixture {
      attach: game.attach,
      observe: game.observe,
      snapshots: Rc::clone(&view_snapshots),
    }
  });
  let (_, groups) = self::begin(&mut runtime, &mut game, &document);
  let observation_id = self::updates(&groups)[0].added[0].observation_id;
  let _ = runtime.poll(&mut game).unwrap().into_groups();
  let current = self::element_geometry();
  let _ = runtime
    .observe_geometry(
      &mut game,
      GeometryObservationBatch {
        generation: self::generation(1),
        changed: vec![GeometryObservationValue {
          observation_id,
          result: GeometryObservationResult::Current(GeometryValue::Element(current)),
        }],
      },
    )
    .unwrap()
    .into_groups();

  game.observe = false;
  let _ = runtime.refresh(&mut game).unwrap().into_groups();
  game.observe = true;
  let _ = runtime
    .begin_session(&mut game)
    .unwrap()
    .into_response(self::snapshot_for(&document, SessionId::new_v4()));
  let recorded = snapshots.borrow();
  let readded = recorded.last().unwrap().measurements[0];
  assert_eq!(readded.latest, Some(current));
  assert_eq!(readded.status, MeasurementStatus::Waiting);
  drop(recorded);

  game.attach = false;
  let _ = runtime
    .begin_session(&mut game)
    .unwrap()
    .into_response(self::snapshot_for(&document, SessionId::new_v4()));
  let recorded = snapshots.borrow();
  assert_eq!(
    recorded.last().unwrap().measurements[0],
    Measurement {
      latest: None,
      status: MeasurementStatus::Waiting,
    }
  );
  let _ = runtime.shutdown(&mut game).into_groups();
}

fn updates(groups: &[Vec<CommandBody>]) -> Vec<GeometryObservationUpdate> {
  groups
    .iter()
    .flatten()
    .filter_map(|body| match body {
      CommandBody::GeometryObservationUpdate(update) => Some(update.clone()),
      _ => None,
    })
    .collect()
}

fn response_groups(response: &Response) -> Vec<Vec<CommandBody>> {
  response
    .messages
    .iter()
    .filter_map(|message| match message {
      ResponseMessage::Snapshot(_) => None,
      ResponseMessage::Batch(batch) => Some(&batch.groups),
    })
    .flatten()
    .map(|group| {
      group
        .commands
        .iter()
        .map(|command| command.body.clone())
        .collect()
    })
    .collect()
}

fn response_snapshot(response: &Response) -> &Snapshot {
  response
    .messages
    .iter()
    .find_map(|message| match message {
      ResponseMessage::Snapshot(snapshot) => Some(snapshot),
      ResponseMessage::Batch(_) => None,
    })
    .expect("session response should contain a snapshot")
}

fn generation(value: u64) -> GeometryGeneration {
  GeometryGeneration(NonZeroU64::new(value).expect("generation is nonzero"))
}

fn viewport_value(
  observation_id: GeometryObservationId,
  display: u32,
  x: f64,
) -> GeometryObservationValue {
  GeometryObservationValue {
    observation_id,
    result: GeometryObservationResult::Current(GeometryValue::Viewport(ViewportGeometry {
      viewport: ViewportRect {
        x,
        y: 0.0,
        width: 100.0,
        height: 100.0,
        display_id: DisplayId(display),
      },
      safe_area: ViewportRect {
        x,
        y: 0.0,
        width: 100.0,
        height: 90.0,
        display_id: DisplayId(display),
      },
      scale: 1.0,
      dpi: Some(96.0),
      orientation: DisplayOrientation::Landscape,
    })),
  }
}

fn element_geometry() -> ElementGeometry {
  ElementGeometry {
    layout: Rect::new(1.0, 2.0, 30.0, 40.0),
    viewport_bound: ViewportRect {
      x: 1.0,
      y: 2.0,
      width: 30.0,
      height: 40.0,
      display_id: DisplayId(0),
    },
    viewport_from_local: self::identity_projective(),
    viewport_from_parent: self::identity_projective(),
    panel_id: ObjectId::new_v4(),
  }
}

fn identity_projective() -> Projective2 {
  Projective2 {
    m11: 1.0,
    m12: 0.0,
    m13: 0.0,
    m21: 0.0,
    m22: 1.0,
    m23: 0.0,
    m31: 0.0,
    m32: 0.0,
    m33: 1.0,
  }
}

fn begin<G: 'static>(
  runtime: &mut Reactant<G>,
  game: &mut G,
  document: &UiDocument,
) -> (Snapshot, Vec<Vec<CommandBody>>) {
  let (snapshot, commit) = runtime
    .begin_session(game)
    .unwrap()
    .into_parts(self::snapshot(document));
  (snapshot, commit.into_groups())
}

fn named_id(snapshot: &Snapshot, name: &str) -> ObjectId {
  snapshot
    .ui
    .iter()
    .flat_map(|document| &document.children)
    .find_map(|node| self::named_node_id(node, name))
    .expect("named host should exist")
}

fn named_node_id(node: &UiNode, name: &str) -> Option<ObjectId> {
  if matches!(&node.element.visual_element().name, Prop::Set(value) if value == name) {
    return Some(node.object_id);
  }
  node
    .children
    .iter()
    .find_map(|child| self::named_node_id(child, name))
}

fn node_text(node: &UiNode, object_id: ObjectId) -> Option<&str> {
  if node.object_id == object_id {
    let UiElement::Label(label) = &node.element else {
      return None;
    };
    return match &label.text {
      Prop::Set(value) => Some(value),
      Prop::Unset | Prop::Reset => None,
    };
  }
  node
    .children
    .iter()
    .find_map(|child| self::node_text(child, object_id))
}

fn document() -> UiDocument {
  UiDocument::with_root_id(ObjectId::new_v4(), ObjectId::new_v4())
}

fn snapshot(document: &UiDocument) -> Snapshot {
  self::snapshot_for(document, SessionId::new_v4())
}

fn snapshot_for(document: &UiDocument, session_id: SessionId) -> Snapshot {
  let camera_id = ObjectId::new_v4();
  Snapshot::new(
    session_id,
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

fn catalog() -> Arc<FakeAssetCatalog> {
  let mut catalog = FakeAssetCatalog::new();
  catalog.add_scene("test/scene");
  Arc::new(catalog)
}
