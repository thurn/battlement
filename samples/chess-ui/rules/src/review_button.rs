//! The gallery’s actions and navigation entries, including keyboard behavior.

use std::rc::Rc;

use battlement::{Color, CurrentPage, MotionColor, Style, TextAnchor, WhiteSpace};
use battlement_reactant::{
  component::Component, host::Button, motion::MotionStyle, render::Render,
};

use battlement_reactant::{
  accessibility::{self, ButtonOptions},
  element_behavior, hooks, semantics,
};

use crate::{review_navigation, review_theme};

/// Visual roles for review navigation and demonstration actions.
#[derive(Clone, Copy)]
pub enum ReviewButtonKind {
  /// A page selector, highlighted when it names the current page.
  Navigation { selected: bool },
  /// A large action inside a demonstration.
  Action,
}

/// A themed action or page selector with built-in accessible button behavior.
///
/// Callers supply intent (label, selection, disabled state, and callback). The
/// component owns the host, keyboard behavior, focus treatment, and scroll reveal.
pub struct ReviewButton {
  label: String,
  name: String,
  disabled: bool,
  on_press: Rc<dyn Fn()>,
  reveal_generation: Option<u64>,
  kind: ReviewButtonKind,
}

impl ReviewButton {
  /// Creates an action with a visible and accessible label.
  pub fn new(label: impl Into<String>) -> Self {
    Self {
      label: label.into(),
      name: String::new(),
      disabled: false,
      on_press: Rc::new(|| {}),
      reveal_generation: None,
      kind: ReviewButtonKind::Action,
    }
  }

  /// Sets the stable host name used for inspection and capture discovery.
  pub fn name(mut self, name: impl Into<String>) -> Self {
    self.name = name.into();
    self
  }
  /// Disables activation while retaining the control’s place in the layout.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }
  /// Handles an accepted pointer, keyboard, or accessibility activation.
  pub fn on_press(mut self, callback: impl Fn() + 'static) -> Self {
    self.on_press = Rc::new(callback);
    self
  }

  /// Styles and announces this button as a page selector.
  pub fn navigation(mut self, selected: bool) -> Self {
    self.kind = ReviewButtonKind::Navigation { selected };
    self
  }

  /// Reveals the current entry in its navigation column on each new visit.
  pub fn reveal_on(mut self, generation: u64) -> Self {
    self.reveal_generation = Some(generation);
    self
  }
}

impl Component for ReviewButton {
  fn render(&self) -> impl Render {
    let selected = match self.kind {
      ReviewButtonKind::Navigation { selected } => selected,
      ReviewButtonKind::Action => true,
    };
    let current = matches!(self.kind, ReviewButtonKind::Navigation { selected: true });
    let reference = element_behavior::use_scroll_reveal(
      hooks::use_context(&review_navigation::SCROLL),
      self.reveal_generation.filter(|_| current),
    );
    let on_press = Rc::clone(&self.on_press);
    let mut behavior = accessibility::use_button(ButtonOptions {
      name: semantics::text(self.label.clone()),
      is_disabled: self.disabled,
      on_press: move || on_press(),
    });
    behavior.semantic.state.current = current.then_some(CurrentPage::Page);
    let style = Style::new()
      .min_height(48)
      .padding((12, 14))
      .margin((0, 0, 6, 0))
      .font_size(16)
      .white_space(WhiteSpace::Normal)
      .unity_text_align(TextAnchor::MiddleLeft)
      .border_radius(6)
      .border_width(1)
      .border_color(if selected {
        review_theme::ACCENT
      } else {
        Color::rgb(0.15, 0.19, 0.24)
      })
      .background_color(if selected {
        Color::rgb(0.10, 0.24, 0.27)
      } else {
        review_theme::SURFACE
      })
      .color(if selected {
        review_theme::ACCENT
      } else {
        review_theme::TEXT
      })
      .flex_shrink(0);
    Button::new(self.label.clone())
      .name(self.name.clone())
      .element_ref(reference)
      .semantic(behavior.semantic)
      .focus_props(behavior.focus)
      .interaction_props(behavior.interaction)
      .style(match self.kind {
        ReviewButtonKind::Navigation { .. } => style,
        ReviewButtonKind::Action => style.font_size(28).min_height(72).margin_top(16),
      })
      .while_focus_visible(
        MotionStyle::new().background_color(MotionColor::new(0.18, 0.37, 0.38, 1.0)),
      )
  }
}
