use battlement::{Color, Style};
use battlement_reactant::{
  component::Component,
  host::View,
  render::{Node, Render},
};

/// A bordered panel for a review page's demonstration content.
pub struct ReviewPanel {
  pub children: Node,
}

impl Component for ReviewPanel {
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
