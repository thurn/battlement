use battlement::{
  AccessibilitySnapshot, CheckedState, ClickEvent, CommandBody, CurrentPage, GameObjectKind,
  KeyModifiers, ObjectId, PanelPoint, PointerButton, SemanticRole, UiAccessibilityAction,
  UiAccessibilityActionEvent, UiEvent, UiEventBody,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::asset_generator;
use battlement_rules::{
  engine::{self, ChessUiEngine},
  pages, select_control, setting_row,
};

#[test]
fn gallery_selection_recreates_each_harness_and_restores_heading_focus() {
  let mut client = self::client();
  self::assert_page(&mut client, 0);
  let change = self::named(&mut client, "demonstration-count");
  assert_eq!(client.ui().element(change).text(), Some("Changes: 0"));
  let button = self::snapshot(&client)
    .nodes
    .iter()
    .find(|node| node.label.as_deref() == Some("Change demonstration"))
    .unwrap()
    .object_id;
  client.ui().click(button);
  client.poll();
  assert_eq!(client.ui().element(change).text(), Some("Changes: 1"));
  for index in 0..40 {
    for _ in 0..2 {
      let old_heading = self::named(&mut client, "page-heading");
      let target = self::named(&mut client, &format!("review-page-{}", index + 1));
      client.ui().click(target);
      client.poll();
      self::assert_page(&mut client, index);
      assert!(
        !client.ui().contains(old_heading),
        "selection must recreate the harness"
      );
      assert_eq!(client.ui().pointer_capture(0), None);
    }
  }
  client.reconnect();
  client.poll();
  self::assert_page(&mut client, 0);
  let count = self::named(&mut client, "demonstration-count");
  assert_eq!(client.ui().element(count).text(), Some("Changes: 0"));
}

#[test]
fn checkbox_accepts_one_proposal_and_parent_updates_reset_authoritatively() {
  let mut client = self::client();
  let page = self::named(&mut client, "review-page-5");
  client.ui().click(page);
  client.poll();
  let checkbox = self::snapshot(&client)
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::Checkbox && node.label.as_deref() == Some("VSync"))
    .unwrap()
    .object_id;
  self::assert_checkbox(&client, false, 0);
  client.ui().click(checkbox);
  client.poll();
  self::assert_checkbox(&client, true, 1);
  client.ui().send_event(UiEvent::new(
    checkbox,
    true,
    false,
    UiEventBody::AccessibilityAction(UiAccessibilityActionEvent {
      backend_generation: 1,
      action: UiAccessibilityAction::Activate,
    }),
  ));
  client.poll();
  self::assert_checkbox(&client, false, 2);
  let external = self::snapshot(&client)
    .nodes
    .iter()
    .find(|node| node.label.as_deref() == Some("Change VSync from parent"))
    .unwrap()
    .object_id;
  client.ui().click(external);
  client.poll();
  self::assert_checkbox(&client, true, 2);
  let mut label = checkbox;
  while client.ui().element(label).name() != Some("toggle-control-label") {
    label = client.ui().element(label).parent_id().unwrap();
  }
  client.ui().send_event(UiEvent::click(
    label,
    ClickEvent::pointer(
      0,
      PanelPoint::default(),
      PointerButton::Left,
      1,
      KeyModifiers::default(),
    ),
  ));
  client.poll();
  assert_eq!(client.ui().focused(), Some(checkbox));
  self::assert_checkbox(&client, false, 3);
  client.ui().click(page);
  client.poll();
  self::assert_checkbox(&client, false, 0);
  assert!(!client.ui().contains(checkbox));
}

#[test]
fn closed_selection_uses_parent_value_and_resets_without_proposals() {
  let mut client = self::client();
  let page = self::named(&mut client, "review-page-6");
  client.ui().click(page);
  client.poll();
  let trigger = self::snapshot(&client)
    .nodes
    .iter()
    .find(|node| node.label.as_deref() == Some("Resolution 1920 × 1080"))
    .unwrap()
    .object_id;
  let update = self::snapshot(&client)
    .nodes
    .iter()
    .find(|node| node.label.as_deref() == Some("Change resolution from parent"))
    .unwrap()
    .object_id;
  client.ui().click(update);
  client.poll();
  let snapshot = self::snapshot(&client);
  let selected = snapshot
    .nodes
    .iter()
    .find(|node| node.object_id == trigger)
    .unwrap();
  assert_eq!(selected.role, SemanticRole::Disclosure);
  assert_eq!(selected.state.expanded, Some(false));
  assert_eq!(selected.label.as_deref(), Some("Resolution 2560 × 1440"));
  assert!(
    snapshot
      .nodes
      .iter()
      .any(|node| node.label.as_deref() == Some("Selection changes: 0"))
  );
  assert!(
    !snapshot
      .nodes
      .iter()
      .any(|node| node.role == SemanticRole::ListBox)
  );
  client.ui().click(page);
  client.poll();
  assert!(!client.ui().contains(trigger));
  let snapshot = self::snapshot(&client);
  assert!(
    snapshot
      .nodes
      .iter()
      .any(|node| node.label.as_deref() == Some("Resolution 1920 × 1080"))
  );
  assert!(
    snapshot
      .nodes
      .iter()
      .any(|node| node.label.as_deref() == Some("Selection changes: 0"))
  );
}

fn assert_checkbox(client: &FakeClient<ChessUiEngine>, checked: bool, changes: u32) {
  let snapshot = self::snapshot(client);
  let checkbox = snapshot
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::Checkbox && node.label.as_deref() == Some("VSync"))
    .unwrap();
  assert_eq!(checkbox.role, SemanticRole::Checkbox);
  assert_eq!(
    checkbox.state.checked,
    Some(if checked {
      CheckedState::True
    } else {
      CheckedState::False
    })
  );
  assert!(
    snapshot
      .nodes
      .iter()
      .any(|node| node.label.as_deref() == Some(&format!("VSync changes: {changes}")))
  );
  let second = snapshot
    .nodes
    .iter()
    .find(|node| node.label.as_deref() == Some("Screen shake"))
    .unwrap();
  assert_eq!(second.state.checked, Some(CheckedState::True));
}

fn assert_page(client: &mut FakeClient<ChessUiEngine>, index: usize) {
  let heading = self::named(client, "page-heading");
  assert_eq!(client.ui().focused(), Some(heading));
  let semantics = self::snapshot(client);
  let current = semantics
    .nodes
    .iter()
    .filter(|node| node.state.current == Some(CurrentPage::Page))
    .collect::<Vec<_>>();
  assert_eq!(current.len(), 1);
  assert_eq!(
    current[0].label.as_deref(),
    Some(pages::ALL[index].semantic_target)
  );
  assert_eq!(
    semantics
      .nodes
      .iter()
      .filter(|node| node.role == SemanticRole::Button
        && node
          .label
          .as_deref()
          .is_some_and(|label| label.split_once(". ").is_some()))
      .count(),
    40
  );
  let navigation = semantics
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::Navigation)
    .unwrap();
  assert_eq!(navigation.label.as_deref(), Some("Chess UI review pages"));
  assert_eq!(current[0].parent_id, Some(navigation.object_id));
  let region = semantics
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::Region)
    .unwrap();
  assert_eq!(region.label.as_deref(), Some(pages::ALL[index].title));
}

fn snapshot(client: &FakeClient<ChessUiEngine>) -> &AccessibilitySnapshot {
  client
    .commands()
    .iter()
    .rev()
    .find_map(|entry| match &entry.command.body {
      CommandBody::AccessibilityUpdate(update) => update.snapshot.as_ref(),
      _ => None,
    })
    .expect("gallery semantics")
}

fn named(client: &mut FakeClient<ChessUiEngine>, name: &str) -> ObjectId {
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

fn client() -> FakeClient<ChessUiEngine> {
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("chess-ui/content");
  assets.add_textures(asset_generator::registrations().map(|asset| asset.address));
  assets.add_ui_font(setting_row::DISPLAY_FONT);
  assets.add_ui_font(select_control::VALUE_FONT);
  let mut client = FakeClient::connect(engine::create_engine().unwrap(), assets);
  client.poll();
  client
}
