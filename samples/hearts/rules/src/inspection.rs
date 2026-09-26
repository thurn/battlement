use battlement::UiFontAddress;
use reactant::{app_context, hooks, overlay::Overlay, portal::PortalTarget, prelude::*};
use trox::ls;

use crate::{
  assets,
  card_input::{self, CardInput},
  domain::Phase,
};

pub(crate) struct Inspection(pub PortalTarget);

impl Component for Inspection {
  fn render(&self) -> impl Render {
    let input = hooks::use_required_context::<CardInput>();
    let card = input.inspection().expect("inspection is open");
    let viewport = app_context::use_viewport_size();
    let close = input.clone();
    let dismiss = input.clone();
    Overlay::modal(self.0.clone(), ls("Card inspection"))
      .on_dismiss(move || dismiss.dismiss())
      .style(Style::new().background_color(Color::rgba(0.0, 0.0, 0.0, 0.18)))
      .child(
        View::new()
          .style(
            Style::new()
              .position(Position::Absolute)
              .left(24.px())
              .top((viewport.height as f32 - 150.0).px())
              .width((viewport.width as f32 - 48.0).px())
              .height(130.px())
              .padding(12.px())
              .background_color(Color::rgb(0.96, 0.92, 0.77))
              .color(Color::rgb(0.06, 0.12, 0.03))
              .unity_font_definition(UiFontAddress::from(assets::hearts::fonts::CONTROL)),
          )
          .child((
            Heading::new(
              ls(card_input::name(card.face.expect("owned inspection face"))),
              2,
            )
            .style(Style::new().font_size(24.px()).height(30.px())),
            Label::new(ls(input.reason(card.token).unwrap_or_else(
              || match input.phase() {
                Phase::Passing => "Choose three cards to pass.".into(),
                _ => "This card is available to play.".into(),
              },
            )))
            .style(Style::new().font_size(18.px()).height(26.px())),
            Button::new(ls("Close inspection"))
              .style(Style::new().width(180.px()).height(40.px()))
              .on_press(move || close.dismiss()),
          )),
      )
  }
}
