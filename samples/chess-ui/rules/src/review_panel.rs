//! A bordered container for an example’s explanation and controls.

use std::rc::Rc;

use battlement::{Color, Style};
use battlement_reactant::{component::Component, host::View, render::Render};

/// A bordered panel for a review page's demonstration content.
pub struct ReviewPanel<R> {
  children: Rc<R>,
}

impl<R: Render> ReviewPanel<R> {
  /// Creates a themed container around typed child content.
  pub fn new(children: R) -> Self {
    Self {
      children: Rc::new(children),
    }
  }
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
