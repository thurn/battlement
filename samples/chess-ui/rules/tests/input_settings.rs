use battlement::{
  AccessibilitySnapshot, CommandBody, GameObjectKind, ObjectId, Prop, SemanticRole,
  UiAccessibilityAction, UiAccessibilityActionEvent, UiElement, UiEvent, UiEventBody,
  UiVisualElementProperties, Vector,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::{app::App, asset_generator};
use battlement_rules::{engine, setting_row};

const BINDINGS: [(&str, &str, &str); 7] = [
  ("Left", "Left arrow", "D-pad left"),
  ("Right", "Right arrow", "D-pad right"),
  ("Up", "Up arrow", "D-pad up"),
  ("Down", "Down arrow", "D-pad down"),
  ("Move Piece", "Space", "A"),
  ("Pause", "Esc", "menu"),
  ("Restart", "R", "Y"),
];

#[test]
fn input_bindings_scroll_under_a_sticky_header_and_reset_to_the_top() {
  let mut client = self::client();
  let page = self::named(&mut client, "review-page-20");
  client.ui().click(page);
  client.poll();

  let snapshot = self::snapshot(&client);
  let table = snapshot
    .nodes
    .iter()
    .find(|node| {
      node.role == SemanticRole::Table && node.label.as_deref() == Some("Input bindings")
    })
    .unwrap();
  let rows = snapshot
    .nodes
    .iter()
    .filter(|node| node.role == SemanticRole::Row && node.parent_id == Some(table.object_id))
    .collect::<Vec<_>>();
  assert_eq!(rows.len(), 8);
  for heading in ["Action", "Keyboard", "Controller"] {
    assert!(snapshot.nodes.iter().any(|node| {
      node.role == SemanticRole::ColumnHeader && node.label.as_deref() == Some(heading)
    }));
  }
  for (action, keyboard, controller) in BINDINGS {
    assert!(snapshot.nodes.iter().any(|node| {
      node.role == SemanticRole::RowHeader && node.label.as_deref() == Some(action)
    }));
    for value in [keyboard, controller] {
      assert!(
        snapshot
          .nodes
          .iter()
          .any(|node| { node.role == SemanticRole::Cell && node.label.as_deref() == Some(value) })
      );
    }
  }
  let scroll = snapshot
    .nodes
    .iter()
    .find(|node| {
      node.role == SemanticRole::ScrollArea && node.label.as_deref() == Some("Input bindings")
    })
    .unwrap()
    .object_id;

  let header = self::named(&mut client, "input-bindings-header");
  assert!(matches!(
    client.ui().element(header).style().background_color,
    Prop::Set(_)
  ));
  assert!(matches!(
    client
      .ui()
      .element(header)
      .element()
      .visual_element()
      .sticky,
    Prop::Set(_)
  ));

  self::assert_offset(&mut client, scroll, 0.0);
  self::accessibility_action(&mut client, scroll, UiAccessibilityAction::ScrollForward);
  self::assert_offset(&mut client, scroll, 470.0);

  client.ui().click(page);
  client.poll();
  let reset = self::named(&mut client, "input-bindings-scroll");
  self::assert_offset(&mut client, reset, 0.0);
}

fn assert_offset(client: &mut FakeClient<App>, target: ObjectId, expected: f32) {
  let ui = client.ui();
  let UiElement::ScrollView(scroll) = ui.element(target).element() else {
    panic!("expected input bindings scroll view")
  };
  assert_eq!(scroll.scroll_offset, Prop::Set(Vector::new(0.0, expected)));
}

fn accessibility_action(
  client: &mut FakeClient<App>,
  target: ObjectId,
  action: UiAccessibilityAction,
) {
  client.ui().send_event(UiEvent::new(
    target,
    true,
    false,
    UiEventBody::AccessibilityAction(UiAccessibilityActionEvent {
      backend_generation: 1,
      action,
    }),
  ));
  client.poll();
}

fn snapshot(client: &FakeClient<App>) -> &AccessibilitySnapshot {
  client
    .commands()
    .iter()
    .rev()
    .find_map(|entry| match &entry.command.body {
      CommandBody::AccessibilityUpdate(update) => update.snapshot.as_ref(),
      _ => None,
    })
    .expect("input settings semantics")
}

fn client() -> FakeClient<App> {
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("chess-ui/content");
  assets.add_textures(asset_generator::registrations().map(|asset| asset.address));
  assets.add_ui_font(setting_row::DISPLAY_FONT);
  let mut client = FakeClient::connect(engine::create_engine(), assets);
  client.poll();
  client
}

fn named(client: &mut FakeClient<App>, name: &str) -> ObjectId {
  let mut pending = client
    .world()
    .objects()
    .filter_map(|object| match object.kind() {
      GameObjectKind::UiDocument(document) => Some(document.root_id()),
      _ => None,
    })
    .collect::<Vec<_>>();
  let ui = client.ui();
  while let Some(id) = pending.pop() {
    let element = ui.element(id);
    if element.name() == Some(name) {
      return id;
    }
    pending.extend(element.children());
  }
  panic!("missing {name}");
}
