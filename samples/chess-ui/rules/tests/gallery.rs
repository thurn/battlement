use battlement::{
  AccessibilitySnapshot, CheckedState, ClickEvent, Color, CommandBody, CurrentPage, GameObjectKind,
  Gradient, KeyModifiers, MotionEventBatch, MotionGestureEvent, MotionGestureEventKind,
  MotionGestureVector, MotionLayer, MotionPointerDevice, MotionProperty, MotionSequence,
  MotionValue, ObjectId, PanelPoint, PointerBoundaryEvent, PointerButton, PointerButtonEvent,
  PointerType, PopupKind, Prop, SemanticRole, UiAccessibilityAction, UiAccessibilityActionEvent,
  UiEvent, UiEventBody, UiVisualElementProperties, Vector,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::{
  app::App, asset_generator, component::Component, control_behavior, hooks, host::View,
  render::Render,
};
use battlement_rules::{
  action_button, engine, review_surface::ReviewSurface, select_control, setting_row,
  toggle_control::ToggleControl,
};
use trox::ls;

struct ToggleInfoFixture;

#[test]
fn interaction_feedback_tracks_hover_press_release_and_drag_out_cancellation() {
  let mut client = self::client();
  let page = self::named(&mut client, "review-page-11");
  client.ui().click(page);
  client.poll();
  let semantic_checkbox = self::snapshot(&client)
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::Checkbox && node.label.as_deref() == Some("VSync"))
    .unwrap()
    .object_id;
  let checkbox = self::named(&mut client, "toggle-control-input");
  assert_eq!(checkbox, semantic_checkbox);
  let surface = self::named(&mut client, "toggle-control-surface");
  self::assert_scale(&mut client, surface, 1.0);

  self::pointer_boundary(&mut client, checkbox, true);
  client.poll();
  self::assert_scale(&mut client, surface, 1.045);
  self::pointer_button(&mut client, checkbox, true);
  client.poll();
  self::assert_scale(&mut client, surface, 0.88);

  self::pointer_boundary(&mut client, checkbox, false);
  client.poll();
  self::assert_scale(&mut client, surface, 1.0);
  assert_eq!(
    self::snapshot(&client)
      .nodes
      .iter()
      .find(|node| node.object_id == checkbox)
      .unwrap()
      .state
      .checked,
    Some(CheckedState::False)
  );

  self::pointer_boundary(&mut client, checkbox, true);
  self::pointer_button(&mut client, checkbox, true);
  client.poll();
  self::pointer_button(&mut client, checkbox, false);
  client.poll();
  self::assert_scale(&mut client, surface, 1.045);
  client.ui().toggle_click(checkbox);
  client.poll();
  assert_eq!(
    self::snapshot(&client)
      .nodes
      .iter()
      .find(|node| node.object_id == checkbox)
      .unwrap()
      .state
      .checked,
    Some(CheckedState::True)
  );

  client.ui().click(page);
  client.poll();
  let reset = self::named(&mut client, "toggle-control-surface");
  self::assert_scale(&mut client, reset, 1.0);
}

#[test]
fn focus_visible_follows_navigation_modality_across_every_interaction_specimen() {
  let mut client = self::client();
  let page = self::named(&mut client, "review-page-12");
  client.ui().click(page);
  client.poll();

  let checkbox = self::named(&mut client, "toggle-control-input");
  let checkbox_surface = self::named(&mut client, "toggle-control-surface");
  self::focus_visible(&mut client, checkbox, true, 0);
  self::assert_gradient_start(&mut client, checkbox_surface, Color::hex(0xfffbd0));
  self::focus_visible(&mut client, checkbox, false, 1);
  self::pointer_boundary(&mut client, checkbox, true);
  client.poll();
  self::assert_gradient_start(&mut client, checkbox_surface, Color::hex(0x91faff));

  self::select_interaction_specimen(&mut client, "SELECT");
  let select = self::named(&mut client, "select-trigger");
  self::focus_visible(&mut client, select, true, 2);
  self::assert_gradient_start(&mut client, select, Color::hex(0xfffbd0));

  self::select_interaction_specimen(&mut client, "SLIDER");
  let slider = self::named(&mut client, "volume-input");
  self::focus_visible(&mut client, slider, true, 3);
  let track = self::named(&mut client, "volume-track");
  let thumb = self::named(&mut client, "volume-thumb");
  self::assert_gradient_start(&mut client, track, Color::hex(0xfffbd0));
  self::assert_gradient_start(&mut client, thumb, Color::hex(0xfffbd0));

  self::select_interaction_specimen(&mut client, "ACTIONS");
  let action = self::snapshot(&client)
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::Button && node.label.as_deref() == Some("PLAY"))
    .unwrap()
    .object_id;
  self::focus_visible(&mut client, action, true, 4);
  self::assert_gradient_start(&mut client, action, Color::hex(0xfffbd0));

  self::select_interaction_specimen(&mut client, "TABS");
  let tab = self::snapshot(&client)
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::Tab && node.label.as_deref() == Some("Graphics"))
    .unwrap()
    .object_id;
  self::focus_visible_with_device(&mut client, tab, true, 5, MotionPointerDevice::Gamepad);
  self::assert_gradient_start(&mut client, tab, Color::hex(0xfffbd0));

  client.ui().click(page);
  client.poll();
  assert_eq!(
    client.ui().focused(),
    Some(self::named(&mut client, "page-heading"))
  );
  let reset = self::named(&mut client, "toggle-control-surface");
  self::assert_gradient_start(&mut client, reset, Color::hex(0x4ba3ff));
}

#[test]
fn toggle_accessibility_preserves_name_description_activation_and_reset() {
  let mut client = self::client();
  let page = self::named(&mut client, "review-page-13");
  client.ui().click(page);
  client.poll();
  let (checkbox, checked, hint) = {
    let node = self::snapshot(&client)
      .nodes
      .iter()
      .find(|node| {
        node.role == SemanticRole::Checkbox && node.label.as_deref() == Some("Upload Crash Reports")
      })
      .unwrap();
    (node.object_id, node.state.checked, node.hint.clone())
  };
  assert_eq!(checked, Some(CheckedState::True));
  assert_eq!(
    hint.as_deref(),
    Some("We upload crash reports to Unity Diagnostics.")
  );
  assert!(
    self::snapshot(&client)
      .nodes
      .iter()
      .all(|node| node.label.as_deref() != Some("About crash report uploads"))
  );
  let surface = self::named(&mut client, "toggle-control-surface");
  self::assert_gradient_start(&mut client, surface, Color::hex(0x4ba3ff));

  client.ui().toggle_click(checkbox);
  client.poll();
  self::assert_checked(&client, checkbox, false);
  for expected in [true, false] {
    client
      .ui()
      .send_event(UiEvent::click(checkbox, ClickEvent::NavigationSubmit));
    client.poll();
    self::assert_checked(&client, checkbox, expected);
  }
  client.ui().send_event(UiEvent::new(
    checkbox,
    true,
    false,
    UiEventBody::AccessibilityAction(UiAccessibilityActionEvent {
      backend_generation: 3,
      action: UiAccessibilityAction::Activate,
    }),
  ));
  client.poll();
  self::assert_checked(&client, checkbox, true);

  client.ui().click(page);
  client.poll();
  let reset = self::snapshot(&client)
    .nodes
    .iter()
    .find(|node| node.label.as_deref() == Some("Upload Crash Reports"))
    .unwrap();
  assert_eq!(reset.state.checked, Some(CheckedState::True));
  assert!(!client.ui().contains(checkbox));
  assert_eq!(
    client.ui().focused(),
    Some(self::named(&mut client, "page-heading"))
  );
}

impl Component for ToggleInfoFixture {
  fn render(&self) -> impl Render {
    let (info_clicks, set_info_clicks) = hooks::use_state(0_u32);
    View::new().child((
      ToggleControl::new()
        .label(control_behavior::name_source_text(ls("Screenshake")))
        .checked(true)
        .on_change(|_| {})
        .aria_label(ls("Screen shake"))
        .with_info(true)
        .on_info_click(move || set_info_clicks.update(|count| count + 1))
        .row_height(190.0)
        .offset_y(-8.0),
      control_behavior::static_label(ls(format!("Info clicks: {info_clicks}"))),
    ))
  }
}

#[test]
fn header_variants_expose_one_native_heading_and_reset() {
  let mut client = self::client();
  let page = self::named(&mut client, "review-page-10");
  client.ui().click(page);
  client.poll();
  for (action, expected) in [
    (None, "Chess Chess Revolution"),
    (Some("Show settings heading"), "Settings"),
    (Some("Show game heading"), "Chess Chess Revolution"),
    (Some("Show settings heading"), "Settings"),
  ] {
    if let Some(action) = action {
      let target = self::snapshot(&client)
        .nodes
        .iter()
        .find(|node| node.label.as_deref() == Some(action))
        .unwrap()
        .object_id;
      client.ui().click(target);
      client.poll();
    }
    let snapshot = self::snapshot(&client);
    let titles = snapshot
      .nodes
      .iter()
      .filter(|node| {
        matches!(
          node.label.as_deref(),
          Some("Settings" | "Chess Chess Revolution")
        )
      })
      .collect::<Vec<_>>();
    assert_eq!(titles.len(), 1);
    assert_eq!(titles[0].role, SemanticRole::Heading);
    assert_eq!(titles[0].label.as_deref(), Some(expected));
  }
  let page = self::named(&mut client, "review-page-10");
  client.ui().click(page);
  client.poll();
  self::assert_page(&mut client, 9);
  assert!(
    self::snapshot(&client)
      .nodes
      .iter()
      .any(|node| { node.label.as_deref() == Some("Chess Chess Revolution") })
  );
}

#[test]
fn gallery_selection_recreates_each_harness_and_restores_heading_focus() {
  let mut client = self::client();
  self::assert_page(&mut client, 0);
  let change = self::named(&mut client, "demonstration-count");
  assert_eq!(
    client.ui().element(change).text(),
    Some("Changes: \u{2068}0\u{2069}")
  );
  let button = self::snapshot(&client)
    .nodes
    .iter()
    .find(|node| node.label.as_deref() == Some("Change demonstration"))
    .unwrap()
    .object_id;
  client.ui().click(button);
  client.poll();
  assert_eq!(
    client.ui().element(change).text(),
    Some("Changes: \u{2068}1\u{2069}")
  );
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
  assert_eq!(
    client.ui().element(count).text(),
    Some("Changes: \u{2068}0\u{2069}")
  );
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
  client.ui().toggle_click(checkbox);
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
  let label = self::control_label(&mut client, checkbox);
  self::click_label(&mut client, label);
  assert_eq!(client.ui().focused(), Some(checkbox));
  self::assert_checkbox(&client, false, 3);
  let screenshake = self::snapshot(&client)
    .nodes
    .iter()
    .find(|node| node.label.as_deref() == Some("Screen shake"))
    .unwrap()
    .object_id;
  client.ui().toggle_click(screenshake);
  client.poll();
  assert_eq!(
    self::snapshot(&client)
      .nodes
      .iter()
      .find(|node| node.object_id == screenshake)
      .unwrap()
      .state
      .checked,
    Some(CheckedState::False)
  );
  client.ui().click(page);
  client.poll();
  self::assert_checkbox(&client, false, 0);
  assert!(!client.ui().contains(checkbox));
}

#[test]
fn optional_info_callback_is_forwarded_without_toggling_the_checkbox() {
  let mut client = self::toggle_info_client();
  let snapshot = self::snapshot(&client);
  let checkbox = snapshot
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::Checkbox)
    .unwrap();
  assert_eq!(checkbox.label.as_deref(), Some("Screen shake"));
  assert_eq!(
    checkbox.hint.as_deref(),
    Some("We upload crash reports to Unity Diagnostics.")
  );
  let checkbox = checkbox.object_id;
  let info = self::named(&mut client, "toggle-info");
  client.ui().click(info);
  client.poll();
  assert!(
    self::snapshot(&client)
      .nodes
      .iter()
      .any(|node| node.label.as_deref() == Some("Info clicks: 1"))
  );
  assert_eq!(
    self::snapshot(&client)
      .nodes
      .iter()
      .find(|node| node.object_id == checkbox)
      .unwrap()
      .state
      .checked,
    Some(CheckedState::True)
  );
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
  let label = self::control_label(&mut client, trigger);
  self::click_label(&mut client, label);
  assert_eq!(client.ui().focused(), Some(trigger));
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
fn select_popover_opens_selects_dismisses_and_resets() {
  let mut client = self::client();
  let page = self::named(&mut client, "review-page-14");
  client.ui().click(page);
  client.poll();
  let trigger = self::snapshot(&client)
    .nodes
    .iter()
    .find(|node| node.label.as_deref() == Some("Display Mode Borderless"))
    .unwrap()
    .object_id;
  assert_eq!(
    self::snapshot(&client)
      .nodes
      .iter()
      .find(|node| node.object_id == trigger)
      .unwrap()
      .state
      .expanded,
    Some(false)
  );

  client.ui().click(trigger);
  client.poll();
  let snapshot = self::snapshot(&client);
  let listbox = snapshot
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::ListBox)
    .unwrap();
  assert_eq!(listbox.label.as_deref(), Some("Display Mode options"));
  let options = snapshot
    .nodes
    .iter()
    .filter(|node| node.role == SemanticRole::Option)
    .collect::<Vec<_>>();
  assert_eq!(options.len(), 3);
  assert_eq!(options[0].label.as_deref(), Some("Borderless"));
  assert_eq!(options[0].state.selected, Some(true));
  assert_eq!(options[1].label.as_deref(), Some("Fullscreen"));
  assert_eq!(options[2].label.as_deref(), Some("Windowed"));
  assert_eq!(
    snapshot
      .nodes
      .iter()
      .find(|node| node.object_id == trigger)
      .unwrap()
      .state
      .expanded,
    Some(true)
  );

  let windowed = options[2].object_id;
  self::pointer_boundary(&mut client, windowed, true);
  client.poll();
  client.ui().click(windowed);
  client.poll();
  let snapshot = self::snapshot(&client);
  assert!(
    snapshot
      .nodes
      .iter()
      .all(|node| node.role != SemanticRole::ListBox)
  );
  assert_eq!(
    snapshot
      .nodes
      .iter()
      .find(|node| node.object_id == trigger)
      .unwrap()
      .label
      .as_deref(),
    Some("Display Mode Windowed")
  );

  client.ui().click(trigger);
  client.poll();
  let dismiss = self::named(&mut client, "select-dismiss-layer");
  self::click_label(&mut client, dismiss);
  assert!(
    self::snapshot(&client)
      .nodes
      .iter()
      .all(|node| node.role != SemanticRole::ListBox)
  );

  client.ui().click(page);
  client.poll();
  let reset = self::snapshot(&client)
    .nodes
    .iter()
    .find(|node| node.label.as_deref() == Some("Display Mode Borderless"))
    .unwrap();
  assert_eq!(reset.state.expanded, Some(false));
  assert!(!client.ui().contains(trigger));
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
  let label = self::control_label(&mut client, slider);
  self::click_label(&mut client, label);
  assert_eq!(client.ui().focused(), Some(slider));
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
      .any(|node| node.label.as_deref() == Some("Volume changes: \u{2068}1\u{2069}"))
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
      .any(|node| node.label.as_deref() == Some("Volume changes: \u{2068}0\u{2069}"))
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
  for (index, target) in targets.iter().enumerate() {
    if index != 3 {
      client.ui().click(*target);
    }
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

#[test]
fn tabs_select_controlled_values_and_reset() {
  let mut client = self::client();
  let page = self::named(&mut client, "review-page-9");
  client.ui().click(page);
  client.poll();
  self::assert_tabs(&client, "Gameplay", 0);
  for (index, name) in ["Graphics", "Sound", "Input", "Gameplay", "Gameplay"]
    .iter()
    .enumerate()
  {
    let tab = self::snapshot(&client)
      .nodes
      .iter()
      .find(|node| node.role == SemanticRole::Tab && node.label.as_deref() == Some(name))
      .unwrap()
      .object_id;
    client.ui().click(tab);
    client.poll();
    self::assert_tabs(&client, name, index + 1);
  }
  let parent = self::snapshot(&client)
    .nodes
    .iter()
    .find(|node| node.label.as_deref() == Some("Select Sound from parent"))
    .unwrap()
    .object_id;
  client.ui().click(parent);
  client.poll();
  self::assert_tabs(&client, "Sound", 5);
  let sound = self::snapshot(&client)
    .nodes
    .iter()
    .find(|node| node.label.as_deref() == Some("Sound"))
    .unwrap()
    .object_id;
  client
    .ui()
    .send_event(UiEvent::click(sound, ClickEvent::NavigationSubmit));
  client.poll();
  self::assert_tabs(&client, "Sound", 6);
  self::range_action(&mut client, sound, UiAccessibilityAction::Activate);
  self::assert_tabs(&client, "Sound", 7);
  client.ui().click(page);
  client.poll();
  self::assert_tabs(&client, "Gameplay", 0);
  assert!(!client.ui().contains(sound));
  self::assert_page(&mut client, 8);
}

fn assert_tabs(client: &FakeClient<App>, selected: &str, count: usize) {
  let snapshot = self::snapshot(client);
  let list = snapshot
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::TabList)
    .unwrap();
  assert_eq!(list.label.as_deref(), Some("Settings categories"));
  let tabs = snapshot
    .nodes
    .iter()
    .filter(|node| node.role == SemanticRole::Tab)
    .collect::<Vec<_>>();
  assert_eq!(tabs.len(), 4);
  for (tab, label) in tabs.iter().zip(["Gameplay", "Graphics", "Sound", "Input"]) {
    assert_eq!(tab.label.as_deref(), Some(label));
    assert_eq!(tab.parent_id, Some(list.object_id));
    assert_eq!(tab.state.selected, Some(label == selected));
    assert!(!tab.state.disabled);
  }
  assert!(
    snapshot
      .nodes
      .iter()
      .any(|node| node.label.as_deref() == Some(&format!("Tab selections: {count}")))
  );
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
    Some(format!("\u{2068}{expected}\u{2069} percent").as_str())
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

fn assert_checked(client: &FakeClient<App>, target: ObjectId, checked: bool) {
  assert_eq!(
    self::snapshot(client)
      .nodes
      .iter()
      .find(|node| node.object_id == target)
      .unwrap()
      .state
      .checked,
    Some(if checked {
      CheckedState::True
    } else {
      CheckedState::False
    })
  );
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
  assert!(snapshot.nodes.iter().any(|node| {
    node.label.as_deref() == Some(&format!("VSync changes: \u{2068}{changes}\u{2069}"))
  }));
  assert_eq!(
    snapshot
      .nodes
      .iter()
      .find(|node| node.label.as_deref() == Some("Screen shake"))
      .unwrap()
      .state
      .checked,
    Some(CheckedState::True)
  );
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
    Some(format!("\u{2068}{}\u{2069}. \u{2068}{title}\u{2069}", index + 1).as_str())
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

fn control_label(client: &mut FakeClient<App>, control: ObjectId) -> ObjectId {
  let mut row = control;
  while client.ui().element(row).name() != Some("setting-row") {
    row = client.ui().element(row).parent_id().unwrap();
  }
  let children = client.ui().element(row).children().to_vec();
  children
    .into_iter()
    .find(|child| client.ui().element(*child).name() == Some("setting-row-label"))
    .unwrap()
}

fn click_label(client: &mut FakeClient<App>, label: ObjectId) {
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
}

fn select_interaction_specimen(client: &mut FakeClient<App>, label: &str) {
  let target = self::snapshot(client)
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::Button && node.label.as_deref() == Some(label))
    .unwrap()
    .object_id;
  client.ui().click(target);
  client.poll();
}

fn focus_visible(client: &mut FakeClient<App>, target: ObjectId, visible: bool, sequence: u64) {
  self::focus_visible_with_device(
    client,
    target,
    visible,
    sequence,
    MotionPointerDevice::Keyboard,
  );
}

fn focus_visible_with_device(
  client: &mut FakeClient<App>,
  target: ObjectId,
  visible: bool,
  sequence: u64,
  device: MotionPointerDevice,
) {
  let descriptor = match &client
    .ui()
    .element(target)
    .element()
    .visual_element()
    .motion
  {
    Prop::Set(value) => value.clone(),
    _ => panic!("focusable control has no motion descriptor"),
  };
  let sequence = MotionSequence(sequence);
  client.submit_motion(MotionEventBatch {
    first_sequence: sequence,
    last_sequence: sequence,
    events: Vec::new(),
    samples: Vec::new(),
    value_samples: Vec::new(),
    playback_events: Vec::new(),
    gesture_events: vec![MotionGestureEvent {
      descriptor_id: descriptor.descriptor_id,
      generation: descriptor.generation,
      kind: if visible {
        MotionGestureEventKind::FocusVisibleStart
      } else {
        MotionGestureEventKind::FocusVisibleEnd
      },
      pointer_id: -1,
      device,
      point: MotionGestureVector::default(),
      delta: MotionGestureVector::default(),
      offset: MotionGestureVector::default(),
      velocity: MotionGestureVector::default(),
      axis: None,
      momentum_generation: 0,
      constrained: false,
    }],
  });
  client.poll();
}

fn pointer_boundary(client: &mut FakeClient<App>, target: ObjectId, enter: bool) {
  let body = PointerBoundaryEvent {
    pointer_id: 0,
    position: PanelPoint::default(),
    pointer_type: PointerType::Mouse,
  };
  client.ui().send_event(UiEvent::new(
    target,
    false,
    false,
    if enter {
      UiEventBody::PointerEnter(body)
    } else {
      UiEventBody::PointerLeave(body)
    },
  ));
}

fn pointer_button(client: &mut FakeClient<App>, target: ObjectId, down: bool) {
  let body = PointerButtonEvent {
    pointer_id: 0,
    position: PanelPoint::default(),
    delta: Vector::default(),
    button: PointerButton::Left,
    buttons: u32::from(down),
    pressure: f32::from(down),
    click_count: 1,
    modifiers: KeyModifiers::default(),
    pointer_type: PointerType::Mouse,
  };
  client.ui().send_event(UiEvent::new(
    target,
    true,
    false,
    if down {
      UiEventBody::PointerDown(body)
    } else {
      UiEventBody::PointerUp(body)
    },
  ));
}

fn assert_scale(client: &mut FakeClient<App>, target: ObjectId, expected: f32) {
  let motion = match &client
    .ui()
    .element(target)
    .element()
    .visual_element()
    .motion
  {
    Prop::Set(value) => value.clone(),
    _ => panic!("interaction surface has no motion descriptor"),
  };
  let value = motion
    .slots
    .iter()
    .find(|slot| slot.layer == MotionLayer::Animate)
    .unwrap()
    .target
    .tracks
    .iter()
    .find(|track| track.property == MotionProperty::Scale)
    .unwrap()
    .values
    .last()
    .unwrap();
  assert!(
    matches!(value, MotionValue::Vector2([x, y]) if (*x - expected).abs() < f32::EPSILON && (*y - expected).abs() < f32::EPSILON),
    "expected scale {expected}, found {value:?}"
  );
}

fn assert_gradient_start(client: &mut FakeClient<App>, target: ObjectId, expected: Color) {
  let motion = match &client
    .ui()
    .element(target)
    .element()
    .visual_element()
    .motion
  {
    Prop::Set(value) => value.clone(),
    _ => panic!("interaction surface has no motion descriptor"),
  };
  let value = motion
    .slots
    .iter()
    .find(|slot| slot.layer == MotionLayer::Animate)
    .unwrap()
    .target
    .tracks
    .iter()
    .find(|track| track.property == MotionProperty::BackgroundGradient)
    .unwrap()
    .values
    .last()
    .unwrap();
  assert!(
    matches!(value, MotionValue::Gradient(Gradient::Linear { stops, .. }) if stops.first().is_some_and(|stop| stop.color == expected)),
    "expected gradient to start with {expected:?}, found {value:?}"
  );
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

fn toggle_info_client() -> FakeClient<App> {
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("chess-ui/content");
  assets.add_textures(asset_generator::registrations().map(|asset| asset.address));
  assets.add_ui_font(setting_row::DISPLAY_FONT);
  let app = App::new("chess-ui/content")
    .ui(ToggleInfoFixture)
    .document(ReviewSurface::document);
  let mut client = FakeClient::connect(app, assets);
  client.poll();
  client
}
