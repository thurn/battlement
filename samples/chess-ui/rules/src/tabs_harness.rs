use trox::{ls, tx};

use crate::{
  review_button::ReviewButton,
  settings_tabs::{SettingsTab, SettingsTabs},
};
use battlement::{Color, Style};
use battlement_reactant::{control_behavior, hooks, prelude::*};

/// Owns category selection and exposes an independent parent update.
#[builder]
pub struct TabsHarness;

impl Component for TabsHarness {
  fn render(&self) -> impl Render {
    let (active, set_active) = hooks::use_state(SettingsTab::Gameplay);
    let (changes, set_changes) = hooks::use_state(0_u32);
    View::new()
      .name("tabs-specimen")
      .style(Style::new().width(887).margin_top(48))
      .child((
        SettingsTabs::new().active_tab(active).on_select(
          set_active.callback().then(
            set_changes
              .update_callback(|count| count + 1)
              .map_input(|_| ()),
          ),
        ),
        control_behavior::static_label(ls(format!("Tab selections: {changes}"))).style(
          Style::new()
            .font_size(28)
            .color(Color::rgb(0.75, 0.86, 0.97))
            .margin_top(40),
        ),
        ReviewButton::new()
          .label(tx(
            "Select Sound from parent",
            "Settings tabs demonstration action.",
          ))
          .on_press(move || set_active.set(SettingsTab::Sound)),
      ))
  }
}
