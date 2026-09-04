//! Accessible names derived from visible labels.

use crate::{
  element_ref::{self, ElementRef},
  semantics::{ControlBehavior, InteractionProps, SemanticName, SemanticProps, SemanticVisibility},
};
use battlement::SemanticRole;

/// Associates visible label content with one or more controls.
///
/// Attach [`Self::reference`] and [`Self::semantic`] to the label host, then
/// pass [`Self::name`] to the control's behavior. This preserves
/// the visible label as the source of truth, including composed label children.
#[derive(Clone)]
pub struct LabelBinding {
  reference: ElementRef,
}

/// Label and control references allocated together before behavior construction.
pub struct ControlLabel {
  label: LabelBinding,
  control: ElementRef,
}

/// Host properties for one visible label associated with a control.
#[derive(Clone)]
pub struct AssociatedLabel {
  pub(crate) reference: ElementRef,
  pub(crate) semantic: SemanticProps,
  pub(crate) interaction: InteractionProps<()>,
}

/// Host properties for one behavior attached to its stable control element.
pub struct AssociatedControl<G> {
  pub(crate) reference: ElementRef,
  pub(crate) behavior: ControlBehavior<G>,
}

/// Allocates a stable label association for the current component.
pub fn use_label() -> LabelBinding {
  LabelBinding {
    reference: element_ref::use_element_ref(),
  }
}

/// Allocates the visible-label and control references for one associated control.
pub fn use_control_label() -> ControlLabel {
  ControlLabel {
    label: use_label(),
    control: element_ref::use_element_ref(),
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
      .name(SemanticName::Contents)
      .visibility(SemanticVisibility::NameSourceOnly)
  }

  /// Names a control using this binding’s visible label content.
  pub fn name(&self) -> SemanticName {
    SemanticName::LabelledBy(vec![self.reference()])
  }

  /// Combines a field label and its visible value in reading order.
  pub fn name_with(&self, value: &Self) -> SemanticName {
    SemanticName::LabelledBy(vec![self.reference(), value.reference()])
  }
}

impl ControlLabel {
  /// Names the control from the eventual visible label host.
  pub fn name(&self) -> SemanticName {
    self.label.name()
  }

  /// Names the control from its visible label followed by another text source.
  pub fn name_with(&self, value: &LabelBinding) -> SemanticName {
    self.label.name_with(value)
  }

  /// Binds behavior to the allocated control and returns atomic host properties.
  pub fn bind<G: 'static>(
    self,
    behavior: ControlBehavior<G>,
  ) -> (AssociatedLabel, AssociatedControl<G>) {
    let interaction = behavior.label_interaction(&self.control).erase();
    (
      AssociatedLabel {
        reference: self.label.reference(),
        semantic: self.label.semantic(),
        interaction,
      },
      AssociatedControl {
        reference: self.control,
        behavior,
      },
    )
  }

  /// Builds and binds one behavior from this label's accessible name.
  pub fn bind_with<G: 'static>(
    self,
    behavior: impl FnOnce(SemanticName) -> ControlBehavior<G>,
  ) -> (AssociatedLabel, AssociatedControl<G>) {
    let name = self.name();
    self.bind(behavior(name))
  }
}
