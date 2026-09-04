//! Typography roles that also provide the appropriate text semantics.

use trox::LocalizedString;

use crate::review_theme;
use battlement::{Color, Style, WhiteSpace};
use battlement_reactant::prelude::builder;
use battlement_reactant::{component::Component, host::Label, motion::StyleTarget, render::Render};
use battlement_reactant::{
  control_behavior, element_behavior, focus::FocusProps, semantics::SemanticProps,
};

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

impl ReviewTextKind {
  fn is_heading(self) -> bool {
    matches!(self, Self::Heading)
  }

  fn use_semantic(self, text: &LocalizedString) -> SemanticProps {
    match (
      self,
      control_behavior::heading(text.clone(), 1),
      control_behavior::static_text_props(text.clone()),
    ) {
      (Self::Heading, heading, _) => heading,
      (_, _, text) => text,
    }
  }

  fn style(self) -> Style {
    match self {
      Self::Brand => Style::new()
        .font_size(56)
        .color(review_theme::ACCENT)
        .margin_bottom(6),
      Self::Caption => Style::new()
        .font_size(28)
        .color(review_theme::MUTED)
        .margin_bottom(24),
      Self::Eyebrow => Style::new()
        .font_size(24)
        .color(review_theme::ACCENT)
        .margin_bottom(28),
      Self::Heading => Style::new()
        .font_size(64)
        .white_space(WhiteSpace::Normal)
        .color(review_theme::TEXT)
        .margin_bottom(24),
      Self::Description => Style::new()
        .font_size(28)
        .white_space(WhiteSpace::Normal)
        .color(review_theme::MUTED)
        .margin_bottom(32),
      Self::Title => Style::new()
        .font_size(36)
        .color(review_theme::TEXT)
        .margin_bottom(24),
    }
  }
}

/// Review typography and accessible text, with automatic focus for page headings.
#[builder]
pub struct ReviewText {
  #[builder(required)]
  text: LocalizedString,
  /// Sets the stable host name used for inspection and capture discovery.
  name: String,
  /// Selects a typography role and its matching semantic behavior.
  #[builder(default = ReviewTextKind::Description)]
  kind: ReviewTextKind,
}

impl Component for ReviewText {
  fn render(&self) -> impl Render {
    let heading = element_behavior::use_focus_when(self.kind.is_heading().then_some(()));
    Label::new(self.text.clone())
      .name(self.name.clone())
      .style(self.kind.style())
      .element_ref(self.kind.is_heading().then_some(heading))
      .semantic(self.kind.use_semantic(&self.text))
      .focus_props(if self.kind.is_heading() {
        FocusProps::new().focusable(true).tab_index(-1)
      } else {
        FocusProps::new()
      })
      .while_focus_visible(StyleTarget::new().background_color(Color::rgba(0.12, 0.23, 0.28, 1.0)))
  }
}
