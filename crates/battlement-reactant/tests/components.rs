use trox::ls;
mod runtime_support;

use battlement::{
  AccessibilityAction, AccessibilityUpdate, CameraState, ClickEvent, CommandBody, CurrentPage,
  GameObject, ObjectId, PreparedAsset, Prop, Scene, SceneId, SemanticRole, SessionId, Snapshot,
  TabSelectionEvent, UiAccessibilityAction, UiAccessibilityActionEvent, UiDocument,
  UiDocumentState, UiElement, UiEvent, UiEventBody, UiEventDisposition, UiNode, UiValue,
  UiVisualElementProperties, ValueChangingEvent, ValueCommitEvent,
};
use battlement_reactant::{
  component::Component,
  components::{
    Button, Checkbox, ColumnHeader, Link, ListBox, ListBoxOption, Navigation, Progress, Radio,
    RadioGroup, Region, RowHeader, Slider, Tab, TabPanel, Table, TableCell, TableRow, Tabs, Text,
  },
  control_behavior,
  element_ref::use_element_ref,
  executor::{BoxFuture, SpawnedTask, Spawner},
  host::{Label, View},
  render::Render,
  semantics::{SemanticName, SemanticProps, SemanticVisibility},
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
    View::new().child((
      Label::new(ls("Account settings"))
        .element_ref(source.clone())
        .semantic(
          SemanticProps::new(SemanticRole::StaticText)
            .name(SemanticName::text(ls("Account settings")))
            .visibility(SemanticVisibility::NameSourceOnly),
        ),
      Button::content(())
        .semantic_name(SemanticName::LabelledBy(vec![source]))
        .on_press(|_game: &mut Game| {}),
    ))
  }
}

struct MultiNameFixture {
  value: &'static str,
}

struct CollectionFixture {
  selection: usize,
}

#[derive(Clone, Copy)]
enum InvalidCollectionCase {
  OrphanRow,
  ListboxCell,
  TableCell,
  CurrentRegion,
}

struct ActionFixture;

struct ContentsFixture;

struct PatternHookFixture {
  heading: bool,
}

struct NativeControlsFixture {
  disabled: bool,
}

struct ControlledHostFixture;

struct TabsFixture {
  selected_index: u32,
}

impl Component for MultiNameFixture {
  fn render(&self) -> impl Render {
    let title = use_element_ref();
    let value = use_element_ref();
    View::new().child((
      Label::new(ls("Quality"))
        .element_ref(title.clone())
        .semantic(
          SemanticProps::new(SemanticRole::StaticText)
            .name(SemanticName::text(ls(" Quality ")))
            .visibility(SemanticVisibility::NameSourceOnly),
        ),
      Button::content(
        Label::new(ls(self.value))
          .element_ref(value.clone())
          .semantic(
            SemanticProps::new(SemanticRole::StaticText)
              .name(SemanticName::text(ls(self.value)))
              .visibility(SemanticVisibility::NameSourceOnly),
          ),
      )
      .semantic_name(SemanticName::LabelledBy(vec![title, value]))
      .on_press(|game: &mut Game| game.presses += 1),
    ))
  }
}

impl Component for ActionFixture {
  fn render(&self) -> impl Render {
    Button::new(ls("Save changes")).on_press(|game: &mut Game| game.presses += 1)
  }
}

impl Component for ContentsFixture {
  fn render(&self) -> impl Render {
    View::new()
      .semantic(SemanticProps::new(SemanticRole::Group))
      .child((Button::content((
        Label::new(ls("Save")).semantic(
          SemanticProps::new(SemanticRole::StaticText)
            .name(SemanticName::Text(ls(" Save   changes "))),
        ),
        View::new()
          .semantic(SemanticProps::new(SemanticRole::Group).visibility(SemanticVisibility::Hidden))
          .child(Label::new(ls("secret")).semantic(
            SemanticProps::new(SemanticRole::StaticText).name(SemanticName::Text(ls("Secret"))),
          )),
      ))
      .semantic_name(SemanticName::Contents)
      .on_press(|_game: &mut Game| {}),))
  }
}

impl Component for PatternHookFixture {
  fn render(&self) -> impl Render {
    View::new().semantic(if self.heading {
      control_behavior::heading(ls("Status"), 2)
    } else {
      control_behavior::image(ls("Status"))
    })
  }
}

impl Component for NativeControlsFixture {
  fn render(&self) -> impl Render {
    View::new().child((
      Checkbox::new(ls("Automatic saves"), false)
        .disabled(self.disabled)
        .host_name("native-checkbox")
        .on_change(|game: &mut Game, _| game.presses += 1),
      Slider::new(ls("Volume"), 0.5, 0.0, 1.0, 0.1)
        .disabled(self.disabled)
        .host_name("native-slider")
        .on_change(|game: &mut Game, _| game.presses += 1),
      RadioGroup::new(ls("Quality")).child(
        Radio::new(ls("High"), false)
          .disabled(self.disabled)
          .on_select(|game: &mut Game| game.presses += 1),
      ),
    ))
  }
}

impl Component for ControlledHostFixture {
  fn render(&self) -> impl Render {
    View::new().child((
      Button::new(ls("Save"))
        .configure_host(|host| host.text(ls("Wrong button")))
        .on_press(|_game: &mut Game| {}),
      Checkbox::new(ls("Automatic saves"), true)
        .configure_host(|host| host.label(ls("Wrong checkbox")).value(false))
        .on_change(|_game: &mut Game, _| {}),
      Slider::new(ls("Volume"), 0.5, 0.0, 1.0, 0.1)
        .configure_host(|host| {
          host
            .label(ls("Wrong slider"))
            .low_value(-10.0)
            .high_value(10.0)
            .value(9.0)
        })
        .on_change(|_game: &mut Game, _| {}),
      RadioGroup::new(ls("Quality")).child(
        Radio::new(ls("High"), true)
          .configure_host(|host| host.label(ls("Wrong radio")).value(false))
          .on_select(|_game: &mut Game| {}),
      ),
      Progress::determinate(
        ls("Loading"),
        battlement_reactant::semantics::SemanticRange {
          current: 4.0,
          minimum: 0.0,
          maximum: 8.0,
          text: None,
        },
      )
      .configure_host(|host| {
        host
          .title(ls("Wrong progress"))
          .low_value(-1.0)
          .high_value(1.0)
          .value(1.0)
      }),
    ))
  }
}

impl Component for TabsFixture {
  fn render(&self) -> impl Render {
    Tabs::new(ls("Settings sections"), self.selected_index)
      .configure_host(|host| host.selected_tab_index(99))
      .on_select(|game: &mut Game, index| game.selection = index as usize)
      .child((
        Tab::new(ls("General"), 0)
          .configure_host(|host| host.text(ls("Wrong general")))
          .child(TabPanel::new(0, Text::new(ls("General content")))),
        Tab::new(ls("Audio"), 1).child(TabPanel::new(1, Text::new(ls("Audio content")))),
      ))
  }
}

#[test]
fn button_children_resolve_ordered_names_and_keep_activation_when_values_update() {
  let document = document();
  let mut runtime = runtime_support::reactant(IdleSpawner);
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
  assert_eq!(semantic.nodes[0].state.popup, None);
  assert_eq!(semantic.nodes[0].state.expanded, None);
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
  let mut runtime = runtime_support::reactant(IdleSpawner);
  runtime.register_root(document.clone(), |_game: &Game| ContentsFixture);
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
  let mut runtime = runtime_support::reactant(IdleSpawner);
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
  let mut runtime = runtime_support::reactant(IdleSpawner);
  runtime.register_root(document.clone(), |_game: &Game| ActionFixture);
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
    .expect("control action renders");
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
fn pure_semantic_constructors_work_without_a_render_slot() {
  assert_eq!(
    control_behavior::heading(ls("Status"), 2).role,
    SemanticRole::Heading
  );

  let document = document();
  let mut runtime = runtime_support::reactant(IdleSpawner);
  runtime.register_root(document.clone(), |game: &Game| PatternHookFixture {
    heading: game.selection == 0,
  });
  let mut game = Game::default();
  let (_, commit) = runtime
    .begin_session(&mut game)
    .unwrap()
    .into_parts(snapshot(&document));
  let _ = commit.into_groups();
  game.selection = 1;
  let _ = runtime.refresh(&mut game).unwrap().into_groups();
  let _ = runtime.shutdown(&mut game).into_groups();
}

#[test]
fn native_controls_gate_stale_input_and_reenable_atomically() {
  let document = document();
  let mut runtime = runtime_support::reactant(IdleSpawner);
  runtime.register_root(document.clone(), |game: &Game| NativeControlsFixture {
    disabled: game.selection == 1,
  });
  let mut game = Game {
    selection: 1,
    ..Game::default()
  };
  let (initial, commit) = runtime
    .begin_session(&mut game)
    .unwrap()
    .into_parts(snapshot(&document));
  let groups = commit.into_groups();
  let semantics = accessibility_update(&groups).snapshot.as_ref().unwrap();
  let checkbox = semantic_id(semantics, "Automatic saves");
  let slider = semantic_id(semantics, "Volume");
  let radio = semantic_id(semantics, "High");
  for target in [checkbox, slider, radio] {
    assert_eq!(
      find_node(&initial.ui[0].children, target)
        .element
        .visual_element()
        .enabled,
      Prop::Set(false)
    );
    assert!(
      semantics
        .nodes
        .iter()
        .find(|node| node.object_id == target)
        .unwrap()
        .state
        .disabled
    );
  }

  dispatch_value(
    &mut runtime,
    &mut game,
    checkbox,
    UiValue::Bool(false),
    UiValue::Bool(true),
  );
  dispatch_slider(&mut runtime, &mut game, slider, 0.7);
  dispatch_value(
    &mut runtime,
    &mut game,
    radio,
    UiValue::Bool(false),
    UiValue::Bool(true),
  );
  for event in [
    activation(checkbox),
    accessibility_action(slider, UiAccessibilityAction::Increment),
    activation(radio),
  ] {
    let _ = runtime.dispatch(&mut game, event).unwrap().into_groups();
  }
  assert_eq!(game.presses, 0);

  game.selection = 0;
  let groups = runtime.refresh(&mut game).unwrap().into_groups();
  let semantics = accessibility_update(&groups).snapshot.as_ref().unwrap();
  for target in [checkbox, slider, radio] {
    assert!(
      !semantics
        .nodes
        .iter()
        .find(|node| node.object_id == target)
        .unwrap()
        .state
        .disabled
    );
  }
  dispatch_value(
    &mut runtime,
    &mut game,
    checkbox,
    UiValue::Bool(false),
    UiValue::Bool(true),
  );
  assert_eq!(game.presses, 1);
  dispatch_slider(&mut runtime, &mut game, slider, 0.7);
  assert_eq!(game.presses, 2);
  dispatch_value(
    &mut runtime,
    &mut game,
    radio,
    UiValue::Bool(false),
    UiValue::Bool(true),
  );
  assert_eq!(game.presses, 3);
  let _ = runtime
    .dispatch(&mut game, activation(checkbox))
    .unwrap()
    .into_groups();
  assert_eq!(game.presses, 4);
  let _ = runtime
    .dispatch(
      &mut game,
      accessibility_action(slider, UiAccessibilityAction::Increment),
    )
    .unwrap()
    .into_groups();
  assert_eq!(game.presses, 5);
  let _ = runtime
    .dispatch(&mut game, activation(radio))
    .unwrap()
    .into_groups();
  assert_eq!(game.presses, 6);
  let _ = runtime.shutdown(&mut game).into_groups();
}

#[test]
fn component_state_wins_over_raw_host_configuration() {
  let document = document();
  let mut runtime = runtime_support::reactant(IdleSpawner);
  runtime.register_root(document.clone(), |_game: &Game| ControlledHostFixture);
  let mut game = Game::default();
  let (initial, commit) = runtime
    .begin_session(&mut game)
    .unwrap()
    .into_parts(snapshot(&document));
  let groups = commit.into_groups();
  let semantics = accessibility_update(&groups).snapshot.as_ref().unwrap();

  match &find_node(&initial.ui[0].children, semantic_id(semantics, "Save")).element {
    UiElement::Button(button) => assert_eq!(button.text, Prop::Set("Save".to_owned())),
    element => panic!("expected button, found {element:?}"),
  }
  match &find_node(
    &initial.ui[0].children,
    semantic_id(semantics, "Automatic saves"),
  )
  .element
  {
    UiElement::Toggle(toggle) => {
      assert_eq!(toggle.label, Prop::Set("Automatic saves".to_owned()));
      assert_eq!(toggle.value, Prop::Set(true));
    }
    element => panic!("expected toggle, found {element:?}"),
  }
  match &find_node(&initial.ui[0].children, semantic_id(semantics, "Volume")).element {
    UiElement::Slider(slider) => {
      assert_eq!(slider.label, Prop::Set("Volume".to_owned()));
      assert_eq!(slider.low_value, Prop::Set(0.0));
      assert_eq!(slider.high_value, Prop::Set(1.0));
      assert_eq!(slider.value, Prop::Set(0.5));
    }
    element => panic!("expected slider, found {element:?}"),
  }
  match &find_node(&initial.ui[0].children, semantic_id(semantics, "High")).element {
    UiElement::RadioButton(radio) => {
      assert_eq!(radio.label, Prop::Set("High".to_owned()));
      assert_eq!(radio.value, Prop::Set(true));
    }
    element => panic!("expected radio button, found {element:?}"),
  }
  match &find_node(&initial.ui[0].children, semantic_id(semantics, "Loading")).element {
    UiElement::ProgressBar(progress) => {
      assert_eq!(progress.title, Prop::Set("Loading".to_owned()));
      assert_eq!(progress.low_value, Prop::Set(0.0));
      assert_eq!(progress.high_value, Prop::Set(8.0));
      assert_eq!(progress.value, Prop::Set(4.0));
    }
    element => panic!("expected progress bar, found {element:?}"),
  }
  let _ = runtime.shutdown(&mut game).into_groups();
}

#[test]
fn tabs_share_one_controlled_selection_path() {
  let document = document();
  let mut runtime = runtime_support::reactant(IdleSpawner);
  runtime.register_root(document.clone(), |game: &Game| TabsFixture {
    selected_index: game.selection as u32,
  });
  let mut game = Game::default();
  let (initial, commit) = runtime
    .begin_session(&mut game)
    .unwrap()
    .into_parts(snapshot(&document));
  let groups = commit.into_groups();
  let semantics = accessibility_update(&groups).snapshot.as_ref().unwrap();
  let tab_list = semantic_id(semantics, "Settings sections");
  let general = semantic_id(semantics, "General");
  let audio = semantic_id(semantics, "Audio");
  let general_content = semantic_id(semantics, "General content");
  assert_eq!(
    semantics
      .nodes
      .iter()
      .find(|node| node.object_id == general)
      .unwrap()
      .state
      .selected,
    Some(true)
  );
  match &find_node(&initial.ui[0].children, tab_list).element {
    UiElement::TabView(tabs) => assert_eq!(tabs.selected_tab_index, Prop::Set(0)),
    element => panic!("expected tab view, found {element:?}"),
  }
  match &find_node(&initial.ui[0].children, general).element {
    UiElement::Tab(tab) => assert_eq!(tab.text, Prop::Set("General".to_owned())),
    element => panic!("expected tab, found {element:?}"),
  }

  let _ = runtime
    .dispatch(
      &mut game,
      UiEvent::click(general_content, ClickEvent::NavigationSubmit),
    )
    .unwrap()
    .into_groups();
  assert_eq!(game.selection, 0);

  let result = runtime
    .dispatch(
      &mut game,
      UiEvent::new(
        tab_list,
        true,
        false,
        UiEventBody::TabSelectionRequested(TabSelectionEvent {
          previous_index: 0,
          proposed_index: 1,
          proposed_tab_id: audio,
        }),
      ),
    )
    .unwrap();
  assert_eq!(game.selection, 1);
  let groups = result.into_groups();
  let semantics = accessibility_update(&groups).snapshot.as_ref().unwrap();
  assert_eq!(
    semantics
      .nodes
      .iter()
      .find(|node| node.object_id == audio)
      .unwrap()
      .state
      .selected,
    Some(true)
  );
  assert!(
    semantics
      .nodes
      .iter()
      .all(|node| node.label.as_deref() != Some("General content"))
  );

  let _ = runtime
    .dispatch(&mut game, activation(general))
    .unwrap()
    .into_groups();
  assert_eq!(game.selection, 0);
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
  let mut runtime = runtime_support::reactant(IdleSpawner);
  runtime.register_root(document.clone(), |game: &Game| CollectionFixture {
    selection: game.selection,
  });
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
  for case in [
    InvalidCollectionCase::OrphanRow,
    InvalidCollectionCase::ListboxCell,
    InvalidCollectionCase::TableCell,
    InvalidCollectionCase::CurrentRegion,
  ] {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
      let mut runtime = runtime_support::reactant(IdleSpawner);
      runtime.register_root(document(), move |_game: &Game| case);
      runtime.begin_session(&mut Game::default()).is_err()
    }));
    assert!(result.is_err() || result.unwrap());
  }
}

impl Component for CollectionFixture {
  fn render(&self) -> impl Render {
    View::new().child((
      Navigation::new(ls("Review pages")).child(
        Button::new(ls("Gallery shell"))
          .current_page(true)
          .on_press(|_game: &mut Game| {}),
      ),
      Region::new(ls("Settings")).child((
        ListBox::new(ls("Quality")).child(
          ["Standard", "High", "Unavailable"]
            .into_iter()
            .enumerate()
            .map(|(index, name)| {
              ListBoxOption::new(ls(name), self.selection == index)
                .disabled(index == 2)
                .on_press(move |game: &mut Game| game.selection = index)
            })
            .collect::<Vec<_>>(),
        ),
        Table::new(ls("Bindings")).child(TableRow::new().child((
          ColumnHeader::new(ls("Keyboard")),
          RowHeader::new(ls("Move")),
          TableCell::new(ls("W")),
        ))),
        Link::new(ls("Privacy policy")).on_press(|game: &mut Game| game.presses += 1),
      )),
    ))
  }
}

impl Component for InvalidCollectionCase {
  fn render(&self) -> impl Render {
    match self {
      Self::OrphanRow => View::new().child(TableRow::new()),
      Self::ListboxCell => {
        View::new().child(ListBox::new(ls("Quality")).child(TableCell::new(ls("Wrong child"))))
      }
      Self::TableCell => {
        View::new().child(Table::new(ls("Bindings")).child(TableCell::new(ls("Wrong child"))))
      }
      Self::CurrentRegion => View::new().semantic(
        SemanticProps::new(SemanticRole::Region)
          .name(SemanticName::Text(ls("Settings")))
          .state(battlement::SemanticState {
            current: Some(CurrentPage::Page),
            ..Default::default()
          }),
      ),
    }
  }
}

fn activation(target: ObjectId) -> UiEvent {
  accessibility_action(target, UiAccessibilityAction::Activate)
}

fn accessibility_action(target: ObjectId, action: UiAccessibilityAction) -> UiEvent {
  UiEvent::new(
    target,
    true,
    false,
    UiEventBody::AccessibilityAction(UiAccessibilityActionEvent {
      backend_generation: 1,
      action,
    }),
  )
}

fn dispatch_value(
  runtime: &mut battlement_reactant::runtime::Reactant<Game>,
  game: &mut Game,
  target: ObjectId,
  previous: UiValue,
  proposed: UiValue,
) {
  let _ = runtime
    .dispatch(
      game,
      UiEvent::new(
        target,
        true,
        false,
        UiEventBody::ValueCommitted(ValueCommitEvent { previous, proposed }),
      ),
    )
    .unwrap()
    .into_groups();
}

fn dispatch_slider(
  runtime: &mut battlement_reactant::runtime::Reactant<Game>,
  game: &mut Game,
  target: ObjectId,
  proposed: f32,
) {
  let _ = runtime
    .dispatch(
      game,
      UiEvent::new(
        target,
        true,
        false,
        UiEventBody::ValueChanging(ValueChangingEvent {
          proposed: UiValue::F32(proposed),
        }),
      ),
    )
    .unwrap()
    .into_groups();
}

fn semantic_id(snapshot: &battlement::AccessibilitySnapshot, name: &str) -> ObjectId {
  snapshot
    .nodes
    .iter()
    .find(|node| node.label.as_deref() == Some(name))
    .unwrap()
    .object_id
}

fn find_node(nodes: &[UiNode], target: ObjectId) -> &UiNode {
  nodes
    .iter()
    .find_map(|node| {
      (node.object_id == target)
        .then_some(node)
        .or_else(|| find_node_optional(&node.children, target))
    })
    .unwrap()
}

fn find_node_optional(nodes: &[UiNode], target: ObjectId) -> Option<&UiNode> {
  nodes.iter().find_map(|node| {
    (node.object_id == target)
      .then_some(node)
      .or_else(|| find_node_optional(&node.children, target))
  })
}

fn accessibility_update(groups: &[Vec<CommandBody>]) -> &AccessibilityUpdate {
  groups
    .iter()
    .flatten()
    .find_map(|body| match body {
      CommandBody::AccessibilityUpdate(update) => Some(update),
      _ => None,
    })
    .expect("control_behavior replacement command")
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
