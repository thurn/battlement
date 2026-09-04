//! Controlled popup triggers using ordinary button activation.

use battlement::PopupKind;
use battlement_builder::builder;

use crate::{
  accessibility::{self, ButtonOptions, ButtonState},
  callback::IntoCallback,
  semantics::{AccessibleBehavior, AccessibleDescription, AccessibleName, LocalizedText},
};

/// Options for a button that controls a popup.
#[builder(support = crate::builder_support)]
pub struct PopupButtonOptions<F, N = LocalizedText> {
  /// Accessible name, independent of popup and expansion context.
  #[builder(required)]
  pub name: N,
  /// Optional accessible description.
  pub description: Option<AccessibleDescription>,
  /// Kind of popup opened by this trigger.
  #[builder(required)]
  pub popup: PopupKind,
  /// Whether the popup is currently open.
  #[builder(required)]
  pub expanded: bool,
  /// Whether activation is unavailable.
  pub is_disabled: bool,
  /// Ordinary press callback; the parent owns expansion state.
  #[builder(required)]
  pub on_press: F,
}

/// Returns canonical button semantics with popup context and unified activation.
/// Declaring a popup does not create one or change its controlled expansion state.
pub fn use_popup_button<G: 'static>(
  options: PopupButtonOptions<impl IntoCallback<(), G>, impl Into<AccessibleName>>,
) -> AccessibleBehavior<G, ButtonState> {
  let mut behavior = accessibility::use_button(
    ButtonOptions::new()
      .name(options.name)
      .description(options.description)
      .is_disabled(options.is_disabled)
      .on_press(options.on_press),
  );
  behavior.semantic.state.popup = Some(options.popup);
  behavior.semantic.state.expanded = Some(options.expanded);
  behavior
}
