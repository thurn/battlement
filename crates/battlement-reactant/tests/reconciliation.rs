use std::{panic, panic::AssertUnwindSafe};

use battlement::{
  Box as UiBox, CameraState, CommandBody, GameObject, GameObjectKind, GroupBox, Label, ObjectId,
  PanelScaleMode, PanelSettings, ParentScene, PreparedAsset, Prop, RadioButtonGroup, Scene,
  SceneId, SessionId, Snapshot, Style, UiDocument, UiDocumentState, UsageHint, VisualElement,
};
use battlement_fake::battlement_ui_fake::{UiJournalEntry, UiWorld};
use battlement_reactant::{
  executor::{BoxFuture, SpawnedTask, Spawner},
  primitive::ContainerRenderExt,
  render::{Either, Render},
  runtime::{Reactant, ReactantCommit},
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
  let mut reactant = Reactant::new(IdleSpawner);
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
  let create = reactant.refresh(&mut game).expect("create renders");
  assert_eq!(create.commands().len(), 1);
  let CommandBody::VisualElementCreate(created) = &create.commands()[0].body else {
    panic!("addition did not use one maximal create");
  };
  assert_eq!(created.node.children.len(), 1);
  let panel_id = created.node.object_id;
  let label_id = created.node.children[0].object_id;
  apply(&mut world, &create);
  assert_eq!(world.element(shell_id).unwrap().children(), [panel_id]);
  assert_eq!(world.element(label_id).unwrap().text(), Some("Ready"));

  game.text = Some("Playing".to_owned());
  game.width = Some(180.0);
  let update = reactant.refresh(&mut game).expect("update renders");
  assert_eq!(update.commands().len(), 1);
  let CommandBody::VisualElementUpdate(update_body) = &update.commands()[0].body else {
    panic!("property change did not use an update");
  };
  let battlement::VisualElementUpdate::Properties { element, .. } = update_body.as_ref() else {
    panic!("property change used a hierarchy update");
  };
  let wire = serde_json::to_value(element).expect("patch serializes");
  assert_eq!(wire["Label"]["text"], "Playing");
  assert!(wire["Label"].get("name").is_none());
  assert_eq!(wire["Label"]["style"]["width"]["Px"], 180.0);
  apply(&mut world, &update);
  assert_eq!(world.element(label_id).unwrap().text(), Some("Playing"));

  let journal_len = world.journal().len();
  let unchanged = reactant
    .refresh(&mut game)
    .expect("identical render succeeds");
  assert!(unchanged.is_empty());
  apply(&mut world, &unchanged);
  assert_eq!(world.journal().len(), journal_len);

  game.text = None;
  game.name = None;
  game.width = None;
  let reset = reactant.refresh(&mut game).expect("reset renders");
  apply(&mut world, &reset);
  let label = world.element(label_id).unwrap();
  assert_eq!(label.text(), None);
  assert_eq!(label.name(), Some(""));
  assert!(matches!(label.style().width, Prop::Reset));

  game.alternate_kind = true;
  let replacement = reactant.refresh(&mut game).expect("replacement renders");
  assert_eq!(replacement.commands().len(), 2);
  assert!(matches!(
    replacement.commands()[0].body,
    CommandBody::VisualElementDestroy(_)
  ));
  let CommandBody::VisualElementCreate(recreated) = &replacement.commands()[1].body else {
    panic!("replacement did not recreate the subtree");
  };
  let replacement_id = recreated.node.object_id;
  let replacement_label_id = recreated.node.children[0].object_id;
  assert_ne!(replacement_id, panel_id);
  assert_ne!(replacement_label_id, label_id);
  apply(&mut world, &replacement);
  assert!(world.element(panel_id).is_none());
  assert!(world.element(label_id).is_none());

  game.show = false;
  let removal = reactant.refresh(&mut game).expect("removal renders");
  assert_eq!(removal.commands().len(), 1);
  apply(&mut world, &removal);
  assert!(world.element(replacement_id).is_none());
  assert!(world.element(replacement_label_id).is_none());
  assert!(world.element(shell_id).unwrap().children().is_empty());
  assert!(
    matches!(world.journal().last(), Some(UiJournalEntry::Destroy(id)) if *id == replacement_id)
  );
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
  let mut reactant = Reactant::new(IdleSpawner);
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
  let mut reactant = Reactant::new(IdleSpawner);
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
  let changed = reactant.refresh(&mut game).expect("hint change renders");
  assert_remount(&changed, first_id, Some(first_child_id));
  apply(&mut world, &changed);
  let second_id = world.element(document.root_id).unwrap().children()[0];
  let second_child_id = world.element(second_id).unwrap().children()[0];
  assert_eq!(
    world.element(second_id).unwrap().usage_hints(),
    Some([UsageHint::DynamicColor].as_slice())
  );

  game.hint = None;
  let removed = reactant.refresh(&mut game).expect("hint removal renders");
  assert_remount(&removed, second_id, Some(second_child_id));
  apply(&mut world, &removed);
  let third_id = world.element(document.root_id).unwrap().children()[0];
  assert_eq!(world.element(third_id).unwrap().usage_hints(), None);
}

#[test]
fn removing_a_conditional_part_remounts_instead_of_emitting_an_invalid_patch() {
  let document = document();
  let mut game = ConditionalPartGame { title: true };
  let mut reactant = Reactant::new(IdleSpawner);
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
  let removed = reactant.refresh(&mut game).expect("part removal renders");
  assert_remount(&removed, first_id, None);
  apply(&mut world, &removed);
  assert!(world.element(first_id).is_none());
}

#[test]
fn removing_an_out_of_range_indexed_part_remounts_the_host() {
  let document = document();
  let mut game = IndexedPartGame { choices: 2 };
  let mut reactant = Reactant::new(IdleSpawner);
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
  let removed = reactant
    .refresh(&mut game)
    .expect("indexed removal renders");
  assert_remount(&removed, first_id, None);
  apply(&mut world, &removed);
  assert!(world.element(first_id).is_none());
}

fn view(game: &Game) -> impl Render + use<> {
  let child = game.show.then(|| {
    if game.alternate_kind {
      Either::right(VisualElement::new().child(label(game)))
    } else {
      Either::left(UiBox::new().child(label(game)))
    }
  });
  VisualElement::new().child(child)
}

fn label(game: &Game) -> Label {
  Label::new("")
    .text(game.text.clone())
    .name(game.name.clone())
    .style(
      game
        .width
        .map_or_else(Style::new, |width| Style::new().width(width)),
    )
}

fn hint_view(game: &HintGame) -> impl Render + use<> {
  let mut host = VisualElement::new();
  host.usage_hints = game.hint.map(|hint| vec![hint]);
  host.child(Label::new("child"))
}

fn conditional_part_view(game: &ConditionalPartGame) -> GroupBox {
  if game.title {
    GroupBox::new()
      .text("Title")
      .title_style(Style::new().width(20.0))
  } else {
    GroupBox::new().text("")
  }
}

fn indexed_part_view(game: &IndexedPartGame) -> RadioButtonGroup {
  if game.choices == 2 {
    RadioButtonGroup::new()
      .choices(["Alpha", "Beta"])
      .selected_index(0)
      .option_style(1, Style::new().width(20.0))
  } else {
    RadioButtonGroup::new().choices(["Alpha"]).selected_index(0)
  }
}

fn assert_remount(commit: &ReactantCommit, previous: ObjectId, previous_child: Option<ObjectId>) {
  assert_eq!(commit.commands().len(), 2);
  let CommandBody::VisualElementDestroy(destroyed) = &commit.commands()[0].body else {
    panic!("remount did not destroy the previous host");
  };
  assert_eq!(destroyed.object_id, previous);
  let CommandBody::VisualElementCreate(created) = &commit.commands()[1].body else {
    panic!("remount did not create the replacement host");
  };
  assert_ne!(created.node.object_id, previous);
  if let Some(previous_child) = previous_child {
    assert_ne!(created.node.children[0].object_id, previous_child);
  }
}

fn apply(world: &mut UiWorld, commit: &ReactantCommit) {
  for command in commit.commands() {
    match &command.body {
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
