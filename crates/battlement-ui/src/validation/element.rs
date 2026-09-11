use std::collections::HashSet;

use crate::{Prop, UiElement, UiVisualElementProperties, elements::parts};

use super::common::{validate_optional_string, validate_style, validate_visual};
use super::{UiValidationError, ValidationMode};

/// Validates sparse properties before applying an update to a live element.
///
/// Usage hints are rejected because Unity makes them read-only after an element
/// is attached to a panel.
///
/// # Errors
///
/// Returns [`UiValidationError::InvalidProperty`] for invalid common or
/// element-specific values and leaves the input unchanged.
pub fn validate_element_update(value: &UiElement) -> Result<(), UiValidationError> {
  if value.visual_element().usage_hints.is_some() {
    return Err(UiValidationError::InvalidProperty);
  }
  validate_element(value, ValidationMode::SparseUpdate)
}

/// Validates one complete element value independently of hierarchy placement.
///
/// Unlike [`validate_element_update`], this accepts create-time usage hints and
/// is intended for executors that have merged a sparse update into live state.
///
/// # Errors
///
/// Returns the first invalid common or element-specific property without
/// modifying the supplied value.
pub fn validate_element_state(value: &UiElement) -> Result<(), UiValidationError> {
  validate_element(value, ValidationMode::Complete)
}

pub(crate) fn validate_element(
  value: &UiElement,
  mode: ValidationMode,
) -> Result<(), UiValidationError> {
  validate_visual(value.visual_element())?;
  if !matches!(value, UiElement::VisualElement(_))
    && value
      .visual_element()
      .paint
      .set_value()
      .is_some_and(|paint| {
        matches!(
          paint.paint_blend_mode(),
          Some(crate::PaintBlendMode::Screen | crate::PaintBlendMode::Additive)
        )
      })
  {
    return Err(UiValidationError::InvalidProperty);
  }
  validate_parts(value, mode)?;
  match value {
    UiElement::VisualElement(_)
    | UiElement::Box(_)
    | UiElement::Label(_)
    | UiElement::TextElement(_)
    | UiElement::Button(_)
    | UiElement::RepeatButton(_)
    | UiElement::GroupBox(_)
    | UiElement::PopupWindow(_)
    | UiElement::Tab(_)
    | UiElement::TabView(_) => {}
    UiElement::Flex(value) => {
      validate_gap(value.row_gap.set_value())?;
      validate_gap(value.column_gap.set_value())?;
      validate_container_align(value.align_items.set_value())?;
    }
    UiElement::Grid(value) => {
      for track in value
        .columns
        .set_value()
        .into_iter()
        .flatten()
        .chain(value.rows.set_value().into_iter().flatten())
        .chain(value.auto_columns.set_value())
        .chain(value.auto_rows.set_value())
      {
        validate_grid_track(*track)?;
      }
      validate_gap(value.row_gap.set_value())?;
      validate_gap(value.column_gap.set_value())?;
      validate_container_align(value.align_items.set_value())?;
      validate_container_align(value.justify_items.set_value())?;
    }
    UiElement::Stack(value) => {
      validate_container_align(value.align_items.set_value())?;
      validate_container_align(value.justify_items.set_value())?;
    }
    UiElement::TextField(field) => {
      validate_optional_string(field.label.set_value().map(String::as_str), true)?;
      validate_optional_string(field.value.set_value().map(String::as_str), true)?;
      validate_optional_string(field.placeholder.set_value().map(String::as_str), true)?;
      let text = field.value.set_value().map(String::as_str).or_else(|| {
        (mode.requires_complete_state() || matches!(field.value, Prop::Reset)).then_some("")
      });
      if let Some(text) = text {
        let length = text.encode_utf16().count();
        if field
          .cursor_index
          .set_value()
          .is_some_and(|index| *index as usize > length)
          || field
            .select_index
            .set_value()
            .is_some_and(|index| *index as usize > length)
        {
          return Err(UiValidationError::InvalidProperty);
        }
      }
    }
    UiElement::Toggle(toggle) => {
      validate_optional_string(toggle.label.set_value().map(String::as_str), true)?;
      validate_optional_string(toggle.text.set_value().map(String::as_str), true)?;
    }
    UiElement::RadioButton(radio) => {
      validate_optional_string(radio.label.set_value().map(String::as_str), true)?;
      validate_optional_string(radio.text.set_value().map(String::as_str), true)?;
    }
    UiElement::RadioButtonGroup(group) => {
      validate_optional_string(group.label.set_value().map(String::as_str), true)?;
      let choices = group.choices.set_value().map_or(&[][..], Vec::as_slice);
      for choice in choices {
        validate_optional_string(Some(choice), true)?;
      }
      if (mode.requires_complete_state() || !matches!(group.choices, Prop::Unset))
        && group
          .selected_index
          .set_value()
          .is_some_and(|index| *index as usize >= choices.len())
      {
        return Err(UiValidationError::InvalidProperty);
      }
    }
    UiElement::ToggleButtonGroup(group) => {
      validate_optional_string(group.label.set_value().map(String::as_str), true)?;
      validate_selected_indices(
        group
          .selected_indices
          .set_value()
          .map_or(&[][..], Vec::as_slice),
      )?;
      if matches!(group.multiple_selection, Prop::Set(false) | Prop::Reset)
        && group
          .selected_indices
          .set_value()
          .is_some_and(|values| values.len() > 1)
      {
        return Err(UiValidationError::InvalidProperty);
      }
    }
    UiElement::DropdownField(field) => {
      validate_optional_string(field.label.set_value().map(String::as_str), true)?;
      let choices = field.choices.set_value().map_or(&[][..], Vec::as_slice);
      for choice in choices {
        validate_optional_string(Some(choice), true)?;
      }
      if choices.iter().collect::<HashSet<_>>().len() != choices.len() {
        return Err(UiValidationError::InvalidProperty);
      }
      if let Some(selection) = field.selection.set_value() {
        validate_dropdown_choice(
          selection,
          choices,
          mode.requires_complete_state() || !matches!(field.choices, Prop::Unset),
        )?;
      }
    }
    UiElement::Scroller(scroller) => {
      let values = [
        scroller.low_value.set_value().copied(),
        scroller.high_value.set_value().copied(),
        scroller.value.set_value().copied(),
      ];
      if values.into_iter().flatten().any(|value| !value.is_finite()) {
        return Err(UiValidationError::InvalidProperty);
      }
      let supplied_reversed = scroller
        .low_value
        .set_value()
        .zip(scroller.high_value.set_value())
        .is_some_and(|(low, high)| low > high);
      let complete_reversed = mode.requires_complete_state() && {
        let low = scroller.low_value.set_value().copied().unwrap_or(0.0);
        let high = scroller.high_value.set_value().copied().unwrap_or(0.0);
        low > high
      };
      if supplied_reversed || complete_reversed {
        return Err(UiValidationError::InvalidProperty);
      }
    }
    UiElement::Slider(slider) => {
      validate_optional_string(slider.label.set_value().map(String::as_str), true)?;
      let values = [
        slider.low_value.set_value().copied(),
        slider.high_value.set_value().copied(),
        slider.value.set_value().copied(),
        slider.page_size.set_value().copied(),
      ];
      if values.into_iter().flatten().any(|value| !value.is_finite())
        || slider
          .page_size
          .set_value()
          .is_some_and(|value| *value < 0.0)
      {
        return Err(UiValidationError::InvalidProperty);
      }
      let reversed = slider
        .low_value
        .set_value()
        .zip(slider.high_value.set_value())
        .is_some_and(|(low, high)| low > high);
      let complete_invalid = mode.requires_complete_state() && {
        let low = slider.low_value.set_value().copied().unwrap_or(0.0);
        let high = slider.high_value.set_value().copied().unwrap_or(10.0);
        !(low..=high).contains(&slider.value.set_value().copied().unwrap_or(0.0))
      };
      if reversed || complete_invalid {
        return Err(UiValidationError::InvalidProperty);
      }
    }
    UiElement::SliderInt(slider) => {
      validate_optional_string(slider.label.set_value().map(String::as_str), true)?;
      if slider
        .page_size
        .set_value()
        .is_some_and(|value| !value.is_finite() || *value < 0.0)
      {
        return Err(UiValidationError::InvalidProperty);
      }
      let reversed = slider
        .low_value
        .set_value()
        .zip(slider.high_value.set_value())
        .is_some_and(|(low, high)| low > high);
      let complete_invalid = mode.requires_complete_state() && {
        let low = slider.low_value.set_value().copied().unwrap_or(0);
        let high = slider.high_value.set_value().copied().unwrap_or(10);
        !(low..=high).contains(&slider.value.set_value().copied().unwrap_or(0))
      };
      if reversed || complete_invalid {
        return Err(UiValidationError::InvalidProperty);
      }
    }
    UiElement::MinMaxSlider(slider) => {
      validate_optional_string(slider.label.set_value().map(String::as_str), true)?;
      let low_limit = slider.low_limit.set_value().map(|value| match value {
        crate::LowerLimit::Unbounded => f32::MIN,
        crate::LowerLimit::Inclusive(value) => *value,
      });
      let high_limit = slider.high_limit.set_value().map(|value| match value {
        crate::UpperLimit::Unbounded => f32::MAX,
        crate::UpperLimit::Inclusive(value) => *value,
      });
      let values = [
        slider.min_value.set_value().copied(),
        slider.max_value.set_value().copied(),
        low_limit,
        high_limit,
      ];
      if values.into_iter().flatten().any(|value| !value.is_finite()) {
        return Err(UiValidationError::InvalidProperty);
      }
      let reversed_values = slider
        .min_value
        .set_value()
        .zip(slider.max_value.set_value())
        .is_some_and(|(min, max)| min > max);
      let reversed_limits = low_limit
        .zip(high_limit)
        .is_some_and(|(low, high)| low > high);
      let supplied_outside = slider
        .min_value
        .set_value()
        .copied()
        .zip(low_limit)
        .is_some_and(|(min, low)| min < low)
        || slider
          .max_value
          .set_value()
          .copied()
          .zip(high_limit)
          .is_some_and(|(max, high)| max > high);
      let complete_invalid = mode.requires_complete_state() && {
        let low = low_limit.unwrap_or(f32::MIN);
        let high = high_limit.unwrap_or(f32::MAX);
        let min = slider.min_value.set_value().copied().unwrap_or(0.0);
        let max = slider.max_value.set_value().copied().unwrap_or(10.0);
        low > high || min > max || min < low || max > high
      };
      if reversed_values || reversed_limits || supplied_outside || complete_invalid {
        return Err(UiValidationError::InvalidProperty);
      }
    }
    UiElement::ProgressBar(progress) => {
      validate_optional_string(progress.title.set_value().map(String::as_str), true)?;
      let values = [
        progress.low_value.set_value().copied(),
        progress.high_value.set_value().copied(),
        progress.value.set_value().copied(),
      ];
      if values.into_iter().flatten().any(|value| !value.is_finite()) {
        return Err(UiValidationError::InvalidProperty);
      }
      let reversed = progress
        .low_value
        .set_value()
        .zip(progress.high_value.set_value())
        .is_some_and(|(low, high)| low > high);
      let supplied_outside = progress
        .value
        .set_value()
        .zip(progress.low_value.set_value())
        .is_some_and(|(selected, low)| selected < low)
        || progress
          .value
          .set_value()
          .zip(progress.high_value.set_value())
          .is_some_and(|(selected, high)| selected > high);
      let complete_invalid = mode.requires_complete_state() && {
        let low = progress.low_value.set_value().copied().unwrap_or(0.0);
        let high = progress.high_value.set_value().copied().unwrap_or(100.0);
        !(low..=high).contains(&progress.value.set_value().copied().unwrap_or(0.0))
      };
      if reversed || supplied_outside || complete_invalid {
        return Err(UiValidationError::InvalidProperty);
      }
    }
    UiElement::ScrollView(scroll) => {
      let values = [
        scroll.scroll_offset.set_value().map(|value| value.x),
        scroll.scroll_offset.set_value().map(|value| value.y),
        scroll.horizontal_page_size.set_value().copied(),
        scroll.vertical_page_size.set_value().copied(),
        scroll.mouse_wheel_scroll_size.set_value().copied(),
        scroll.scroll_deceleration_rate.set_value().copied(),
        scroll.elasticity.set_value().copied(),
      ];
      if values.into_iter().flatten().any(|value| !value.is_finite()) {
        return Err(UiValidationError::InvalidProperty);
      }
    }
    UiElement::Image(image) => validate_image(image)?,
  }
  validate_text(value)?;
  if let UiElement::RepeatButton(value) = value
    && mode.requires_complete_state()
    && (matches!(value.delay_ms, Prop::Unset) || matches!(value.interval_ms, Prop::Unset))
  {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}

fn validate_text(value: &UiElement) -> Result<(), UiValidationError> {
  let text = match value {
    UiElement::Label(value) => value.text.set_value().map(String::as_str),
    UiElement::TextElement(value) => value.text.set_value().map(String::as_str),
    UiElement::TextField(value) => value.value.set_value().map(String::as_str),
    UiElement::Toggle(value) => value
      .text
      .set_value()
      .or(value.label.set_value())
      .map(String::as_str),
    UiElement::RadioButton(value) => value
      .text
      .set_value()
      .or(value.label.set_value())
      .map(String::as_str),
    UiElement::DropdownField(value) => value.label.set_value().map(String::as_str),
    UiElement::Slider(value) => value.label.set_value().map(String::as_str),
    UiElement::SliderInt(value) => value.label.set_value().map(String::as_str),
    UiElement::ProgressBar(value) => value.title.set_value().map(String::as_str),
    UiElement::Button(value) => value.text.set_value().map(String::as_str),
    UiElement::RepeatButton(value) => value.text.set_value().map(String::as_str),
    UiElement::GroupBox(value) => value.text.set_value().map(String::as_str),
    UiElement::PopupWindow(value) => value.text.set_value().map(String::as_str),
    UiElement::Tab(value) => value.text.set_value().map(String::as_str),
    UiElement::VisualElement(_)
    | UiElement::Flex(_)
    | UiElement::Grid(_)
    | UiElement::Stack(_)
    | UiElement::Box(_)
    | UiElement::RadioButtonGroup(_)
    | UiElement::ToggleButtonGroup(_)
    | UiElement::ScrollView(_)
    | UiElement::Scroller(_)
    | UiElement::MinMaxSlider(_)
    | UiElement::TabView(_)
    | UiElement::Image(_) => None,
  };
  validate_optional_string(text, true)
}

fn validate_gap(value: Option<&f32>) -> Result<(), UiValidationError> {
  if value.is_some_and(|value| !value.is_finite() || *value < 0.0) {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}

fn validate_container_align(value: Option<&crate::Align>) -> Result<(), UiValidationError> {
  if value == Some(&crate::Align::Auto) {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}

fn validate_grid_track(value: crate::GridTrack) -> Result<(), UiValidationError> {
  let valid = match value {
    crate::GridTrack::Px(value) => value.is_finite() && value >= 0.0,
    crate::GridTrack::Fraction(value) => value.is_finite() && value > 0.0,
    crate::GridTrack::Auto => true,
  };
  if !valid {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}

fn validate_parts(value: &UiElement, mode: ValidationMode) -> Result<(), UiValidationError> {
  let Some(part_styles) = parts::styles(value) else {
    return Ok(());
  };
  let mut keys = HashSet::new();
  for part in part_styles {
    let indexed = matches!(
      part.part,
      parts::Part::RadioButtonGroupOption
        | parts::Part::RadioButtonGroupOptionCheckmarkBackground
        | parts::Part::RadioButtonGroupOptionCheckmark
        | parts::Part::RadioButtonGroupOptionText
    );
    if !keys.insert((part.part, part.index))
      || !parts::belongs_to(value, part.part)
      || indexed != part.index.is_some()
      || part.index.is_some_and(|index| {
        !matches!(
            value,
            UiElement::RadioButtonGroup(group)
                if group
                    .choices
                    .set_value()
                    .is_some_and(|choices| (index as usize) < choices.len())
        )
      })
      || (mode.requires_complete_state() && !parts::exists_in_complete_state(value, part.part))
    {
      return Err(UiValidationError::InvalidProperty);
    }
    validate_style(&part.style)?;
  }
  Ok(())
}

fn validate_dropdown_choice(
  selection: &crate::Choice,
  choices: &[String],
  validate_against_choices: bool,
) -> Result<(), UiValidationError> {
  match (selection.index, selection.value.as_deref()) {
    (None, None) => Ok(()),
    (Some(index), Some(value))
      if !validate_against_choices
        || choices
          .get(index as usize)
          .is_some_and(|choice| choice == value) =>
    {
      Ok(())
    }
    _ => Err(UiValidationError::InvalidProperty),
  }
}

fn validate_selected_indices(values: &[u32]) -> Result<(), UiValidationError> {
  if values.windows(2).any(|pair| pair[0] >= pair[1]) {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}

fn validate_image(value: &crate::UiImage) -> Result<(), UiValidationError> {
  if matches!(value.source, Prop::Set(crate::ImageSource::Sprite(_)))
    && matches!(value.source_rect, Prop::Set(_))
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if let Prop::Set(rect) = value.source_rect {
    validate_rect(rect, false)?;
  }
  if let Prop::Set(rect) = value.uv {
    validate_rect(rect, true)?;
  }
  if let Prop::Set(color) = value.tint_color
    && [color.r, color.g, color.b, color.a]
      .into_iter()
      .any(|channel| !channel.is_finite() || !(0.0..=1.0).contains(&channel))
  {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}

fn validate_rect(value: battlement_types::Rect, normalized: bool) -> Result<(), UiValidationError> {
  let fields = [value.x, value.y, value.width, value.height];
  if fields.into_iter().any(|field| !field.is_finite())
    || value.x < 0.0
    || value.y < 0.0
    || value.width < 0.0
    || value.height < 0.0
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if normalized
    && (value.x < 0.0
      || value.y < 0.0
      || value.x + value.width > 1.0
      || value.y + value.height > 1.0)
  {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}
