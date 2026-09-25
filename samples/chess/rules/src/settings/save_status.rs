//! Visible storage acknowledgement and recovery action.

use battlement::{Color, FlexDirection, Position, Style};
use reactant::{announcement, hooks, prelude::*};
use trox::tx;

use crate::settings;

pub struct SettingsSaveStatus;

impl Component for SettingsSaveStatus {
  fn render(&self) -> impl Render {
    let settings = settings::use_settings();
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
            .top(1480)
            .width(887)
            .height(56)
            .flex_direction(FlexDirection::Row)
            .font_size(40)
            .color(Color::WHITE),
        )
        .child(if settings.failed {
          Either::left((
            Text::new(tx(
              "Changes aren’t saved.",
              "Visible settings storage failure status.",
            ))
            .style(Style::new().width(640)),
            Button::new(tx("Retry", "Retry saving current settings."))
              .style(
                Style::new()
                  .width(230)
                  .height(56)
                  .font_size(40)
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
