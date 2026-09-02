use battlement::{Color, MotionColor, Style, TextAnchor, WhiteSpace};
use battlement_reactant::{
  component::Component, host::Button, motion::MotionStyle, render::Render,
};

use crate::review_theme;

/// Visual roles for review navigation and demonstration actions.
#[derive(Clone, Copy)]
pub enum ReviewButtonKind {
  Navigation { selected: bool },
  Action,
}

/// Styles a button without replacing its input behavior or accessibility.
pub struct ReviewButton {
  pub button: Button,
  pub kind: ReviewButtonKind,
}

impl ReviewButton {
  pub fn new(button: Button, kind: ReviewButtonKind) -> Self {
    Self { button, kind }
  }
}

impl Component for ReviewButton {
  fn render(&self) -> impl Render {
    let selected = match self.kind {
      ReviewButtonKind::Navigation { selected } => selected,
      ReviewButtonKind::Action => true,
    };
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
    self
      .button
      .clone()
      .style(match self.kind {
        ReviewButtonKind::Navigation { .. } => style,
        ReviewButtonKind::Action => style.font_size(28).min_height(72).margin_top(16),
      })
      .while_focus_visible(
        MotionStyle::new().background_color(MotionColor::new(0.18, 0.37, 0.38, 1.0)),
      )
  }
}
