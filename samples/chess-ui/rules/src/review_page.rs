//! The title, explanation, and example content of one gallery visit.

use crate::review_text::{ReviewText, ReviewTextKind};
use battlement::{LengthUnits, Style};
use battlement_reactant::prelude::builder;
use battlement_reactant::{
  accessibility_collections,
  component::Component,
  host::View,
  render::{Fragment, Render},
  semantics,
};

/// A named review region whose heading receives focus when the page mounts.
///
/// Key this component by the visit identity to reset its example and focus.
#[builder]
#[derive(Clone)]
pub struct ReviewPage {
  /// Sets the page identifier above the heading.
  eyebrow: String,
  #[builder(required)]
  title: String,
  /// Explains what the example demonstrates.
  description: String,
  #[builder(default = Fragment::empty())]
  content: Fragment,
}

impl ReviewPage {
  /// The heading also used to label this page in gallery navigation.
  pub fn title_text(&self) -> &str {
    &self.title
  }
  /// Adds the live example below the page explanation.
  pub fn child(mut self, content: impl Render) -> Self {
    self.content = self.content.child(content);
    self
  }
}

impl Component for ReviewPage {
  fn render(&self) -> impl Render {
    View::new()
      .name("page-content")
      .style(Style::new().width(100.pct()).height(100.pct()).padding(64))
      .semantic(accessibility_collections::use_region(semantics::text(
        self.title.clone(),
      )))
      .child((
        (!self.eyebrow.is_empty()).then(|| {
          ReviewText::new()
            .text(self.eyebrow.clone())
            .kind(ReviewTextKind::Eyebrow)
        }),
        ReviewText::new()
          .text(self.title.clone())
          .name("page-heading")
          .kind(ReviewTextKind::Heading),
        (!self.description.is_empty()).then(|| {
          ReviewText::new()
            .text(self.description.clone())
            .name("page-description")
        }),
        self.content.clone(),
      ))
  }
}
