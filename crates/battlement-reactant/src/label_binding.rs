//! Accessible names derived from visible labels.

use crate::{
  element_ref::{self, ElementRef},
  semantics::{AccessibleName, SemanticProps, SemanticVisibility},
};
use battlement::SemanticRole;

/// Associates visible label content with one or more controls.
///
/// Attach [`Self::reference`] and [`Self::semantic`] to the label host, then
/// pass [`Self::name`] to the control's accessibility behavior. This preserves
/// the visible label as the source of truth, including composed label children.
#[derive(Clone)]
pub struct LabelBinding {
  reference: ElementRef,
}

/// Allocates a stable label association for the current component.
pub fn use_label() -> LabelBinding {
  LabelBinding {
    reference: element_ref::use_element_ref(),
  }
}

impl LabelBinding {
  /// Returns the stable reference to attach to the visible label host.
  pub fn reference(&self) -> ElementRef {
    self.reference.clone()
  }

  /// Marks the host as text used to name another element.
  pub fn semantic(&self) -> SemanticProps {
    SemanticProps::new(SemanticRole::StaticText)
      .name(AccessibleName::Contents)
      .visibility(SemanticVisibility::NameSourceOnly)
  }

  /// Names a control using this binding’s visible label content.
  pub fn name(&self) -> AccessibleName {
    AccessibleName::LabelledBy(vec![self.reference()])
  }

  /// Combines a field label and its visible value in reading order.
  pub fn name_with(&self, value: &Self) -> AccessibleName {
    AccessibleName::LabelledBy(vec![self.reference(), value.reference()])
  }
}
