//! Host-backed collection, link, and landmark semantics.

use battlement::{SemanticRole, SemanticState};

use crate::{
  accessibility::{self, ButtonOptions, ChoiceOptions, PressState},
  callback::IntoCallback,
  semantics::{AccessibleBehavior, AccessibleName, LocalizedText, SemanticProps},
};

/// Returns a named single-selection listbox without input-navigation policy.
pub fn use_listbox(name: LocalizedText) -> SemanticProps {
  SemanticProps::new(SemanticRole::ListBox).name(AccessibleName::Text(name))
}

/// Returns an option whose selection remains application-owned.
pub fn use_option<G: 'static>(
  options: ChoiceOptions<impl IntoCallback<(), G>, impl Into<AccessibleName>>,
) -> AccessibleBehavior<G, bool> {
  let mut behavior = accessibility::use_button(ButtonOptions {
    name: options.name,
    description: None,
    is_disabled: options.is_disabled,
    on_press: options.on_select,
  });
  behavior.semantic.role = SemanticRole::Option;
  behavior.semantic.state = SemanticState {
    disabled: options.is_disabled,
    selected: Some(options.selected),
    ..SemanticState::default()
  };
  AccessibleBehavior {
    semantic: behavior.semantic,
    focus: behavior.focus,
    interaction: behavior.interaction,
    motion: behavior.motion,
    state: options.selected,
  }
}

/// Returns a link; the application callback owns the external-URL request.
pub fn use_link<G: 'static>(
  options: ButtonOptions<impl IntoCallback<(), G>, impl Into<AccessibleName>>,
) -> AccessibleBehavior<G, PressState> {
  let mut behavior = accessibility::use_button(options);
  behavior.semantic.role = SemanticRole::Link;
  behavior
}

/// Returns a named table whose semantic children are rows.
pub fn use_table(name: LocalizedText) -> SemanticProps {
  SemanticProps::new(SemanticRole::Table).name(AccessibleName::Text(name))
}

/// Returns a table row without adding input focus.
pub fn use_row() -> SemanticProps {
  SemanticProps::new(SemanticRole::Row)
}

/// Returns a named data cell in its logical row.
pub fn use_cell(name: LocalizedText) -> SemanticProps {
  SemanticProps::new(SemanticRole::Cell).name(AccessibleName::Text(name))
}

/// Returns a header scoped to its table column.
pub fn use_column_header(name: LocalizedText) -> SemanticProps {
  SemanticProps::new(SemanticRole::ColumnHeader).name(AccessibleName::Text(name))
}

/// Returns a header scoped to its containing row.
pub fn use_row_header(name: LocalizedText) -> SemanticProps {
  SemanticProps::new(SemanticRole::RowHeader).name(AccessibleName::Text(name))
}

/// Returns a named navigation landmark without adding input focus.
pub fn use_navigation(name: LocalizedText) -> SemanticProps {
  SemanticProps::new(SemanticRole::Navigation).name(AccessibleName::Text(name))
}

/// Returns a named content region without adding input focus.
pub fn use_region(name: LocalizedText) -> SemanticProps {
  SemanticProps::new(SemanticRole::Region).name(AccessibleName::Text(name))
}
