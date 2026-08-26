use battlement_ui::UiElement;

use crate::UiWorldError;

pub(crate) fn validate_state(value: &UiElement, child_count: usize) -> Result<(), UiWorldError> {
    if let UiElement::RadioButtonGroup(group) = value {
        let choice_count = group.choices.as_ref().map_or(0, Vec::len);
        if group
            .selected_index
            .is_some_and(|index| index as usize >= choice_count)
        {
            return Err(UiWorldError::InvalidProperty);
        }
    }
    if let UiElement::ToggleButtonGroup(group) = value {
        let default_selected = [0];
        let selected = match group.selected_indices.as_deref() {
            Some(values) => values,
            None if child_count == 0 || group.allow_empty_selection == Some(true) => &[],
            None => &default_selected,
        };
        let invalid_indices = selected.iter().any(|index| *index as usize >= child_count)
            || selected.windows(2).any(|pair| pair[0] >= pair[1]);
        let invalid_cardinality = group.multiple_selection != Some(true) && selected.len() > 1;
        let invalid_empty =
            child_count > 0 && group.allow_empty_selection != Some(true) && selected.is_empty();
        if invalid_indices || invalid_cardinality || invalid_empty {
            return Err(UiWorldError::InvalidProperty);
        }
    }
    if let UiElement::DropdownField(field) = value {
        let choices = field.choices.as_deref().unwrap_or_default();
        let selection = field
            .selection
            .clone()
            .unwrap_or_else(battlement_ui::Choice::none);
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
    Some(value.selected_indices.clone().unwrap_or_else(|| {
        if child_count == 0 || value.allow_empty_selection == Some(true) {
            Vec::new()
        } else {
            vec![0]
        }
    }))
}

fn set_selection(element: &mut UiElement, mut selected: Vec<u32>, child_count: usize) {
    let UiElement::ToggleButtonGroup(value) = element else {
        return;
    };
    if selected.is_empty() && child_count > 0 && value.allow_empty_selection != Some(true) {
        selected.push(0);
    }
    value.selected_indices = Some(selected);
}
