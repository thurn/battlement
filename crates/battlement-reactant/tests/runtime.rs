use std::{
  cell::{Cell, RefCell},
  num::NonZeroU64,
  panic::{self, AssertUnwindSafe},
  rc::Rc,
  slice,
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
};

use battlement::{
  CameraState, ClickEvent, ClientMessage, Command, Connect, GameObject, GameObjectKind,
  GeometryGeneration, GeometryObservationBatch, Label, ObjectId, PanelScaleMode, PanelSettings,
  ParentScene, PreparedAsset, Response, ResponseMessage, Scene, SceneId, SessionId, Snapshot,
  UiDocument, UiDocumentState, UiElementKind, UiEvent, UiNode,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_native::{Engine, EngineError};
use battlement_reactant::{
  executor::{BoxFuture, SpawnedTask, Spawner},
  render::{Either, Fragment, Node, Render},
  runtime::Reactant,
};
use uuid::Uuid;

struct Game {
  label: String,
}

struct IdleSpawner {
  calls: Rc<Cell<usize>>,
}

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    self.calls.set(self.calls.get() + 1);
    SpawnedTask::detached()
  }
}

struct ReactantEngine {
  game: Game,
  reactant: Reactant<Game>,
  snapshot: Option<Snapshot>,
  recorded: Rc<RefCell<Option<Response>>>,
}

struct StructuralGame {
  optional: Rc<Cell<bool>>,
  left_branch: Rc<Cell<bool>>,
  rc_branch: Rc<Cell<bool>>,
  nested_node: Rc<Cell<bool>>,
  wrapped_fragment: Rc<Cell<bool>>,
}

struct StructuralEngine {
  game: StructuralGame,
  reactant: Reactant<StructuralGame>,
  document: UiDocument,
  snapshots: Rc<RefCell<Vec<Vec<ObjectId>>>>,
}

impl Engine for ReactantEngine {
  type ActionPayload = ();
  type ErrorCode = ();
  type Command = Command;

  fn connect(&mut self, _message: Connect) -> Result<Response, EngineError> {
    let response = self
      .reactant
      .begin_session(&mut self.game)
      .expect("fixture render is infallible")
      .into_response(self.snapshot.take().expect("fixture connected twice"));
    self.recorded.replace(Some(response.clone()));
    Ok(response)
  }

  fn submit(&mut self, _message: ClientMessage<(), ()>) -> Result<Response, EngineError> {
    Err(EngineError::new("fixture does not accept actions"))
  }

  fn poll(&mut self) -> Result<Option<Response>, EngineError> {
    Ok(None)
  }
}

impl Engine for StructuralEngine {
  type ActionPayload = ();
  type ErrorCode = ();
  type Command = Command;

  fn connect(&mut self, _message: Connect) -> Result<Response, EngineError> {
    let response = self
      .reactant
      .begin_session(&mut self.game)
      .expect("fixture render is infallible")
      .into_response(snapshot(
        SessionId::new_v4(),
        slice::from_ref(&self.document),
      ));
    let ResponseMessage::Snapshot(rendered) = &response.messages[0] else {
      panic!("session response did not begin with a snapshot");
    };
    self.snapshots.borrow_mut().push(
      rendered.ui[0]
        .children
        .iter()
        .map(|node| node.object_id)
        .collect(),
    );
    Ok(response)
  }

  fn submit(&mut self, _message: ClientMessage<(), ()>) -> Result<Response, EngineError> {
    Err(EngineError::new("fixture does not accept actions"))
  }

  fn poll(&mut self) -> Result<Option<Response>, EngineError> {
    Ok(None)
  }
}

#[test]
fn fake_client_applies_the_complete_rendered_session_hierarchy() {
  let calls = Rc::new(Cell::new(0));
  let first = document(10, 11);
  let second = document(20, 21);
  let first_root = first.root_id;
  let second_root = second.root_id;
  let mut reactant = Reactant::new(IdleSpawner {
    calls: Rc::clone(&calls),
  });
  let first_handle =
    reactant.register_root(first.clone(), |game: &Game| Label::new(game.label.clone()));
  let second_handle = reactant.register_root(second.clone(), |_game: &Game| ());
  assert_ne!(first_handle, second_handle);
  let session_id = SessionId::new_v4();
  let recorded = Rc::new(RefCell::new(None));
  let engine = ReactantEngine {
    game: Game {
      label: "Ready".to_owned(),
    },
    reactant,
    snapshot: Some(snapshot(session_id, &[first, second])),
    recorded: Rc::clone(&recorded),
  };

  let mut client = FakeClient::connect(engine, catalog());

  let response = recorded.borrow();
  let ResponseMessage::Snapshot(rendered) =
    &response.as_ref().expect("response was recorded").messages[0]
  else {
    panic!("session response did not begin with a snapshot");
  };
  assert_eq!(rendered.ui.len(), 2);
  assert_eq!(rendered.ui[0].children.len(), 1);
  assert!(rendered.ui[1].children.is_empty());
  let label_id = rendered.ui[0].children[0].object_id;
  let ui = client.ui();
  assert_eq!(ui.element(first_root).children(), &[label_id]);
  assert_eq!(ui.element(label_id).kind(), UiElementKind::Label);
  assert_eq!(ui.element(label_id).text(), Some("Ready"));
  assert!(ui.element(second_root).children().is_empty());
  assert_eq!(calls.get(), 0, "the executor must remain idle");
}

#[test]
fn structural_values_preserve_empty_positions_and_erased_type_identity() {
  let optional = Rc::new(Cell::new(true));
  let left_branch = Rc::new(Cell::new(true));
  let rc_branch = Rc::new(Cell::new(true));
  let nested_node = Rc::new(Cell::new(true));
  let wrapped_fragment = Rc::new(Cell::new(true));
  let document = document(90, 91);
  let root_id = document.root_id;
  let snapshots = Rc::new(RefCell::new(Vec::new()));
  let mut reactant = Reactant::new(idle_spawner());
  reactant.register_root(document.clone(), structural_view);
  let engine = StructuralEngine {
    game: StructuralGame {
      optional: Rc::clone(&optional),
      left_branch: Rc::clone(&left_branch),
      rc_branch: Rc::clone(&rc_branch),
      nested_node: Rc::clone(&nested_node),
      wrapped_fragment: Rc::clone(&wrapped_fragment),
    },
    reactant,
    document,
    snapshots: Rc::clone(&snapshots),
  };

  let mut client = FakeClient::connect(engine, catalog());

  let first = snapshots.borrow()[0].clone();
  assert_eq!(first.len(), 12);
  assert_eq!(client.ui().element(root_id).children(), first);
  optional.set(false);
  left_branch.set(false);
  rc_branch.set(false);
  nested_node.set(false);
  wrapped_fragment.set(false);
  client.reconnect();

  let second = snapshots.borrow()[1].clone();
  assert_eq!(second.len(), 11);
  assert_eq!(client.ui().element(root_id).children(), second);
  assert_eq!(&first[1..7], &second[..6]);
  assert_ne!(first[7], second[6]);
  assert_eq!(first[8], second[7]);
  assert_eq!(&first[9..], &second[8..]);
  assert_eq!(client.ui().element(second[0]).text(), Some("tuple"));
  assert_eq!(client.ui().element(second[6]).text(), Some("either-right"));
  assert_eq!(client.ui().element(second[7]).text(), Some("node"));
}

#[test]
fn invalid_registrations_fail_without_changing_the_root_set() {
  let mut reactant = Reactant::<()>::new(idle_spawner());
  let first = document(30, 31);
  reactant.register_root(first.clone(), |_| ());
  let populated = document(40, 41).child(UiNode::new(ObjectId::new_v4(), Label::new("invalid")));
  assert_panics(|| reactant.register_root(populated, |_| ()));
  assert_panics(|| reactant.register_root(document(30, 42), |_| ()));
  let second = document(50, 51);
  reactant.register_root(second.clone(), |_| ());
  let session_id = SessionId::new_v4();

  let response = reactant
    .begin_session(&mut ())
    .expect("empty roots render")
    .into_response(snapshot(session_id, &[first, second]));

  let ResponseMessage::Snapshot(snapshot) = &response.messages[0] else {
    panic!("session response did not begin with a snapshot");
  };
  assert_eq!(snapshot.ui.len(), 2);
}

#[test]
fn lifecycle_guards_and_baseline_entries_are_stable() {
  let mut registering = Reactant::<()>::new(idle_spawner());
  assert_panics(|| registering.refresh(&mut ()));
  assert_panics(|| registering.poll(&mut ()));
  assert_panics(|| registering.dispatch(&mut (), event()));
  assert_panics(|| registering.observe_geometry(&mut (), geometry()));
  assert!(registering.shutdown(&mut ()).is_empty());
  assert!(registering.shutdown(&mut ()).is_empty());
  assert_panics(|| registering.begin_session(&mut ()));

  let root = document(60, 61);
  let mut active = Reactant::<()>::new(idle_spawner());
  active.register_root(root.clone(), |_| Label::new("active"));
  let (first_snapshot, first_commit) = active
    .begin_session(&mut ())
    .expect("root renders")
    .into_parts(snapshot(SessionId::new_v4(), &[root]));
  assert!(first_commit.is_empty());
  let first_host = first_snapshot.ui[0].children[0].object_id;

  assert!(active.refresh(&mut ()).expect("active refresh").is_empty());
  assert!(active.poll(&mut ()).expect("active poll").is_empty());
  assert!(
    active
      .dispatch(&mut (), event())
      .expect("active dispatch")
      .is_empty()
  );
  assert!(
    active
      .observe_geometry(&mut (), geometry())
      .expect("active geometry")
      .is_empty()
  );
  assert_panics(|| active.register_root(document(70, 71), |_| ()));

  let reconnect_root = document(60, 61);
  let response = active
    .begin_session(&mut ())
    .expect("active reconnect renders")
    .into_response(snapshot(SessionId::new_v4(), &[reconnect_root]));
  let ResponseMessage::Snapshot(reconnected) = &response.messages[0] else {
    panic!("reconnect response did not begin with a snapshot");
  };
  assert_eq!(reconnected.ui[0].children[0].object_id, first_host);
  assert!(active.shutdown(&mut ()).is_empty());
  assert_panics(|| active.poll(&mut ()));
}

#[test]
fn dropping_an_unconverted_session_poisons_the_runtime() {
  let mut reactant = Reactant::<()>::new(idle_spawner());
  reactant.register_root(document(80, 81), |_| ());

  assert_panics(|| {
    let _session = reactant.begin_session(&mut ()).expect("empty root renders");
  });

  assert_panics(|| reactant.shutdown(&mut ()));
}

#[test]
fn spawned_task_cancels_once_or_can_be_disarmed() {
  let cancellations = Arc::new(AtomicUsize::new(0));
  let cancelled = Arc::clone(&cancellations);
  SpawnedTask::new(move || {
    cancelled.fetch_add(1, Ordering::Relaxed);
  })
  .cancel();
  let disarmed = Arc::clone(&cancellations);
  SpawnedTask::new(move || {
    disarmed.fetch_add(1, Ordering::Relaxed);
  })
  .disarm();
  assert_eq!(cancellations.load(Ordering::Relaxed), 1);
}

fn idle_spawner() -> IdleSpawner {
  IdleSpawner {
    calls: Rc::new(Cell::new(0)),
  }
}

fn document(document: u128, root: u128) -> UiDocument {
  UiDocument::with_root_id(object_id(document), object_id(root))
}

fn snapshot(session_id: SessionId, documents: &[UiDocument]) -> Snapshot {
  let camera_id = object_id(1);
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
    session_id,
    vec![PreparedAsset::Scene("test/scene".into())],
    vec![Scene::new(SceneId::new_v4(), "test/scene")],
    objects,
    camera_id,
  )
}

fn catalog() -> Arc<FakeAssetCatalog> {
  let mut catalog = FakeAssetCatalog::new();
  catalog.add_scene("test/scene");
  Arc::new(catalog)
}

fn event() -> UiEvent {
  UiEvent::click(ObjectId::new_v4(), ClickEvent::NavigationSubmit)
}

fn geometry() -> GeometryObservationBatch {
  GeometryObservationBatch {
    generation: GeometryGeneration(NonZeroU64::new(1).expect("one is nonzero")),
    changed: Vec::new(),
  }
}

fn object_id(value: u128) -> ObjectId {
  ObjectId::from_uuid(Uuid::from_u128(value)).expect("fixture ID is nonzero")
}

fn structural_view(game: &StructuralGame) -> impl Render + use<> {
  (
    (),
    game.optional.get().then(|| Label::new("optional")),
    (Label::new("tuple"),),
    [Label::new("array-a"), Label::new("array-b")],
    vec![Label::new("vector")],
    Rc::new(Label::new("rc")),
    Fragment::new((Label::new("fragment"), ())),
    structural_branch(game.left_branch.get()),
    Node::new(Label::new("node")),
    rc_branch(game.rc_branch.get()),
    nested_node(game.nested_node.get()),
    wrapped_fragment(game.wrapped_fragment.get()),
  )
}

fn structural_branch(left: bool) -> Either<Label, Fragment<Label>> {
  if left {
    Either::left(Label::new("either-left"))
  } else {
    Either::right(Fragment::new(Label::new("either-right")))
  }
}

fn rc_branch(left: bool) -> Either<Rc<Label>, Label> {
  if left {
    Either::left(Rc::new(Label::new("rc-branch")))
  } else {
    Either::right(Label::new("rc-branch"))
  }
}

fn nested_node(nested: bool) -> Node {
  if nested {
    Node::new(Node::new(Label::new("nested-node")))
  } else {
    Node::new(Label::new("nested-node"))
  }
}

fn wrapped_fragment(wrapped: bool) -> Either<Fragment<Rc<Label>>, Fragment<Label>> {
  if wrapped {
    Either::left(Fragment::new(Rc::new(Label::new("wrapped-fragment"))))
  } else {
    Either::right(Fragment::new(Label::new("wrapped-fragment")))
  }
}

fn assert_panics<R>(operation: impl FnOnce() -> R) {
  assert!(panic::catch_unwind(AssertUnwindSafe(operation)).is_err());
}
