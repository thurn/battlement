//! A fixed-position Return action with accessible button behavior.

use crate::{action_button::ActionButton, action_skin, frame_styles};
use battlement::{PickingMode, Position, Style};
use battlement_reactant::prelude::builder;
use battlement_reactant::{
  accessibility,
  component::Component,
  host::{TextElement, View},
  paint::{PaintFill, PaintStyle},
  render::Render,
  semantics::{self, SemanticVisibility},
};
use std::rc::Rc;

/// The fixed portrait-stage Return control; its parent owns navigation.
#[builder]
pub struct ReturnButton {
  /// Disables activation while retaining the control’s place in the layout.
  disabled: bool,
  #[builder(required)]
  on_click: Rc<dyn Fn()>,
}

impl Component for ReturnButton {
  fn render(&self) -> impl Render {
    let on_click = Rc::clone(&self.on_click);
    View::new()
      .name("return-button")
      .style(
        Style::new()
          .position(Position::Absolute)
          .left(328)
          .top(1358)
          .width(368)
          .height(120),
      )
      .child((
        View::new()
          .picking_mode(PickingMode::Ignore)
          .style(frame_styles::cover())
          .paint(
            PaintStyle::new()
              .background(PaintFill::Color(action_skin::INTERIOR))
              .clip_polygon(action_skin::clip(18.0, 17.0)),
          ),
        ActionButton::new()
          .children(
            TextElement::new("RETURN").semantic(
              accessibility::use_static_text(semantics::text("RETURN"))
                .visibility(SemanticVisibility::NameSourceOnly),
            ),
          )
          .max_text_scale(1.35)
          .disabled(self.disabled)
          .on_click(move || on_click()),
      ))
  }
}
