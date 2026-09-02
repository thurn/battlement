use battlement::{LengthUnits, Style};
use battlement_reactant::{
  accessibility, accessibility_collections as collections,
  component::Component,
  element_ref::ElementRef,
  focus::FocusProps,
  host::{Label, View},
  render::{Node, Render},
  semantics::{self, AccessibleName},
};

use crate::review_text::{ReviewText, ReviewTextKind};

/// A labeled review region with a focusable heading and arbitrary content.
pub struct ReviewPage {
  pub eyebrow: String,
  pub title: String,
  pub description: String,
  pub heading: ElementRef,
  pub children: Node,
}

impl Component for ReviewPage {
  fn render(&self) -> impl Render {
    let mut region = collections::use_region(semantics::text(self.title.clone()));
    region.name = Some(AccessibleName::LabelledBy(vec![self.heading.clone()]));
    View::new()
      .name("page-content")
      .style(Style::new().width(100.pct()).height(100.pct()).padding(64))
      .semantic(region)
      .child((
        ReviewText::new(Label::new(self.eyebrow.clone()), ReviewTextKind::Eyebrow),
        ReviewText::new(
          Label::new(self.title.clone())
            .name("page-heading")
            .element_ref(self.heading.clone())
            .semantic(accessibility::use_heading(
              semantics::text(self.title.clone()),
              1,
            ))
            .focus_props(FocusProps::new().focusable(true).tab_index(-1)),
          ReviewTextKind::Heading,
        ),
        ReviewText::new(
          Label::new(self.description.clone())
            .name("page-description")
            .semantic(accessibility::use_static_text(semantics::text(
              self.description.clone(),
            ))),
          ReviewTextKind::Description,
        ),
        self.children.clone(),
      ))
  }
}
