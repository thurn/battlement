use battlement_types::ObjectId;
use battlement_ui::{
  Prop, SliderDirection, UiDocument, UiElement, UiMinMaxSlider, UiNode, UiProgressBar, UiSlider,
  UiSliderInt, VisualElementUpdate,
};
use battlement_ui_fake::{UiJournalEntry, UiWorld, UiWorldError};

#[test]
fn every_range_property_has_a_reset_executor() {
  let slider_id = ObjectId::new_v4();
  let integer_id = ObjectId::new_v4();
  let range_id = ObjectId::new_v4();
  let progress_id = ObjectId::new_v4();
  let mut world = UiWorld::default();
  world
    .replace(vec![
      UiDocument::new(ObjectId::new_v4())
        .child(UiNode::new(
          slider_id,
          UiSlider::new()
            .label("Level")
            .low_value(-10.0)
            .high_value(20.0)
            .value(5.0)
            .fill(true)
            .page_size(2.0)
            .show_input_field(true)
            .direction(SliderDirection::Vertical)
            .inverted(true),
        ))
        .child(UiNode::new(
          integer_id,
          UiSliderInt::new().low_value(-4).high_value(12).value(3),
        ))
        .child(UiNode::new(
          range_id,
          UiMinMaxSlider::new().min_value(2.0).max_value(8.0),
        ))
        .child(UiNode::new(
          progress_id,
          UiProgressBar::new()
            .low_value(-10.0)
            .high_value(20.0)
            .value(5.0)
            .title("Loading"),
        )),
    ])
    .unwrap();

  update(
    &mut world,
    slider_id,
    UiSlider::new()
      .label(Prop::Reset)
      .low_value(Prop::Reset)
      .high_value(Prop::Reset)
      .value(Prop::Reset)
      .fill(Prop::Reset)
      .page_size(Prop::Reset)
      .show_input_field(Prop::Reset)
      .direction(Prop::Reset)
      .inverted(Prop::Reset)
      .into(),
  );
  update(
    &mut world,
    integer_id,
    UiSliderInt::new()
      .label(Prop::Reset)
      .low_value(Prop::Reset)
      .high_value(Prop::Reset)
      .value(Prop::Reset)
      .fill(Prop::Reset)
      .page_size(Prop::Reset)
      .show_input_field(Prop::Reset)
      .direction(Prop::Reset)
      .inverted(Prop::Reset)
      .into(),
  );
  update(
    &mut world,
    range_id,
    UiMinMaxSlider::new()
      .label(Prop::Reset)
      .min_value(Prop::Reset)
      .max_value(Prop::Reset)
      .low_limit(Prop::Reset)
      .high_limit(Prop::Reset)
      .into(),
  );
  update(
    &mut world,
    progress_id,
    UiProgressBar::new()
      .low_value(Prop::Reset)
      .high_value(Prop::Reset)
      .value(Prop::Reset)
      .title(Prop::Reset)
      .into(),
  );

  assert!(matches!(
    world.element(slider_id).unwrap().element(),
    UiElement::Slider(value)
      if value.label == Prop::Reset
        && value.low_value == Prop::Reset
        && value.high_value == Prop::Reset
        && value.value == Prop::Reset
        && value.fill == Prop::Reset
        && value.page_size == Prop::Reset
        && value.show_input_field == Prop::Reset
        && value.direction == Prop::Reset
        && value.inverted == Prop::Reset
  ));
  assert_eq!(world.journal().len(), 4);
  assert!(
    world
      .journal()
      .iter()
      .all(|entry| matches!(entry, UiJournalEntry::Update(_)))
  );
}

#[test]
fn invalid_sparse_range_reset_is_atomic() {
  let slider_id = ObjectId::new_v4();
  let mut world = UiWorld::default();
  world
    .replace(vec![
      UiDocument::new(ObjectId::new_v4()).child(UiNode::new(
        slider_id,
        UiSlider::new()
          .low_value(50.0)
          .high_value(100.0)
          .value(75.0),
      )),
    ])
    .unwrap();
  let before = world.element(slider_id).unwrap().element().clone();

  let result = world.update(VisualElementUpdate::Properties {
    object_id: slider_id,
    element: std::boxed::Box::new(UiSlider::new().high_value(Prop::Reset).into()),
  });

  assert_eq!(result, Err(UiWorldError::InvalidProperty));
  assert_eq!(world.element(slider_id).unwrap().element(), &before);
}

fn update(world: &mut UiWorld, object_id: ObjectId, element: UiElement) {
  world
    .update(VisualElementUpdate::Properties {
      object_id,
      element: std::boxed::Box::new(element),
    })
    .unwrap();
}
