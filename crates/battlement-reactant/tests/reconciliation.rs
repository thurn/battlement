mod runtime_support;

use std::{panic, panic::AssertUnwindSafe};
use trox::ls;

use battlement::{
  CameraState, CommandBody, GameObject, GameObjectKind, ObjectId, PanelScaleMode, PanelSettings,
  ParentScene, PreparedAsset, Prop, Scene, SceneId, SessionId, Snapshot, Style, UiDocument,
  UiDocumentState, UsageHint,
};
use battlement_fake::battlement_ui_fake::{UiJournalEntry, UiWorld};
use battlement_reactant::{
  executor::{BoxFuture, SpawnedTask, Spawner},
  render::{Either, Render},
  runtime::ReactantCommit,
};

struct IdleSpawner;

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

struct Game {
  show: bool,
  alternate_kind: bool,
  text: Option<String>,
  name: Option<String>,
  width: Option<f32>,
}

struct HintGame {
  hint: Option<UsageHint>,
}

struct ConditionalPartGame {
  title: bool,
}

struct IndexedPartGame {
  choices: usize,
}

#[test]
fn refresh_reconciles_maximal_subtrees_sparse_properties_resets_and_replacement() {
  let document = document();
  let mut game = Game {
    show: false,
    alternate_kind: false,
    text: Some("Ready".to_owned()),
    name: Some("status".to_owned()),
    width: Some(120.0),
  };
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), view);
  let initial = reactant
    .begin_session(&mut game)
    .expect("initial render succeeds")
    .into_parts(snapshot(&document))
    .0;
  let mut world = UiWorld::default();
  world.replace(initial.ui).expect("initial tree is valid");
  let shell_id = world.element(document.root_id).unwrap().children()[0];

  game.show = true;
  let create = self::bodies(reactant.refresh(&mut game).expect("create renders"));
  assert_eq!(create.len(), 1);
  let CommandBody::VisualElementCreate(created) = &create[0] else {
    panic!("addition did not use one maximal create");
  };
  assert_eq!(created.node.children.len(), 1);
  let panel_id = created.node.object_id;
  let label_id = created.node.children[0].object_id;
  self::apply(&mut world, &create);
  assert_eq!(world.element(shell_id).unwrap().children(), [panel_id]);
  assert_eq!(world.element(label_id).unwrap().text(), Some("Ready"));

  game.text = Some("Playing".to_owned());
  game.width = Some(180.0);
  let update = self::bodies(reactant.refresh(&mut game).expect("update renders"));
  assert_eq!(update.len(), 1);
  let CommandBody::VisualElementUpdate(update_body) = &update[0] else {
    panic!("property change did not use an update");
  };
  let battlement::VisualElementUpdate::Properties { element, .. } = update_body.as_ref() else {
    panic!("property change used a hierarchy update");
  };
  let wire = serde_json::to_value(element).expect("patch serializes");
  assert_eq!(wire["Label"]["text"], "Playing");
  assert!(wire["Label"].get("name").is_none());
  assert_eq!(wire["Label"]["style"]["width"]["Px"], 180.0);
  self::apply(&mut world, &update);
  assert_eq!(world.element(label_id).unwrap().text(), Some("Playing"));

  let journal_len = world.journal().len();
  let unchanged = reactant
    .refresh(&mut game)
    .expect("identical render succeeds");
  assert!(unchanged.is_empty());
  self::apply(&mut world, &self::bodies(unchanged));
  assert_eq!(world.journal().len(), journal_len);

  game.text = None;
  game.name = None;
  game.width = None;
  let reset = reactant.refresh(&mut game).expect("reset renders");
  self::apply(&mut world, &self::bodies(reset));
  let label = world.element(label_id).unwrap();
  assert_eq!(label.text(), None);
  assert_eq!(label.name(), Some(""));
  assert!(matches!(label.style().width, Prop::Reset));

  game.alternate_kind = true;
  let replacement = self::bodies(reactant.refresh(&mut game).expect("replacement renders"));
  assert_eq!(replacement.len(), 2);
  assert!(matches!(
    replacement[0],
    CommandBody::VisualElementDestroy(_)
  ));
  let CommandBody::VisualElementCreate(recreated) = &replacement[1] else {
    panic!("replacement did not recreate the subtree");
  };
  let replacement_id = recreated.node.object_id;
  let replacement_label_id = recreated.node.children[0].object_id;
  assert_ne!(replacement_id, panel_id);
  assert_ne!(replacement_label_id, label_id);
  self::apply(&mut world, &replacement);
  assert!(world.element(panel_id).is_none());
  assert!(world.element(label_id).is_none());

  game.show = false;
  let removal = self::bodies(reactant.refresh(&mut game).expect("removal renders"));
  assert_eq!(removal.len(), 1);
  self::apply(&mut world, &removal);
  assert!(world.element(replacement_id).is_none());
  assert!(world.element(replacement_label_id).is_none());
  assert!(world.element(shell_id).unwrap().children().is_empty());
  assert!(
    matches!(world.journal().last(), Some(UiJournalEntry::Destroy(id)) if *id == replacement_id)
  );
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn failed_validation_leaves_the_committed_and_fake_trees_unchanged() {
  let document = document();
  let mut game = Game {
    show: true,
    alternate_kind: false,
    text: Some("Ready".to_owned()),
    name: None,
    width: Some(120.0),
  };
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), view);
  let initial = reactant
    .begin_session(&mut game)
    .expect("initial render succeeds")
    .into_parts(snapshot(&document))
    .0;
  let mut world = UiWorld::default();
  world.replace(initial.ui).expect("initial tree is valid");
  let shell_id = world.element(document.root_id).unwrap().children()[0];
  let panel_id = world.element(shell_id).unwrap().children()[0];
  let label_id = world.element(panel_id).unwrap().children()[0];
  let journal_len = world.journal().len();

  game.width = Some(f32::NAN);
  assert!(panic::catch_unwind(AssertUnwindSafe(|| reactant.refresh(&mut game))).is_err());
  assert_eq!(world.journal().len(), journal_len);
  assert_eq!(world.element(label_id).unwrap().text(), Some("Ready"));
  assert!(matches!(
    world.element(label_id).unwrap().style().width,
    Prop::Set(_)
  ));
}

#[test]
fn usage_hint_changes_and_removal_remount_the_maximal_subtree() {
  let document = document();
  let mut game = HintGame {
    hint: Some(UsageHint::DynamicTransform),
  };
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), hint_view);
  let initial = reactant
    .begin_session(&mut game)
    .expect("initial render succeeds")
    .into_parts(snapshot(&document))
    .0;
  let mut world = UiWorld::default();
  world.replace(initial.ui).expect("initial tree is valid");
  let first_id = world.element(document.root_id).unwrap().children()[0];
  let first_child_id = world.element(first_id).unwrap().children()[0];

  game.hint = Some(UsageHint::DynamicColor);
  let changed = self::bodies(reactant.refresh(&mut game).expect("hint change renders"));
  self::assert_remount(&changed, first_id, Some(first_child_id));
  self::apply(&mut world, &changed);
  let second_id = world.element(document.root_id).unwrap().children()[0];
  let second_child_id = world.element(second_id).unwrap().children()[0];
  assert_eq!(
    world.element(second_id).unwrap().usage_hints(),
    Some([UsageHint::DynamicColor].as_slice())
  );

  game.hint = None;
  let removed = self::bodies(reactant.refresh(&mut game).expect("hint removal renders"));
  self::assert_remount(&removed, second_id, Some(second_child_id));
  self::apply(&mut world, &removed);
  let third_id = world.element(document.root_id).unwrap().children()[0];
  assert_eq!(world.element(third_id).unwrap().usage_hints(), None);
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn removing_a_conditional_part_remounts_instead_of_emitting_an_invalid_patch() {
  let document = document();
  let mut game = ConditionalPartGame { title: true };
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), conditional_part_view);
  let initial = reactant
    .begin_session(&mut game)
    .expect("initial render succeeds")
    .into_parts(snapshot(&document))
    .0;
  let mut world = UiWorld::default();
  world.replace(initial.ui).expect("initial tree is valid");
  let first_id = world.element(document.root_id).unwrap().children()[0];

  game.title = false;
  let removed = self::bodies(reactant.refresh(&mut game).expect("part removal renders"));
  self::assert_remount(&removed, first_id, None);
  self::apply(&mut world, &removed);
  assert!(world.element(first_id).is_none());
  let _ = reactant.shutdown(&mut game).into_groups();
}

#[test]
fn removing_an_out_of_range_indexed_part_remounts_the_host() {
  let document = document();
  let mut game = IndexedPartGame { choices: 2 };
  let mut reactant = runtime_support::reactant(IdleSpawner);
  reactant.register_root(document.clone(), indexed_part_view);
  let initial = reactant
    .begin_session(&mut game)
    .expect("initial render succeeds")
    .into_parts(snapshot(&document))
    .0;
  let mut world = UiWorld::default();
  world.replace(initial.ui).expect("initial tree is valid");
  let first_id = world.element(document.root_id).unwrap().children()[0];

  game.choices = 1;
  let removed = self::bodies(
    reactant
      .refresh(&mut game)
      .expect("indexed removal renders"),
  );
  self::assert_remount(&removed, first_id, None);
  self::apply(&mut world, &removed);
  assert!(world.element(first_id).is_none());
  let _ = reactant.shutdown(&mut game).into_groups();
}

fn view(game: &Game) -> impl Render + use<> {
  let child = game.show.then(|| {
    if game.alternate_kind {
      Either::right(battlement_reactant::host::View::new().child(label(game)))
    } else {
      Either::left(battlement_reactant::host::Box::new().child(label(game)))
    }
  });
  battlement_reactant::host::View::new().child(child)
}

fn label(game: &Game) -> battlement_reactant::host::Label {
  battlement_reactant::host::Label::new(ls(""))
    .text(game.text.clone().map(trox::ls))
    .name(game.name.clone())
    .style(
      game
        .width
        .map_or_else(Style::new, |width| Style::new().width(width)),
    )
}

fn hint_view(game: &HintGame) -> impl Render + use<> {
  let host = match game.hint {
    Some(hint) => battlement_reactant::host::View::new().usage_hints([hint]),
    None => battlement_reactant::host::View::new(),
  };
  host.child(battlement_reactant::host::Label::new(ls("child")))
}

fn conditional_part_view(game: &ConditionalPartGame) -> battlement_reactant::host::GroupBox {
  if game.title {
    battlement_reactant::host::GroupBox::new()
      .text(ls("Title"))
      .title_style(Style::new().width(20.0))
  } else {
    battlement_reactant::host::GroupBox::new().text(ls(""))
  }
}

fn indexed_part_view(game: &IndexedPartGame) -> battlement_reactant::host::RadioButtonGroup {
  if game.choices == 2 {
    battlement_reactant::host::RadioButtonGroup::new()
      .choices([ls("Alpha"), ls("Beta")])
      .selected_index(0)
      .option_style(1, Style::new().width(20.0))
  } else {
    battlement_reactant::host::RadioButtonGroup::new()
      .choices([ls("Alpha")])
      .selected_index(0)
  }
}

fn assert_remount(commands: &[CommandBody], previous: ObjectId, previous_child: Option<ObjectId>) {
  assert_eq!(commands.len(), 2);
  let CommandBody::VisualElementDestroy(destroyed) = &commands[0] else {
    panic!("remount did not destroy the previous host");
  };
  assert_eq!(destroyed.object_id, previous);
  let CommandBody::VisualElementCreate(created) = &commands[1] else {
    panic!("remount did not create the replacement host");
  };
  assert_ne!(created.node.object_id, previous);
  if let Some(previous_child) = previous_child {
    assert_ne!(created.node.children[0].object_id, previous_child);
  }
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
        GameObjectKind::UiDocument(UiDocumentState::new(document.root_id).panel_settings(
          PanelSettings::new().scale_mode(PanelScaleMode::ConstantLogicalPixelSize),
        )),
      )
      .parent_scene(ParentScene::Persistent),
    ],
    camera_id,
  )
}
