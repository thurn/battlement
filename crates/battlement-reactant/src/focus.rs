//! Composable focus declarations for Reactant hosts.

use battlement::{Prop, UiVisualElement};

/// Persistent focus declarations that can be composed by behavior hooks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FocusProps {
  focusable: Prop<bool>,
  tab_index: Prop<i32>,
  delegates_focus: Prop<bool>,
  auto_focus: Prop<bool>,
  inert: Prop<bool>,
}

impl FocusProps {
  /// Creates an empty focus declaration bundle.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// Sets whether the host may receive focus.
  #[must_use]
  pub fn focusable(mut self, value: bool) -> Self {
    assign(&mut self.focusable, Prop::Set(value), "focusable");
    self
  }

  /// Sets the host's position in Unity's sequential focus ring.
  #[must_use]
  pub fn tab_index(mut self, value: i32) -> Self {
    assign(&mut self.tab_index, Prop::Set(value), "tab_index");
    self
  }

  /// Sets whether focus requested here delegates to a descendant.
  #[must_use]
  pub fn delegates_focus(mut self, value: bool) -> Self {
    assign(
      &mut self.delegates_focus,
      Prop::Set(value),
      "delegates_focus",
    );
    self
  }

  /// Requests focus once when this keyed host is mounted.
  #[must_use]
  pub fn auto_focus(mut self, value: bool) -> Self {
    assign(&mut self.auto_focus, Prop::Set(value), "auto_focus");
    self
  }

  /// Sets whether this logical subtree is unavailable to user interaction.
  #[must_use]
  pub fn inert(mut self, value: bool) -> Self {
    assign(&mut self.inert, Prop::Set(value), "inert");
    self
  }

  pub(crate) fn apply(self, target: &mut UiVisualElement) {
    assign(&mut target.focusable, self.focusable, "focusable");
    assign(&mut target.tab_index, self.tab_index, "tab_index");
    assign(
      &mut target.delegates_focus,
      self.delegates_focus,
      "delegates_focus",
    );
    assign(&mut target.auto_focus, self.auto_focus, "auto_focus");
    assign(&mut target.inert, self.inert, "inert");
  }

  pub(crate) fn accepts_focus(&self) -> bool {
    matches!(self.focusable, Prop::Set(true))
  }
}

fn assign<T: PartialEq>(target: &mut Prop<T>, value: Prop<T>, property: &str) {
  if matches!(value, Prop::Unset) {
    return;
  }
  assert!(
    matches!(target, Prop::Unset) || *target == value,
    "conflicting Reactant FocusProps assignment for {property}"
  );
  *target = value;
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn bundle_matches_direct_visual_properties() {
    let direct = UiVisualElement::new()
      .focusable(true)
      .tab_index(4)
      .delegates_focus(true)
      .auto_focus(true)
      .inert(false);
    let mut bundled = UiVisualElement::new();
    FocusProps::new()
      .focusable(true)
      .tab_index(4)
      .delegates_focus(true)
      .auto_focus(true)
      .inert(false)
      .apply(&mut bundled);

    assert_eq!(bundled, direct);
  }

  #[test]
  #[should_panic(expected = "conflicting Reactant FocusProps assignment")]
  fn conflicting_bundle_panics() {
    let mut visual = UiVisualElement::new().focusable(false);
    FocusProps::new().focusable(true).apply(&mut visual);
  }
}
