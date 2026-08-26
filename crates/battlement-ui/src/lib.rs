//! Author Unity UI Toolkit interfaces from a Battlement rules engine.
//!
//! This crate models a UI as a serializable [`UiDocument`] containing identified
//! [`UiNode`] values. Each node holds one concrete [`UiElement`], such as a
//! [`Box`], [`Label`], or [`Button`]. Unity creates the corresponding runtime
//! `VisualElement` hierarchy and applies [`Style`] values as inline overrides.
//!
//! Element and document builders consume and return their value, which makes
//! nested UI declarations read in the same order as the resulting visual tree:
//!
//! ```
//! use battlement_types::{Color, ObjectId};
//! use battlement_ui::{Box, Button, Label, Style, UiDocument, UiEventKind, UiNode};
//!
//! let document = UiDocument::new(ObjectId::new_v4()).child(
//!     UiNode::new(
//!         ObjectId::new_v4(),
//!         Box::new().class("dialog").style(
//!             Style::new()
//!                 .background_color(Color::rgb(0.08, 0.10, 0.14))
//!                 .padding(24.0),
//!         ),
//!     )
//!     .child(UiNode::new(ObjectId::new_v4(), Label::new("Ready?")))
//!     .child(UiNode::new(
//!         ObjectId::new_v4(),
//!         Button::new("Start").events([UiEventKind::Click]),
//!     )),
//! );
//!
//! assert!(battlement_ui::validate_documents(&[document]).is_ok());
//! ```
//!
//! The same element values also carry sparse command updates: populated fields
//! replace the corresponding live properties, while omitted fields leave them
//! unchanged. Object identities are shared by documents, nodes, commands, and
//! events, so callers should preserve them for as long as the logical UI object
//! exists.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod commands;
mod documents;
mod elements;
mod events;
/// Deterministic logical routing for native UI events.
pub mod routing;
mod validation;

pub use commands::*;
pub use documents::*;
pub use elements::*;
pub use events::*;
pub use validation::*;

/// Returns authored private-part styles for protocol adapters and fake execution.
///
/// Application code should use each control's named `<part>_style` builders.
#[doc(hidden)]
#[must_use]
pub fn authored_private_part_styles(value: &UiElement) -> Vec<&Style> {
    elements::parts::styles(value)
        .unwrap_or_default()
        .iter()
        .map(|value| &value.style)
        .collect()
}

fn is_default<T>(value: &T) -> bool
where
    T: Default + PartialEq,
{
    value == &T::default()
}
