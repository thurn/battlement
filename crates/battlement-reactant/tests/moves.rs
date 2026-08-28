use std::collections::HashMap;

use battlement::{
  Button, CameraState, CommandBody, GameObject, GameObjectKind, Label, ObjectId, PanelScaleMode,
  PanelSettings, ParentScene, PreparedAsset, Scene, SceneId, SessionId, Snapshot, Tab, TabView,
  ToggleButtonGroup, UiDocument, UiDocumentState, UiElement, VisualElementUpdate,
};
use battlement_fake::battlement_ui_fake::UiWorld;
use battlement_reactant::{
  executor::{BoxFuture, SpawnedTask, Spawner},
  key::KeyRenderExt,
  primitive::ContainerRenderExt,
  render::{Fragment, Render},
  runtime::{Reactant, ReactantCommit},
};

struct IdleSpawner;

struct Game {
  order: Vec<u8>,
}

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

#[test]
fn randomized_small_reorders_match_the_fake_tree_with_minimal_moves() {
  let document = document();
  let mut game = Game {
    order: vec![0, 1, 2, 3, 4, 5],
  };
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), keyed_labels);
  let initial = begin(&mut reactant, &mut game, &document);
  let mut world = UiWorld::default();
  world.replace(initial).expect("initial tree is valid");

  for desired in generated_orders() {
    let previous = game.order.clone();
    let previous_ids = ids_by_key(&world, document.root_id, &previous);
    game.order = desired.clone();
    let commands = self::bodies(reactant.refresh(&mut game).expect("reorder renders"));
    assert_eq!(
      self::index_move_count(&commands),
      retained_count(&previous, &desired) - lis_length(&previous, &desired)
    );
    self::apply(&mut world, &commands);
    assert_eq!(
      child_text(&world, document.root_id),
      expected_text(&desired)
    );
    let current_ids = ids_by_key(&world, document.root_id, &desired);
    for key in desired.iter().filter(|key| previous.contains(key)) {
      assert_eq!(current_ids[key], previous_ids[key]);
    }
  }
}

#[test]
fn lis_ties_retain_the_lexicographically_earliest_desired_indices() {
  let document = document();
  let mut game = Game {
    order: vec![1, 2, 3, 4],
  };
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), keyed_labels);
  let initial = begin(&mut reactant, &mut game, &document);
  let mut world = UiWorld::default();
  world.replace(initial).expect("initial tree is valid");
  let ids = ids_by_key(&world, document.root_id, &game.order);

  game.order = vec![3, 4, 1, 2];
  let commands = self::bodies(reactant.refresh(&mut game).expect("reorder renders"));
  let moved = commands
    .iter()
    .filter_map(|command| match command {
      CommandBody::VisualElementUpdate(value) => match value.as_ref() {
        VisualElementUpdate::Index { object_id, .. } => Some(*object_id),
        _ => None,
      },
      _ => None,
    })
    .collect::<Vec<_>>();
  assert_eq!(moved, [ids[&2], ids[&1]]);
  self::apply(&mut world, &commands);
  assert_eq!(
    child_text(&world, document.root_id),
    expected_text(&game.order)
  );
}

#[test]
fn zero_and_multi_host_ranges_reorder_and_restore_as_physical_children() {
  let document = document();
  let mut game = Game {
    order: vec![1, 2, 3],
  };
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), keyed_ranges);
  let initial = begin(&mut reactant, &mut game, &document);
  let mut world = UiWorld::default();
  world.replace(initial).expect("initial tree is valid");
  let original = world.element(document.root_id).unwrap().children().to_vec();

  game.order.reverse();
  let reversed = self::bodies(reactant.refresh(&mut game).expect("range reorder renders"));
  assert_eq!(self::index_move_count(&reversed), 2);
  self::apply(&mut world, &reversed);
  assert_eq!(
    world.element(document.root_id).unwrap().children(),
    [original[2], original[3], original[0], original[1]]
  );

  game.order.reverse();
  let restored = self::bodies(reactant.refresh(&mut game).expect("range restore renders"));
  assert_eq!(self::index_move_count(&restored), 2);
  self::apply(&mut world, &restored);
  assert_eq!(
    world.element(document.root_id).unwrap().children(),
    original
  );
}

#[test]
fn toggle_group_reorders_and_replacements_restore_the_controlled_selection() {
  let document = document();
  let mut game = Game { order: vec![1, 2] };
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), toggle_group);
  let initial = begin(&mut reactant, &mut game, &document);
  let mut world = UiWorld::default();
  world.replace(initial).expect("initial tree is valid");
  let group_id = world.element(document.root_id).unwrap().children()[0];

  game.order.reverse();
  let reordered = reactant.refresh(&mut game).expect("group reorder renders");
  self::apply(&mut world, &self::bodies(reordered));
  assert_eq!(
    world.element(group_id).unwrap().selected_indices(),
    Some(&[0][..])
  );

  game.order = vec![3, 2];
  let replaced = reactant
    .refresh(&mut game)
    .expect("group replacement renders");
  self::apply(&mut world, &self::bodies(replaced));
  assert_eq!(
    world.element(group_id).unwrap().selected_indices(),
    Some(&[0][..])
  );
}

#[test]
fn tab_reorders_restore_the_controlled_selected_index() {
  let document = document();
  let mut game = Game { order: vec![1, 2] };
  let mut reactant = Reactant::new(IdleSpawner);
  reactant.register_root(document.clone(), tabs);
  let initial = begin(&mut reactant, &mut game, &document);
  let mut world = UiWorld::default();
  world.replace(initial).expect("initial tree is valid");
  let tabs_id = world.element(document.root_id).unwrap().children()[0];

  game.order.reverse();
  let reordered = reactant.refresh(&mut game).expect("tab reorder renders");
  self::apply(&mut world, &self::bodies(reordered));
  let UiElement::TabView(value) = world.element(tabs_id).unwrap().element() else {
    panic!("rendered host is not a tab view");
  };
  assert!(matches!(value.selected_tab_index, battlement::Prop::Set(0)));
}

fn keyed_labels(game: &Game) -> impl Render + use<> {
  game
    .order
    .iter()
    .map(|key| Label::new(format!("Label {key}")).key(*key))
    .collect::<Vec<_>>()
}

fn keyed_ranges(game: &Game) -> impl Render + use<> {
  game
    .order
    .iter()
    .map(|key| {
      let children = if *key == 2 {
        Vec::new()
      } else {
        vec![Label::new(format!("{key}a")), Label::new(format!("{key}b"))]
      };
      Fragment::new(children).key(*key)
    })
    .collect::<Vec<_>>()
}

fn toggle_group(game: &Game) -> impl Render + use<> {
  ToggleButtonGroup::new().selected_indices([0]).child(
    game
      .order
      .iter()
      .map(|key| Button::new(format!("Button {key}")).key(*key))
      .collect::<Vec<_>>(),
  )
}

fn tabs(game: &Game) -> impl Render + use<> {
  TabView::new().selected_tab_index(0).child(
    game
      .order
      .iter()
      .map(|key| Tab::new(format!("Tab {key}")).key(*key))
      .collect::<Vec<_>>(),
  )
}

fn generated_orders() -> Vec<Vec<u8>> {
  let mut seed = 0x9e37_79b9_u32;
  (0..64)
    .map(|_| {
      let mut values = vec![0, 1, 2, 3, 4, 5];
      for index in (1..values.len()).rev() {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        values.swap(index, seed as usize % (index + 1));
      }
      seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
      values.truncate(seed as usize % 7);
      values
    })
    .collect()
}

fn retained_count(previous: &[u8], desired: &[u8]) -> usize {
  desired.iter().filter(|key| previous.contains(key)).count()
}

fn lis_length(previous: &[u8], desired: &[u8]) -> usize {
  let indices = desired
    .iter()
    .filter_map(|key| previous.iter().position(|candidate| candidate == key))
    .collect::<Vec<_>>();
  let mut lengths = vec![1; indices.len()];
  for index in 0..indices.len() {
    for earlier in 0..index {
      if indices[earlier] < indices[index] {
        lengths[index] = lengths[index].max(lengths[earlier] + 1);
      }
    }
  }
  lengths.into_iter().max().unwrap_or(0)
}

fn index_move_count(commands: &[CommandBody]) -> usize {
  commands
    .iter()
    .filter(|command| {
      matches!(
        command,
        CommandBody::VisualElementUpdate(value)
          if matches!(value.as_ref(), VisualElementUpdate::Index { .. })
      )
    })
    .count()
}

fn ids_by_key(world: &UiWorld, root_id: ObjectId, keys: &[u8]) -> HashMap<u8, ObjectId> {
  keys
    .iter()
    .copied()
    .zip(world.element(root_id).unwrap().children().iter().copied())
    .collect()
}

fn child_text(world: &UiWorld, root_id: ObjectId) -> Vec<String> {
  world
    .element(root_id)
    .unwrap()
    .children()
    .iter()
    .map(|object_id| {
      world
        .element(*object_id)
        .unwrap()
        .text()
        .unwrap()
        .to_owned()
    })
    .collect()
}

fn expected_text(keys: &[u8]) -> Vec<String> {
  keys.iter().map(|key| format!("Label {key}")).collect()
}

fn begin(reactant: &mut Reactant<Game>, game: &mut Game, document: &UiDocument) -> Vec<UiDocument> {
  reactant
    .begin_session(game)
    .expect("initial render succeeds")
    .into_parts(snapshot(document))
    .0
    .ui
}

fn bodies(commit: ReactantCommit) -> Vec<CommandBody> {
  commit.into_groups().into_iter().flatten().collect()
}

fn apply(world: &mut UiWorld, commands: &[CommandBody]) {
  for command in commands {
    match command {
      CommandBody::VisualElementCreate(value) => world.create(*value.clone()).unwrap(),
      CommandBody::VisualElementUpdate(value) => world.update(*value.clone()).unwrap(),
      CommandBody::VisualElementDestroy(value) => world.destroy(value.object_id).unwrap(),
      _ => panic!("Reactant emitted a non-UI command"),
    }
  }
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
