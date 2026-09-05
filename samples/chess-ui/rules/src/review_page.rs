//! The title, explanation, and example content of one gallery visit.

use trox::LocalizedString;

use crate::review_text::{ReviewText, ReviewTextKind};
use battlement::Style;
use battlement_reactant::prelude::builder;
use battlement_reactant::{
  component::Component,
  components::Region,
  render::{Fragment, Render},
};

/// A named review region whose heading receives focus when the page mounts.
///
/// Key this component by the visit identity to reset its example and focus.
#[builder]
#[derive(Clone)]
pub struct ReviewPage {
  /// Sets the page identifier above the heading.
  eyebrow: Option<LocalizedString>,
  #[builder(required)]
  title: LocalizedString,
  /// Explains what the example demonstrates.
  description: Option<LocalizedString>,
  #[builder(default = Fragment::empty())]
  content: Fragment,
  /// Overrides the review region layout.
  style: Style,
}

impl ReviewPage {
  /// The heading also used to label this page in gallery navigation.
  pub fn title_text(&self) -> LocalizedString {
    self.title.clone()
  }
  /// Adds the live example below the page explanation.
  pub fn child(mut self, content: impl Render) -> Self {
    self.content = self.content.child(content);
    self
  }
}

impl Component for ReviewPage {
  fn render(&self) -> impl Render {
    Region::new(self.title.clone())
      .host_name("page-content")
      .style(
        Style::new()
          .full_size()
          .padding(64)
          .merge(self.style.clone()),
      )
      .child((
        self.eyebrow.as_ref().map(|eyebrow| {
          ReviewText::new()
            .text(eyebrow.clone())
            .kind(ReviewTextKind::Eyebrow)
        }),
        ReviewText::new()
          .text(self.title.clone())
          .name("page-heading")
          .kind(ReviewTextKind::Heading),
        self.description.as_ref().map(|description| {
          ReviewText::new()
            .text(description.clone())
            .name("page-description")
        }),
        self.content.clone(),
      ))
  }
}
