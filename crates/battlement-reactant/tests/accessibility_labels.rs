use battlement::{
  CameraState, CheckedState, ClickEvent, CommandBody, GameObject, ObjectId, PreparedAsset, Scene,
  SceneId, SemanticRole, SessionId, Snapshot, UiAccessibilityAction, UiAccessibilityActionEvent,
  UiDocument, UiDocumentState, UiEvent, UiEventBody, UiNode, UiVisualElementProperties,
  VisualElementAction,
};
use battlement_reactant::{
  accessibility::{self, ButtonOptions, ToggleOptions},
  component::Component,
  element_ref,
  executor::{BoxFuture, SpawnedTask, Spawner},
  host::{Label, View},
  render::Render,
  runtime::Reactant,
  semantics::{AccessibleName, SemanticProps, SemanticVisibility, text},
};

#[derive(Clone, Default)]
struct Game {
  renamed: bool,
  explicit: bool,
  checked: bool,
  disabled: bool,
  prevent: bool,
  reject: bool,
  hide: bool,
  changes: usize,
  help: usize,
  bubbles: usize,
}

struct LabelFixture(Game);

impl Component for LabelFixture {
  fn render(&self) -> impl Render {
    self::fixture(&self.0)
  }
}

struct IdleSpawner;

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    SpawnedTask::detached()
  }
}

#[test]
fn label_and_nested_controls_share_activation_without_suppressing_bubbling() {
  let document = UiDocument::with_root_id(ObjectId::new_v4(), ObjectId::new_v4());
  let mut runtime = Reactant::new(IdleSpawner);
  runtime.register_root(document.clone(), self::view);
  let mut game = Game::default();
  let (initial, commit) = runtime
    .begin_session(&mut game)
    .unwrap()
    .into_parts(self::snapshot(&document));
  let groups = commit.into_groups();
  let checkbox = self::named(&initial.ui[0].children, "checkbox");
  let label = self::named(&initial.ui[0].children, "label");
  let child = self::named(&initial.ui[0].children, "decoration");
  let help = self::named(&initial.ui[0].children, "help");
  self::assert_checked(&groups, false);

  for target in [checkbox, child, label] {
    let before = game.checked;
    let changes = game.changes;
    let groups = runtime
      .dispatch(
        &mut game,
        UiEvent::click(target, ClickEvent::NavigationSubmit),
      )
      .unwrap()
      .into_commit()
      .into_groups();
    assert_eq!(game.checked, !before);
    assert_eq!(game.changes, changes + 1);
    self::assert_checked(&groups, !before);
    if target == label {
      assert!(groups.iter().flatten().any(|body| matches!(body,
        CommandBody::VisualElementPerformAction(value)
          if value.object_id == checkbox && matches!(value.action, VisualElementAction::Focus))));
    }
  }
  assert_eq!(game.bubbles, 3);
  let groups = runtime
    .dispatch(
      &mut game,
      UiEvent::new(
        checkbox,
        true,
        false,
        UiEventBody::AccessibilityAction(UiAccessibilityActionEvent {
          backend_generation: 1,
          action: UiAccessibilityAction::Activate,
        }),
      ),
    )
    .unwrap()
    .into_commit()
    .into_groups();
  self::assert_checked(&groups, false);
  assert_eq!(game.changes, 4);
  let _ = runtime
    .dispatch(
      &mut game,
      UiEvent::click(help, ClickEvent::NavigationSubmit),
    )
    .unwrap()
    .into_commit()
    .into_groups();
  assert_eq!((game.help, game.changes, game.bubbles), (1, 4, 4));

  game.reject = true;
  let _ = runtime.refresh(&mut game).unwrap().into_groups();
  let _ = runtime
    .dispatch(
      &mut game,
      UiEvent::click(label, ClickEvent::NavigationSubmit),
    )
    .unwrap()
    .into_commit()
    .into_groups();
  assert_eq!((game.checked, game.changes), (false, 5));
  game.checked = true;
  let groups = runtime.refresh(&mut game).unwrap().into_groups();
  self::assert_checked(&groups, true);
  assert_eq!(game.changes, 5);
  let wrapper = self::named(&initial.ui[0].children, "wrapper");
  game.reject = false;
  let _ = runtime.refresh(&mut game).unwrap().into_groups();
  let groups = runtime
    .dispatch(
      &mut game,
      UiEvent::click(wrapper, ClickEvent::NavigationSubmit),
    )
    .unwrap()
    .into_commit()
    .into_groups();
  assert_eq!((game.checked, game.changes), (false, 6));
  self::assert_checked(&groups, false);
  assert!(groups.iter().flatten().any(|body| matches!(body,
    CommandBody::VisualElementPerformAction(value)
      if value.object_id == checkbox && matches!(value.action, VisualElementAction::Focus))));
  game.checked = true;
  let _ = runtime.refresh(&mut game).unwrap().into_groups();
  game.reject = false;
  game.prevent = true;
  let _ = runtime.refresh(&mut game).unwrap().into_groups();
  for target in [label, checkbox] {
    let _ = runtime
      .dispatch(
        &mut game,
        UiEvent::click(target, ClickEvent::NavigationSubmit),
      )
      .unwrap()
      .into_commit()
      .into_groups();
    assert_eq!((game.checked, game.changes), (true, 6));
  }
  game.prevent = false;
  game.disabled = true;
  let _ = runtime.refresh(&mut game).unwrap().into_groups();
  for target in [label, checkbox] {
    let groups = runtime
      .dispatch(
        &mut game,
        UiEvent::click(target, ClickEvent::NavigationSubmit),
      )
      .unwrap()
      .into_commit()
      .into_groups();
    assert_eq!(game.changes, 6);
    assert!(
      !groups
        .iter()
        .flatten()
        .any(|body| matches!(body, CommandBody::VisualElementPerformAction(_)))
    );
  }
  game.disabled = false;
  game.hide = true;
  let _ = runtime.refresh(&mut game).unwrap().into_groups();
  let groups = runtime
    .dispatch(
      &mut game,
      UiEvent::click(label, ClickEvent::NavigationSubmit),
    )
    .unwrap()
    .into_commit()
    .into_groups();
  assert_eq!(game.changes, 6);
  assert!(
    !groups
      .iter()
      .flatten()
      .any(|body| matches!(body, CommandBody::VisualElementPerformAction(_)))
  );
  let _ = runtime.shutdown(&mut game).into_groups();
}

#[test]
fn referenced_names_update_and_explicit_names_remain_authoritative() {
  let document = UiDocument::with_root_id(ObjectId::new_v4(), ObjectId::new_v4());
  let mut runtime = Reactant::new(IdleSpawner);
  runtime.register_root(document.clone(), self::view);
  let mut game = Game::default();
  let groups = runtime
    .begin_session(&mut game)
    .unwrap()
    .into_parts(self::snapshot(&document))
    .1
    .into_groups();
  self::assert_name(&groups, "Enable sound");
  game.renamed = true;
  self::assert_name(
    &runtime.refresh(&mut game).unwrap().into_groups(),
    "Play audio",
  );
  game.explicit = true;
  self::assert_name(
    &runtime.refresh(&mut game).unwrap().into_groups(),
    "Explicit sound",
  );
  game.renamed = false;
  game.checked = true;
  self::assert_name(
    &runtime.refresh(&mut game).unwrap().into_groups(),
    "Explicit sound",
  );
  let _ = runtime.shutdown(&mut game).into_groups();
}

fn assert_name(groups: &[Vec<CommandBody>], name: &str) {
  let snapshot = groups
    .iter()
    .flatten()
    .find_map(|body| match body {
      CommandBody::AccessibilityUpdate(update) => update.snapshot.as_ref(),
      _ => None,
    })
    .unwrap();
  let checkboxes = snapshot
    .nodes
    .iter()
    .filter(|node| node.role == SemanticRole::Checkbox)
    .collect::<Vec<_>>();
  assert_eq!(checkboxes.len(), 1);
  assert_eq!(checkboxes[0].label.as_deref(), Some(name));
  assert_eq!(snapshot.nodes.len(), 2, "only checkbox and help are spoken");
}

fn view(game: &Game) -> impl Render + use<> {
  LabelFixture(game.clone())
}

fn fixture(game: &Game) -> impl Render + use<> {
  let visible_name = if game.renamed {
    "Play audio"
  } else {
    "Enable sound"
  };
  let label = element_ref::use_element_ref();
  let input = element_ref::use_element_ref();
  let checkbox = accessibility::use_checkbox(ToggleOptions {
    name: if game.explicit {
      AccessibleName::text("Explicit sound")
    } else {
      AccessibleName::LabelledBy(label.clone())
    },
    checked: game.checked,
    is_disabled: game.disabled,
    on_change: |game: &mut Game, value| {
      game.changes += 1;
      if !game.reject {
        game.checked = value;
      }
    },
  });
  let help = accessibility::use_button(ButtonOptions {
    name: text("Help"),
    is_disabled: false,
    on_press: |game: &mut Game| game.help += 1,
  });
  let associated = checkbox.label_interaction(&input);
  View::new()
    .on_click_capture_event(|game: &mut Game, event| {
      if game.prevent {
        event.prevent_default();
      }
    })
    .on_click(|game: &mut Game| game.bubbles += 1)
    .child(
      View::new()
        .name("wrapper")
        .interaction_props(associated)
        .child((
          Label::new(visible_name)
            .name("label")
            .element_ref(label)
            .semantic(
              SemanticProps::new(SemanticRole::StaticText)
                .name(AccessibleName::text(visible_name))
                .visibility(SemanticVisibility::NameSourceOnly),
            ),
          (!game.hide).then(|| {
            View::new()
              .name("checkbox")
              .element_ref(input)
              .semantic(checkbox.semantic)
              .focus_props(checkbox.focus)
              .interaction_props(checkbox.interaction)
              .child(View::new().name("decoration"))
          }),
          View::new()
            .name("help")
            .semantic(help.semantic)
            .focus_props(help.focus)
            .interaction_props(help.interaction),
        )),
    )
}

fn assert_checked(groups: &[Vec<CommandBody>], checked: bool) {
  let node = groups
    .iter()
    .flatten()
    .find_map(|body| match body {
      CommandBody::AccessibilityUpdate(update) => update.snapshot.as_ref().and_then(|snapshot| {
        snapshot
          .nodes
          .iter()
          .find(|node| node.role == SemanticRole::Checkbox)
      }),
      _ => None,
    })
    .expect("checkbox update");
  assert_eq!(node.label.as_deref(), Some("Enable sound"));
  assert_eq!(
    node.state.checked,
    Some(if checked {
      CheckedState::True
    } else {
      CheckedState::False
    })
  );
}

fn named(nodes: &[UiNode], name: &str) -> ObjectId {
  self::find(nodes, name).unwrap()
}

fn find(nodes: &[UiNode], name: &str) -> Option<ObjectId> {
  nodes.iter().find_map(|node| {
    if node.element.visual_element().name == battlement::Prop::Set(name.to_owned()) {
      Some(node.object_id)
    } else {
      self::find(&node.children, name)
    }
  })
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
