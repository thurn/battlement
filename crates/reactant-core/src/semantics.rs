//! Host-owned semantic and control declarations.

use std::{collections::BTreeSet, marker::PhantomData};

use battlement::{
  AccessibilityAction, AccessibilityActionSet, AccessibilityScrollAxis,
  AccessibilityScrollDirection, CurrentPage, SemanticRole, SemanticState,
};
use trox::LocalizedString;

use crate::{
  activation::{self, Activation},
  element_ref::ElementRef,
  event_handler::Handler,
  focus::FocusProps,
  motion::MotionProps,
};

/// A finite accessible range whose display text remains unresolved until presentation.
#[derive(Clone, Debug, PartialEq)]
pub struct SemanticRange {
  /// Current value.
  pub current: f64,
  /// Inclusive minimum.
  pub minimum: f64,
  /// Inclusive maximum.
  pub maximum: f64,
  /// Optional localized display text.
  pub text: Option<LocalizedString>,
}

/// Source of a resolved accessible name.
#[derive(Clone)]
pub enum SemanticName {
  /// Explicit localized text resolved while building the semantic snapshot.
  Text(LocalizedString),
  /// Text resolved from live hosts in the authored order.
  LabelledBy(Vec<ElementRef>),
  /// Text gathered from eligible logical descendants.
  Contents,
}

/// Source of a resolved accessible description.
#[derive(Clone)]
pub enum SemanticDescription {
  /// Explicit localized text resolved while building the semantic snapshot.
  Text(LocalizedString),
  /// Text resolved from one live host.
  DescribedBy(ElementRef),
}

/// Participation in the canonical semantic tree.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SemanticVisibility {
  /// Publish this declaration.
  #[default]
  Exposed,
  /// Permit explicit text references and contents-derived names without publishing
  /// this declaration.
  NameSourceOnly,
  /// Prune this declaration and its complete logical subtree.
  Hidden,
}

/// One host-owned semantic declaration.
#[derive(Clone)]
pub struct SemanticProps {
  /// Canonical role.
  pub role: SemanticRole,
  /// Accessible-name source.
  pub name: Option<SemanticName>,
  /// Accessible-description source.
  pub description: Option<SemanticDescription>,
  /// Canonical semantic state.
  pub state: SemanticState,
  /// Optional finite range value.
  pub value: Option<SemanticRange>,
  /// Canonical-tree participation.
  pub visibility: SemanticVisibility,
  /// Direct actions derived from interaction handlers.
  pub actions: AccessibilityActionSet,
  /// Heading level when this is a heading.
  pub heading_level: Option<u8>,
  /// Scroll axis when this is a scroll area.
  pub scroll_axis: Option<AccessibilityScrollAxis>,
  pub(crate) membership: Option<SemanticMembership>,
}

/// Ordinary interaction callbacks owned by a control behavior.
pub struct InteractionProps<G> {
  pub(crate) handlers: Vec<Handler>,
  pub(crate) activation: Option<Activation>,
  _model: PhantomData<fn(&mut G)>,
}

/// Composable semantic, focus, interaction, and styling state.
pub struct ControlBehavior<G> {
  /// Host semantic declaration.
  pub semantic: SemanticProps,
  /// Existing input-focus declarations.
  pub focus: FocusProps,
  /// Ordinary logical interaction callbacks.
  pub interaction: InteractionProps<G>,
  /// Native motion declarations owned by this behavior.
  pub motion: MotionProps,
}

/// Immediate disposition of one target-default control action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionDisposition {
  /// The target handled the action.
  Handled,
  /// The target declined the action.
  Unhandled,
}

#[derive(Clone)]
pub(crate) enum SemanticMembership {
  Radio(ElementRef),
  Tab(ElementRef),
  TabPanel(ElementRef),
}

impl<G: 'static> ControlBehavior<G> {
  /// Transforms this behavior's semantic declaration without splitting the bundle.
  #[must_use]
  pub fn map_semantic(mut self, map: impl FnOnce(SemanticProps) -> SemanticProps) -> Self {
    self.semantic = map(self.semantic);
    self
  }

  /// Binds a visible label or wrapper to this control's activation and focus.
  /// Attach `control` to the same host as this behavior's interaction props.
  /// Child activations and prevented clicks do not activate the label again.
  /// Behaviors without activation, such as sliders, receive focus only.
  #[must_use]
  pub fn label_interaction(&self, control: &ElementRef) -> InteractionProps<G> {
    self.interaction.activation.as_ref().map_or_else(
      || activation::label_focus_interaction(control, self.focus.accepts_focus()),
      |activation| activation.label_interaction(control),
    )
  }
}

impl From<LocalizedString> for SemanticName {
  fn from(value: LocalizedString) -> Self {
    Self::Text(value)
  }
}

impl From<LocalizedString> for SemanticDescription {
  fn from(value: LocalizedString) -> Self {
    Self::Text(value)
  }
}

impl SemanticProps {
  /// Starts one exposed declaration for `role`.
  #[must_use]
  pub fn new(role: SemanticRole) -> Self {
    Self {
      role,
      name: None,
      description: None,
      state: SemanticState::default(),
      value: None,
      visibility: SemanticVisibility::Exposed,
      actions: AccessibilityActionSet::default(),
      heading_level: None,
      scroll_axis: None,
      membership: None,
    }
  }

  /// Sets the accessible-name source.
  #[must_use]
  pub fn name(mut self, value: SemanticName) -> Self {
    assert!(self.name.is_none(), "duplicate accessible name declaration");
    self.name = Some(value);
    self
  }

  /// Sets the accessible-description source.
  #[must_use]
  pub fn description(mut self, value: SemanticDescription) -> Self {
    assert!(
      self.description.is_none(),
      "duplicate accessible description declaration"
    );
    self.description = Some(value);
    self
  }

  /// Sets canonical state.
  #[must_use]
  pub fn state(mut self, value: SemanticState) -> Self {
    self.state = value;
    self
  }

  /// Marks whether this button or link represents the current page.
  #[must_use]
  pub fn current_page(mut self, current: bool) -> Self {
    self.state.current = current.then_some(CurrentPage::Page);
    self
  }

  /// Sets a finite range value.
  #[must_use]
  pub fn value(mut self, value: SemanticRange) -> Self {
    assert!(self.value.is_none(), "duplicate semantic range declaration");
    self.value = Some(value);
    self
  }

  /// Sets canonical-tree participation.
  #[must_use]
  pub fn visibility(mut self, value: SemanticVisibility) -> Self {
    self.visibility = value;
    self
  }

  /// Sets the heading level.
  #[must_use]
  pub fn heading_level(mut self, value: u8) -> Self {
    self.heading_level = Some(value);
    self
  }

  /// Sets the scroll axis.
  #[must_use]
  pub fn scroll_axis(mut self, value: AccessibilityScrollAxis) -> Self {
    self.scroll_axis = Some(value);
    self
  }

  pub(crate) fn action(mut self, action: AccessibilityAction) -> Self {
    match action {
      AccessibilityAction::Activate => self.actions.activate = true,
      AccessibilityAction::Increment => self.actions.increment = true,
      AccessibilityAction::Decrement => self.actions.decrement = true,
      AccessibilityAction::Dismiss => self.actions.dismiss = true,
      AccessibilityAction::Scroll(direction) => {
        let mut directions = self.actions.scroll.iter().copied().collect::<BTreeSet<_>>();
        directions.insert(direction);
        self.actions.scroll = directions.into_iter().collect();
      }
    }
    self
  }

  pub(crate) fn membership(mut self, value: SemanticMembership) -> Self {
    self.membership = Some(value);
    self
  }
}

impl<G> Default for InteractionProps<G> {
  fn default() -> Self {
    Self {
      handlers: Vec::new(),
      activation: None,
      _model: PhantomData,
    }
  }
}

impl<G: 'static> InteractionProps<G> {
  /// Creates an empty interaction bundle.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  pub(crate) fn erase(self) -> InteractionProps<()> {
    InteractionProps {
      handlers: self.handlers,
      activation: self.activation,
      _model: PhantomData,
    }
  }
}

impl<G> Clone for InteractionProps<G> {
  fn clone(&self) -> Self {
    Self {
      handlers: self.handlers.clone(),
      activation: self.activation.clone(),
      _model: PhantomData,
    }
  }
}

impl SemanticName {
  /// Creates an explicit-text name.
  #[must_use]
  pub fn text(value: LocalizedString) -> Self {
    Self::Text(value)
  }
}

impl SemanticDescription {
  /// Creates an explicit-text description.
  #[must_use]
  pub fn text(value: LocalizedString) -> Self {
    Self::Text(value)
  }
}

pub(crate) fn to_ui_action(action: battlement::UiAccessibilityAction) -> AccessibilityAction {
  match action {
    battlement::UiAccessibilityAction::Activate => AccessibilityAction::Activate,
    battlement::UiAccessibilityAction::Increment => AccessibilityAction::Increment,
    battlement::UiAccessibilityAction::Decrement => AccessibilityAction::Decrement,
    battlement::UiAccessibilityAction::Dismiss => AccessibilityAction::Dismiss,
    battlement::UiAccessibilityAction::ScrollForward => {
      AccessibilityAction::Scroll(AccessibilityScrollDirection::Forward)
    }
    battlement::UiAccessibilityAction::ScrollBackward => {
      AccessibilityAction::Scroll(AccessibilityScrollDirection::Backward)
    }
  }
}
