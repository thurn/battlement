use battlement_types::ObjectId;
use battlement_ui::{
  Button, DropdownField, Prop, RadioButton, RadioButtonGroup, ScrollerVisibility, TextField,
  Toggle, ToggleButtonGroup, UiDocument, UiElement, UiNode, VisualElementUpdate,
};
use battlement_ui_fake::{UiJournalEntry, UiWorld, UiWorldError};

#[test]
fn text_and_choice_properties_preserve_omissions_and_reset_without_remounting() {
  let text_id = ObjectId::new_v4();
  let toggle_id = ObjectId::new_v4();
  let radio_id = ObjectId::new_v4();
  let radio_group_id = ObjectId::new_v4();
  let toggle_group_id = ObjectId::new_v4();
  let dropdown_id = ObjectId::new_v4();
  let mut world = UiWorld::default();
  world
    .replace(vec![
      UiDocument::new(ObjectId::new_v4())
        .child(UiNode::new(text_id, TextField::new()))
        .child(UiNode::new(toggle_id, Toggle::new()))
        .child(UiNode::new(radio_id, RadioButton::new()))
        .child(UiNode::new(radio_group_id, RadioButtonGroup::new()))
        .child(
          UiNode::new(toggle_group_id, ToggleButtonGroup::new()).children([
            UiNode::new(ObjectId::new_v4(), Button::new("One")),
            UiNode::new(ObjectId::new_v4(), Button::new("Two")),
          ]),
        )
        .child(UiNode::new(dropdown_id, DropdownField::new())),
    ])
    .unwrap();

  update(
    &mut world,
    text_id,
    TextField::new()
      .label("Name")
      .value("Rook")
      .multiline(true)
      .vertical_scroller_visibility(ScrollerVisibility::AlwaysVisible)
      .password(true)
      .read_only(true)
      .placeholder("Callsign")
      .hide_placeholder_on_focus(true)
      .cursor_index(4)
      .select_index(1)
      .select_all_on_focus(true)
      .select_all_on_mouse_up(true)
      .into(),
  );
  update(
    &mut world,
    toggle_id,
    Toggle::new()
      .label("Audio")
      .text("Muted")
      .value(true)
      .into(),
  );
  update(
    &mut world,
    radio_id,
    RadioButton::new()
      .label("Mode")
      .text("Fast")
      .value(true)
      .into(),
  );
  update(
    &mut world,
    radio_group_id,
    RadioButtonGroup::new()
      .label("Quality")
      .choices(["Low", "High"])
      .selected_index(1)
      .into(),
  );
  update(
    &mut world,
    toggle_group_id,
    ToggleButtonGroup::new()
      .label("Tools")
      .multiple_selection(true)
      .allow_empty_selection(true)
      .selected_indices([0, 1])
      .into(),
  );
  update(
    &mut world,
    dropdown_id,
    DropdownField::new()
      .label("Difficulty")
      .show_mixed_value(true)
      .choices(["Story", "Veteran"])
      .selection(1, "Veteran")
      .into(),
  );
  update(&mut world, text_id, TextField::new().into());
  assert_eq!(world.element(text_id).unwrap().text(), Some("Rook"));

  update(
    &mut world,
    text_id,
    TextField::new()
      .label(Prop::Reset)
      .value(Prop::Reset)
      .multiline(Prop::Reset)
      .vertical_scroller_visibility(Prop::Reset)
      .password(Prop::Reset)
      .read_only(Prop::Reset)
      .placeholder(Prop::Reset)
      .hide_placeholder_on_focus(Prop::Reset)
      .cursor_index(Prop::Reset)
      .select_index(Prop::Reset)
      .select_all_on_focus(Prop::Reset)
      .select_all_on_mouse_up(Prop::Reset)
      .into(),
  );
  update(
    &mut world,
    toggle_id,
    Toggle::new()
      .label(Prop::Reset)
      .text(Prop::Reset)
      .value(Prop::Reset)
      .into(),
  );
  update(
    &mut world,
    radio_id,
    RadioButton::new()
      .label(Prop::Reset)
      .text(Prop::Reset)
      .value(Prop::Reset)
      .into(),
  );
  update(
    &mut world,
    radio_group_id,
    RadioButtonGroup::new()
      .label(Prop::Reset)
      .choices_value(Prop::Reset)
      .selected_index(Prop::Reset)
      .into(),
  );
  update(
    &mut world,
    toggle_group_id,
    ToggleButtonGroup::new()
      .label(Prop::Reset)
      .multiple_selection(Prop::Reset)
      .allow_empty_selection(Prop::Reset)
      .selected_indices_value(Prop::Reset)
      .into(),
  );
  update(
    &mut world,
    dropdown_id,
    DropdownField::new()
      .label(Prop::Reset)
      .show_mixed_value(Prop::Reset)
      .choices_value(Prop::Reset)
      .selection_value(Prop::Reset)
      .into(),
  );

  assert_eq!(world.element(text_id).unwrap().object_id(), text_id);
  assert_eq!(world.element(toggle_id).unwrap().bool_value(), None);
  assert_eq!(
    world.element(radio_group_id).unwrap().selected_index(),
    None
  );
  assert_eq!(world.element(dropdown_id).unwrap().choice(), None);
  assert!(
    world
      .journal()
      .iter()
      .all(|entry| matches!(entry, UiJournalEntry::Update(_)))
  );
}

#[test]
fn invalid_choice_reset_rejects_without_partial_change() {
  let dropdown_id = ObjectId::new_v4();
  let mut world = UiWorld::default();
  world
    .replace(vec![UiDocument::new(ObjectId::new_v4()).child(
      UiNode::new(
        dropdown_id,
        DropdownField::new().choices(["One"]).selection(0, "One"),
      ),
    )])
    .unwrap();
  let before = world.element(dropdown_id).unwrap().element().clone();

  let result = world.update(VisualElementUpdate::Properties {
    object_id: dropdown_id,
    element: std::boxed::Box::new(DropdownField::new().choices_value(Prop::Reset).into()),
  });

  assert_eq!(result, Err(UiWorldError::InvalidProperty));
  assert_eq!(world.element(dropdown_id).unwrap().element(), &before);
}

fn update(world: &mut UiWorld, object_id: ObjectId, element: UiElement) {
  world
    .update(VisualElementUpdate::Properties {
      object_id,
      element: std::boxed::Box::new(element),
    })
    .unwrap();
}
