//! Source-positioned heading artwork and clipped arcade stripes.

use trox::tx;

use crate::menu::action_button::ACTION_FONT;
use crate::menu::{font_scale, header_artwork};
use crate::settings::{self, Language};
use battlement::{Color, Length, PickingMode, Position, Style, TextAnchor, Translate};
use reactant::{control_behavior, element_behavior, focus::FocusProps, prelude::*};

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
  autofocus: bool,
}

impl Component for ScreenHeader {
  fn render(&self) -> impl Render {
    let font_scale = font_scale::use_font_scale();
    let language = settings::use_settings().desired.language;
    let live_heading = self.variant == HeaderVariant::Settings
      && (language == Language::French || font_scale.factor() > 1.0);
    let heading = element_behavior::use_focus_when(self.autofocus.then_some(()));
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
            (122.0 * font_scale.factor()) as i32
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
          .element_ref(self.autofocus.then_some(heading))
          .focus_props(if self.autofocus {
            FocusProps::new().focusable(true).tab_index(-1)
          } else {
            FocusProps::new()
          })
          .semantic(control_behavior::heading(
            if self.variant == HeaderVariant::Game {
              tx("Chess Chess Revolution", "Game title.")
            } else {
              tx("Settings", "Settings screen title.")
            },
            1,
          ))
          .style(
            Style::new()
              .position(Position::Absolute)
              .inset(0)
              .left(if font_scale.factor() > 1.0 { -80 } else { 0 })
              .right(if font_scale.factor() > 1.0 { -80 } else { 0 }),
          )
          .child((
            live_heading.then(|| {
              Text::new(tx("SETTINGS", "Main menu settings action.")).style(
                Style::new()
                  .full_size()
                  .font_size(94.0 * font_scale.factor())
                  .unity_font_definition(ACTION_FONT)
                  .color(Color::WHITE)
                  .unity_text_align(TextAnchor::MiddleCenter),
              )
            }),
            (!live_heading).then(|| {
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
                  .width(854.0)
                  .height(if self.variant == HeaderVariant::Game {
                    330.0
                  } else {
                    240.0
                  })
                  .translate(Translate::two_dimensional(
                    Length::Percent(-50.0),
                    Length::Percent(-50.0),
                  )),
              )
            }),
          )),
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
