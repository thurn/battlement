use battlement::{
  AccessibilitySnapshot, CheckedState, ClickEvent, CommandBody, CurrentPage, GameObjectKind,
  KeyModifiers, ObjectId, PanelPoint, PointerButton, PopupKind, SemanticRole,
  UiAccessibilityAction, UiAccessibilityActionEvent, UiEvent, UiEventBody,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::{app::App, asset_generator};
use battlement_rules::{action_button, engine, select_control, setting_row};

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
  let initial = self::snapshot(&client)
    .nodes
    .iter()
    .find(|node| node.object_id == trigger)
    .unwrap();
  assert_eq!(initial.role, SemanticRole::Button);
  assert_eq!(initial.state.popup, Some(PopupKind::ListBox));
  assert_eq!(initial.state.expanded, Some(false));
  client.ui().click(trigger);
  client.poll();
  client.ui().send_event(UiEvent::new(
    trigger,
    true,
    false,
    UiEventBody::AccessibilityAction(UiAccessibilityActionEvent {
      backend_generation: 1,
      action: UiAccessibilityAction::Activate,
    }),
  ));
  client.poll();
  client.ui().click(update);
  client.poll();
  let snapshot = self::snapshot(&client);
  let selected = snapshot
    .nodes
    .iter()
    .find(|node| node.object_id == trigger)
    .unwrap();
  assert_eq!(selected.role, SemanticRole::Button);
  assert_eq!(selected.state.popup, Some(PopupKind::ListBox));
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
  let reset = snapshot
    .nodes
    .iter()
    .find(|node| node.label.as_deref() == Some("Resolution 1920 × 1080"))
    .unwrap();
  assert_eq!(reset.role, SemanticRole::Button);
  assert_eq!(reset.state.popup, Some(PopupKind::ListBox));
  assert_eq!(reset.state.expanded, Some(false));
  assert!(
    snapshot
      .nodes
      .iter()
      .any(|node| node.label.as_deref() == Some("Selection changes: 0"))
  );
}

#[test]
fn volume_uses_parent_values_and_resets_without_retaining_proposals() {
  let mut client = self::client();
  let page = self::named(&mut client, "review-page-7");
  client.ui().click(page);
  client.poll();
  let slider = self::assert_volume(&client, "Master Volume", 80);
  self::assert_volume(&client, "Minimum", 0);
  let maximum = self::assert_volume(&client, "Maximum", 100);
  self::range_action(&mut client, slider, UiAccessibilityAction::Increment);
  self::assert_volume(&client, "Master Volume", 85);
  self::range_action(&mut client, maximum, UiAccessibilityAction::Decrement);
  self::assert_volume(&client, "Maximum", 100);
  let update = self::snapshot(&client)
    .nodes
    .iter()
    .find(|node| node.label.as_deref() == Some("Change volume from parent"))
    .unwrap()
    .object_id;
  client.ui().click(update);
  client.poll();
  self::assert_volume(&client, "Master Volume", 25);
  assert!(
    self::snapshot(&client)
      .nodes
      .iter()
      .any(|node| node.label.as_deref() == Some("Volume changes: 1"))
  );
  self::range_action(&mut client, slider, UiAccessibilityAction::Decrement);
  self::assert_volume(&client, "Master Volume", 20);
  client.ui().click(page);
  client.poll();
  self::assert_volume(&client, "Master Volume", 80);
  self::assert_volume(&client, "Minimum", 0);
  self::assert_volume(&client, "Maximum", 100);
  assert!(!client.ui().contains(slider));
  assert!(
    self::snapshot(&client)
      .nodes
      .iter()
      .any(|node| node.label.as_deref() == Some("Volume changes: 0"))
  );
  self::assert_page(&mut client, 6);
}

#[test]
fn action_children_activate_once_and_reselection_resets_callbacks() {
  let mut client = self::client();
  let page = self::named(&mut client, "review-page-8");
  client.ui().click(page);
  client.poll();
  let snapshot = self::snapshot(&client);
  let region = snapshot
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::Region)
    .unwrap();
  let mut targets = Vec::new();
  for name in ["PLAY", "COMPOSED LABEL", "ABOUT", "DISABLED", "RETURN"] {
    let node = snapshot
      .nodes
      .iter()
      .find(|node| node.label.as_deref() == Some(name))
      .unwrap();
    assert_eq!(node.role, SemanticRole::Button);
    assert_eq!(node.parent_id, Some(region.object_id));
    assert_eq!(node.state.disabled, name == "DISABLED");
    targets.push(node.object_id);
  }
  for target in &targets {
    client.ui().click(*target);
    client.poll();
  }
  self::assert_actions(&client, 2, 1);
  for target in &targets {
    client
      .ui()
      .send_event(UiEvent::click(*target, ClickEvent::NavigationSubmit));
    client.poll();
  }
  self::assert_actions(&client, 4, 2);
  for target in &targets {
    self::range_action(&mut client, *target, UiAccessibilityAction::Activate);
  }
  self::assert_actions(&client, 6, 3);
  client.ui().click(page);
  client.poll();
  self::assert_actions(&client, 0, 0);
  for target in targets {
    assert!(!client.ui().contains(target));
  }
  self::assert_page(&mut client, 7);
}

fn assert_actions(client: &FakeClient<App>, clicks: u32, returns: u32) {
  for name in [
    format!("Action clicks: {clicks}"),
    format!("Return clicks: {returns}"),
  ] {
    assert!(
      self::snapshot(client)
        .nodes
        .iter()
        .any(|node| node.label.as_deref() == Some(&name))
    );
  }
}

fn assert_volume(client: &FakeClient<App>, label: &str, expected: u32) -> ObjectId {
  let snapshot = self::snapshot(client);
  let slider = snapshot
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::Slider && node.label.as_deref() == Some(label))
    .unwrap();
  let range = slider.value.as_ref().unwrap();
  assert_eq!(
    (range.minimum, range.maximum, range.current),
    (0.0, 100.0, f64::from(expected))
  );
  assert_eq!(
    range.text.as_deref(),
    Some(format!("{expected} percent").as_str())
  );
  let region = snapshot
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::Region)
    .unwrap();
  assert_eq!(slider.parent_id, Some(region.object_id));
  slider.object_id
}

fn range_action(client: &mut FakeClient<App>, target: ObjectId, action: UiAccessibilityAction) {
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

fn assert_checkbox(client: &FakeClient<App>, checked: bool, changes: u32) {
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

fn assert_page(client: &mut FakeClient<App>, index: usize) {
  let heading = self::named(client, "page-heading");
  assert_eq!(client.ui().focused(), Some(heading));
  let title = client.ui().element(heading).text().unwrap().to_owned();
  let semantics = self::snapshot(client);
  let current = semantics
    .nodes
    .iter()
    .filter(|node| node.state.current == Some(CurrentPage::Page))
    .collect::<Vec<_>>();
  assert_eq!(current.len(), 1);
  assert_eq!(
    current[0].label.as_deref(),
    Some(format!("{}. {title}", index + 1).as_str())
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
  assert_eq!(region.label.as_deref(), Some(title.as_str()));
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
    .expect("gallery semantics")
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

fn client() -> FakeClient<App> {
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("chess-ui/content");
  assets.add_textures(asset_generator::registrations().map(|asset| asset.address));
  assets.add_ui_font(setting_row::DISPLAY_FONT);
  assets.add_ui_font(select_control::VALUE_FONT);
  assets.add_ui_font(action_button::ACTION_FONT);
  let mut client = FakeClient::connect(engine::create_engine(), assets);
  client.poll();
  client
}
