use battlement::{SemanticRole, Style};
use trox::LocalizedString;

use crate::{
  callback::{Callback, IntoCallback, StoredCallback},
  component::Component,
  control_behavior,
  host::ToggleHost,
  props::Missing,
  render::Render,
  semantics::{ControlBehavior, SemanticDescription, SemanticName},
};

/// A controlled checkbox backed by Unity's native ToggleHost.
pub type Checkbox<C = Missing> = BooleanControl<CheckboxKind, C>;

/// A controlled immediate setting backed by Unity's native ToggleHost.
pub type Switch<C = Missing> = BooleanControl<SwitchKind, C>;

#[doc(hidden)]
pub struct BooleanControl<K, C> {
  callback: C,
  checked: bool,
  description: Option<SemanticDescription>,
  disabled: bool,
  host: ToggleHost,
  kind: K,
  label: LocalizedString,
  name: SemanticName,
}

#[doc(hidden)]
pub struct CheckboxKind;

#[doc(hidden)]
pub struct SwitchKind;

trait BooleanKind: 'static {
  fn behavior(
    name: SemanticName,
    description: Option<SemanticDescription>,
    checked: bool,
    disabled: bool,
    callback: Callback<bool>,
  ) -> ControlBehavior<StoredCallback>;
}

impl Checkbox<Missing> {
  /// Creates a controlled checkbox whose semantic name follows its native label.
  pub fn new(label: LocalizedString, checked: bool) -> Self {
    Self::from_label(label, checked, CheckboxKind)
  }
}

impl Switch<Missing> {
  /// Creates a controlled switch whose semantic name follows its native label.
  pub fn new(label: LocalizedString, checked: bool) -> Self {
    Self::from_label(label, checked, SwitchKind)
  }
}

impl<K> BooleanControl<K, Missing> {
  /// Supplies the authoritative Boolean change callback.
  pub fn on_change<G: 'static>(
    self,
    callback: impl IntoCallback<bool, G>,
  ) -> BooleanControl<K, Callback<bool>> {
    BooleanControl {
      callback: callback.into_callback(),
      checked: self.checked,
      description: self.description,
      disabled: self.disabled,
      host: self.host,
      kind: self.kind,
      label: self.label,
      name: self.name,
    }
  }
}

impl<K> BooleanControl<K, Missing> {
  fn from_label(label: LocalizedString, checked: bool, kind: K) -> Self {
    Self {
      callback: Missing,
      checked,
      description: None,
      disabled: false,
      host: ToggleHost::new().label(label.clone()).value(checked),
      kind,
      label: label.clone(),
      name: SemanticName::Text(label),
    }
  }
}

impl<K, C> BooleanControl<K, C> {
  /// Sets whether every change route is unavailable.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Supplies an optional semantic description.
  pub fn description(mut self, description: SemanticDescription) -> Self {
    self.description = Some(description);
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

  /// Applies advanced native ToggleHost customization.
  pub fn configure_host(mut self, configure: impl FnOnce(ToggleHost) -> ToggleHost) -> Self {
    self.host = configure(self.host);
    self
  }
}

impl<K> Component for BooleanControl<K, Callback<bool>>
where
  K: BooleanKind,
{
  fn render(&self) -> impl Render {
    let disabled = self.disabled;
    self
      .host
      .clone()
      .label(self.label.clone())
      .value(self.checked)
      .enabled(!self.disabled)
      .behavior(K::behavior(
        self.name.clone(),
        self.description.clone(),
        self.checked,
        self.disabled,
        self.callback.clone(),
      ))
      .on_change_value(
        self
          .callback
          .clone()
          .filter_map_input(move |value| (!disabled).then_some(value)),
      )
  }
}

impl BooleanKind for CheckboxKind {
  fn behavior(
    name: SemanticName,
    description: Option<SemanticDescription>,
    checked: bool,
    disabled: bool,
    callback: Callback<bool>,
  ) -> ControlBehavior<StoredCallback> {
    control_behavior::checkbox_native(name, description, checked, disabled, callback)
  }
}

impl BooleanKind for SwitchKind {
  fn behavior(
    name: SemanticName,
    description: Option<SemanticDescription>,
    checked: bool,
    disabled: bool,
    callback: Callback<bool>,
  ) -> ControlBehavior<StoredCallback> {
    let behavior = control_behavior::switch_native(name, description, checked, disabled, callback);
    assert_eq!(behavior.semantic.role, SemanticRole::Switch);
    behavior
  }
}
