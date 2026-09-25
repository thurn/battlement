//! Visible storage acknowledgement and recovery action.

use battlement::{Color, FlexDirection, Position, Style, WhiteSpace};
use reactant::{announcement, hooks, prelude::*};
use trox::tx;

use crate::settings;

pub struct SettingsSaveStatus;

impl Component for SettingsSaveStatus {
  fn render(&self) -> impl Render {
    let settings = settings::use_settings();
    let scale = settings.desired.text_size.factor();
    let announce = announcement::use_announce();
    hooks::use_effect(
      move || {
        if settings.failed {
          announce.send(tx(
            "Settings aren't saved. Changes apply for this session.",
            "Settings storage failure status.",
          ));
        } else if settings.pending {
          announce.send(tx("Saving settings…", "Settings storage pending status."));
        }
      },
      (settings.failed, settings.pending),
    );
    (settings.failed || settings.pending).then(|| {
      View::new()
        .name("settings-save-status")
        .style(
          Style::new()
            .position(Position::Absolute)
            .left(68)
            .top(if scale > 1.0 { 1090 } else { 1250 })
            .width(887)
            .min_height(56.0 * scale)
            .flex_direction(FlexDirection::Row)
            .font_size(40.0 * scale)
            .color(Color::WHITE),
        )
        .child(if settings.failed {
          Either::left((
            Text::new(tx(
              "Changes aren’t saved.",
              "Visible settings storage failure status.",
            ))
            .style(Style::new().width(640).white_space(WhiteSpace::Normal)),
            Button::new(tx("Retry", "Retry the failed storage operation."))
              .style(
                Style::new()
                  .width(230)
                  .min_height(56.0 * scale)
                  .font_size(40.0 * scale)
                  .white_space(WhiteSpace::Normal)
                  .color(Color::WHITE)
                  .background_color(Color::rgb(0.03, 0.09, 0.18))
                  .border_width(2)
                  .border_color(Color::rgb(0.0, 0.8, 1.0)),
              )
              .on_press(move || settings.retry()),
          ))
        } else {
          Either::right(Text::new(tx(
            "Saving settings…",
            "Settings storage pending status.",
          )))
        })
    })
  }
}
