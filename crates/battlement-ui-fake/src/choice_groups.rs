use battlement_ui::{Prop, UiElement};

use crate::UiWorldError;

pub(crate) fn validate_state(value: &UiElement, child_count: usize) -> Result<(), UiWorldError> {
  if let UiElement::RadioButtonGroup(group) = value {
    let choice_count = match &group.choices {
      Prop::Set(values) => values.len(),
      Prop::Unset | Prop::Reset => 0,
    };
    if matches!(group.selected_index, Prop::Set(index) if index as usize >= choice_count) {
      return Err(UiWorldError::InvalidProperty);
    }
  }
  if let UiElement::ToggleButtonGroup(group) = value {
    let default_selected = [0];
    let selected: &[u32] = match &group.selected_indices {
      Prop::Set(values) => values,
      Prop::Unset | Prop::Reset
        if child_count == 0 || matches!(group.allow_empty_selection, Prop::Set(true)) =>
      {
        &[]
      }
      Prop::Unset | Prop::Reset => &default_selected,
    };
    let invalid_indices = selected.iter().any(|index| *index as usize >= child_count)
      || selected.windows(2).any(|pair| pair[0] >= pair[1]);
    let invalid_cardinality =
      !matches!(group.multiple_selection, Prop::Set(true)) && selected.len() > 1;
    let invalid_empty = child_count > 0
      && !matches!(group.allow_empty_selection, Prop::Set(true))
      && selected.is_empty();
    if invalid_indices || invalid_cardinality || invalid_empty {
      return Err(UiWorldError::InvalidProperty);
    }
  }
  if let UiElement::DropdownField(field) = value {
    let choices = match &field.choices {
      Prop::Set(values) => values.as_slice(),
      Prop::Unset | Prop::Reset => &[],
    };
    let selection = match &field.selection {
      Prop::Set(value) => value.clone(),
      Prop::Unset | Prop::Reset => battlement_ui::Choice::none(),
    };
    let valid = match (selection.index, selection.value.as_deref()) {
      (None, None) => true,
      (Some(index), Some(value)) => choices
        .get(index as usize)
        .is_some_and(|choice| choice == value),
      _ => false,
    };
    if !valid {
      return Err(UiWorldError::InvalidProperty);
    }
  }
  Ok(())
}

pub(crate) fn insert(element: &mut UiElement, index: usize, child_count: usize) {
  let Some(mut selected) = self::selection(element, child_count.saturating_sub(1)) else {
    return;
  };
  for value in &mut selected {
    if *value as usize >= index {
      *value += 1;
    }
  }
  self::set_selection(element, selected, child_count);
}

pub(crate) fn remove(element: &mut UiElement, index: usize, child_count: usize) {
  let Some(mut selected) = self::selection(element, child_count + 1) else {
    return;
  };
  selected.retain(|value| *value as usize != index);
  for value in &mut selected {
    if *value as usize > index {
      *value -= 1;
    }
  }
  self::set_selection(element, selected, child_count);
}

pub(crate) fn reorder(
  element: &mut UiElement,
  previous_index: usize,
  next_index: usize,
  child_count: usize,
) {
  let Some(mut selected) = self::selection(element, child_count) else {
    return;
  };
  for value in &mut selected {
    let position = *value as usize;
    *value = if position == previous_index {
      next_index as u32
    } else if previous_index < next_index && position > previous_index {
      if position <= next_index {
        *value - 1
      } else {
        *value
      }
    } else if previous_index > next_index && position >= next_index {
      if position < previous_index {
        *value + 1
      } else {
        *value
      }
    } else {
      *value
    };
  }
  selected.sort_unstable();
  self::set_selection(element, selected, child_count);
}

fn selection(element: &UiElement, child_count: usize) -> Option<Vec<u32>> {
  let UiElement::ToggleButtonGroup(value) = element else {
    return None;
  };
  Some(match &value.selected_indices {
    Prop::Set(values) => values.clone(),
    Prop::Unset | Prop::Reset
      if child_count == 0 || matches!(value.allow_empty_selection, Prop::Set(true)) =>
    {
      Vec::new()
    }
    Prop::Unset | Prop::Reset => vec![0],
  })
}

fn set_selection(element: &mut UiElement, mut selected: Vec<u32>, child_count: usize) {
  let UiElement::ToggleButtonGroup(value) = element else {
    return;
  };
  if selected.is_empty()
    && child_count > 0
    && !matches!(value.allow_empty_selection, Prop::Set(true))
  {
    selected.push(0);
  }
  value.selected_indices = Prop::Set(selected);
}
