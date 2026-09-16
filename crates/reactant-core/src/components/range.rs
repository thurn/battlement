use battlement::{AccessibilityScrollAxis, AccessibilityScrollDirection, ScrollViewMode, Style};
use trox::LocalizedString;

use crate::{
  callback::{Callback, IntoCallback},
  component::Component,
  control_behavior,
  host::{ProgressBar, ScrollView, SliderHost},
  props::Missing,
  render::Render,
  semantics::{SemanticDescription, SemanticName, SemanticRange},
};

/// A controlled floating-point range input backed by Unity's native Slider.
pub struct Slider<C = Missing> {
  callback: C,
  description: Option<SemanticDescription>,
  disabled: bool,
  host: SliderHost,
  label: LocalizedString,
  maximum: f64,
  minimum: f64,
  name: SemanticName,
  step: f64,
  value: f64,
  value_text: Option<LocalizedString>,
}

/// A read-only native progress indicator with complete semantic state.
pub struct Progress {
  host: ProgressBar,
  name: LocalizedString,
  value: Option<SemanticRange>,
}

/// A native scroll view with semantic scroll actions.
pub struct ScrollArea<C = Missing> {
  axis: AccessibilityScrollAxis,
  backward: bool,
  callback: C,
  forward: bool,
  host: ScrollView,
  name: Option<LocalizedString>,
}

impl Slider<Missing> {
  /// Creates a controlled native slider.
  pub fn new(name: LocalizedString, value: f64, minimum: f64, maximum: f64, step: f64) -> Self {
    Self {
      callback: Missing,
      description: None,
      disabled: false,
      host: SliderHost::new()
        .label(name.clone())
        .low_value(minimum as f32)
        .high_value(maximum as f32)
        .value(value as f32),
      label: name.clone(),
      maximum,
      minimum,
      name: SemanticName::Text(name),
      step,
      value,
      value_text: None,
    }
  }

  /// Supplies the authoritative change callback.
  pub fn on_change<G: 'static>(self, callback: impl IntoCallback<f64, G>) -> Slider<Callback<f64>> {
    Slider {
      callback: callback.into_callback(),
      description: self.description,
      disabled: self.disabled,
      host: self.host,
      label: self.label,
      maximum: self.maximum,
      minimum: self.minimum,
      name: self.name,
      step: self.step,
      value: self.value,
      value_text: self.value_text,
    }
  }
}

impl<C> Slider<C> {
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

  /// Supplies localized text for the current numeric value.
  pub fn value_text(mut self, value_text: LocalizedString) -> Self {
    self.value_text = Some(value_text);
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

  /// Applies advanced native Slider customization.
  pub fn configure_host(mut self, configure: impl FnOnce(SliderHost) -> SliderHost) -> Self {
    self.host = configure(self.host);
    self
  }
}

impl Component for Slider<Callback<f64>> {
  fn render(&self) -> impl Render {
    let callback = self.callback.clone();
    let disabled = self.disabled;
    self
      .host
      .clone()
      .label(self.label.clone())
      .low_value(self.minimum as f32)
      .high_value(self.maximum as f32)
      .value(self.value as f32)
      .enabled(!self.disabled)
      .behavior(control_behavior::slider(
        self.name.clone(),
        self.description.clone(),
        SemanticRange {
          current: self.value,
          minimum: self.minimum,
          maximum: self.maximum,
          text: self.value_text.clone(),
        },
        self.step,
        self.disabled,
        callback.clone(),
      ))
      .on_change_value(
        callback.filter_map_input(move |value: f32| (!disabled).then_some(f64::from(value))),
      )
  }
}

impl Progress {
  /// Creates a determinate native progress indicator.
  pub fn determinate(name: LocalizedString, value: SemanticRange) -> Self {
    Self {
      host: ProgressBar::new()
        .title(name.clone())
        .low_value(value.minimum as f32)
        .high_value(value.maximum as f32)
        .value(value.current as f32),
      name,
      value: Some(value),
    }
  }

  /// Creates an indeterminate busy progress indicator.
  pub fn busy(name: LocalizedString) -> Self {
    Self {
      host: ProgressBar::new().title(name.clone()),
      name,
      value: None,
    }
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

  /// Applies advanced native ProgressBar customization.
  pub fn configure_host(mut self, configure: impl FnOnce(ProgressBar) -> ProgressBar) -> Self {
    self.host = configure(self.host);
    self
  }
}

impl Component for Progress {
  fn render(&self) -> impl Render {
    let host = self.host.clone().title(self.name.clone());
    match &self.value {
      Some(value) => host
        .low_value(value.minimum as f32)
        .high_value(value.maximum as f32)
        .value(value.current as f32)
        .semantic(control_behavior::progress(self.name.clone(), value.clone())),
      None => host.semantic(control_behavior::busy_progress(self.name.clone())),
    }
  }
}

impl ScrollArea<Missing> {
  /// Creates a native semantic scroll area.
  pub fn new(
    name: Option<LocalizedString>,
    axis: AccessibilityScrollAxis,
    can_scroll_forward: bool,
    can_scroll_backward: bool,
  ) -> Self {
    Self {
      axis,
      backward: can_scroll_backward,
      callback: Missing,
      forward: can_scroll_forward,
      host: ScrollView::new(),
      name,
    }
  }

  /// Supplies the authoritative semantic scroll callback.
  pub fn on_scroll<G: 'static>(
    self,
    callback: impl IntoCallback<AccessibilityScrollDirection, G>,
  ) -> ScrollArea<Callback<AccessibilityScrollDirection>> {
    ScrollArea {
      axis: self.axis,
      backward: self.backward,
      callback: callback.into_callback(),
      forward: self.forward,
      host: self.host,
      name: self.name,
    }
  }
}

impl<C> ScrollArea<C> {
  /// Appends logical scroll content.
  pub fn child(mut self, child: impl Render) -> Self {
    self.host = self.host.child(child);
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

  /// Applies advanced native ScrollView customization.
  pub fn configure_host(mut self, configure: impl FnOnce(ScrollView) -> ScrollView) -> Self {
    self.host = configure(self.host);
    self
  }
}

impl Component for ScrollArea<Callback<AccessibilityScrollDirection>> {
  fn render(&self) -> impl Render {
    self
      .host
      .clone()
      .mode(match self.axis {
        AccessibilityScrollAxis::Horizontal => ScrollViewMode::Horizontal,
        AccessibilityScrollAxis::Vertical => ScrollViewMode::Vertical,
      })
      .behavior(control_behavior::scroll_area(
        self.name.clone(),
        self.axis,
        self.forward,
        self.backward,
        self.callback.clone(),
      ))
  }
}
