use battlement::{UiFontAddress, object_id};
use reactant::{app_context, hooks, portal::PortalTarget, prelude::*};
use trox::ls;

use crate::{
  assets,
  card_input::{self, CardInput},
  domain::{Phase, Seat},
  inspection::Inspection,
  settings::Preferences,
};

pub(crate) struct CardControls(pub PortalTarget);

impl Component for CardControls {
  fn render(&self) -> impl Render {
    let input = hooks::use_required_context::<CardInput>();
    let viewport = app_context::use_viewport_size();
    let preferences = hooks::use_context::<Preferences>();
    let portrait = viewport.width < viewport.height || preferences.larger_text;
    let font = if preferences.larger_text { 24.0 } else { 16.0 };
    let height = if preferences.larger_text {
      142.0
    } else if portrait {
      106.0
    } else {
      52.0
    };
    let selected = input.selected_cards().last().copied();
    let description = selected
      .and_then(|token| input.card(token))
      .and_then(|card| card.face)
      .map(|card| format!("Selected: {}", card_input::name(card)));
    let prompt = match input.phase() {
      Phase::Passing => format!(
        "Choose three to pass · {} / 3",
        input.selected_cards().len()
      ),
      Phase::Playing { turn: Seat::South } => selected
        .and_then(|token| input.reason(token))
        .unwrap_or_else(|| "Your turn · select a card, then Play or drag to the center.".into()),
      Phase::Playing { turn } => format!("Waiting for {turn:?}…"),
      Phase::HandOver => "Hand complete.".into(),
      Phase::MatchOver { .. } => "Match complete.".into(),
    };
    let pass = input.clone();
    let play = input.clone();
    let inspect = input.clone();
    let cancel = input.clone();
    Stack::new()
      .on_navigation_cancel(move |_| cancel.cancel())
      .picking_mode(PickingMode::Ignore)
      .style(
        Style::new()
          .position(Position::Absolute)
          .left(0)
          .top(0)
          .width(100.pct())
          .height(100.pct()),
      )
      .child((
        View::new()
          .picking_mode(PickingMode::Ignore)
          .style(Style::new().width(100.pct()).height(100.pct()))
          .child(
            View::new()
              .picking_mode(PickingMode::Ignore)
              .style(
                Style::new()
                  .position(Position::Absolute)
                  .left(18.px())
                  .top((viewport.height as f32 - height - 10.0).px())
                  .height(height.px())
                  .width((viewport.width as f32 - 36.0).px())
                  .flex_direction(if portrait {
                    FlexDirection::Column
                  } else {
                    FlexDirection::Row
                  })
                  .unity_font_definition(UiFontAddress::from(assets::hearts::fonts::CONTROL))
                  .color(Color::rgb(0.06, 0.12, 0.03)),
              )
              .child((
                View::new()
                  .id(object_id!("6644ed66-12dc-4590-9af8-19d174a47014").into())
                  .enabled(input.enabled())
                  .child(
                    Heading::new(ls(prompt), 2).style(
                      Style::new()
                        .font_size(font.px())
                        .width(if portrait { 100.pct() } else { 350.px() })
                        .height(if portrait { 32.px() } else { 44.px() }),
                    ),
                  ),
                Text::new(ls(description.unwrap_or_else(|| "No card selected".into()))).style(
                  Style::new()
                    .font_size(font.px())
                    .width(if preferences.larger_text {
                      420.px()
                    } else {
                      205.px()
                    })
                    .height(if portrait { 32.px() } else { 44.px() }),
                ),
                View::new()
                  .picking_mode(PickingMode::Ignore)
                  .style(
                    Style::new()
                      .flex_direction(FlexDirection::Row)
                      .height(52.px())
                      .font_size(font.px()),
                  )
                  .child((
                    Button::new(ls("Pass three cards"))
                      .style(
                        Style::new()
                          .height(48.px())
                          .width(if preferences.larger_text {
                            205.px()
                          } else {
                            140.px()
                          }),
                      )
                      .disabled(!input.can_pass())
                      .on_press(move || pass.pass()),
                    Button::new(ls("Play selected card"))
                      .style(
                        Style::new()
                          .height(48.px())
                          .width(if preferences.larger_text {
                            205.px()
                          } else {
                            140.px()
                          }),
                      )
                      .disabled(!selected.is_some_and(|token| input.can_play(token)))
                      .on_press(move || {
                        if let Some(token) = selected {
                          play.play(token);
                        }
                      }),
                    Button::new(ls("Inspect selected card"))
                      .style(
                        Style::new()
                          .height(48.px())
                          .width(if preferences.larger_text {
                            205.px()
                          } else {
                            140.px()
                          }),
                      )
                      .disabled(selected.is_none())
                      .on_press(move || inspect.inspect()),
                  )),
              )),
          ),
        input.inspection().map(|_| Inspection(self.0.clone())),
      ))
  }
}
