//! Source-positioned heading artwork and clipped arcade stripes.

use trox::tx;

use crate::{
  font_scale::{self, FontScaleRole},
  header_artwork,
};
use battlement::{Length, PickingMode, Position, Style, Translate};
use battlement_reactant::{control_behavior, prelude::*};

/// Selects the fixed decorative heading.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum HeaderVariant {
  #[default]
  Game,
  Settings,
}

/// A native semantic heading with prepared lettering and stripe artwork.
#[builder]
pub struct ScreenHeader {
  #[builder(required)]
  variant: HeaderVariant,
}

impl Component for ScreenHeader {
  fn render(&self) -> impl Render {
    let font_scale = font_scale::use_font_scale();
    View::new()
      .name("screen-header")
      .picking_mode(PickingMode::Ignore)
      .style(
        Style::new()
          .position(Position::Absolute)
          .left(84)
          .top(if self.variant == HeaderVariant::Game {
            103
          } else {
            74
          })
          .width(854)
          .height(if self.variant == HeaderVariant::Game {
            330
          } else {
            122
          }),
      )
      .child((
        StripeBar::new()
          .left(true)
          .top(if self.variant == HeaderVariant::Game {
            132.0
          } else {
            44.0
          }),
        StripeBar::new()
          .left(false)
          .top(if self.variant == HeaderVariant::Game {
            132.0
          } else {
            44.0
          }),
        View::new()
          .name("screen-header-heading")
          .semantic(control_behavior::heading(
            if self.variant == HeaderVariant::Game {
              tx("Chess Chess Revolution", "Game title.")
            } else {
              tx("Settings", "Settings screen title.")
            },
            1,
          ))
          .style(Style::new().position(Position::Absolute).inset(0))
          .child(
            (if self.variant == HeaderVariant::Game {
              header_artwork::GAME_LOGO
            } else {
              header_artwork::SETTINGS_TITLE
            })
            .image()
            .name("screen-header-artwork")
            .picking_mode(PickingMode::Ignore)
            .style(
              Style::new()
                .position(Position::Absolute)
                .left(Length::Percent(50.0))
                .top(if self.variant == HeaderVariant::Game {
                  165
                } else {
                  62
                })
                .width(854.0 * font_scale.dynamic(FontScaleRole::Heading))
                .height(
                  (if self.variant == HeaderVariant::Game {
                    330.0
                  } else {
                    240.0
                  }) * font_scale.dynamic(FontScaleRole::Heading),
                )
                .translate(Translate::two_dimensional(
                  Length::Percent(-50.0),
                  Length::Percent(-50.0),
                )),
            ),
          ),
      ))
  }
}

#[builder]
struct StripeBar {
  #[builder(required)]
  left: bool,
  #[builder(required)]
  top: f32,
}

impl Component for StripeBar {
  fn render(&self) -> impl Render {
    (if self.left {
      header_artwork::STRIPE_LEFT
    } else {
      header_artwork::STRIPE_RIGHT
    })
    .image()
    .name(if self.left {
      "header-stripe-left"
    } else {
      "header-stripe-right"
    })
    .picking_mode(PickingMode::Ignore)
    .style(
      Style::new()
        .position(Position::Absolute)
        .left(if self.left { 0 } else { 540 })
        .top(self.top)
        .width(314)
        .height(58),
    )
  }
}
