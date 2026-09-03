//! Typography roles that also provide the appropriate text semantics.

use battlement::{MotionColor, Style, WhiteSpace};
use battlement_reactant::{component::Component, host::Label, motion::MotionStyle, render::Render};

use battlement_reactant::{accessibility, element_behavior, focus::FocusProps, semantics};

use crate::review_theme;

/// Typography roles within a review surface.
#[derive(Clone, Copy)]
pub enum ReviewTextKind {
  /// Application identity above the navigation.
  Brand,
  /// Small supporting navigation text.
  Caption,
  /// Page number above the main heading.
  Eyebrow,
  /// Level-one page heading, focused when mounted.
  Heading,
  /// Wrapping explanatory body text.
  Description,
  /// Emphasized text inside an example.
  Title,
}

/// Review typography and accessible text, with automatic focus for page headings.
pub struct ReviewText {
  text: String,
  name: String,
  kind: ReviewTextKind,
}

impl ReviewText {
  /// Creates accessible description text in the gallery theme.
  pub fn new(text: impl Into<String>) -> Self {
    Self {
      text: text.into(),
      name: String::new(),
      kind: ReviewTextKind::Description,
    }
  }
  /// Selects a typography role and its matching accessibility behavior.
  pub fn kind(mut self, kind: ReviewTextKind) -> Self {
    self.kind = kind;
    self
  }
  /// Sets the stable host name used for inspection and capture discovery.
  pub fn name(mut self, name: impl Into<String>) -> Self {
    self.name = name.into();
    self
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
    let label = Label::new(self.text.clone())
      .name(self.name.clone())
      .style(style);
    let heading =
      element_behavior::use_focus_when(matches!(self.kind, ReviewTextKind::Heading).then_some(()));
    if matches!(self.kind, ReviewTextKind::Heading) {
      label
        .element_ref(heading)
        .semantic(accessibility::use_heading(
          semantics::text(self.text.clone()),
          1,
        ))
        .focus_props(FocusProps::new().focusable(true).tab_index(-1))
        .while_focus_visible(
          MotionStyle::new().background_color(MotionColor::new(0.12, 0.23, 0.28, 1.0)),
        )
    } else {
      label.semantic(accessibility::use_static_text(semantics::text(
        self.text.clone(),
      )))
    }
  }
}
