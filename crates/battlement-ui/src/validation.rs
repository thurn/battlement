use std::collections::HashSet;

use battlement_types::ObjectId;

mod common;
mod document;
mod element;
mod panel;

const MAXIMUM_HIERARCHY_DEPTH: usize = 256;
const MAXIMUM_IDENTITIES: usize = 100_000;
const MAXIMUM_STRING_BYTES: usize = 65_536;

/// The category of invariant violated by authored UI state.
///
/// Validation deliberately reports stable categories rather than exposing
/// implementation paths or Unity exceptions. Callers should correct the
/// authored document or panel settings before submitting them to a client.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiValidationError {
  /// A document host, document root, or element identity appears more than once.
  DuplicateObject,
  /// An identity does not resolve to the required object or relationship.
  InvalidReference,
  /// A hierarchy is too deep, too wide, or gives children to a leaf element.
  InvalidHierarchy,
  /// A property is nonfinite, out of range, duplicated, or incompatible with its mode.
  InvalidProperty,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ValidationMode {
  Complete,
  SparseUpdate,
}

impl ValidationMode {
  const fn requires_complete_state(self) -> bool {
    matches!(self, Self::Complete)
  }
}

pub use document::{validate_create_subtree, validate_documents};
pub use element::{validate_element_state, validate_element_update};
pub use panel::validate_panel_settings;

pub(crate) use common::validate_visual;
pub(crate) use element::validate_element;

fn insert_identity(
  identities: &mut HashSet<ObjectId>,
  object_id: ObjectId,
) -> Result<(), UiValidationError> {
  if object_id.as_uuid().is_nil() {
    return Err(UiValidationError::InvalidReference);
  }
  if !identities.insert(object_id) {
    return Err(UiValidationError::DuplicateObject);
  }
  if identities.len() > MAXIMUM_IDENTITIES {
    return Err(UiValidationError::InvalidHierarchy);
  }
  Ok(())
}
