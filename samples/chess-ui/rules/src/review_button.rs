//! The gallery’s actions and navigation entries, including keyboard behavior.

use crate::{review_navigation, review_theme};
use battlement::{Align, Color, CurrentPage, FontStyle, Style, TextAnchor, WhiteSpace};
use battlement_reactant::prelude::builder;
use battlement_reactant::{
  accessibility::{self, ButtonOptions},
  element_behavior, hooks, semantics,
};
use battlement_reactant::{
  component::Component, host::Button, motion::StyleTarget, render::Render,
};
use std::rc::Rc;

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
#[builder]
pub struct ReviewButton {
  #[builder(required)]
  label: String,
  /// Sets the stable host name used for inspection and capture discovery.
  name: String,
  /// Disables activation while retaining the control’s place in the layout.
  disabled: bool,
  /// Handles an accepted pointer, keyboard, or accessibility activation.
  #[builder(default = Rc::new(||{}))]
  on_press: Rc<dyn Fn()>,
  /// Reveals the current entry in its navigation column on each new visit.
  reveal_generation: Option<u64>,
  #[builder(default = ReviewButtonKind::Action)]
  kind: ReviewButtonKind,
}

impl ReviewButton {
  /// Styles and announces this button as a page selector.
  pub fn navigation(mut self, selected: bool) -> Self {
    self.kind = ReviewButtonKind::Navigation { selected };
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
      description: None,
      is_disabled: self.disabled,
      on_press: move || on_press(),
    });
    behavior.semantic.state.current = current.then_some(CurrentPage::Page);
    let style = Style::new()
      .min_height(72)
      .padding((16, 14))
      .margin((0, 0, 8, 0))
      .font_size(32)
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
    let button = Button::new(self.label.clone())
      .name(self.name.clone())
      .element_ref(reference)
      .behavior(behavior);
    match self.kind {
      ReviewButtonKind::Navigation { .. } => button.style(style).while_focus_visible(
        StyleTarget::new().background_color(Color::rgba(0.18, 0.37, 0.38, 1.0)),
      ),
      ReviewButtonKind::Action => button
        .style(
          style
            .align_self(Align::FlexStart)
            .font_size(36)
            .unity_font_style_and_weight(FontStyle::Bold)
            .unity_text_align(TextAnchor::MiddleCenter)
            .min_height(88)
            .padding((18, 32))
            .margin_top(20)
            .border_radius(14)
            .border_width(2)
            .border_bottom_width(6)
            .border_color(Color::rgb(0.18, 0.54, 0.52))
            .background_color(review_theme::ACCENT)
            .color(review_theme::BACKGROUND),
        )
        .hover_style(StyleTarget::new().background_color(Color::rgba(0.60, 1.0, 0.96, 1.0)))
        .active_style(
          StyleTarget::new()
            .background_color(Color::rgba(0.28, 0.78, 0.74, 1.0))
            .y(3.0),
        )
        .while_focus_visible(
          StyleTarget::new()
            .background_color(Color::rgba(0.94, 1.0, 1.0, 1.0))
            .scale(1.02),
        )
        .disabled_style(StyleTarget::new().opacity(0.4)),
    }
  }
}
