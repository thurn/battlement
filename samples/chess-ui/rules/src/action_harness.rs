use crate::{action_button::ActionButton, return_button::ReturnButton};
use battlement::{Color, FlexDirection, Position, Style};
use battlement_reactant::prelude::builder;
use battlement_reactant::{
  accessibility,
  component::Component,
  hooks,
  host::{Flex, TextElement, View},
  render::Render,
};

/// Counts action and Return presses, including a disabled action.
#[builder]
pub struct ActionHarness;

impl Component for ActionHarness {
  fn render(&self) -> impl Render {
    let (clicks, set_clicks) = hooks::use_state(0_u32);
    let (returns, set_returns) = hooks::use_state(0_u32);
    View::new()
      .style(
        Style::new()
          .position(Position::Absolute)
          .left(0)
          .top(0)
          .width(1024)
          .height(1536),
      )
      .child((
        View::new()
          .name("action-specimen")
          .style(
            Style::new()
              .position(Position::Absolute)
              .left(132)
              .top(450)
              .width(760),
          )
          .child((
            self::slot(
              ActionButton::new()
                .children(self::text("PLAY"))
                .on_click(set_clicks.update_callback(|count| count + 1)),
            ),
            self::slot(
              ActionButton::new()
                .children(
                  Flex::new()
                    .direction(FlexDirection::Row)
                    .gap(14.0)
                    .child((self::text("COMPOSED"), self::text("LABEL"))),
                )
                .on_click(set_clicks.update_callback(|count| count + 1)),
            ),
            self::slot(ActionButton::new().children(self::text("ABOUT"))),
            self::slot(
              ActionButton::new()
                .children(self::text("DISABLED"))
                .disabled(true)
                .on_click(move || set_clicks.update(|count| count + 1)),
            ),
            self::status(format!("Action clicks: {clicks}")),
            self::status(format!("Return clicks: {returns}")),
          )),
        ReturnButton::new().on_click(move || set_returns.update(|count| count + 1)),
      ))
  }
}

fn slot(button: ActionButton) -> impl Render {
  View::new()
    .style(Style::new().width(760).height(140).margin_bottom(28))
    .child(button)
}

fn status(value: String) -> impl Render {
  accessibility::static_label(value).style(
    Style::new()
      .font_size(28)
      .color(Color::rgb(0.75, 0.86, 0.97))
      .margin_top(16),
  )
}

fn text(value: &str) -> TextElement {
  accessibility::name_source_text(value)
}
