use std::collections::HashSet;

use crate::{Position, Prop, Style, StyleValue};

use super::{MAXIMUM_STRING_BYTES, UiValidationError};

pub(crate) fn validate_visual(visual: &crate::UiVisualElement) -> Result<(), UiValidationError> {
  validate_optional_string(visual.name.set_value().map(String::as_str), true)?;
  let mut classes = HashSet::new();
  if let Some(values) = visual.classes.set_value() {
    for class_name in values {
      validate_optional_string(Some(class_name), false)?;
      if !classes.insert(class_name) {
        return Err(UiValidationError::InvalidProperty);
      }
    }
  }
  if let Some(values) = visual.events.set_value()
    && values.iter().collect::<HashSet<_>>().len() != values.len()
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if let Some(values) = visual.event_subscriptions.set_value() {
    if values.iter().collect::<HashSet<_>>().len() != values.len() {
      return Err(UiValidationError::InvalidProperty);
    }
    if values
      .iter()
      .any(|value| value.phase != crate::UiEventPhase::Target && !value.kind.propagates())
    {
      return Err(UiValidationError::InvalidProperty);
    }
    if visual.events.set_value().is_some_and(|shorthand| {
      values
        .iter()
        .any(|value| value.phase == crate::UiEventPhase::Target && shorthand.contains(&value.kind))
    }) {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if let Some(values) = &visual.usage_hints
    && values.iter().collect::<HashSet<_>>().len() != values.len()
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if visual
    .motion
    .set_value()
    .is_some_and(|descriptor| descriptor.validate().is_err())
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if visual
    .paint
    .set_value()
    .is_some_and(|value| !value.is_valid())
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if visual
    .grid_item
    .set_value()
    .is_some_and(|value| !valid_grid_item(value))
    || visual
      .stack_item
      .set_value()
      .is_some_and(|value| !valid_stack_item(value))
    || visual
      .sticky
      .set_value()
      .is_some_and(|value| !valid_sticky(value))
    || visual
      .overlay_placement
      .set_value()
      .is_some_and(|value| !valid_overlay(value))
  {
    return Err(UiValidationError::InvalidProperty);
  }
  let has_sticky = matches!(visual.sticky, Prop::Set(_));
  if has_sticky
    && (matches!(visual.stack_item, Prop::Set(_))
      || matches!(visual.overlay_placement, Prop::Set(_)))
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if has_sticky
    && matches!(
      visual.style.position,
      Prop::Set(StyleValue::Value(Position::Absolute))
    )
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if let Some(overlay) = visual.overlay_placement.set_value()
    && !valid_overlay_style(&visual.style, overlay)
  {
    return Err(UiValidationError::InvalidProperty);
  }
  validate_style(&visual.style)
}

fn valid_grid_item(value: &crate::GridItem) -> bool {
  let starts_are_positive =
    value.row.is_none_or(|value| value > 0) && value.column.is_none_or(|value| value > 0);
  let spans_are_positive = value.row_span > 0 && value.column_span > 0;
  if !starts_are_positive || !spans_are_positive {
    return false;
  }
  let rows_are_finite = value
    .row
    .is_none_or(|start| start.checked_add(value.row_span - 1).is_some());
  let columns_are_finite = value
    .column
    .is_none_or(|start| start.checked_add(value.column_span - 1).is_some());
  rows_are_finite && columns_are_finite
}

fn valid_stack_item(value: &crate::StackItem) -> bool {
  [value.top, value.right, value.bottom, value.left]
    .into_iter()
    .flatten()
    .all(|value| value.is_finite() && value >= 0.0)
}

fn valid_sticky(value: &crate::Sticky) -> bool {
  let horizontal_edges = usize::from(value.left.is_some()) + usize::from(value.right.is_some());
  let vertical_edges = usize::from(value.top.is_some()) + usize::from(value.bottom.is_some());
  let has_edge = horizontal_edges + vertical_edges > 0;
  let compatible_edges = horizontal_edges <= 1 && vertical_edges <= 1;
  let finite = [value.top, value.right, value.bottom, value.left]
    .into_iter()
    .flatten()
    .all(f32::is_finite);
  has_edge && compatible_edges && finite
}

fn valid_overlay(value: &crate::OverlayPlacement) -> bool {
  let crate::OverlayPlacement::Popover { placement, .. } = value else {
    return true;
  };
  placement.main_offset.is_finite()
    && placement.cross_offset.is_finite()
    && placement.collision_padding.is_finite()
    && placement.collision_padding >= 0.0
}

fn valid_overlay_style(style: &Style, overlay: &crate::OverlayPlacement) -> bool {
  let forbidden = [
    &style.margin_top,
    &style.margin_right,
    &style.margin_bottom,
    &style.margin_left,
    &style.top,
    &style.right,
    &style.bottom,
    &style.left,
  ];
  if forbidden
    .into_iter()
    .any(|value| matches!(value, Prop::Set(_)))
    || matches!(style.position, Prop::Set(_))
    || matches!(style.display, Prop::Set(_))
    || matches!(style.visibility, Prop::Set(_))
  {
    return false;
  }
  if matches!(overlay, crate::OverlayPlacement::Popover { .. }) {
    return true;
  }
  [
    &style.width,
    &style.height,
    &style.min_width,
    &style.min_height,
    &style.max_width,
    &style.max_height,
  ]
  .into_iter()
  .all(|value| !matches!(value, Prop::Set(_)))
}

pub(super) fn validate_optional_string(
  value: Option<&str>,
  allow_empty: bool,
) -> Result<(), UiValidationError> {
  let too_long = value.is_some_and(|text| text.len() > MAXIMUM_STRING_BYTES);
  let invalid_empty = value.is_some_and(str::is_empty) && !allow_empty;
  if too_long || invalid_empty {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}

/// Validates the shared inline style value family.
pub(crate) fn validate_style(value: &Style) -> Result<(), UiValidationError> {
  validate_prop_length(&value.font_size, true)?;
  if prop_concrete(&value.font_size).is_some_and(|length| {
    let [px, percent] = length.components();
    px <= 0.0 && percent <= 0.0
  }) {
    return Err(UiValidationError::InvalidProperty);
  }
  for property in [
    &value.letter_spacing,
    &value.unity_paragraph_spacing,
    &value.word_spacing,
  ] {
    validate_prop_length(property, false)?;
  }
  for property in [
    &value.width,
    &value.height,
    &value.min_width,
    &value.min_height,
  ] {
    validate_prop_length_or_auto(property, true)?;
  }
  for property in [&value.max_width, &value.max_height] {
    validate_prop_length_or_auto(property, true)?;
  }
  for property in [
    &value.bottom,
    &value.flex_basis,
    &value.left,
    &value.margin_bottom,
    &value.margin_left,
    &value.margin_right,
    &value.margin_top,
    &value.right,
    &value.top,
  ] {
    validate_prop_length_or_auto(property, false)?;
  }
  for property in [
    &value.padding_bottom,
    &value.padding_left,
    &value.padding_right,
    &value.padding_top,
  ] {
    validate_prop_length(property, true)?;
  }
  for property in [
    &value.border_bottom_left_radius,
    &value.border_bottom_right_radius,
    &value.border_top_left_radius,
    &value.border_top_right_radius,
  ] {
    validate_prop_length(property, true)?;
  }
  for property in [
    &value.border_bottom_width,
    &value.border_left_width,
    &value.border_right_width,
    &value.border_top_width,
    &value.flex_grow,
    &value.flex_shrink,
  ] {
    if prop_concrete(property).is_some_and(|number| !number.0.is_finite() || number.0 < 0.0) {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if prop_concrete(&value.opacity)
    .is_some_and(|number| !number.0.is_finite() || !(0.0..=1.0).contains(&number.0))
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if prop_concrete(&value.unity_slice_scale)
    .is_some_and(|number| !number.0.is_finite() || number.0 <= 0.0)
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if prop_concrete(&value.unity_text_outline_width)
    .is_some_and(|number| !number.0.is_finite() || number.0 < 0.0)
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if let Some(crate::TextShadow {
    x,
    y,
    blur_radius,
    color,
  }) = prop_concrete(&value.text_shadow)
  {
    if !x.is_finite() || !y.is_finite() || !blur_radius.is_finite() || *blur_radius < 0.0 {
      return Err(UiValidationError::InvalidProperty);
    }
    validate_color(color)?;
  }
  if let Some(crate::TextAutoSize::BestFit { min_size, max_size }) =
    prop_concrete(&value.unity_text_auto_size)
    && [
      !min_size.is_finite(),
      !max_size.is_finite(),
      *min_size <= 0.0,
      min_size > max_size,
    ]
    .into_iter()
    .any(|invalid| invalid)
  {
    return Err(UiValidationError::InvalidProperty);
  }
  for property in [
    &value.unity_slice_bottom,
    &value.unity_slice_left,
    &value.unity_slice_right,
    &value.unity_slice_top,
  ] {
    if prop_concrete(property).is_some_and(|number| *number < 0) {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if let Some(crate::AspectRatio::Ratio { width, height }) = prop_concrete(&value.aspect_ratio) {
    let valid_components = width.is_finite() && height.is_finite();
    let valid_range = *width > 0.0 && *height > 0.0;
    if !valid_components || !valid_range || !(width / height).is_finite() {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if let Some(position) = prop_concrete(&value.background_position_x) {
    validate_concrete_length(&position.offset, false)?;
    if !matches!(
      position.keyword,
      crate::BackgroundPositionKeyword::Left
        | crate::BackgroundPositionKeyword::Center
        | crate::BackgroundPositionKeyword::Right
    ) {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if let Some(position) = prop_concrete(&value.background_position_y) {
    validate_concrete_length(&position.offset, false)?;
    if !matches!(
      position.keyword,
      crate::BackgroundPositionKeyword::Top
        | crate::BackgroundPositionKeyword::Center
        | crate::BackgroundPositionKeyword::Bottom
    ) {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if let Some(crate::BackgroundSize::Axes { x, y }) = prop_concrete(&value.background_size) {
    validate_concrete_length_or_auto(x, true)?;
    validate_concrete_length_or_auto(y, true)?;
  }
  if let Some(crate::Cursor::Texture { hotspot, .. }) = prop_concrete(&value.cursor) {
    let finite = hotspot.x.is_finite() && hotspot.y.is_finite();
    let nonnegative = hotspot.x >= 0.0 && hotspot.y >= 0.0;
    if !finite || !nonnegative {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if let Some(rotation) = prop_concrete(&value.rotate) {
    let axis = [rotation.x, rotation.y, rotation.z];
    if axis.into_iter().any(|number| !number.is_finite())
      || !rotation.degrees.is_finite()
      || axis == [0.0, 0.0, 0.0]
    {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if let Some(scale) = prop_concrete(&value.scale)
    && (!scale.x.is_finite() || !scale.y.is_finite())
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if let Some(origin) = prop_concrete(&value.transform_origin) {
    validate_concrete_length(&origin.x, false)?;
    validate_concrete_length(&origin.y, false)?;
    if !origin.z.is_finite() {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  if let Some(translation) = prop_concrete(&value.translate) {
    validate_concrete_length(&translation.x, false)?;
    validate_concrete_length(&translation.y, false)?;
    if !translation.z.is_finite() {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  validate_transition_times(&value.transition_delay, false)?;
  validate_transition_times(&value.transition_duration, true)?;
  for color in [
    &value.background_color,
    &value.border_bottom_color,
    &value.border_left_color,
    &value.border_right_color,
    &value.border_top_color,
    &value.color,
    &value.unity_background_image_tint_color,
    &value.unity_text_outline_color,
  ]
  .into_iter()
  .filter_map(prop_concrete)
  {
    if [color.r, color.g, color.b, color.a]
      .into_iter()
      .any(|channel| !channel.is_finite() || !(0.0..=1.0).contains(&channel))
    {
      return Err(UiValidationError::InvalidProperty);
    }
  }
  Ok(())
}

fn validate_transition_times(
  value: &crate::Prop<crate::StyleValue<crate::TransitionList<crate::TimeValue>>>,
  nonnegative: bool,
) -> Result<(), UiValidationError> {
  let Some(values) = prop_concrete(value) else {
    return Ok(());
  };
  if values
    .as_slice()
    .iter()
    .any(|value| !value.0.is_finite() || nonnegative && value.0 < 0.0)
  {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}

fn validate_color(value: &battlement_types::Color) -> Result<(), UiValidationError> {
  if [value.r, value.g, value.b, value.a]
    .into_iter()
    .any(|channel| !channel.is_finite() || !(0.0..=1.0).contains(&channel))
  {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}

fn concrete<T>(value: Option<&crate::StyleValue<T>>) -> Option<&T> {
  match value {
    Some(crate::StyleValue::Value(value)) => Some(value),
    Some(crate::StyleValue::Keyword { .. }) | None => None,
  }
}

fn prop_concrete<T>(value: &crate::Prop<crate::StyleValue<T>>) -> Option<&T> {
  concrete(value.set_value())
}

fn validate_prop_length(
  value: &crate::Prop<crate::StyleValue<crate::Length>>,
  nonnegative: bool,
) -> Result<(), UiValidationError> {
  let Some(value) = prop_concrete(value) else {
    return Ok(());
  };
  validate_concrete_length(value, nonnegative)
}

fn validate_concrete_length(
  value: &crate::Length,
  nonnegative: bool,
) -> Result<(), UiValidationError> {
  let [px, percent] = value.components();
  let negative = match value {
    crate::Length::Px(_) => px < 0.0,
    crate::Length::Percent(_) => percent < 0.0,
    crate::Length::Calc { .. } => return Err(UiValidationError::InvalidProperty),
  };
  if !px.is_finite() || !percent.is_finite() || nonnegative && negative {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}

fn validate_prop_length_or_auto(
  value: &crate::Prop<crate::StyleValue<crate::LengthOrAuto>>,
  nonnegative: bool,
) -> Result<(), UiValidationError> {
  let Some(value) = prop_concrete(value) else {
    return Ok(());
  };
  validate_concrete_length_or_auto(value, nonnegative)
}

fn validate_concrete_length_or_auto(
  value: &crate::LengthOrAuto,
  nonnegative: bool,
) -> Result<(), UiValidationError> {
  let number = match value {
    crate::LengthOrAuto::Px(value) | crate::LengthOrAuto::Percent(value) => Some(*value),
    crate::LengthOrAuto::Auto => None,
  };
  if number.is_some_and(|number| !number.is_finite() || nonnegative && number < 0.0) {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}
