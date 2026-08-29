use std::{
  cell::{Cell, RefCell},
  num::NonZeroU64,
  panic::{self, AssertUnwindSafe},
  rc::Rc,
};

use battlement::{
  CameraState, CommandBody, DisplayId, DisplayOrientation, GameObject, GameObjectKind,
  GeometryGeneration, GeometryObservationBatch, GeometryObservationId, GeometryObservationResult,
  GeometryObservationUpdate, GeometryObservationValue, GeometryValue, Label, ObjectId,
  PanelScaleMode, PanelSettings, ParentScene, PreparedAsset, Scene, SceneId, SessionId, Snapshot,
  UiDocument, UiDocumentState, ViewportGeometry, ViewportRect,
};
use battlement_reactant::{
  component::Component,
  executor::{BoxFuture, SpawnedTask, Spawner},
  geometry::{self, GeometrySnapshot, Measurement, ViewportRef},
  hooks,
  render::Render,
  runtime::Reactant,
};

type TupleMeasurements = (Measurement<ViewportGeometry>, Measurement<ViewportGeometry>);
type TupleSnapshots = Rc<RefCell<Vec<GeometrySnapshot<TupleMeasurements>>>>;

struct IdleSpawner;

#[derive(Default)]
struct EffectGame {
  dependency: u8,
  model_updates: usize,
  show_parent: bool,
}

struct GeometryEffectFixture {
  dependency: u8,
  label: &'static str,
  log: Rc<RefCell<Vec<String>>>,
  local_ready: bool,
  set_local_ready: Option<hooks::StateSetter<bool>>,
}

struct OrderedParent {
  dependency: u8,
  log: Rc<RefCell<Vec<String>>>,
}

struct OrderedChild {
  dependency: u8,
  log: Rc<RefCell<Vec<String>>>,
}

struct PanicGeometryEffect;

struct CleanupPanicGeometryEffect {
  dependency: u8,
}

#[derive(Default)]
struct CleanupPanicGame {
  dependency: u8,
}

struct TupleGeometryEffect {
  snapshots: TupleSnapshots,
}

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

impl Component for GeometryEffectFixture {
  fn render(&self) -> impl Render {
    let dependency = self.dependency;
    let label = self.label;
    let log = Rc::clone(&self.log);
    let set_local_ready = self.set_local_ready.clone();
    geometry::use_geometry_effect(
      move |game: &mut EffectGame, snapshot: GeometrySnapshot<Measurement<ViewportGeometry>>| {
        game.model_updates += 1;
        if let Some(set_local_ready) = set_local_ready {
          set_local_ready.set(true);
        }
        log.borrow_mut().push(format!(
          "{label} setup {dependency} {:?}",
          snapshot.generation
        ));
        move |game: &mut EffectGame| {
          game.model_updates += 1;
          log
            .borrow_mut()
            .push(format!("{label} cleanup {dependency}"));
        }
      },
      ViewportRef::display(DisplayId(0)),
      dependency,
    );
    Label::new(if self.local_ready { "ready" } else { "waiting" })
  }
}

impl Component for OrderedParent {
  fn render(&self) -> impl Render {
    let (local_ready, set_local_ready) = hooks::use_state(false);
    (
      OrderedChild {
        dependency: self.dependency,
        log: Rc::clone(&self.log),
      },
      GeometryEffectFixture {
        dependency: self.dependency,
        label: "parent",
        log: Rc::clone(&self.log),
        local_ready,
        set_local_ready: Some(set_local_ready),
      },
    )
  }
}

impl Component for OrderedChild {
  fn render(&self) -> impl Render {
    GeometryEffectFixture {
      dependency: self.dependency,
      label: "child",
      log: Rc::clone(&self.log),
      local_ready: true,
      set_local_ready: None,
    }
  }
}

impl Component for PanicGeometryEffect {
  fn render(&self) -> impl Render {
    geometry::use_geometry_effect(
      |_: &mut (), _: GeometrySnapshot<Measurement<ViewportGeometry>>| -> () {
        panic!("geometry effect failed");
      },
      ViewportRef::display(DisplayId(0)),
      (),
    );
    Label::new("panic")
  }
}

impl Component for CleanupPanicGeometryEffect {
  fn render(&self) -> impl Render {
    geometry::use_geometry_effect(
      |_: &mut CleanupPanicGame, _: GeometrySnapshot<Measurement<ViewportGeometry>>| {
        |_: &mut CleanupPanicGame| panic!("geometry cleanup failed")
      },
      ViewportRef::display(DisplayId(0)),
      self.dependency,
    );
    Label::new("cleanup panic")
  }
}

impl Component for TupleGeometryEffect {
  fn render(&self) -> impl Render {
    let snapshots = Rc::clone(&self.snapshots);
    geometry::use_geometry_effect(
      move |_: &mut (), snapshot| snapshots.borrow_mut().push(snapshot),
      (
        ViewportRef::display(DisplayId(0)),
        ViewportRef::display(DisplayId(1)),
      ),
      (),
    );
    Label::new("tuple")
  }
}

#[test]
fn coherent_generations_and_dependencies_replace_child_before_parent() {
  let document = self::document();
  let log = Rc::new(RefCell::new(Vec::new()));
  let view_log = Rc::clone(&log);
  let sibling_renders = Rc::new(Cell::new(0));
  let view_sibling_renders = Rc::clone(&sibling_renders);
  let sibling_document = self::document();
  let mut game = EffectGame {
    show_parent: true,
    ..EffectGame::default()
  };
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), move |game: &EffectGame| {
    game.show_parent.then(|| OrderedParent {
      dependency: game.dependency,
      log: Rc::clone(&view_log),
    })
  });
  reactant.register_root(sibling_document.clone(), move |game: &EffectGame| {
    view_sibling_renders.set(view_sibling_renders.get() + 1);
    Label::new(format!("model updates {}", game.model_updates))
  });
  let groups = self::begin(
    &mut reactant,
    &mut game,
    &[document.clone(), sibling_document],
  );
  let observation_id = self::updates(&groups)[0].added[0].observation_id;
  let _ = reactant.poll(&mut game).unwrap().into_groups();
  assert!(log.borrow().is_empty());

  let sibling_before_effect = sibling_renders.get();
  let groups = reactant
    .observe_geometry(
      &mut game,
      self::batch(observation_id, 1, Some(self::viewport(10.0))),
    )
    .unwrap()
    .into_groups();
  assert!(!groups.is_empty(), "local effect state joins the refresh");
  assert_eq!(game.model_updates, 2);
  assert!(sibling_renders.get() > sibling_before_effect);
  assert!(log.borrow()[0].starts_with("child setup 0 Some("));
  assert!(log.borrow()[1].starts_with("parent setup 0 Some("));
  assert!(reactant.poll(&mut game).unwrap().is_empty());

  let _ = reactant
    .observe_geometry(&mut game, self::batch(observation_id, 2, None))
    .unwrap()
    .into_groups();
  let _ = reactant.poll(&mut game).unwrap().into_groups();
  assert_eq!(log.borrow().len(), 2, "unchanged values do not rerun");

  game.dependency = 1;
  let _ = reactant.refresh(&mut game).unwrap().into_groups();
  assert_eq!(log.borrow().len(), 2);
  let _ = reactant.poll(&mut game).unwrap().into_groups();
  assert_eq!(
    &*log.borrow(),
    &[
      "child setup 0 Some(GeometryGeneration(1))",
      "parent setup 0 Some(GeometryGeneration(1))",
      "child cleanup 0",
      "child setup 1 Some(GeometryGeneration(2))",
      "parent cleanup 0",
      "parent setup 1 Some(GeometryGeneration(2))",
    ]
  );

  game.show_parent = false;
  let _ = reactant.refresh(&mut game).unwrap().into_groups();
  let _ = reactant.poll(&mut game).unwrap().into_groups();
  assert_eq!(&log.borrow()[6..], &["child cleanup 1", "parent cleanup 1"]);
}

#[test]
fn setup_panics_poison_the_runtime() {
  let document = self::document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |_: &()| PanicGeometryEffect);
  let groups = self::begin(&mut reactant, &mut (), &[document]);
  let observation_id = self::updates(&groups)[0].added[0].observation_id;
  let failure = panic::catch_unwind(AssertUnwindSafe(|| {
    reactant.observe_geometry(
      &mut (),
      self::batch(observation_id, 1, Some(self::viewport(0.0))),
    )
  }))
  .err()
  .expect("geometry setup should panic");
  assert_eq!(self::panic_message(failure), "geometry effect failed");
  assert!(panic::catch_unwind(AssertUnwindSafe(|| reactant.poll(&mut ()))).is_err());
}

#[test]
fn partial_target_generations_do_not_run_setup() {
  let document = self::document();
  let snapshots = Rc::new(RefCell::new(Vec::new()));
  let view_snapshots = Rc::clone(&snapshots);
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), move |_: &()| TupleGeometryEffect {
    snapshots: Rc::clone(&view_snapshots),
  });
  let groups = self::begin(&mut reactant, &mut (), &[document]);
  let observations = self::updates(&groups)[0].added.clone();
  assert_eq!(observations.len(), 2);

  let _ = reactant
    .observe_geometry(
      &mut (),
      self::batch(
        observations[0].observation_id,
        1,
        Some(self::viewport(10.0)),
      ),
    )
    .unwrap()
    .into_groups();
  assert!(snapshots.borrow().is_empty());

  let _ = reactant
    .observe_geometry(
      &mut (),
      self::batch(
        observations[1].observation_id,
        2,
        Some(self::viewport(20.0)),
      ),
    )
    .unwrap()
    .into_groups();
  let snapshots = snapshots.borrow();
  assert_eq!(snapshots.len(), 1);
  assert_eq!(snapshots[0].generation, Some(self::generation(2)));
  assert_eq!(snapshots[0].measurements.0.latest.unwrap().viewport.x, 10.0);
  assert_eq!(snapshots[0].measurements.1.latest.unwrap().viewport.x, 20.0);
}

#[test]
fn cleanup_panics_poison_the_runtime() {
  let document = self::document();
  let mut game = CleanupPanicGame::default();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |game: &CleanupPanicGame| {
    CleanupPanicGeometryEffect {
      dependency: game.dependency,
    }
  });
  let groups = self::begin(&mut reactant, &mut game, &[document]);
  let observation_id = self::updates(&groups)[0].added[0].observation_id;
  let _ = reactant
    .observe_geometry(
      &mut game,
      self::batch(observation_id, 1, Some(self::viewport(0.0))),
    )
    .unwrap()
    .into_groups();
  game.dependency = 1;
  let _ = reactant.refresh(&mut game).unwrap().into_groups();

  let failure = panic::catch_unwind(AssertUnwindSafe(|| reactant.poll(&mut game)))
    .err()
    .expect("geometry cleanup should panic");
  assert_eq!(self::panic_message(failure), "geometry cleanup failed");
  assert!(panic::catch_unwind(AssertUnwindSafe(|| reactant.poll(&mut game))).is_err());
}

fn begin<G: 'static>(
  reactant: &mut Reactant<G>,
  game: &mut G,
  documents: &[UiDocument],
) -> Vec<Vec<CommandBody>> {
  reactant
    .begin_session(game)
    .unwrap()
    .into_parts(self::snapshot(documents))
    .1
    .into_groups()
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

fn batch(
  observation_id: GeometryObservationId,
  generation: u64,
  value: Option<ViewportGeometry>,
) -> GeometryObservationBatch {
  GeometryObservationBatch {
    generation: self::generation(generation),
    changed: value.map_or_else(Vec::new, |value| {
      vec![GeometryObservationValue {
        observation_id,
        result: GeometryObservationResult::Current(GeometryValue::Viewport(value)),
      }]
    }),
  }
}

fn generation(value: u64) -> GeometryGeneration {
  GeometryGeneration(NonZeroU64::new(value).unwrap())
}

fn viewport(x: f64) -> ViewportGeometry {
  ViewportGeometry {
    viewport: ViewportRect {
      x,
      y: 0.0,
      width: 100.0,
      height: 100.0,
      display_id: DisplayId(0),
    },
    safe_area: ViewportRect {
      x,
      y: 0.0,
      width: 100.0,
      height: 90.0,
      display_id: DisplayId(0),
    },
    scale: 1.0,
    dpi: Some(96.0),
    orientation: DisplayOrientation::Landscape,
  }
}

fn document() -> UiDocument {
  UiDocument::with_root_id(ObjectId::new_v4(), ObjectId::new_v4())
}

fn snapshot(documents: &[UiDocument]) -> Snapshot {
  let camera_id = ObjectId::new_v4();
  let mut objects = vec![GameObject::new(camera_id, CameraState::new())];
  objects.extend(documents.iter().map(|document| {
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
    vec![Scene::new(SceneId::new_v4(), "test/scene")],
    objects,
    camera_id,
  )
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
  payload.downcast_ref::<&str>().map_or_else(
    || payload.downcast_ref::<String>().cloned().unwrap(),
    |message| (*message).to_owned(),
  )
}
