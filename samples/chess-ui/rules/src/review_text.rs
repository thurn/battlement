use battlement::{MotionColor, Style, WhiteSpace};
use battlement_reactant::{component::Component, host::Label, motion::MotionStyle, render::Render};

use crate::review_theme;

/// Typography roles within a review surface.
#[derive(Clone, Copy)]
pub enum ReviewTextKind {
  Brand,
  Caption,
  Eyebrow,
  Heading,
  Description,
  Title,
}

/// Applies review typography while preserving a label's semantics and references.
pub struct ReviewText {
  pub label: Label,
  pub kind: ReviewTextKind,
}

impl ReviewText {
  pub fn new(label: Label, kind: ReviewTextKind) -> Self {
    Self { label, kind }
  }
}

impl Component for ReviewText {
  fn render(&self) -> impl Render {
    let style = match self.kind {
      ReviewTextKind::Brand => Style::new()
        .font_size(28)
        .color(review_theme::ACCENT)
        .margin_bottom(6),
      ReviewTextKind::Caption => Style::new()
        .font_size(14)
        .color(review_theme::MUTED)
        .margin_bottom(24),
      ReviewTextKind::Eyebrow => Style::new()
        .font_size(24)
        .color(review_theme::ACCENT)
        .margin_bottom(28),
      ReviewTextKind::Heading => Style::new()
        .font_size(64)
        .white_space(WhiteSpace::Normal)
        .color(review_theme::TEXT)
        .margin_bottom(24),
      ReviewTextKind::Description => Style::new()
        .font_size(28)
        .white_space(WhiteSpace::Normal)
        .color(review_theme::MUTED)
        .margin_bottom(32),
      ReviewTextKind::Title => Style::new()
        .font_size(36)
        .color(review_theme::TEXT)
        .margin_bottom(24),
    };
    let label = self.label.clone().style(style);
    if matches!(self.kind, ReviewTextKind::Heading) {
      label.while_focus_visible(
        MotionStyle::new().background_color(MotionColor::new(0.12, 0.23, 0.28, 1.0)),
      )
    } else {
      label
    }
  }
}
