use std::{
  cell::RefCell,
  error::Error,
  fmt,
  num::NonZeroU64,
  panic::{self, AssertUnwindSafe},
  rc::Rc,
};

use battlement::{
  CameraState, ClickEvent, CommandBody, GameObject, GameObjectKind, GeometryGeneration,
  GeometryObservationBatch, ObjectId, PanelScaleMode, PanelSettings, ParentScene, PreparedAsset,
  Scene, SceneId, SessionId, Snapshot, UiDocument, UiDocumentState, UiEvent, UiNode,
  UiVisualElement,
};
use battlement_fake::battlement_ui_fake::UiWorld;
use battlement_reactant::{
  component::Component,
  executor::{BoxFuture, SpawnedTask, Spawner},
  hooks,
  render::Render,
  resource::Resource,
  runtime::{Reactant, ReactantCommit},
};

struct IdleSpawner;

#[derive(Clone, Copy, Debug)]
enum Entry {
  BeginSession,
  Clear,
  CreatePortalTarget,
  Dispatch,
  Invalidate,
  ObserveGeometry,
  Poll,
  Preload,
  Refresh,
  RegisterExternalContainer,
  RegisterRoot,
  Shutdown,
  StageExternalRebind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FixtureState {
  Active,
  Closed,
  Poisoned,
  Registering,
}

#[derive(Default)]
struct Game {
  panic_render: bool,
}

struct Fixture {
  document: UiDocument,
  external: UiDocument,
  external_target: battlement_reactant::portal::PortalTarget,
  game: Game,
  reactant: Reactant<Game>,
  resource: Resource<u8, u8>,
}

struct Cleanup {
  fail: bool,
  log: Rc<RefCell<Vec<&'static str>>>,
  name: &'static str,
}

#[derive(Debug)]
struct DomainError;

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

impl Component for Cleanup {
  fn render(&self) -> impl Render {
    let fail = self.fail;
    let log = Rc::clone(&self.log);
    let name = self.name;
    hooks::use_effect(
      move || {
        log.borrow_mut().push(match name {
          "first" => "first setup",
          "second" => "second setup",
          _ => "single setup",
        });
        move || {
          log.borrow_mut().push(match name {
            "first" => "first cleanup",
            "second" => "second cleanup",
            _ => "single cleanup",
          });
          assert!(!fail, "cleanup failed");
        }
      },
      (),
    );
    battlement_reactant::host::Label::new(name)
  }
}

impl fmt::Display for DomainError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str("domain error")
  }
}

impl Error for DomainError {}

impl Entry {
  const ALL: [Self; 13] = [
    Self::RegisterRoot,
    Self::CreatePortalTarget,
    Self::RegisterExternalContainer,
    Self::StageExternalRebind,
    Self::Preload,
    Self::Invalidate,
    Self::Clear,
    Self::BeginSession,
    Self::Dispatch,
    Self::ObserveGeometry,
    Self::Refresh,
    Self::Poll,
    Self::Shutdown,
  ];

  fn allowed(self, state: FixtureState) -> bool {
    match state {
      FixtureState::Registering => matches!(
        self,
        Self::RegisterRoot
          | Self::CreatePortalTarget
          | Self::RegisterExternalContainer
          | Self::Preload
          | Self::Invalidate
          | Self::Clear
          | Self::BeginSession
          | Self::Shutdown
      ),
      FixtureState::Active => !matches!(
        self,
        Self::RegisterRoot | Self::CreatePortalTarget | Self::RegisterExternalContainer
      ),
      FixtureState::Closed => matches!(self, Self::Shutdown),
      FixtureState::Poisoned => false,
    }
  }
}

#[test]
fn public_runtime_state_and_entry_matrix_is_stable() {
  for state in [
    FixtureState::Registering,
    FixtureState::Active,
    FixtureState::Closed,
    FixtureState::Poisoned,
  ] {
    for entry in Entry::ALL {
      self::exercise_entry(state, entry);
    }
  }
}

#[test]
fn active_shutdown_destroys_native_hosts_and_failed_cleanup_emits_nothing() {
  let document = self::document();
  let log = Rc::new(RefCell::new(Vec::new()));
  let first_log = Rc::clone(&log);
  let second_log = Rc::clone(&log);
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), move |_| {
    (
      Cleanup {
        fail: true,
        log: Rc::clone(&first_log),
        name: "first",
      },
      Cleanup {
        fail: false,
        log: Rc::clone(&second_log),
        name: "second",
      },
    )
  });
  let initial = reactant
    .begin_session(&mut ())
    .unwrap()
    .into_parts(self::snapshot(&document, None))
    .0;
  let mut world = UiWorld::default();
  world.replace(initial.ui).unwrap();
  assert!(reactant.poll(&mut ()).unwrap().is_empty());
  assert_eq!(&*log.borrow(), &["first setup", "second setup"]);

  let failed = panic::catch_unwind(AssertUnwindSafe(|| reactant.shutdown(&mut ())));
  assert!(failed.is_err());
  assert_eq!(world.element(document.root_id).unwrap().children().len(), 2);
  assert!(panic::catch_unwind(AssertUnwindSafe(|| reactant.poll(&mut ()))).is_err());
  drop(reactant);
  assert_eq!(
    &*log.borrow(),
    &[
      "first setup",
      "second setup",
      "first cleanup",
      "second cleanup"
    ]
  );

  let document = self::document();
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), |_| {
    battlement_reactant::host::Label::new("mounted")
  });
  let initial = reactant
    .begin_session(&mut ())
    .unwrap()
    .into_parts(self::snapshot(&document, None))
    .0;
  let mut world = UiWorld::default();
  world.replace(initial.ui).unwrap();
  self::apply(&mut world, reactant.shutdown(&mut ()));
  assert!(
    world
      .element(document.root_id)
      .unwrap()
      .children()
      .is_empty()
  );
  assert!(reactant.shutdown(&mut ()).is_empty());
}

#[test]
fn dropping_an_active_runtime_runs_passive_cleanup() {
  let document = self::document();
  let log = Rc::new(RefCell::new(Vec::new()));
  let view_log = Rc::clone(&log);
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), move |_| Cleanup {
    fail: false,
    log: Rc::clone(&view_log),
    name: "single",
  });
  let _ = reactant
    .begin_session(&mut ())
    .unwrap()
    .into_parts(self::snapshot(&document, None));
  assert!(reactant.poll(&mut ()).unwrap().is_empty());

  let dropped = panic::catch_unwind(AssertUnwindSafe(|| drop(reactant)));
  assert!(dropped.is_err());

  assert_eq!(&*log.borrow(), &["single setup", "single cleanup"]);
}

fn exercise_entry(state: FixtureState, entry: Entry) {
  let mut fixture = self::fixture(state);
  let result = panic::catch_unwind(AssertUnwindSafe(|| match entry {
    Entry::BeginSession => {
      let snapshot = self::snapshot(&fixture.document, Some(fixture.external.clone()));
      let (_, commit) = fixture
        .reactant
        .begin_session(&mut fixture.game)
        .unwrap()
        .into_parts(snapshot);
      let _ = commit.into_groups();
    }
    Entry::CreatePortalTarget => {
      let _ = fixture.reactant.create_portal_target();
    }
    Entry::Clear => fixture.reactant.clear(&fixture.resource),
    Entry::Dispatch => {
      let commit = fixture
        .reactant
        .dispatch(
          &mut fixture.game,
          UiEvent::click(ObjectId::new_v4(), ClickEvent::NavigationSubmit),
        )
        .unwrap();
      let _ = commit.into_groups();
    }
    Entry::ObserveGeometry => {
      let commit = fixture
        .reactant
        .observe_geometry(
          &mut fixture.game,
          GeometryObservationBatch {
            generation: GeometryGeneration(NonZeroU64::new(1).unwrap()),
            changed: Vec::new(),
          },
        )
        .unwrap();
      let _ = commit.into_groups();
    }
    Entry::Invalidate => fixture.reactant.invalidate(&fixture.resource, &1),
    Entry::Poll => {
      let _ = fixture
        .reactant
        .poll(&mut fixture.game)
        .unwrap()
        .into_groups();
    }
    Entry::Preload => fixture.reactant.preload(&fixture.resource, 1),
    Entry::Refresh => {
      let _ = fixture
        .reactant
        .refresh(&mut fixture.game)
        .unwrap()
        .into_groups();
    }
    Entry::RegisterExternalContainer => {
      let _ = fixture
        .reactant
        .register_external_container(ObjectId::new_v4());
    }
    Entry::RegisterRoot => {
      fixture.reactant.register_root(self::document(), |_| {
        battlement_reactant::host::Label::new("additional")
      });
    }
    Entry::Shutdown => {
      let _ = fixture.reactant.shutdown(&mut fixture.game).into_groups();
    }
    Entry::StageExternalRebind => fixture
      .reactant
      .stage_external_container_rebind(&fixture.external_target, ObjectId::new_v4()),
  }));
  assert_eq!(result.is_ok(), entry.allowed(state), "{state:?} {entry:?}");

  match state {
    FixtureState::Registering => {
      if matches!(entry, Entry::BeginSession) {
        let _ = fixture.reactant.shutdown(&mut fixture.game).into_groups();
      } else if !matches!(entry, Entry::Shutdown) {
        assert!(fixture.reactant.shutdown(&mut fixture.game).is_empty());
      }
    }
    FixtureState::Active => {
      if !matches!(entry, Entry::Shutdown) {
        let _ = fixture
          .reactant
          .poll(&mut fixture.game)
          .unwrap()
          .into_groups();
        let _ = fixture.reactant.shutdown(&mut fixture.game).into_groups();
      }
    }
    FixtureState::Closed => assert!(fixture.reactant.shutdown(&mut fixture.game).is_empty()),
    FixtureState::Poisoned => {}
  }
}

fn fixture(state: FixtureState) -> Fixture {
  let document = self::document();
  let target_id = ObjectId::new_v4();
  let external =
    UiDocument::new(ObjectId::new_v4()).child(UiNode::new(target_id, UiVisualElement::new()));
  let mut reactant = Reactant::new(IdleSpawner);
  let external_target = reactant.register_external_container(target_id);
  reactant.register_root(document.clone(), |game: &Game| {
    assert!(!game.panic_render, "render failed");
    Ok::<_, DomainError>(battlement_reactant::host::Label::new("stable"))
  });
  let mut fixture = Fixture {
    document,
    external,
    external_target,
    game: Game::default(),
    reactant,
    resource: Resource::new(|key: u8| async move { key }),
  };
  if state != FixtureState::Registering {
    let snapshot = self::snapshot(&fixture.document, Some(fixture.external.clone()));
    let (_, commit) = fixture
      .reactant
      .begin_session(&mut fixture.game)
      .unwrap()
      .into_parts(snapshot);
    let _ = commit.into_groups();
  }
  if state == FixtureState::Closed {
    let _ = fixture.reactant.shutdown(&mut fixture.game).into_groups();
  } else if state == FixtureState::Poisoned {
    fixture.game.panic_render = true;
    assert!(
      panic::catch_unwind(AssertUnwindSafe(|| {
        let _ = fixture.reactant.refresh(&mut fixture.game);
      }))
      .is_err()
    );
  }
  fixture
}

fn apply(world: &mut UiWorld, commit: ReactantCommit) {
  for body in commit.into_groups().into_iter().flatten() {
    match body {
      CommandBody::VisualElementCreate(value) => world.create(*value).unwrap(),
      CommandBody::VisualElementUpdate(value) => world.update(*value).unwrap(),
      CommandBody::VisualElementDestroy(value) => world.destroy(value.object_id).unwrap(),
      CommandBody::GeometryObservationUpdate(_) | CommandBody::VisualElementPerformAction(_) => {}
      _ => panic!("Reactant emitted an unexpected command"),
    }
  }
}

fn document() -> UiDocument {
  UiDocument::with_root_id(ObjectId::new_v4(), ObjectId::new_v4())
}

fn snapshot(document: &UiDocument, external: Option<UiDocument>) -> Snapshot {
  let scene_id = SceneId::new_v4();
  let camera_id = ObjectId::new_v4();
  let mut objects = vec![GameObject::new(camera_id, CameraState::new())];
  objects.push(self::document_object(document));
  if let Some(external) = &external {
    objects.push(self::document_object(external));
  }
  let mut snapshot = Snapshot::new(
    SessionId::new_v4(),
    vec![PreparedAsset::Scene("test/scene".into())],
    vec![Scene::new(scene_id, "test/scene")],
    objects,
    camera_id,
  );
  snapshot.ui = external.into_iter().collect();
  snapshot
}

fn document_object(document: &UiDocument) -> GameObject {
  GameObject::new(
    document.document_id,
    GameObjectKind::UiDocument(
      UiDocumentState::new(document.root_id)
        .panel_settings(PanelSettings::new().scale_mode(PanelScaleMode::ConstantLogicalPixelSize)),
    ),
  )
  .parent_scene(ParentScene::Persistent)
}
