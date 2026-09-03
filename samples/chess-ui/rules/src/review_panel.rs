//! A bordered container for an example’s explanation and controls.

use battlement::{Color, Style};
use battlement_reactant::prelude::builder;
use battlement_reactant::{component::Component, host::View, render::Render};
use std::rc::Rc;

/// A bordered panel for a review page's demonstration content.
#[builder]
pub struct ReviewPanel<R> {
  #[builder(required, into)]
  children: Rc<R>,
}

impl<R: Render> Component for ReviewPanel<R> {
  fn render(&self) -> impl Render {
    View::new()
      .style(
        Style::new()
          .margin_top(64)
          .padding(40)
          .border_width(1)
          .border_color(Color::rgb(0.19, 0.26, 0.33))
          .border_radius(12),
      )
      .child(self.children.clone())
  }
}
