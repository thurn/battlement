use battlement::{UiFontAddress, object_id};
use reactant::{hooks, portal::PortalTarget, prelude::*};
use trox::ls;

use crate::{
  assets,
  card_input::{self, CardInput},
  domain::{Phase, Seat},
  inspection::Inspection,
  layout,
  settings::Preferences,
};

pub(crate) struct CardControls(pub PortalTarget);

impl Component for CardControls {
  fn render(&self) -> impl Render {
    let input = hooks::use_required_context::<CardInput>();
    let layout = layout::use_layout();
    let preferences = hooks::use_context::<Preferences>();
    let mobile = layout.portrait;
    let portrait = mobile || preferences.larger_text;
    let font = if preferences.larger_text {
      24.0
    } else if mobile {
      20.0
    } else {
      16.0
    };
    let height = layout.footer as f32;
    let button_width = if mobile {
      ((layout.safe.width - 42.0) / 2.0) as f32
    } else if preferences.larger_text {
      205.0
    } else {
      140.0
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
                  .left((layout.safe.x as f32 + 18.0).px())
                  .top((layout.safe.y as f32 + layout.safe.height as f32 - height - 10.0).px())
                  .height(height.px())
                  .width((layout.safe.width as f32 - 36.0).px())
                  .padding_left(12.px())
                  .padding_right(12.px())
                  .background_color(if mobile {
                    Color::rgba(0.0, 0.0, 0.0, 0.0)
                  } else {
                    Color::rgba(0.96, 0.92, 0.77, 0.88)
                  })
                  .border_radius(10.px())
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
                  .picking_mode(PickingMode::Ignore)
                  .enabled(input.enabled())
                  .child(
                    Heading::new(ls(prompt), 2).style(
                      Style::new()
                        .font_size(font.px())
                        .width(if portrait { 100.pct() } else { 350.px() })
                        .height(if mobile {
                          48.px()
                        } else if portrait {
                          32.px()
                        } else {
                          44.px()
                        })
                        .white_space(WhiteSpace::Normal),
                    ),
                  ),
                Text::new(ls(description.unwrap_or_else(|| "No card selected".into()))).style(
                  Style::new()
                    .font_size(font.px())
                    .width(if mobile {
                      100.pct()
                    } else if preferences.larger_text {
                      420.px()
                    } else {
                      205.px()
                    })
                    .height(if portrait { 32.px() } else { 44.px() }),
                ),
                Text::new(ls("Swipe sideways to browse · drag up to play.")).style(
                  Style::new()
                    .display(if mobile { Display::Flex } else { Display::None })
                    .font_size(16.px())
                    .height(28.px())
                    .white_space(WhiteSpace::Normal),
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
                          .display(if mobile && !input.passing() {
                            Display::None
                          } else {
                            Display::Flex
                          })
                          .height(48.px())
                          .width(button_width.px())
                          .white_space(if mobile {
                            WhiteSpace::Normal
                          } else {
                            WhiteSpace::NoWrap
                          }),
                      )
                      .disabled(!input.can_pass())
                      .on_press(move || pass.pass()),
                    Button::new(ls("Play selected card"))
                      .style(
                        Style::new()
                          .display(if mobile && input.passing() {
                            Display::None
                          } else {
                            Display::Flex
                          })
                          .height(48.px())
                          .width(button_width.px())
                          .white_space(if mobile {
                            WhiteSpace::Normal
                          } else {
                            WhiteSpace::NoWrap
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
                          .width(button_width.px())
                          .white_space(if mobile {
                            WhiteSpace::Normal
                          } else {
                            WhiteSpace::NoWrap
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
