//! The gallery’s actions and navigation entries, including keyboard behavior.

use trox::LocalizedString;

use crate::{review_navigation, review_theme};
use battlement::{Align, Color, FontStyle, Style, TextAnchor, WhiteSpace};
use battlement_reactant::prelude::{EventCallback, builder};
use battlement_reactant::{
  accessibility::{self, ButtonOptions},
  element_behavior, hooks,
};
use battlement_reactant::{
  component::Component, host::Button, motion::StyleTarget, render::Render,
};

/// Visual roles for review navigation and demonstration actions.
#[derive(Clone, Copy)]
pub enum ReviewButtonKind {
  /// A page selector, highlighted when it names the current page.
  Navigation { selected: bool },
  /// A large action inside a demonstration.
  Action,
}

impl ReviewButtonKind {
  fn active_style(self) -> StyleTarget {
    match self {
      Self::Navigation { .. } => StyleTarget::new(),
      Self::Action => StyleTarget::new()
        .background_color(Color::rgba(0.28, 0.78, 0.74, 1.0))
        .y(3.0),
    }
  }

  fn disabled_style(self) -> StyleTarget {
    match self {
      Self::Navigation { .. } => StyleTarget::new(),
      Self::Action => StyleTarget::new().opacity(0.4),
    }
  }

  fn focus_style(self) -> StyleTarget {
    match self {
      Self::Navigation { .. } => {
        StyleTarget::new().background_color(Color::rgba(0.18, 0.37, 0.38, 1.0))
      }
      Self::Action => StyleTarget::new()
        .background_color(Color::rgba(0.94, 1.0, 1.0, 1.0))
        .scale(1.02),
    }
  }

  fn hover_style(self) -> StyleTarget {
    match self {
      Self::Navigation { .. } => StyleTarget::new(),
      Self::Action => StyleTarget::new().background_color(Color::rgba(0.60, 1.0, 0.96, 1.0)),
    }
  }

  fn is_current(self) -> bool {
    matches!(self, Self::Navigation { selected: true })
  }

  fn is_selected(self) -> bool {
    match self {
      Self::Navigation { selected } => selected,
      Self::Action => true,
    }
  }

  fn style(self) -> Style {
    let style = Style::new()
      .min_height(72)
      .padding((16, 14))
      .margin((0, 0, 8, 0))
      .font_size(32)
      .white_space(WhiteSpace::Normal)
      .unity_text_align(TextAnchor::MiddleLeft)
      .border_radius(6)
      .border_width(1)
      .border_color(if self.is_selected() {
        review_theme::ACCENT
      } else {
        Color::rgb(0.15, 0.19, 0.24)
      })
      .background_color(if self.is_selected() {
        Color::rgb(0.10, 0.24, 0.27)
      } else {
        review_theme::SURFACE
      })
      .color(if self.is_selected() {
        review_theme::ACCENT
      } else {
        review_theme::TEXT
      })
      .flex_shrink(0);
    match self {
      Self::Navigation { .. } => style,
      Self::Action => style
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
    }
  }
}

/// A themed action or page selector with built-in accessible button behavior.
///
/// Callers supply intent (label, selection, disabled state, and callback). The
/// component owns the host, keyboard behavior, focus treatment, and scroll reveal.
#[builder]
pub struct ReviewButton {
  #[builder(required)]
  label: LocalizedString,
  /// Sets the stable host name used for inspection and capture discovery.
  name: String,
  /// Disables activation while retaining the control’s place in the layout.
  disabled: bool,
  /// Handles an accepted pointer, keyboard, or accessibility activation.
  #[builder(default = EventCallback::noop())]
  on_press: EventCallback<()>,
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
    let reference = element_behavior::use_scroll_reveal(
      hooks::use_context(&review_navigation::SCROLL),
      self.reveal_generation.filter(|_| self.kind.is_current()),
    );
    Button::new(self.label.clone())
      .name(self.name.clone())
      .element_ref(reference)
      .behavior(
        accessibility::use_button(
          ButtonOptions::new()
            .name(self.label.clone())
            .is_disabled(self.disabled)
            .on_press(self.on_press.clone()),
        )
        .map_semantic(|semantic| semantic.current_page(self.kind.is_current())),
      )
      .style(self.kind.style())
      .hover_style(self.kind.hover_style())
      .active_style(self.kind.active_style())
      .while_focus_visible(self.kind.focus_style())
      .disabled_style(self.kind.disabled_style())
  }
}
