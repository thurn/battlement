use super::*;

#[test]
fn complex_parts_page_updates_conditional_parts_without_rebuilding() {
  let mut client = FakeClient::connect(
    battlement_rules::create_engine().expect("UI sample engine should initialize"),
    sample_assets(),
  );

  client.ui().click(COMPLEX_PARTS_BUTTON_ID);
  assert_eq!(
    client.ui().element(COMPLEX_PARTS_TOGGLE_ID).text(),
    Some("Create conditional parts")
  );
  client.ui().click(COMPLEX_PARTS_TOGGLE_ID);
  assert_eq!(
    client.ui().element(COMPLEX_PARTS_TOGGLE_ID).text(),
    Some("Remove conditional parts")
  );
  assert!(matches!(
      client.ui().element(COMPLEX_PARTS_SLIDER_ID).element(),
      UiElement::Slider(value)
        if value.fill == Prop::Set(true) && value.show_input_field == Prop::Set(true)
  ));
  assert_eq!(
    client.ui().element(COMPLEX_PARTS_TITLE_ID).text(),
    Some("AUTHORED TITLE")
  );

  client.ui().click(COMPLEX_PARTS_TOGGLE_ID);
  assert_eq!(
    client.ui().element(COMPLEX_PARTS_TOGGLE_ID).text(),
    Some("Create conditional parts")
  );
  assert!(matches!(
      client.ui().element(COMPLEX_PARTS_SLIDER_ID).element(),
      UiElement::Slider(value)
        if value.fill == Prop::Set(false) && value.show_input_field == Prop::Set(false)
  ));
  assert_eq!(client.ui().element(COMPLEX_PARTS_TITLE_ID).text(), Some(""));
}

#[test]
fn containers_page_preserves_logical_children_across_conditional_titles() {
  let mut client = FakeClient::connect(
    battlement_rules::create_engine().expect("UI sample engine should initialize"),
    sample_assets(),
  );

  client.ui().click(CONTAINERS_BUTTON_ID);
  {
    let ui = client.ui();
    assert_eq!(ui.element(TITLED_GROUP_ID).text(), Some("AUDIO SETTINGS"));
    assert_eq!(ui.element(TITLED_GROUP_ID).children().len(), 2);
    assert_eq!(ui.element(EMPTY_GROUP_ID).text(), None);
    assert!(ui.element(EMPTY_GROUP_ID).children().is_empty());
    assert_eq!(
      ui.element(DYNAMIC_GROUP_ID).children(),
      [DYNAMIC_GROUP_CHILD_ID, DYNAMIC_GROUP_ACTION_ID]
    );
    assert_eq!(ui.element(DYNAMIC_GROUP_ID).text(), Some(""));
    let popup_children = ui.element(POPUP_WINDOW_ID).children();
    assert_eq!(popup_children.len(), 2);
    assert_eq!(
      ui.element(popup_children[0]).text(),
      Some("Sector 7  /  clear")
    );
    assert_eq!(
      ui.element(popup_children[1]).text(),
      Some("Squad ETA  /  04:20")
    );
  }

  client.ui().click(DYNAMIC_GROUP_ACTION_ID);
  {
    let ui = client.ui();
    assert_eq!(
      ui.element(DYNAMIC_GROUP_ID).text(),
      Some("TACTICAL OVERRIDES")
    );
    assert_eq!(
      ui.element(DYNAMIC_GROUP_ID).children(),
      [DYNAMIC_GROUP_CHILD_ID, DYNAMIC_GROUP_ACTION_ID]
    );
    assert_eq!(
      ui.element(DYNAMIC_GROUP_CHILD_ID).text(),
      Some("Title created; authored content stayed in place.")
    );
  }

  client.ui().click(DYNAMIC_GROUP_ACTION_ID);
  let ui = client.ui();
  assert_eq!(ui.element(DYNAMIC_GROUP_ID).text(), Some(""));
  assert_eq!(
    ui.element(DYNAMIC_GROUP_ID).children(),
    [DYNAMIC_GROUP_CHILD_ID, DYNAMIC_GROUP_ACTION_ID]
  );
}

#[test]
fn scroll_page_matches_manual_settlement_and_controlled_value_round_trip() {
  let (mut client, clock) = FakeClient::connect_clocked(
    |_| battlement_rules::create_engine().expect("UI sample engine should initialize"),
    sample_assets(),
  );

  client.ui().click(SCROLL_BUTTON_ID);
  {
    let ui = client.ui();
    assert_eq!(
      ui.element(PRIMARY_SCROLL_ID).kind(),
      UiElementKind::ScrollView
    );
    let UiElement::ScrollView(scroll) = ui.element(PRIMARY_SCROLL_ID).element() else {
      unreachable!("scroll specimen kind changed")
    };
    assert_eq!(scroll.scroll_offset, Prop::Unset);
    assert_eq!(scroll.horizontal_page_size, Prop::Unset);
    assert_eq!(scroll.vertical_page_size, Prop::Unset);
    assert_eq!(scroll.mouse_wheel_scroll_size, Prop::Set(1.0));
    assert_eq!(scroll.touch_scroll_behavior, Prop::Unset);
    assert_eq!(scroll.scroll_deceleration_rate, Prop::Unset);
    assert_eq!(scroll.elasticity, Prop::Unset);
    assert_eq!(scroll.elastic_animation_interval, Prop::Unset);
    assert_eq!(
      ui.element(CONTROLLED_SCROLLER_ID).kind(),
      UiElementKind::Scroller
    );
    assert_eq!(ui.element(SCROLL_STATUS_ID).text(), Some("Settled 0 × 0"));
    assert_eq!(ui.element(SCROLLER_STATUS_ID).text(), Some("Committed 42"));
  }

  client.ui().scroll_begin(PRIMARY_SCROLL_ID);
  client
    .ui()
    .scroll_change(PRIMARY_SCROLL_ID, Vector::new(72.0, 204.0));
  assert_eq!(client.ui().element(SCROLL_STATUS_ID).text(), Some("Moving"));
  clock.advance(std::time::Duration::from_millis(100));
  client.ui().scroll_end(PRIMARY_SCROLL_ID);
  client.ui().advance();
  assert_eq!(
    client.ui().element(SCROLL_STATUS_ID).text(),
    Some("Settled 72 × 204")
  );

  client.ui().scroller_begin(CONTROLLED_SCROLLER_ID);
  client.ui().scroller_change(CONTROLLED_SCROLLER_ID, 68.0);
  assert_eq!(
    client.ui().element(SCROLLER_STATUS_ID).text(),
    Some("Preview 68")
  );
  client.ui().scroller_commit(CONTROLLED_SCROLLER_ID);
  assert_eq!(
    client.ui().element(SCROLLER_STATUS_ID).text(),
    Some("Committed 68")
  );
  let ui = client.ui();
  let battlement::UiElement::Scroller(value) = ui.element(CONTROLLED_SCROLLER_ID).element() else {
    panic!("controlled element changed kind");
  };
  assert_eq!(value.value, Prop::Set(68.0));
}

#[test]
fn tabs_page_round_trips_selection_reorder_and_close_veto() {
  let mut client = FakeClient::connect(
    battlement_rules::create_engine().expect("UI sample engine should initialize"),
    sample_assets(),
  );

  client.ui().click(TABS_BUTTON_ID);
  assert_eq!(
    client.ui().element(TAB_VIEW_ID).children(),
    [
      BOARD_TAB_ID,
      NOTES_TAB_ID,
      LOADOUT_TAB_ID,
      TIMELINE_TAB_ID,
      SIGNAL_TAB_ID,
    ]
  );

  client.ui().tab_select(TAB_VIEW_ID, 3);
  let ui = client.ui();
  let battlement::UiElement::TabView(view) = ui.element(TAB_VIEW_ID).element() else {
    panic!("workspace changed kind");
  };
  assert_eq!(view.selected_tab_index, battlement::Prop::Set(3));

  client.ui().tab_reorder(TAB_VIEW_ID, 3, 1);
  assert_eq!(
    client.ui().element(TAB_VIEW_ID).children(),
    [
      BOARD_TAB_ID,
      TIMELINE_TAB_ID,
      NOTES_TAB_ID,
      LOADOUT_TAB_ID,
      SIGNAL_TAB_ID,
    ]
  );

  client.ui().tab_close(TAB_VIEW_ID, 0);
  assert!(client.ui().contains(BOARD_TAB_ID));
  assert_eq!(
    client.ui().element(TAB_STATUS_ID).text(),
    Some("Rejected close | BOARD is pinned")
  );

  client.ui().tab_close(TAB_VIEW_ID, 2);
  assert!(!client.ui().contains(NOTES_TAB_ID));
  assert_eq!(client.ui().element(TAB_VIEW_ID).children().len(), 4);
  assert_eq!(
    client.ui().element(TAB_STATUS_ID).text(),
    Some("Closed | 4 tabs remain")
  );
}

#[test]
fn text_field_page_separates_drafts_from_accepted_normalized_and_rejected_commits() {
  let mut client = FakeClient::connect(
    battlement_rules::create_engine().expect("UI sample engine should initialize"),
    sample_assets(),
  );

  client.ui().click(TEXT_FIELDS_BUTTON_ID);
  client.ui().text_input(ACCEPTED_TEXT_ID, "Knight");
  assert_eq!(
    client.ui().element(TEXT_DRAFT_ID).text(),
    Some("LOCAL DRAFT  Knight")
  );
  assert_eq!(
    client.ui().element(TEXT_COMMITTED_ID).text(),
    Some("RUST COMMITTED  Rook")
  );
  assert_eq!(client.ui().element(ACCEPTED_TEXT_ID).text(), Some("Rook"));
  client.ui().text_selection(ACCEPTED_TEXT_ID, 6, 0);
  client.ui().text_commit(ACCEPTED_TEXT_ID);
  assert_eq!(client.ui().element(ACCEPTED_TEXT_ID).text(), Some("Knight"));
  assert_eq!(
    client.ui().element(TEXT_STATUS_ID).text(),
    Some("ACCEPTED · exact value authored")
  );

  client.ui().text_input(NORMALIZED_TEXT_ID, "  bravo-9  ");
  client.ui().text_commit(NORMALIZED_TEXT_ID);
  assert_eq!(
    client.ui().element(NORMALIZED_TEXT_ID).text(),
    Some("BRAVO-9")
  );
  assert_eq!(
    client.ui().element(TEXT_STATUS_ID).text(),
    Some("NORMALIZED · BRAVO-9")
  );

  client.ui().text_input(REJECTED_TEXT_ID, "South Gate");
  client.ui().text_commit(REJECTED_TEXT_ID);
  assert_eq!(
    client.ui().element(REJECTED_TEXT_ID).text(),
    Some("North Gate")
  );
  assert_eq!(
    client.ui().element(TEXT_STATUS_ID).text(),
    Some("REJECTED · kept prior value")
  );
}

#[test]
fn boolean_controls_restore_native_proposals_until_rust_authors_state() {
  let mut client = FakeClient::connect(
    battlement_rules::create_engine().expect("UI sample engine should initialize"),
    sample_assets(),
  );

  client.ui().click(BOOLEAN_CONTROLS_BUTTON_ID);

  client.ui().toggle_click(ACCEPTED_TOGGLE_ID);
  assert_eq!(
    client.ui().element(ACCEPTED_TOGGLE_ID).bool_value(),
    Some(true)
  );
  assert_eq!(
    client.ui().element(BOOLEAN_STATUS_ID).text(),
    Some("ACCEPTED · threat alerts committed ON")
  );
  client.ui().toggle_click(ACCEPTED_TOGGLE_ID);
  assert_eq!(
    client.ui().element(ACCEPTED_TOGGLE_ID).bool_value(),
    Some(false)
  );
  assert_eq!(
    client.ui().element(BOOLEAN_STATUS_ID).text(),
    Some("ACCEPTED · threat alerts committed OFF")
  );

  client.ui().toggle_click(REJECTED_TOGGLE_ID);
  assert_eq!(
    client.ui().element(REJECTED_TOGGLE_ID).bool_value(),
    Some(true)
  );
  assert_eq!(
    client.ui().element(BOOLEAN_HISTORY_ID).text(),
    Some("PROPOSAL  ON → OFF  |  committed before callback: ON")
  );

  client.ui().radio_click(ACCEPTED_RADIO_ID);
  assert_eq!(
    client.ui().element(ACCEPTED_RADIO_ID).bool_value(),
    Some(true)
  );

  client.ui().radio_click(REJECTED_RADIO_ID);
  assert_eq!(
    client.ui().element(REJECTED_RADIO_ID).bool_value(),
    Some(false)
  );
  assert_eq!(
    client.ui().element(BOOLEAN_STATUS_ID).text(),
    Some("REJECTED · restricted channel stays OFF")
  );
  assert_eq!(
    client.ui().element(BOOLEAN_HISTORY_ID).text(),
    Some("PROPOSAL  OFF → ON  |  committed before callback: OFF")
  );
}

#[test]
fn choice_groups_commit_exclusive_and_sorted_multi_selection_indices() {
  let mut client = FakeClient::connect(
    battlement_rules::create_engine().expect("UI sample engine should initialize"),
    sample_assets(),
  );

  client.ui().click(CHOICE_GROUPS_BUTTON_ID);
  assert_eq!(client.ui().element(FORMATION_ID).selected_index(), Some(0));
  assert_eq!(
    client.ui().element(FILTER_ID).selected_indices(),
    Some([0, 2].as_slice())
  );

  client.ui().radio_group_select(FORMATION_ID, 1);
  assert_eq!(client.ui().element(FORMATION_ID).selected_index(), Some(1));
  assert_eq!(
    client.ui().element(CHOICE_STATUS_ID).text(),
    Some("FORMATION · WEDGE committed")
  );
  assert_eq!(
    client.ui().element(CHOICE_HISTORY_ID).text(),
    Some("EXCLUSIVE  LINE → WEDGE  |  index 0 → 1")
  );

  client.ui().toggle_group_click(FILTER_ID, 1);
  assert_eq!(
    client.ui().element(FILTER_ID).selected_indices(),
    Some([0, 1, 2].as_slice())
  );
  assert_eq!(
    client.ui().element(FILTER_SUMMARY_ID).text(),
    Some("SELECTED INDICES · [0, 1, 2]")
  );
  assert_eq!(
    client.ui().element(CHOICE_STATUS_ID).text(),
    Some("FILTERS · AIR + LAND + SEA")
  );
}

#[test]
fn dropdowns_accept_reject_and_clear_coherent_choices() {
  let mut client = FakeClient::connect(
    battlement_rules::create_engine().expect("UI sample engine should initialize"),
    sample_assets(),
  );

  client.ui().click(DROPDOWNS_BUTTON_ID);
  client.ui().dropdown_select(THEME_DROPDOWN_ID, 1);
  assert_eq!(
    client.ui().element(THEME_DROPDOWN_ID).choice(),
    Some(&battlement::Choice::selected(1, "SOLAR"))
  );
  assert_eq!(
    client.ui().element(THEME_SUMMARY_ID).text(),
    Some("COMMITTED · SOLAR (index 1)")
  );

  client.ui().dropdown_select(LOADOUT_DROPDOWN_ID, 1);
  assert_eq!(
    client.ui().element(LOADOUT_DROPDOWN_ID).choice(),
    Some(&battlement::Choice::selected(0, "SCOUT"))
  );
  assert_eq!(
    client.ui().element(DROPDOWN_STATUS_ID).text(),
    Some("REJECTED · HEAVY remains uncommitted")
  );
  assert_eq!(
    client.ui().element(DROPDOWN_HISTORY_ID).text(),
    Some("REJECTED  SCOUT → HEAVY  |  native proposal rolled back")
  );

  client.ui().click(CLEAR_LOADOUT_ID);
  assert_eq!(
    client.ui().element(LOADOUT_DROPDOWN_ID).choice(),
    Some(&battlement::Choice::none())
  );
  assert_eq!(
    client.ui().element(LOADOUT_SUMMARY_ID).text(),
    Some("CLEARED · no selected index or value")
  );
}

#[test]
fn sliders_keep_drag_values_transient_and_author_one_typed_release_value() {
  let mut client = FakeClient::connect(
    battlement_rules::create_engine().expect("UI sample engine should initialize"),
    sample_assets(),
  );

  client.ui().click(SLIDERS_BUTTON_ID);
  {
    let ui = client.ui();
    let UiElement::Slider(continuous) = ui.element(CONTINUOUS_SLIDER_ID).element() else {
      unreachable!("continuous specimen kind changed")
    };
    assert_eq!(continuous.value, Prop::Set(42.0));
    assert_eq!(continuous.fill, Prop::Set(true));
    assert_eq!(continuous.show_input_field, Prop::Set(true));
  }

  client.ui().slider_begin(CONTINUOUS_SLIDER_ID);
  client.ui().slider_change(CONTINUOUS_SLIDER_ID, 73.5);
  {
    let ui = client.ui();
    let UiElement::Slider(continuous) = ui.element(CONTINUOUS_SLIDER_ID).element() else {
      unreachable!("continuous specimen kind changed")
    };
    assert_eq!(
      continuous.value,
      Prop::Set(42.0),
      "drag state remains native-local"
    );
  }
  assert_eq!(
    client.ui().element(SLIDER_LIVE_STATUS_ID).text(),
    Some("LIVE  thrust trim  73.5%")
  );
  client.ui().slider_commit(CONTINUOUS_SLIDER_ID);
  {
    let ui = client.ui();
    let UiElement::Slider(continuous) = ui.element(CONTINUOUS_SLIDER_ID).element() else {
      unreachable!("continuous specimen kind changed")
    };
    assert_eq!(continuous.value, Prop::Set(73.5));
  }
  assert_eq!(
    client.ui().element(CONTINUOUS_VALUE_ID).text(),
    Some("FINAL · 73.5%")
  );

  client.ui().slider_int_begin(STEPPED_SLIDER_ID);
  client.ui().slider_int_change(STEPPED_SLIDER_ID, 6.6);
  client.ui().slider_int_commit(STEPPED_SLIDER_ID);
  {
    let ui = client.ui();
    let UiElement::SliderInt(stepped) = ui.element(STEPPED_SLIDER_ID).element() else {
      unreachable!("stepped specimen kind changed")
    };
    assert_eq!(stepped.value, Prop::Set(7));
    assert_eq!(stepped.inverted, Prop::Set(true));
  }
  assert_eq!(
    client.ui().element(STEPPED_VALUE_ID).text(),
    Some("FINAL · STEP 7")
  );
  assert_eq!(
    client.ui().element(SLIDER_COMMIT_STATUS_ID).text(),
    Some("COMMITTED  vertical integer 7")
  );
}

#[test]
fn range_sample_previews_and_authors_one_ordered_release_value() {
  let mut client = FakeClient::connect(
    battlement_rules::create_engine().expect("UI sample engine should initialize"),
    sample_assets(),
  );

  client.ui().click(RANGES_BUTTON_ID);
  client.ui().min_max_slider_begin(RESOURCE_RANGE_ID);
  client
    .ui()
    .min_max_slider_change(RESOURCE_RANGE_ID, 31.0, 68.0);
  assert_eq!(
    client.ui().element(RANGE_STATUS_ID).text(),
    Some("LIVE  reserve 31-68%")
  );
  {
    let ui = client.ui();
    let UiElement::MinMaxSlider(range) = ui.element(RESOURCE_RANGE_ID).element() else {
      unreachable!("resource range kind changed")
    };
    assert_eq!(
      (range.min_value, range.max_value),
      (Prop::Set(24.0), Prop::Set(76.0))
    );
  }

  client.ui().min_max_slider_commit(RESOURCE_RANGE_ID);
  {
    let ui = client.ui();
    let UiElement::MinMaxSlider(range) = ui.element(RESOURCE_RANGE_ID).element() else {
      unreachable!("resource range kind changed")
    };
    assert_eq!(
      (range.min_value, range.max_value),
      (Prop::Set(31.0), Prop::Set(68.0))
    );
  }
  assert_eq!(
    client.ui().element(RANGE_STATUS_ID).text(),
    Some("COMMITTED  reserve 31-68%")
  );
}
