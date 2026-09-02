use battlement::{
  AccessibilityAction, AccessibilityUpdate, CameraState, ClickEvent, CommandBody, CurrentPage,
  GameObject, ObjectId, PreparedAsset, Scene, SceneId, SemanticRole, SessionId, Snapshot,
  UiAccessibilityAction, UiAccessibilityActionEvent, UiDocument, UiDocumentState, UiEvent,
  UiEventBody, UiEventDisposition,
};
use battlement_reactant::{
  accessibility::{ButtonOptions, ChoiceOptions, use_button},
  accessibility_collections as collections,
  component::Component,
  element_ref::use_element_ref,
  executor::{BoxFuture, SpawnedTask, Spawner},
  host::{Button, Label, View},
  render::Render,
  runtime::Reactant,
  semantics::{AccessibleName, SemanticProps, SemanticVisibility, text},
};

#[derive(Default)]
struct Game {
  presses: usize,
  selection: usize,
}

#[derive(Clone, Copy)]
struct IdleSpawner;

#[derive(Clone, Copy)]
struct NameSourceFixture;

impl Component for NameSourceFixture {
  fn render(&self) -> impl Render {
    let source = use_element_ref();
    let button = use_button(ButtonOptions {
      name: AccessibleName::LabelledBy(vec![source.clone()]),
      is_disabled: false,
      on_press: |_game: &mut Game| {},
    });
    View::new().child((
      Label::new("Account settings").element_ref(source).semantic(
        SemanticProps::new(SemanticRole::StaticText)
          .name(AccessibleName::text("Account settings"))
          .visibility(SemanticVisibility::NameSourceOnly),
      ),
      View::new()
        .semantic(button.semantic)
        .focus_props(button.focus)
        .interaction_props(button.interaction),
    ))
  }
}

struct MultiNameFixture {
  value: &'static str,
}

impl Component for MultiNameFixture {
  fn render(&self) -> impl Render {
    let title = use_element_ref();
    let value = use_element_ref();
    let behavior = use_button(ButtonOptions {
      name: AccessibleName::LabelledBy(vec![title.clone(), value.clone()]),
      is_disabled: false,
      on_press: |game: &mut Game| game.presses += 1,
    });
    View::new().child((
      Label::new("Quality").element_ref(title).semantic(
        SemanticProps::new(SemanticRole::StaticText)
          .name(AccessibleName::text(" Quality "))
          .visibility(SemanticVisibility::NameSourceOnly),
      ),
      Button::new("")
        .semantic(behavior.semantic)
        .focus_props(behavior.focus)
        .interaction_props(behavior.interaction)
        .child(
          Label::new(self.value).element_ref(value).semantic(
            SemanticProps::new(SemanticRole::StaticText)
              .name(AccessibleName::text(self.value))
              .visibility(SemanticVisibility::NameSourceOnly),
          ),
        ),
    ))
  }
}

#[test]
fn button_children_resolve_ordered_names_and_keep_activation_when_values_update() {
  let document = document();
  let mut runtime = Reactant::new(IdleSpawner);
  runtime.register_root(document.clone(), |game: &Game| MultiNameFixture {
    value: if game.selection == 0 {
      " High "
    } else {
      " Low "
    },
  });
  let mut game = Game::default();
  let (initial, commit) = runtime
    .begin_session(&mut game)
    .unwrap()
    .into_parts(snapshot(&document));
  let groups = commit.into_groups();
  let semantic = accessibility_update(&groups).snapshot.as_ref().unwrap();
  assert_eq!(semantic.nodes.len(), 1);
  assert_eq!(semantic.nodes[0].label.as_deref(), Some("Quality High"));
  let button = semantic.nodes[0].object_id;
  let wrapper = &initial.ui[0].children[0];
  let child = wrapper.children[1].children[0].object_id;
  let _ = runtime
    .dispatch(
      &mut game,
      UiEvent::click(child, ClickEvent::NavigationSubmit),
    )
    .unwrap()
    .into_commit()
    .into_groups();
  assert_eq!(game.presses, 1);
  game.selection = 1;
  let groups = runtime.refresh(&mut game).unwrap().into_groups();
  let semantic = accessibility_update(&groups).snapshot.as_ref().unwrap();
  assert_eq!(semantic.nodes.len(), 1);
  assert_eq!(semantic.nodes[0].object_id, button);
  assert_eq!(semantic.nodes[0].label.as_deref(), Some("Quality Low"));
  assert!(!groups.iter().flatten().any(|body| matches!(
    body,
    CommandBody::VisualElementCreate(_) | CommandBody::VisualElementDestroy(_)
  )));
  let _ = runtime
    .dispatch(
      &mut game,
      UiEvent::click(child, ClickEvent::NavigationSubmit),
    )
    .unwrap()
    .into_commit()
    .into_groups();
  assert_eq!(game.presses, 2);
  let _ = runtime.shutdown(&mut game).into_groups();
}

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

#[test]
fn complete_snapshot_resolves_contents_and_prunes_hidden_subtrees() {
  let document = document();
  let mut runtime = Reactant::new(IdleSpawner);
  runtime.register_root(document.clone(), |_game: &Game| {
    let button = use_button(ButtonOptions {
      name: AccessibleName::Contents,
      is_disabled: false,
      on_press: |_game: &mut Game| {},
    });
    View::new()
      .semantic(SemanticProps::new(SemanticRole::Group))
      .child((View::new()
        .semantic(button.semantic)
        .focus_props(button.focus)
        .interaction_props(button.interaction)
        .child((
          Label::new("Save").semantic(
            SemanticProps::new(SemanticRole::StaticText)
              .name(AccessibleName::Text(text(" Save   changes "))),
          ),
          View::new()
            .semantic(
              SemanticProps::new(SemanticRole::Group).visibility(SemanticVisibility::Hidden),
            )
            .child(
              Label::new("secret").semantic(
                SemanticProps::new(SemanticRole::StaticText)
                  .name(AccessibleName::Text(text("Secret"))),
              ),
            ),
        )),))
  });
  let mut game = Game::default();
  let groups = runtime
    .begin_session(&mut game)
    .expect("semantic render succeeds")
    .into_parts(snapshot(&document))
    .1
    .into_groups();
  let update = accessibility_update(&groups);
  let snapshot = update.snapshot.as_ref().expect("complete snapshot");
  assert_eq!(snapshot.nodes.len(), 3);
  let button = snapshot
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::Button)
    .expect("button node");
  assert_eq!(button.label.as_deref(), Some("Save changes"));
  assert!(
    snapshot
      .nodes
      .iter()
      .all(|node| node.label.as_deref() != Some("Secret"))
  );
  let _ = runtime.shutdown(&mut game).into_groups();
}

#[test]
fn name_source_only_hosts_resolve_without_becoming_nodes() {
  let document = document();
  let mut runtime = Reactant::new(IdleSpawner);
  runtime.register_root(document.clone(), |_game: &Game| NameSourceFixture);
  let mut game = Game::default();
  let groups = runtime
    .begin_session(&mut game)
    .expect("semantic render succeeds")
    .into_parts(snapshot(&document))
    .1
    .into_groups();
  let snapshot = accessibility_update(&groups).snapshot.as_ref().unwrap();
  assert_eq!(snapshot.nodes.len(), 1);
  assert_eq!(snapshot.nodes[0].label.as_deref(), Some("Account settings"));
  let _ = runtime.shutdown(&mut game).into_groups();
}

#[test]
fn accessibility_activation_uses_the_ordinary_logical_event_path() {
  let document = document();
  let mut runtime = Reactant::new(IdleSpawner);
  runtime.register_root(document.clone(), |_game: &Game| {
    let button = use_button(ButtonOptions {
      name: text("Save changes"),
      is_disabled: false,
      on_press: |game: &mut Game| game.presses += 1,
    });
    View::new()
      .semantic(button.semantic)
      .focus_props(button.focus)
      .interaction_props(button.interaction)
  });
  let mut game = Game::default();
  let groups = runtime
    .begin_session(&mut game)
    .expect("semantic render succeeds")
    .into_parts(snapshot(&document))
    .1
    .into_groups();
  let target = accessibility_update(&groups)
    .snapshot
    .as_ref()
    .unwrap()
    .roots[0];
  let result = runtime
    .dispatch(
      &mut game,
      UiEvent::new(
        target,
        true,
        false,
        UiEventBody::AccessibilityAction(UiAccessibilityActionEvent {
          backend_generation: 1,
          action: UiAccessibilityAction::Activate,
        }),
      ),
    )
    .expect("accessibility action renders");
  assert_eq!(result.disposition(), UiEventDisposition::PreventDefault);
  assert_eq!(game.presses, 1);
  assert!(
    result
      .into_groups()
      .iter()
      .flatten()
      .all(|command| !matches!(command, CommandBody::AccessibilityUpdate(_)))
  );
  let _ = runtime.shutdown(&mut game).into_groups();
}

#[test]
fn protocol_round_trips_direct_actions_and_complete_snapshot() {
  let update = AccessibilityUpdate {
    snapshot: Some(battlement::AccessibilitySnapshot {
      commit_sequence: 7,
      roots: vec![],
      nodes: vec![],
    }),
    announcements: vec!["Saved".to_owned()],
  };
  let bytes = battlement::json::to_vec(&CommandBody::AccessibilityUpdate(update.clone())).unwrap();
  assert_eq!(
    battlement::json::from_slice::<CommandBody>(&bytes).unwrap(),
    CommandBody::AccessibilityUpdate(update)
  );
  let action = AccessibilityAction::Scroll(battlement::AccessibilityScrollDirection::Forward);
  let bytes = battlement::json::to_vec(&action).unwrap();
  assert_eq!(
    battlement::json::from_slice::<AccessibilityAction>(&bytes).unwrap(),
    action
  );
}

#[test]
fn collections_preserve_roles_ancestry_current_page_and_controlled_selection() {
  let document = document();
  let mut runtime = Reactant::new(IdleSpawner);
  runtime.register_root(document.clone(), collection_fixture);
  let mut game = Game::default();
  let groups = runtime
    .begin_session(&mut game)
    .unwrap()
    .into_parts(snapshot(&document))
    .1
    .into_groups();
  let initial = accessibility_update(&groups).snapshot.as_ref().unwrap();
  let by_name = |name: &str| {
    initial
      .nodes
      .iter()
      .find(|node| node.label.as_deref() == Some(name))
      .unwrap()
  };
  let navigation = by_name("Review pages");
  assert_eq!(navigation.role, SemanticRole::Navigation);
  let page = by_name("Gallery shell");
  assert_eq!(page.state.current, Some(CurrentPage::Page));
  assert_eq!(page.parent_id, Some(navigation.object_id));
  let region = by_name("Settings");
  assert_eq!(region.role, SemanticRole::Region);
  let table = by_name("Bindings");
  let row = initial
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::Row)
    .unwrap();
  assert_eq!(row.parent_id, Some(table.object_id));
  for (name, role) in [
    ("Keyboard", SemanticRole::ColumnHeader),
    ("Move", SemanticRole::RowHeader),
    ("W", SemanticRole::Cell),
  ] {
    assert_eq!(by_name(name).parent_id, Some(row.object_id));
    assert_eq!(by_name(name).role, role);
    assert!(!by_name(name).actions.activate);
  }
  let listbox = by_name("Quality");
  let selected = by_name("Standard");
  assert_eq!(selected.parent_id, Some(listbox.object_id));
  assert_eq!(selected.state.selected, Some(true));
  let high = by_name("High").object_id;
  let unavailable = by_name("Unavailable").object_id;
  let link = by_name("Privacy policy").object_id;
  assert_eq!(by_name("Privacy policy").role, SemanticRole::Link);
  assert!(by_name("Privacy policy").actions.activate);
  let result = runtime.dispatch(&mut game, activation(high)).unwrap();
  assert_eq!(game.selection, 1);
  let groups = result.into_groups();
  let changed = accessibility_update(&groups).snapshot.as_ref().unwrap();
  assert_eq!(
    changed
      .nodes
      .iter()
      .filter(|node| node.state.selected == Some(true))
      .count(),
    1
  );
  assert_eq!(
    changed
      .nodes
      .iter()
      .find(|node| node.object_id == high)
      .unwrap()
      .state
      .selected,
    Some(true)
  );
  let _ = runtime
    .dispatch(&mut game, activation(unavailable))
    .unwrap()
    .into_groups();
  assert_eq!(game.selection, 1);
  let _ = runtime
    .dispatch(&mut game, activation(link))
    .unwrap()
    .into_groups();
  assert_eq!(game.presses, 1);
  let bytes = battlement::json::to_vec(initial).unwrap();
  assert_eq!(
    battlement::json::from_slice::<battlement::AccessibilitySnapshot>(&bytes).unwrap(),
    *initial
  );
  let _ = runtime.shutdown(&mut game).into_groups();
}

#[test]
fn invalid_collection_relationships_and_page_states_fail_before_commit() {
  let cases = [
    View::new().semantic(collections::use_row()),
    View::new()
      .semantic(collections::use_listbox(text("Quality")))
      .child(Label::new("Wrong child").semantic(collections::use_cell(text("Wrong child")))),
    View::new()
      .semantic(collections::use_table(text("Bindings")))
      .child(Label::new("Wrong child").semantic(collections::use_cell(text("Wrong child")))),
    View::new().semantic(collections::use_region(text("Settings")).state(
      battlement::SemanticState {
        current: Some(CurrentPage::Page),
        ..Default::default()
      },
    )),
  ];
  for view in cases {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
      let mut runtime = Reactant::new(IdleSpawner);
      runtime.register_root(document(), move |_game: &Game| view.clone());
      runtime.begin_session(&mut Game::default()).is_err()
    }));
    assert!(result.is_err() || result.unwrap());
  }
}

fn collection_fixture(game: &Game) -> View {
  let mut page = use_button(ButtonOptions {
    name: text("Gallery shell"),
    is_disabled: false,
    on_press: |_game: &mut Game| {},
  });
  page.semantic.state.current = Some(CurrentPage::Page);
  let link = collections::use_link(ButtonOptions {
    name: text("Privacy policy"),
    is_disabled: false,
    on_press: |game: &mut Game| game.presses += 1,
  });
  View::new().child((
    View::new()
      .semantic(collections::use_navigation(text("Review pages")))
      .child(
        View::new()
          .semantic(page.semantic)
          .focus_props(page.focus)
          .interaction_props(page.interaction),
      ),
    View::new()
      .semantic(collections::use_region(text("Settings")))
      .child((
        View::new()
          .semantic(collections::use_listbox(text("Quality")))
          .child(
            ["Standard", "High", "Unavailable"]
              .into_iter()
              .enumerate()
              .map(|(index, name)| {
                let option = collections::use_option(ChoiceOptions {
                  name: text(name),
                  selected: game.selection == index,
                  is_disabled: index == 2,
                  on_select: move |game: &mut Game| game.selection = index,
                });
                View::new()
                  .semantic(option.semantic)
                  .focus_props(option.focus)
                  .interaction_props(option.interaction)
              })
              .collect::<Vec<_>>(),
          ),
        View::new()
          .semantic(collections::use_table(text("Bindings")))
          .child(View::new().semantic(collections::use_row()).child((
            Label::new("Keyboard").semantic(collections::use_column_header(text("Keyboard"))),
            Label::new("Move").semantic(collections::use_row_header(text("Move"))),
            Label::new("W").semantic(collections::use_cell(text("W"))),
          ))),
        View::new()
          .semantic(link.semantic)
          .focus_props(link.focus)
          .interaction_props(link.interaction),
      )),
  ))
}

fn activation(target: ObjectId) -> UiEvent {
  UiEvent::new(
    target,
    true,
    false,
    UiEventBody::AccessibilityAction(UiAccessibilityActionEvent {
      backend_generation: 1,
      action: UiAccessibilityAction::Activate,
    }),
  )
}

fn accessibility_update(groups: &[Vec<CommandBody>]) -> &AccessibilityUpdate {
  groups
    .iter()
    .flatten()
    .find_map(|body| match body {
      CommandBody::AccessibilityUpdate(update) => Some(update),
      _ => None,
    })
    .expect("accessibility replacement command")
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
        battlement::GameObjectKind::UiDocument(UiDocumentState::new(document.root_id)),
      ),
    ],
    camera_id,
  )
}
