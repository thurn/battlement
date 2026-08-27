use battlement_types::{Color, ObjectId};
use battlement_ui::{
  Box, EasingFunction, FilterFunction, FilterList, LengthUnits, Rotate, Scale, Style, StyleValue,
  TimeValue, TransformOrigin, TransitionList, TransitionProperty, Translate, UiDocument, UiElement,
  UiNode, VisualElementUpdate,
};
use battlement_ui_fake::{UiWorld, UiWorldError};

#[test]
fn transform_and_transition_updates_merge_and_reject_invalid_values_atomically() {
  let target_id = ObjectId::new_v4();
  let initial = Style::new()
    .filter(FilterList::new([
      FilterFunction::Tint(Color::rgb(0.5, 0.8, 1.0)),
      FilterFunction::Opacity(0.9),
      FilterFunction::Invert(0.1),
      FilterFunction::Grayscale(0.2),
      FilterFunction::Sepia(0.3),
      FilterFunction::Blur(2.0),
      FilterFunction::Contrast(1.2),
      FilterFunction::HueRotate(20.0),
    ]))
    .rotate(Rotate::degrees(12.0))
    .scale(Scale::new(1.1, 0.9))
    .transform_origin(TransformOrigin::two_dimensional(0.pct(), 100.pct()))
    .transition_delay(TransitionList::new([TimeValue(-40.0), TimeValue(20.0)]))
    .transition_duration(TransitionList::new([TimeValue(180.0)]))
    .transition_property(TransitionList::new([
      TransitionProperty::Rotate,
      TransitionProperty::Translate,
    ]))
    .transition_timing_function(TransitionList::new([EasingFunction::EaseInOutCubic]))
    .translate(Translate::two_dimensional(12.pct(), 8.px()));
  let mut world = UiWorld::default();
  world
    .replace(vec![
      UiDocument::new(ObjectId::new_v4()).child(UiNode::new(target_id, Box::new().style(initial))),
    ])
    .unwrap();

  world
    .update(VisualElementUpdate::Properties {
      object_id: target_id,
      element: UiElement::from(Box::default().style(Style::new().scale(Scale::uniform(1.25))))
        .into(),
    })
    .unwrap();
  let committed = world.element(target_id).unwrap().style().clone();
  assert_eq!(
    committed.scale,
    Some(StyleValue::Value(Scale::uniform(1.25)))
  );
  assert_eq!(
    committed.rotate,
    Some(StyleValue::Value(Rotate::degrees(12.0)))
  );

  for invalid in [
    Style::new().rotate(Rotate::new(0.0, 0.0, 0.0, 20.0)),
    Style::new().scale(Scale::new(f32::NAN, 1.0)),
    Style::new().translate(Translate::new(0.px(), 0.px(), f32::INFINITY)),
    Style::new().transition_duration(TransitionList::new([TimeValue(-1.0)])),
    Style::new().filter(FilterList::new([FilterFunction::Blur(f32::NAN)])),
  ] {
    assert_eq!(
      world.update(VisualElementUpdate::Properties {
        object_id: target_id,
        element: UiElement::from(Box::default().style(invalid)).into(),
      }),
      Err(UiWorldError::InvalidProperty)
    );
    assert_eq!(world.element(target_id).unwrap().style(), &committed);
  }
}

#[test]
fn transition_lists_repeat_every_supported_easing_curve() {
  let curves = TransitionList::new([
    EasingFunction::Ease,
    EasingFunction::EaseIn,
    EasingFunction::EaseOut,
    EasingFunction::EaseInOut,
    EasingFunction::Linear,
    EasingFunction::EaseInSine,
    EasingFunction::EaseOutSine,
    EasingFunction::EaseInOutSine,
    EasingFunction::EaseInCubic,
    EasingFunction::EaseOutCubic,
    EasingFunction::EaseInOutCubic,
    EasingFunction::EaseInCirc,
    EasingFunction::EaseOutCirc,
    EasingFunction::EaseInOutCirc,
    EasingFunction::EaseInElastic,
    EasingFunction::EaseOutElastic,
    EasingFunction::EaseInOutElastic,
    EasingFunction::EaseInBack,
    EasingFunction::EaseOutBack,
    EasingFunction::EaseInOutBack,
    EasingFunction::EaseInBounce,
    EasingFunction::EaseOutBounce,
    EasingFunction::EaseInOutBounce,
  ]);

  assert_eq!(curves.as_slice().len(), 23);
  assert_eq!(curves.repeated(23), Some(&EasingFunction::Ease));
  assert_eq!(curves.repeated(47), Some(&EasingFunction::EaseIn));
  assert_eq!(TransitionList::<TimeValue>::new([]).repeated(0), None);
}
