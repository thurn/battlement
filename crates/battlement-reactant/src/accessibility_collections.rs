//! Host-backed collection, link, and landmark semantics.

use battlement::{SemanticRole, SemanticState};
use trox::LocalizedString;

use crate::{
  accessibility::{self, ButtonOptions, ButtonState, ChoiceOptions},
  accessibility_hook,
  callback::IntoCallback,
  semantics::{AccessibleBehavior, AccessibleName, SemanticProps},
};

/// Returns a named single-selection listbox without input-navigation policy.
pub fn use_listbox(name: LocalizedString) -> SemanticProps {
  accessibility_hook::use_pattern("use_listbox");
  SemanticProps::new(SemanticRole::ListBox).name(AccessibleName::Text(name))
}

/// Returns an option whose selection remains application-owned.
pub fn use_option<G: 'static>(
  options: ChoiceOptions<impl IntoCallback<(), G>, impl Into<AccessibleName>>,
) -> AccessibleBehavior<G, bool> {
  let mut behavior = accessibility::use_button(
    ButtonOptions::new()
      .name(options.name)
      .is_disabled(options.is_disabled)
      .on_press(options.on_select),
  );
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
) -> AccessibleBehavior<G, ButtonState> {
  let mut behavior = accessibility::use_button(options);
  behavior.semantic.role = SemanticRole::Link;
  behavior
}

/// Returns a named table whose semantic children are rows.
pub fn use_table(name: LocalizedString) -> SemanticProps {
  accessibility_hook::use_pattern("use_table");
  SemanticProps::new(SemanticRole::Table).name(AccessibleName::Text(name))
}

/// Returns a table row without adding input focus.
pub fn use_row() -> SemanticProps {
  accessibility_hook::use_pattern("use_row");
  SemanticProps::new(SemanticRole::Row)
}

/// Returns a named data cell in its logical row.
pub fn use_cell(name: LocalizedString) -> SemanticProps {
  accessibility_hook::use_pattern("use_cell");
  SemanticProps::new(SemanticRole::Cell).name(AccessibleName::Text(name))
}

/// Returns a header scoped to its table column.
pub fn use_column_header(name: LocalizedString) -> SemanticProps {
  accessibility_hook::use_pattern("use_column_header");
  SemanticProps::new(SemanticRole::ColumnHeader).name(AccessibleName::Text(name))
}

/// Returns a header scoped to its containing row.
pub fn use_row_header(name: LocalizedString) -> SemanticProps {
  accessibility_hook::use_pattern("use_row_header");
  SemanticProps::new(SemanticRole::RowHeader).name(AccessibleName::Text(name))
}

/// Returns a named navigation landmark without adding input focus.
pub fn use_navigation(name: LocalizedString) -> SemanticProps {
  accessibility_hook::use_pattern("use_navigation");
  SemanticProps::new(SemanticRole::Navigation).name(AccessibleName::Text(name))
}

/// Returns a named content region without adding input focus.
pub fn use_region(name: LocalizedString) -> SemanticProps {
  accessibility_hook::use_pattern("use_region");
  SemanticProps::new(SemanticRole::Region).name(AccessibleName::Text(name))
}
