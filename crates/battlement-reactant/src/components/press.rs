use std::hash::Hash;
use trox::ls;

use battlement::{
  CurrentPage, GridItem, IconSource, PopupKind, Prop, SemanticRole, StackItem, Style,
};
use trox::LocalizedString;

use crate::{
  callback::{Callback, IntoCallback},
  component::Component,
  control_behavior,
  element_ref::ElementRef,
  host::ButtonHost,
  layout::Layout,
  motion::{InitialValue, MotionTarget, Transition},
  motion_css::{Decoration, IntoPseudoStyle, StyleTransition},
  paint::PaintStyle,
  props::Missing,
  render::{Children, Render},
  semantics::{SemanticDescription, SemanticName, SemanticProps},
};

/// A complete pressable control backed by Unity's native button host.
pub type Button<P = Missing, N = SemanticName> = PressControl<ButtonKind, P, N>;

/// A button that declares controlled popup state.
pub type PopupButton<P = Missing, N = SemanticName> = PressControl<PopupKindMarker, P, N>;

/// A pressable navigation target with link semantics.
pub type Link<P = Missing, N = SemanticName> = PressControl<LinkKind, P, N>;

/// A button that declares controlled expanded state.
pub type Disclosure<P = Missing, N = SemanticName> = PressControl<DisclosureKind, P, N>;

/// A controlled option inside a [`crate::components::ListBox`].
pub type ListBoxOption<P = Missing, N = SemanticName> = PressControl<OptionKind, P, N>;

#[doc(hidden)]
pub struct PressControl<K, P, N> {
  callback: P,
  description: Option<SemanticDescription>,
  disabled: bool,
  host: ButtonHost,
  kind: K,
  label: LocalizedString,
  semantic_name: N,
}

#[doc(hidden)]
pub struct ButtonKind {
  current: bool,
}

#[doc(hidden)]
pub struct PopupKindMarker {
  expanded: bool,
  popup: PopupKind,
}

#[doc(hidden)]
pub struct LinkKind {
  current: bool,
}

#[doc(hidden)]
pub struct DisclosureKind {
  expanded: bool,
}

#[doc(hidden)]
pub struct OptionKind {
  selected: bool,
}

trait PressSemantics: 'static {
  fn apply(&self, semantic: &mut SemanticProps);
}

impl Button<Missing, SemanticName> {
  /// Creates a native text button whose semantic name follows its visible label.
  pub fn new(label: LocalizedString) -> Self {
    Self::from_label(label, ButtonKind { current: false })
  }
}

impl Button<Missing, Missing> {
  /// Creates a composed-content button that requires an explicit semantic name.
  pub fn content(children: impl Into<Children>) -> Self {
    Self::from_content(children, ButtonKind { current: false })
  }
}

impl PopupButton<Missing, SemanticName> {
  /// Creates a native text button that controls a popup.
  pub fn new(label: LocalizedString, popup: PopupKind, expanded: bool) -> Self {
    Self::from_label(label, PopupKindMarker { expanded, popup })
  }
}

impl PopupButton<Missing, Missing> {
  /// Creates a composed-content popup button requiring an explicit semantic name.
  pub fn content(children: impl Into<Children>, popup: PopupKind, expanded: bool) -> Self {
    Self::from_content(children, PopupKindMarker { expanded, popup })
  }
}

impl Link<Missing, SemanticName> {
  /// Creates a pressable navigation target.
  pub fn new(label: LocalizedString) -> Self {
    Self::from_label(label, LinkKind { current: false })
  }
}

impl Disclosure<Missing, SemanticName> {
  /// Creates a disclosure trigger with controlled expanded state.
  pub fn new(label: LocalizedString, expanded: bool) -> Self {
    Self::from_label(label, DisclosureKind { expanded })
  }
}

impl ListBoxOption<Missing, SemanticName> {
  /// Creates a controlled option inside a list box.
  pub fn new(label: LocalizedString, selected: bool) -> Self {
    Self::from_label(label, OptionKind { selected })
  }
}

impl<K, N> PressControl<K, Missing, N> {
  /// Supplies the authoritative press callback.
  pub fn on_press<G: 'static>(
    self,
    callback: impl IntoCallback<(), G>,
  ) -> PressControl<K, Callback<()>, N> {
    PressControl {
      callback: callback.into_callback(),
      description: self.description,
      disabled: self.disabled,
      host: self.host,
      kind: self.kind,
      label: self.label,
      semantic_name: self.semantic_name,
    }
  }
}

impl<K, P> PressControl<K, P, Missing> {
  /// Supplies the required semantic name for composed content.
  pub fn semantic_name(
    self,
    semantic_name: impl Into<SemanticName>,
  ) -> PressControl<K, P, SemanticName> {
    PressControl {
      callback: self.callback,
      description: self.description,
      disabled: self.disabled,
      host: self.host,
      kind: self.kind,
      label: self.label,
      semantic_name: semantic_name.into(),
    }
  }
}

impl<P, N> PressControl<ButtonKind, P, N> {
  /// Marks whether this button represents the current page.
  pub fn current_page(mut self, current: bool) -> Self {
    self.kind.current = current;
    self
  }
}

impl<P, N> PressControl<LinkKind, P, N> {
  /// Marks whether this link represents the current page.
  pub fn current_page(mut self, current: bool) -> Self {
    self.kind.current = current;
    self
  }
}

impl<K> PressControl<K, Missing, SemanticName> {
  fn from_label(label: LocalizedString, kind: K) -> Self {
    Self {
      callback: Missing,
      description: None,
      disabled: false,
      host: ButtonHost::new(label.clone()),
      kind,
      label: label.clone(),
      semantic_name: SemanticName::Text(label),
    }
  }
}

impl<K> PressControl<K, Missing, Missing> {
  fn from_content(children: impl Into<Children>, kind: K) -> Self {
    Self {
      callback: Missing,
      description: None,
      disabled: false,
      host: ButtonHost::new(ls("")).child(children.into().render()),
      kind,
      label: ls(""),
      semantic_name: Missing,
    }
  }
}

impl<K, P, N> PressControl<K, P, N> {
  /// Sets whether every activation route is unavailable.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Supplies an optional semantic description.
  pub fn description(mut self, description: impl Into<SemanticDescription>) -> Self {
    self.description = Some(description.into());
    self
  }

  /// Sets the Unity query and USS selector name on the native host.
  pub fn host_name(mut self, name: impl Into<String>) -> Self {
    self.host = self.host.name(name.into());
    self
  }

  /// Replaces the native host's inline style.
  pub fn style(mut self, style: Style) -> Self {
    self.host = self.host.style(style);
    self
  }

  /// Appends one USS class to the native host.
  pub fn class(mut self, class: impl Into<String>) -> Self {
    self.host = self.host.class(class);
    self
  }

  /// Attaches one stable element reference to the native host.
  pub fn element_ref(mut self, element_ref: ElementRef) -> Self {
    self.host = self.host.element_ref(element_ref);
    self
  }

  /// Assigns stable sibling identity to the native host.
  pub fn key<T: Clone + Eq + Hash + 'static>(mut self, key: T) -> Self {
    self.host = self.host.key(key);
    self
  }

  /// Sets the native grid item placement.
  pub fn grid_item(mut self, item: impl Into<Prop<GridItem>>) -> Self {
    self.host = self.host.grid_item(item);
    self
  }

  /// Sets the native stack item placement.
  pub fn stack_item(mut self, item: impl Into<Prop<StackItem>>) -> Self {
    self.host = self.host.stack_item(item);
    self
  }

  /// Selects the host's layout participation policy.
  pub fn layout(mut self, layout: Layout) -> Self {
    self.host = self.host.layout(layout);
    self
  }

  /// Appends logical child content.
  pub fn child(mut self, child: impl Render) -> Self {
    self.host = self.host.child(child);
    self
  }

  /// Applies a static clipped paint treatment.
  pub fn paint(mut self, paint: PaintStyle) -> Self {
    self.host = self.host.paint(paint);
    self
  }

  /// Sets or resets the native button icon.
  pub fn icon(mut self, icon: impl Into<Prop<IconSource>>) -> Self {
    self.host = self.host.icon(icon);
    self
  }

  /// Replaces inline declarations on the native icon part.
  pub fn icon_style(mut self, style: Style) -> Self {
    self.host = self.host.icon_style(style);
    self
  }

  /// Applies the native hover pseudo-style.
  pub fn hover_style(mut self, target: impl IntoPseudoStyle) -> Self {
    self.host = self.host.hover_style(target);
    self
  }

  /// Applies the native active pseudo-style.
  pub fn active_style(mut self, target: impl IntoPseudoStyle) -> Self {
    self.host = self.host.active_style(target);
    self
  }

  /// Applies the native disabled pseudo-style.
  pub fn disabled_style(mut self, target: impl IntoPseudoStyle) -> Self {
    self.host = self.host.disabled_style(target);
    self
  }

  /// Applies a target while keyboard- or controller-visible focus is active.
  pub fn while_focus_visible(mut self, target: impl Into<MotionTarget>) -> Self {
    self.host = self.host.while_focus_visible(target);
    self
  }

  /// Applies the native exact-focus pseudo-style.
  pub fn focus_style(mut self, target: impl IntoPseudoStyle) -> Self {
    self.host = self.host.focus_style(target);
    self
  }

  /// Replaces native pseudo-style transitions.
  pub fn style_transition(mut self, transition: StyleTransition) -> Self {
    self.host = self.host.style_transition(transition);
    self
  }

  /// Sets the initial motion target.
  pub fn initial(mut self, target: impl InitialValue) -> Self {
    self.host = self.host.initial(target);
    self
  }

  /// Sets the animated motion target.
  pub fn animate(mut self, target: impl Into<MotionTarget>) -> Self {
    self.host = self.host.animate(target);
    self
  }

  /// Sets the exit motion target.
  pub fn exit(mut self, target: impl Into<MotionTarget>) -> Self {
    self.host = self.host.exit(target);
    self
  }

  /// Replaces the native motion transition.
  pub fn transition(mut self, transition: Transition) -> Self {
    self.host = self.host.transition(transition);
    self
  }

  /// Replaces all native after decorations.
  pub fn after_all(mut self, decorations: impl IntoIterator<Item = Decoration>) -> Self {
    self.host = self.host.after_all(decorations);
    self
  }

  /// Applies additional raw-host customization for advanced presentation needs.
  pub fn configure_host(mut self, configure: impl FnOnce(ButtonHost) -> ButtonHost) -> Self {
    self.host = configure(self.host);
    self
  }
}

impl<K> Component for PressControl<K, Callback<()>, SemanticName>
where
  K: PressSemantics,
{
  fn render(&self) -> impl Render {
    let behavior = control_behavior::button(
      self.semantic_name.clone(),
      self.description.clone(),
      self.disabled,
      self.callback.clone(),
    );
    let behavior = behavior.map_semantic(|mut semantic| {
      self.kind.apply(&mut semantic);
      semantic
    });
    self
      .host
      .clone()
      .text(self.label.clone())
      .enabled(!self.disabled)
      .behavior(behavior)
  }
}

impl PressSemantics for ButtonKind {
  fn apply(&self, semantic: &mut SemanticProps) {
    semantic.state.current = self.current.then_some(CurrentPage::Page);
  }
}

impl PressSemantics for PopupKindMarker {
  fn apply(&self, semantic: &mut SemanticProps) {
    semantic.state.popup = Some(self.popup);
    semantic.state.expanded = Some(self.expanded);
  }
}

impl PressSemantics for LinkKind {
  fn apply(&self, semantic: &mut SemanticProps) {
    semantic.role = SemanticRole::Link;
    semantic.state.current = self.current.then_some(CurrentPage::Page);
  }
}

impl PressSemantics for DisclosureKind {
  fn apply(&self, semantic: &mut SemanticProps) {
    semantic.role = SemanticRole::Disclosure;
    semantic.state.expanded = Some(self.expanded);
  }
}

impl PressSemantics for OptionKind {
  fn apply(&self, semantic: &mut SemanticProps) {
    semantic.role = SemanticRole::Option;
    semantic.state.selected = Some(self.selected);
  }
}
