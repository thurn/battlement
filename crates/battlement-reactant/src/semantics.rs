//! Host-owned accessibility declarations.

use std::{collections::BTreeSet, marker::PhantomData};

use battlement::{
  AccessibilityAction, AccessibilityActionSet, AccessibilityRangeValue, AccessibilityScrollAxis,
  AccessibilityScrollDirection, SemanticRole, SemanticState,
};

use crate::{element_ref::ElementRef, event_handler::Handler, focus::FocusProps};

/// Already-localized application text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalizedText(String);

/// Source of a resolved accessible name.
#[derive(Clone)]
pub enum AccessibleName {
  /// Explicit already-localized text.
  Text(LocalizedText),
  /// Text resolved from one live host.
  LabelledBy(ElementRef),
  /// Text gathered from eligible logical descendants.
  Contents,
}

/// Source of a resolved accessible description.
#[derive(Clone)]
pub enum AccessibleDescription {
  /// Explicit already-localized text.
  Text(LocalizedText),
  /// Text resolved from one live host.
  DescribedBy(ElementRef),
}

/// Participation in the canonical semantic tree.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SemanticVisibility {
  /// Publish this declaration.
  #[default]
  Exposed,
  /// Permit explicit name references without publishing this declaration.
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
  pub name: Option<AccessibleName>,
  /// Accessible-description source.
  pub description: Option<AccessibleDescription>,
  /// Canonical semantic state.
  pub state: SemanticState,
  /// Optional finite range value.
  pub value: Option<AccessibilityRangeValue>,
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

/// Ordinary interaction callbacks returned by an accessible behavior hook.
pub struct InteractionProps<G> {
  pub(crate) handlers: Vec<Handler>,
  _model: PhantomData<fn(&mut G)>,
}

/// Composable semantic, focus, interaction, and styling state.
pub struct AccessibleBehavior<G, S> {
  /// Host semantic declaration.
  pub semantic: SemanticProps,
  /// Existing input-focus declarations.
  pub focus: FocusProps,
  /// Ordinary logical interaction callbacks.
  pub interaction: InteractionProps<G>,
  /// Pattern styling state.
  pub state: S,
}

/// Immediate disposition of one target-default accessibility action.
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

impl LocalizedText {
  /// Creates already-localized application text.
  #[must_use]
  pub fn new(value: impl Into<String>) -> Self {
    Self(value.into())
  }

  pub(crate) fn resolved(&self) -> String {
    self.0.split_whitespace().collect::<Vec<_>>().join(" ")
  }
}

impl From<&str> for LocalizedText {
  fn from(value: &str) -> Self {
    Self::new(value)
  }
}

impl From<String> for LocalizedText {
  fn from(value: String) -> Self {
    Self::new(value)
  }
}

/// Creates already-localized semantic text.
#[must_use]
pub fn text(value: impl Into<String>) -> LocalizedText {
  LocalizedText::new(value)
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
  pub fn name(mut self, value: AccessibleName) -> Self {
    assert!(self.name.is_none(), "duplicate accessible name declaration");
    self.name = Some(value);
    self
  }

  /// Sets the accessible-description source.
  #[must_use]
  pub fn description(mut self, value: AccessibleDescription) -> Self {
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

  /// Sets a finite range value.
  #[must_use]
  pub fn value(mut self, value: AccessibilityRangeValue) -> Self {
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

  pub(crate) fn accessibility(
    mut self,
    slot: &'static str,
    callback: impl Fn(&mut G, AccessibilityAction) -> ActionDisposition + 'static,
  ) -> Self {
    self.handlers.push(Handler::accessibility(slot, callback));
    self
  }
}

impl<G> Clone for InteractionProps<G> {
  fn clone(&self) -> Self {
    Self {
      handlers: self.handlers.clone(),
      _model: PhantomData,
    }
  }
}

impl AccessibleName {
  /// Creates an explicit-text name.
  #[must_use]
  pub fn text(value: impl Into<LocalizedText>) -> Self {
    Self::Text(value.into())
  }
}

impl AccessibleDescription {
  /// Creates an explicit-text description.
  #[must_use]
  pub fn text(value: impl Into<LocalizedText>) -> Self {
    Self::Text(value.into())
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
