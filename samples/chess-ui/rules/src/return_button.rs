//! A fixed-position Return action with accessible button behavior.

use trox::tx;

use crate::{action_button::ActionButton, action_skin};
use battlement::{Position, Style};
use battlement_reactant::prelude::{EventCallback, builder};
use battlement_reactant::{
  component::Component,
  control_behavior,
  host::View,
  paint::{PaintFill, PaintStyle},
  render::Render,
};

/// The fixed portrait-stage Return control; its parent owns navigation.
#[builder]
pub struct ReturnButton {
  /// Disables activation while retaining the control’s place in the layout.
  disabled: bool,
  #[builder(required)]
  on_press: EventCallback<()>,
}

impl Component for ReturnButton {
  fn render(&self) -> impl Render {
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
        View::decorative()
          .style(Style::new().absolute_fill())
          .paint(
            PaintStyle::new()
              .background(PaintFill::Color(action_skin::INTERIOR))
              .clip_polygon(action_skin::clip(18.0, 17.0)),
          ),
        ActionButton::new()
          .children(control_behavior::name_source_text(tx(
            "RETURN",
            "Return button section heading.",
          )))
          .max_text_scale(1.35)
          .disabled(self.disabled)
          .on_press(self.on_press.clone()),
      ))
  }
}
