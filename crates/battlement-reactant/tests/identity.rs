mod runtime_support;

use std::{cell::Cell, panic, panic::AssertUnwindSafe, rc::Rc};
use trox::ls;

use battlement::{
  CameraState, GameObject, GameObjectKind, ObjectId, PanelScaleMode, PanelSettings, ParentScene,
  PreparedAsset, Scene, SceneId, SessionId, Snapshot, UiDocument, UiDocumentState, UiNode,
};
use battlement_reactant::{
  component::Component,
  executor::{BoxFuture, SpawnedTask, Spawner},
  key::KeyRenderExt,
  render::{Either, Fragment, Render},
  runtime::Reactant,
};
use uuid::Uuid;

struct IdleSpawner;

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

struct KeyedGame {
  order: Vec<u8>,
}

struct OptionalGame {
  visible: bool,
}

struct MixedGame {
  keyed_first: bool,
}

struct DuplicateGame {
  keys: Vec<u8>,
  renders: Rc<Cell<usize>>,
}

#[derive(Clone)]
struct Badge {
  number: u8,
}

struct CountingBadge {
  renders: Rc<Cell<usize>>,
}

impl Component for Badge {
  fn render(&self) -> impl Render {
    battlement_reactant::host::Label::new(ls(format!("Badge {}", self.number)))
  }
}

impl Component for CountingBadge {
  fn render(&self) -> impl Render {
    self.renders.set(self.renders.get() + 1);
    battlement_reactant::host::Label::new(ls("counted"))
  }
}

#[test]
fn keyed_hosts_survive_insertion_removal_and_reorder() {
  let document = document();
  let mut game = KeyedGame {
    order: vec![1, 2, 3],
  };
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), keyed_labels);

  let initial = host_ids(render(&mut reactant, &mut game, &document));
  game.order = vec![0, 1, 2, 3];
  let inserted = host_ids(render(&mut reactant, &mut game, &document));
  assert_ne!(inserted[0], initial[0]);
  assert_eq!(&inserted[1..], initial);

  game.order = vec![0, 2, 3];
  let removed = host_ids(render(&mut reactant, &mut game, &document));
  assert_eq!(removed, [inserted[0], inserted[2], inserted[3]]);

  game.order = vec![3, 0, 2];
  let reordered = host_ids(render(&mut reactant, &mut game, &document));
  assert_eq!(reordered, [inserted[3], inserted[0], inserted[2]]);
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn keyed_components_and_fragments_match_their_semantic_nodes() {
  let component_document = document();
  let mut component_game = KeyedGame { order: vec![1, 2] };
  let mut components = runtime_support::reactant(IdleSpawner);
  components.register_root(component_document.clone(), keyed_badges);
  let component_ids = host_ids(render(
    &mut components,
    &mut component_game,
    &component_document,
  ));
  component_game.order.reverse();
  assert_eq!(
    host_ids(render(
      &mut components,
      &mut component_game,
      &component_document,
    )),
    [component_ids[1], component_ids[0]]
  );

  let fragment_document = document();
  let mut fragment_game = KeyedGame { order: vec![1, 2] };
  let mut fragments = runtime_support::reactant(IdleSpawner);
  fragments.register_root(fragment_document.clone(), keyed_fragments);
  let fragment_ids = host_ids(render(
    &mut fragments,
    &mut fragment_game,
    &fragment_document,
  ));
  fragment_game.order.reverse();
  assert_eq!(
    host_ids(render(
      &mut fragments,
      &mut fragment_game,
      &fragment_document,
    )),
    [
      fragment_ids[2],
      fragment_ids[3],
      fragment_ids[0],
      fragment_ids[1],
    ]
  );
  let _ = components.shutdown(&mut component_game).into_groups();
  let _ = fragments.shutdown(&mut fragment_game).into_groups();
}

#[test]
fn keyed_structural_ranges_preserve_every_host_when_reordered() {
  let document = document();
  let mut game = KeyedGame { order: vec![1, 2] };
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), keyed_ranges);

  let initial = host_ids(render(&mut reactant, &mut game, &document));
  game.order.reverse();
  assert_eq!(
    host_ids(render(&mut reactant, &mut game, &document)),
    [initial[2], initial[3], initial[0], initial[1]]
  );
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn empty_unkeyed_positions_retain_later_absolute_identity() {
  let document = document();
  let mut game = OptionalGame { visible: true };
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), optional_labels);

  let initial = host_ids(render(&mut reactant, &mut game, &document));
  game.visible = false;
  assert_eq!(
    host_ids(render(&mut reactant, &mut game, &document)),
    [initial[1]]
  );
  game.visible = true;
  let restored = host_ids(render(&mut reactant, &mut game, &document));
  assert_ne!(restored[0], initial[0]);
  assert_eq!(restored[1], initial[1]);
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn unkeyed_identity_uses_absolute_positions_around_keyed_siblings() {
  let document = document();
  let mut game = MixedGame { keyed_first: false };
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), mixed_labels);

  let initial = host_ids(render(&mut reactant, &mut game, &document));
  game.keyed_first = true;
  let shifted = host_ids(render(&mut reactant, &mut game, &document));
  assert_eq!(shifted[0], initial[1], "the keyed host follows its key");
  assert_ne!(
    shifted[1], initial[0],
    "an unkeyed host does not search another position"
  );
  assert_eq!(
    shifted[2], initial[2],
    "an unkeyed host at the same absolute position remains stable"
  );
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn keys_with_different_types_are_distinct() {
  let document = document();
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), |_| {
    (
      battlement_reactant::host::Label::new(ls("byte")).key(1_u8),
      battlement_reactant::host::Label::new(ls("word")).key(1_u16),
    )
  });
  let mut game = ();
  let initial = host_ids(render(&mut reactant, &mut game, &document));
  assert_ne!(initial[0], initial[1]);
  assert_eq!(
    host_ids(render(&mut reactant, &mut game, &document)),
    initial
  );
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn duplicate_same_typed_keys_panic_before_a_session_can_commit() {
  let document = document();
  let renders = Rc::new(Cell::new(0));
  let mut game = DuplicateGame {
    keys: vec![1, 2],
    renders: Rc::clone(&renders),
  };
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), counted_badges);
  let _committed = render(&mut reactant, &mut game, &document);
  assert_eq!(renders.get(), 2);
  game.keys = vec![1, 1];

  assert!(
    panic::catch_unwind(AssertUnwindSafe(|| {
      let _session = reactant.begin_session(&mut game);
    }))
    .is_err()
  );
  assert_eq!(
    renders.get(),
    3,
    "the duplicate component must be rejected before it renders"
  );
}

fn keyed_labels(game: &KeyedGame) -> impl Render + use<> {
  game
    .order
    .iter()
    .map(|number| battlement_reactant::host::Label::new(ls(format!("Label {number}"))).key(*number))
    .collect::<Vec<_>>()
}

fn keyed_badges(game: &KeyedGame) -> impl Render + use<> {
  game
    .order
    .iter()
    .map(|number| Badge { number: *number }.key(*number))
    .collect::<Vec<_>>()
}

fn keyed_fragments(game: &KeyedGame) -> impl Render + use<> {
  game
    .order
    .iter()
    .map(|number| {
      Fragment::new((
        battlement_reactant::host::Label::new(ls(format!("{number}a"))),
        battlement_reactant::host::Label::new(ls(format!("{number}b"))),
      ))
      .key(*number)
    })
    .collect::<Vec<_>>()
}

fn keyed_ranges(game: &KeyedGame) -> impl Render + use<> {
  game
    .order
    .iter()
    .map(|number| {
      (
        battlement_reactant::host::Label::new(ls(format!("{number}a"))),
        battlement_reactant::host::Label::new(ls(format!("{number}b"))),
      )
        .key(*number)
    })
    .collect::<Vec<_>>()
}

fn counted_badges(game: &DuplicateGame) -> impl Render + use<> {
  game
    .keys
    .iter()
    .map(|key| {
      CountingBadge {
        renders: Rc::clone(&game.renders),
      }
      .key(*key)
    })
    .collect::<Vec<_>>()
}

fn optional_labels(game: &OptionalGame) -> impl Render + use<> {
  (
    game
      .visible
      .then(|| battlement_reactant::host::Label::new(ls("optional"))),
    battlement_reactant::host::Label::new(ls("tail")),
  )
}

fn mixed_labels(game: &MixedGame) -> impl Render + use<> {
  if game.keyed_first {
    vec![
      Either::left(battlement_reactant::host::Label::new(ls("keyed")).key(7_u8)),
      Either::right(battlement_reactant::host::Label::new(ls("first"))),
      Either::right(battlement_reactant::host::Label::new(ls("last"))),
    ]
  } else {
    vec![
      Either::right(battlement_reactant::host::Label::new(ls("first"))),
      Either::left(battlement_reactant::host::Label::new(ls("keyed")).key(7_u8)),
      Either::right(battlement_reactant::host::Label::new(ls("last"))),
    ]
  }
}

fn render<G: 'static>(
  reactant: &mut Reactant<G>,
  game: &mut G,
  document: &UiDocument,
) -> Vec<UiNode> {
  reactant
    .begin_session(game)
    .expect("render succeeds")
    .into_parts(snapshot(document))
    .0
    .ui
    .remove(0)
    .children
}

fn host_ids(nodes: Vec<UiNode>) -> Vec<ObjectId> {
  nodes.into_iter().map(|node| node.object_id).collect()
}

fn document() -> UiDocument {
  UiDocument::with_root_id(ObjectId::new_v4(), ObjectId::new_v4())
}

fn snapshot(document: &UiDocument) -> Snapshot {
  let camera_id = object_id(1);
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

fn object_id(value: u128) -> ObjectId {
  ObjectId::from_uuid(Uuid::from_u128(value)).expect("fixture ID is nonzero")
}
