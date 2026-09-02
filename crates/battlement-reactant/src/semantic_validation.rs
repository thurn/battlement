//! Developer-error validation for semantic declarations.

use battlement::{AccessibilityActionSet, CheckedState, SemanticRole};

use crate::semantics::{SemanticProps, SemanticVisibility};

pub(crate) fn validate(value: &SemanticProps, is_modal_wrapper: bool) {
  if value.visibility != SemanticVisibility::Exposed {
    assert_eq!(
      value.state,
      Default::default(),
      "non-exposed semantic declarations cannot carry state"
    );
    assert!(
      value.value.is_none(),
      "non-exposed semantic declarations cannot carry a value"
    );
    assert_eq!(
      value.actions,
      AccessibilityActionSet::default(),
      "non-exposed semantic declarations cannot carry actions"
    );
  }
  if let Some(range) = &value.value {
    assert!(range.minimum.is_finite(), "semantic minimum must be finite");
    assert!(range.maximum.is_finite(), "semantic maximum must be finite");
    assert!(
      range.current.is_finite(),
      "semantic current value must be finite"
    );
    assert!(
      range.minimum <= range.current && range.current <= range.maximum,
      "semantic range value must be within its bounds"
    );
  }
  validate_role(value);
  if value.role == SemanticRole::Dialog {
    assert!(
      is_modal_wrapper,
      "dialog semantics require the Overlay::modal wrapper host"
    );
  }
}

fn validate_role(value: &SemanticProps) {
  let actions = &value.actions;
  if value.role != SemanticRole::Heading {
    assert!(
      value.heading_level.is_none(),
      "heading level is only valid for heading semantics"
    );
  }
  if value.role != SemanticRole::ScrollArea {
    assert!(
      value.scroll_axis.is_none(),
      "scroll axis is only valid for scroll-area semantics"
    );
  }
  if !matches!(value.role, SemanticRole::Button | SemanticRole::Link) {
    assert!(
      value.state.current.is_none(),
      "current page requires a button or link"
    );
  }
  if value.state.popup.is_some() {
    assert_eq!(
      value.role,
      SemanticRole::Button,
      "popup context requires a button"
    );
    assert!(
      value.state.expanded.is_some(),
      "popup buttons require expansion state"
    );
  }
  match value.role {
    SemanticRole::Button | SemanticRole::Link => {
      if value.state.popup.is_some() {
        self::validate_disabled_only_state_except_expanded(value);
      } else {
        self::validate_disabled_only_state(value);
      }
      assert!(
        value.value.is_none(),
        "button semantics cannot expose a range"
      );
      assert_activate_only(actions);
    }
    SemanticRole::Checkbox => {
      validate_disabled_only_state_except_checked(value);
      assert!(
        value.state.checked.is_some(),
        "checkbox semantics require checked state"
      );
      assert_activate_only(actions);
    }
    SemanticRole::Switch => {
      validate_disabled_only_state_except_checked(value);
      assert!(
        matches!(
          value.state.checked,
          Some(CheckedState::False | CheckedState::True)
        ),
        "switch semantics require Boolean checked state"
      );
      assert_activate_only(actions);
    }
    SemanticRole::Radio | SemanticRole::Tab | SemanticRole::Option => {
      validate_disabled_only_state_except_selected(value);
      assert!(
        value.state.selected.is_some(),
        "choice semantics require selected state"
      );
      assert_activate_only(actions);
    }
    SemanticRole::Slider => {
      validate_disabled_only_state(value);
      assert!(value.value.is_some(), "slider semantics require a range");
      assert!(
        actions.increment && actions.decrement,
        "slider semantics require increment and decrement"
      );
      assert!(!actions.activate, "slider semantics cannot activate");
      assert!(!actions.dismiss, "slider semantics cannot dismiss");
      assert!(actions.scroll.is_empty(), "slider semantics cannot scroll");
    }
    SemanticRole::Progress => {
      validate_progress_state(value);
      assert!(
        value.state.busy != value.value.is_some(),
        "progress requires exactly one of busy or range"
      );
      assert_no_actions(actions);
    }
    SemanticRole::ScrollArea => {
      assert!(value.scroll_axis.is_some(), "scroll area requires an axis");
      assert!(value.value.is_none(), "scroll area cannot expose a range");
      assert_default_state(value);
      assert!(!actions.activate, "scroll area cannot activate");
      assert!(!actions.increment, "scroll area cannot increment");
      assert!(!actions.decrement, "scroll area cannot decrement");
      assert!(!actions.dismiss, "scroll area cannot dismiss");
    }
    SemanticRole::Disclosure => {
      validate_disabled_only_state_except_expanded(value);
      assert!(
        value.state.expanded.is_some(),
        "disclosure semantics require expanded state"
      );
      assert_activate_only(actions);
    }
    SemanticRole::Dialog => {
      assert!(value.value.is_none(), "dialog cannot expose a range");
      assert_default_state(value);
      assert!(!actions.activate, "dialog cannot activate");
      assert!(!actions.increment, "dialog cannot increment");
      assert!(!actions.decrement, "dialog cannot decrement");
      assert!(actions.scroll.is_empty(), "dialog cannot scroll");
    }
    SemanticRole::Heading => {
      assert!(
        matches!(value.heading_level, Some(1..=6)),
        "heading level must be one through six"
      );
      validate_passive(value);
    }
    SemanticRole::RadioGroup
    | SemanticRole::TabList
    | SemanticRole::TabPanel
    | SemanticRole::Image
    | SemanticRole::StaticText
    | SemanticRole::Group
    | SemanticRole::ListBox
    | SemanticRole::Table
    | SemanticRole::Row
    | SemanticRole::ColumnHeader
    | SemanticRole::RowHeader
    | SemanticRole::Cell
    | SemanticRole::Navigation
    | SemanticRole::Region => validate_passive(value),
  }
}

fn validate_disabled_only_state(value: &SemanticProps) {
  assert!(
    value.state.checked.is_none(),
    "checked state is unsupported"
  );
  assert!(
    value.state.selected.is_none(),
    "selected state is unsupported"
  );
  assert!(
    value.state.expanded.is_none(),
    "expanded state is unsupported"
  );
  assert!(!value.state.busy, "busy state is unsupported");
}

fn validate_disabled_only_state_except_checked(value: &SemanticProps) {
  assert!(
    value.state.selected.is_none(),
    "selected state is unsupported"
  );
  assert!(
    value.state.expanded.is_none(),
    "expanded state is unsupported"
  );
  assert!(!value.state.busy, "busy state is unsupported");
  assert!(
    value.value.is_none(),
    "checked controls cannot expose a range"
  );
}

fn validate_disabled_only_state_except_selected(value: &SemanticProps) {
  assert!(
    value.state.checked.is_none(),
    "checked state is unsupported"
  );
  assert!(
    value.state.expanded.is_none(),
    "expanded state is unsupported"
  );
  assert!(!value.state.busy, "busy state is unsupported");
  assert!(value.value.is_none(), "choices cannot expose a range");
}

fn validate_disabled_only_state_except_expanded(value: &SemanticProps) {
  assert!(
    value.state.checked.is_none(),
    "checked state is unsupported"
  );
  assert!(
    value.state.selected.is_none(),
    "selected state is unsupported"
  );
  assert!(!value.state.busy, "busy state is unsupported");
  assert!(value.value.is_none(), "disclosure cannot expose a range");
}

fn validate_progress_state(value: &SemanticProps) {
  assert!(!value.state.disabled, "progress cannot be disabled");
  assert!(value.state.checked.is_none(), "progress cannot be checked");
  assert!(
    value.state.selected.is_none(),
    "progress cannot be selected"
  );
  assert!(
    value.state.expanded.is_none(),
    "progress cannot be expanded"
  );
}

fn validate_passive(value: &SemanticProps) {
  assert!(
    value.value.is_none(),
    "passive semantics cannot expose a range"
  );
  assert_default_state(value);
  assert_no_actions(&value.actions);
}

fn assert_default_state(value: &SemanticProps) {
  assert_eq!(
    value.state,
    Default::default(),
    "semantic role does not support state"
  );
}

fn assert_activate_only(actions: &AccessibilityActionSet) {
  assert!(actions.activate, "interactive semantics require activate");
  assert!(
    !actions.increment,
    "activate-only semantics cannot increment"
  );
  assert!(
    !actions.decrement,
    "activate-only semantics cannot decrement"
  );
  assert!(!actions.dismiss, "activate-only semantics cannot dismiss");
  assert!(
    actions.scroll.is_empty(),
    "activate-only semantics cannot scroll"
  );
}

fn assert_no_actions(actions: &AccessibilityActionSet) {
  assert_eq!(
    actions,
    &AccessibilityActionSet::default(),
    "semantic role does not support actions"
  );
}
