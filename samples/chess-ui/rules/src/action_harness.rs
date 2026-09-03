use battlement::{Color, FlexDirection, Position, Style};
use battlement_reactant::{
  accessibility,
  component::Component,
  hooks,
  host::{Flex, Label, TextElement, View},
  render::Render,
  semantics::{self, SemanticVisibility},
};

use crate::{action_button::ActionButton, return_button::ReturnButton};

/// Counts action and Return presses, including a disabled action.
pub struct ActionHarness;

impl Component for ActionHarness {
  fn render(&self) -> impl Render {
    let (clicks, set_clicks) = hooks::use_state(0_u32);
    let (returns, set_returns) = hooks::use_state(0_u32);
    let play = set_clicks.clone();
    let disabled = set_clicks.clone();
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
              ActionButton::new(self::text("PLAY")).on_click(move || play.update(|n| n + 1)),
            ),
            self::slot(
              ActionButton::new(
                Flex::new()
                  .direction(FlexDirection::Row)
                  .gap(14.0)
                  .child((self::text("COMPOSED"), self::text("LABEL"))),
              )
              .on_click(move || set_clicks.update(|n| n + 1)),
            ),
            self::slot(ActionButton::new(self::text("ABOUT"))),
            self::slot(
              ActionButton::new(self::text("DISABLED"))
                .disabled(true)
                .on_click(move || disabled.update(|n| n + 1)),
            ),
            self::status(format!("Action clicks: {clicks}")),
            self::status(format!("Return clicks: {returns}")),
          )),
        ReturnButton::new(move || set_returns.update(|n| n + 1)),
      ))
  }
}

fn slot<R: Render>(button: ActionButton<R>) -> impl Render {
  View::new()
    .style(Style::new().width(760).height(140).margin_bottom(28))
    .child(button)
}

fn status(value: String) -> impl Render {
  Label::new(value.clone())
    .semantic(accessibility::use_static_text(semantics::text(value)))
    .style(
      Style::new()
        .font_size(28)
        .color(Color::rgb(0.75, 0.86, 0.97))
        .margin_top(16),
    )
}

fn text(value: &str) -> TextElement {
  TextElement::new(value).semantic(
    accessibility::use_static_text(semantics::text(value))
      .visibility(SemanticVisibility::NameSourceOnly),
  )
}
