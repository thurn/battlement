use std::{
  cell::{Cell, RefCell},
  error::Error,
  fmt,
  panic::{self, AssertUnwindSafe},
  rc::Rc,
};

use battlement::{
  CameraState, CommandBody, GameObject, GameObjectKind, ObjectId, PanelScaleMode, PanelSettings,
  ParentScene, PreparedAsset, Prop, Scene, SceneId, SessionId, Snapshot, UiDocument,
  UiDocumentState, UiElement,
};
use battlement_fake::battlement_ui_fake::UiWorld;
use battlement_reactant::{
  component::Component,
  error_boundary::ErrorBoundary,
  executor::{BoxFuture, SpawnedTask, Spawner},
  hooks::use_state,
  render::{Either, Node, Render},
  runtime::{Reactant, ReactantCommit, RenderError},
};

#[derive(Default)]
struct BoundaryGame {
  fail: bool,
  revision: u32,
  string_reset: bool,
}

#[derive(Default)]
struct ReportGame {
  reports: Vec<&'static str>,
}

struct RootGame {
  fail: bool,
  text: &'static str,
}

struct IdleSpawner;

struct Fallible {
  fail: bool,
  label: &'static str,
}

struct BoxedFailure;

struct Panicking;

struct UpdatingFailure {
  fail: Rc<Cell<bool>>,
  observed: Rc<RefCell<Vec<i32>>>,
}

#[derive(Debug)]
struct DomainError(String);

#[derive(Debug)]
struct FallbackError;

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

impl Component for Fallible {
  fn render(&self) -> impl Render {
    if self.fail {
      Err(DomainError(self.label.to_owned()))
    } else {
      Ok(battlement_reactant::host::Label::new(self.label))
    }
  }
}

impl Component for BoxedFailure {
  fn render(&self) -> impl Render {
    Err::<battlement_reactant::host::Label, _>(RenderError::from_boxed(Box::new(DomainError(
      "boxed".to_owned(),
    ))))
  }
}

impl Component for Panicking {
  fn render(&self) -> impl Render {
    if std::hint::black_box(true) {
      panic!("fixture panic");
    }
  }
}

impl Component for UpdatingFailure {
  fn render(&self) -> impl Render {
    let (value, setter) = use_state(0);
    self.observed.borrow_mut().push(value);
    if self.fail.get() {
      setter.set(1);
      Err(DomainError("update abandoned".to_owned()))
    } else {
      Ok(battlement_reactant::host::Label::new(value.to_string()))
    }
  }
}

impl fmt::Display for DomainError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str(&self.0)
  }
}

impl fmt::Display for FallbackError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("fallback failed")
  }
}

impl Error for DomainError {}
impl Error for FallbackError {}

#[test]
fn nearest_boundaries_preserve_concrete_and_boxed_error_types() {
  let document = self::document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |_| {
    ErrorBoundary::new(|_: &RenderError| battlement_reactant::host::Label::new("outer")).child((
      ErrorBoundary::new(|error: &RenderError| {
        battlement_reactant::host::Label::new(format!(
          "inner:{}",
          error.downcast_ref::<DomainError>().unwrap()
        ))
      })
      .child(Fallible {
        fail: true,
        label: "concrete",
      }),
      ErrorBoundary::new(|error: &RenderError| {
        battlement_reactant::host::Label::new(format!(
          "boxed:{}",
          error.downcast_ref::<DomainError>().unwrap()
        ))
      })
      .child(BoxedFailure),
    ))
  });

  let rendered = self::begin(&mut reactant, &mut (), &document);

  assert_eq!(
    self::snapshot_texts(&rendered, document.root_id),
    ["inner:concrete", "boxed:boxed"]
  );
  let normalized = RenderError::new(RenderError::new(DomainError("direct".to_owned())));
  assert_eq!(
    normalized.downcast_ref::<DomainError>().unwrap().0,
    "direct"
  );
  let _ = reactant.shutdown(&mut ()).into_groups();
}

#[test]
fn shared_and_erased_results_reach_boundaries_without_losing_downcasts() {
  let document = self::document();
  let shared = Rc::new(Err::<battlement_reactant::host::Label, _>(DomainError(
    "shared".to_owned(),
  )));
  let erased = Node::new(Err::<battlement_reactant::host::Label, _>(DomainError(
    "erased".to_owned(),
  )));
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), move |_| {
    (
      ErrorBoundary::new(|error: &RenderError| {
        battlement_reactant::host::Label::new(
          error.downcast_ref::<DomainError>().unwrap().to_string(),
        )
      })
      .child(Rc::clone(&shared)),
      ErrorBoundary::new(|error: &RenderError| {
        battlement_reactant::host::Label::new(
          error.downcast_ref::<DomainError>().unwrap().to_string(),
        )
      })
      .child(erased.clone()),
    )
  });

  let rendered = self::begin(&mut reactant, &mut (), &document);

  assert_eq!(
    self::snapshot_texts(&rendered, document.root_id),
    ["shared", "erased"]
  );
  let _ = reactant.shutdown(&mut ()).into_groups();
}

#[test]
fn reset_values_and_types_retry_with_a_fresh_primary() {
  let document = self::document();
  let mut game = BoundaryGame::default();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |game: &BoundaryGame| {
    if game.string_reset {
      Either::Left(
        ErrorBoundary::new(|_: &RenderError| battlement_reactant::host::Label::new("fallback"))
          .reset_on(game.revision.to_string())
          .child(Fallible {
            fail: game.fail,
            label: "primary",
          }),
      )
    } else {
      Either::Right(
        ErrorBoundary::new(|_: &RenderError| battlement_reactant::host::Label::new("fallback"))
          .reset_on(game.revision)
          .child(Fallible {
            fail: game.fail,
            label: "primary",
          }),
      )
    }
  });
  let rendered = self::begin(&mut reactant, &mut game, &document);
  let mut world = UiWorld::default();
  world.replace(rendered.ui).unwrap();
  let first_primary = self::only_child(&world, document.root_id);

  game.fail = true;
  self::apply(&mut world, reactant.refresh(&mut game).unwrap());
  assert_eq!(self::only_text(&world, document.root_id), "fallback");
  game.fail = false;
  self::apply(&mut world, reactant.refresh(&mut game).unwrap());
  assert_eq!(self::only_text(&world, document.root_id), "fallback");

  game.revision = 1;
  self::apply(&mut world, reactant.refresh(&mut game).unwrap());
  let value_reset_primary = self::only_child(&world, document.root_id);
  assert_eq!(self::only_text(&world, document.root_id), "primary");
  assert_ne!(value_reset_primary, first_primary);

  game.fail = true;
  self::apply(&mut world, reactant.refresh(&mut game).unwrap());
  game.fail = false;
  game.string_reset = true;
  self::apply(&mut world, reactant.refresh(&mut game).unwrap());
  assert_eq!(self::only_text(&world, document.root_id), "primary");
  assert_ne!(
    self::only_child(&world, document.root_id),
    value_reset_primary
  );
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn fallback_errors_escalate_to_the_next_boundary() {
  let document = self::document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |_| {
    ErrorBoundary::new(|error: &RenderError| {
      assert!(error.downcast_ref::<FallbackError>().is_some());
      battlement_reactant::host::Label::new("outer fallback")
    })
    .child(
      ErrorBoundary::new(|_: &RenderError| {
        Err::<battlement_reactant::host::Label, _>(FallbackError)
      })
      .child(Fallible {
        fail: true,
        label: "primary",
      }),
    )
  });

  let rendered = self::begin(&mut reactant, &mut (), &document);

  assert_eq!(
    self::snapshot_texts(&rendered, document.root_id),
    ["outer fallback"]
  );
  let _ = reactant.shutdown(&mut ()).into_groups();
}

#[test]
fn errors_erased_by_host_children_reach_the_nearest_boundary() {
  let document = self::document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |_| {
    ErrorBoundary::new(|error: &RenderError| {
      battlement_reactant::host::Label::new(
        error.downcast_ref::<DomainError>().unwrap().to_string(),
      )
    })
    .child(battlement_reactant::host::View::new().child(Err::<
      battlement_reactant::host::Label,
      _,
    >(DomainError(
      "host child failed".to_owned(),
    ))))
  });

  let rendered = self::begin(&mut reactant, &mut (), &document);

  assert_eq!(
    self::snapshot_texts(&rendered, document.root_id),
    ["host child failed"]
  );
  let _ = reactant.shutdown(&mut ()).into_groups();
}

#[test]
fn sibling_reports_run_once_in_logical_catch_order_then_render_all_roots() {
  let document = self::document();
  let renders = std::rc::Rc::new(std::cell::Cell::new(0));
  let view_renders = std::rc::Rc::clone(&renders);
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), move |_: &ReportGame| {
    view_renders.set(view_renders.get() + 1);
    (
      ErrorBoundary::new(|_: &RenderError| battlement_reactant::host::Label::new("first fallback"))
        .on_error(|game: &mut ReportGame, _| game.reports.push("first"))
        .child(Fallible {
          fail: true,
          label: "first",
        }),
      ErrorBoundary::new(|_: &RenderError| {
        battlement_reactant::host::Label::new("second fallback")
      })
      .on_error(|game: &mut ReportGame, _| game.reports.push("second"))
      .child(Fallible {
        fail: true,
        label: "second",
      }),
    )
  });
  let mut game = ReportGame::default();
  let _ = self::begin(&mut reactant, &mut game, &document);
  assert!(game.reports.is_empty());

  assert!(reactant.poll(&mut game).unwrap().is_empty());

  assert_eq!(game.reports, ["first", "second"]);
  assert_eq!(renders.get(), 2);
  assert!(reactant.poll(&mut game).unwrap().is_empty());
  assert_eq!(renders.get(), 2);
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn escaped_errors_are_atomic_and_a_corrected_root_can_retry() {
  let document = self::document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |game: &RootGame| {
    if game.fail {
      Err(DomainError("root failed".to_owned()))
    } else {
      Ok(battlement_reactant::host::Label::new(game.text))
    }
  });
  let mut game = RootGame {
    fail: false,
    text: "stable",
  };
  let rendered = self::begin(&mut reactant, &mut game, &document);
  let mut world = UiWorld::default();
  world.replace(rendered.ui).unwrap();
  let stable_id = self::only_child(&world, document.root_id);

  game.fail = true;
  let error = match reactant.refresh(&mut game) {
    Ok(_) => panic!("root error unexpectedly rendered"),
    Err(error) => error,
  };
  assert_eq!(
    error.downcast_ref::<DomainError>().unwrap().0,
    "root failed"
  );
  assert_eq!(self::only_text(&world, document.root_id), "stable");

  game.fail = false;
  game.text = "restored";
  self::apply(&mut world, reactant.refresh(&mut game).unwrap());
  assert_eq!(self::only_text(&world, document.root_id), "restored");
  assert_eq!(self::only_child(&world, document.root_id), stable_id);
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn escaped_errors_rollback_render_phase_updates_before_retry() {
  let document = self::document();
  let fail = Rc::new(Cell::new(false));
  let observed = Rc::new(RefCell::new(Vec::new()));
  let mut reactant = Reactant::new(IdleSpawner);
  let view_fail = Rc::clone(&fail);
  let view_observed = Rc::clone(&observed);
  reactant.register_root(document.clone(), move |_| UpdatingFailure {
    fail: Rc::clone(&view_fail),
    observed: Rc::clone(&view_observed),
  });
  let _ = self::begin(&mut reactant, &mut (), &document);

  fail.set(true);
  assert!(reactant.refresh(&mut ()).is_err());
  fail.set(false);
  let commit = reactant.refresh(&mut ()).unwrap();
  let _ = commit.into_groups();

  assert_eq!(*observed.borrow(), [0, 0, 0]);
  let _ = reactant.shutdown(&mut ()).into_groups();
}

#[test]
fn boundaries_do_not_catch_panics_and_the_runtime_poisons() {
  let document = self::document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document, |_| {
    ErrorBoundary::new(|_: &RenderError| battlement_reactant::host::Label::new("not reached"))
      .child(Panicking)
  });

  assert!(
    panic::catch_unwind(AssertUnwindSafe(|| {
      let _ = reactant.begin_session(&mut ());
    }))
    .is_err()
  );
  assert!(panic::catch_unwind(AssertUnwindSafe(|| reactant.shutdown(&mut ()))).is_err());
}

fn begin<G: 'static>(reactant: &mut Reactant<G>, game: &mut G, document: &UiDocument) -> Snapshot {
  reactant
    .begin_session(game)
    .unwrap()
    .into_parts(self::snapshot(document))
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

fn only_child(world: &UiWorld, root: ObjectId) -> ObjectId {
  world.element(root).unwrap().children()[0]
}

fn only_text(world: &UiWorld, root: ObjectId) -> &str {
  world
    .element(self::only_child(world, root))
    .unwrap()
    .text()
    .unwrap()
}

fn snapshot_texts(snapshot: &Snapshot, root: ObjectId) -> Vec<&str> {
  snapshot
    .ui
    .iter()
    .find(|document| document.root_id == root)
    .unwrap()
    .children
    .iter()
    .map(|child| match &child.element {
      UiElement::Label(label) => match &label.text {
        Prop::Set(text) => text.as_str(),
        Prop::Unset | Prop::Reset => panic!("label text is missing"),
      },
      _ => panic!("fixture expected a label"),
    })
    .collect()
}

fn document() -> UiDocument {
  UiDocument::with_root_id(ObjectId::new_v4(), ObjectId::new_v4())
}

fn snapshot(document: &UiDocument) -> Snapshot {
  let scene_id = SceneId::new_v4();
  let camera_id = ObjectId::new_v4();
  Snapshot::new(
    SessionId::new_v4(),
    vec![PreparedAsset::Scene("test/scene".into())],
    vec![Scene::new(scene_id, "test/scene")],
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
