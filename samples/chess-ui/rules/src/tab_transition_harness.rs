//! Resettable directional settings-tab transition specimen.

use battlement::{Align, Color, FlexDirection, Style, TextAnchor};
use battlement_reactant::{hooks, prelude::*};
use trox::{ls, tx};

use crate::{
  arcade_tab_transition::ArcadeTabTransition,
  review_button::ReviewButton,
  select_control::VALUE_FONT,
  settings_panel::SettingsPanel,
  settings_tabs::{SettingsTab, SettingsTabs},
};

/// Owns tab direction, reduced-motion policy, and reset state for Page 27.
#[builder]
pub struct TabTransitionHarness;

impl Component for TabTransitionHarness {
  fn render(&self) -> impl Render {
    let (active, set_active) = hooks::use_state(SettingsTab::Gameplay);
    let (direction, set_direction) = hooks::use_state(1_i32);
    let (reduce_motion, set_reduce_motion) = hooks::use_state(false);
    let (animate_transition, set_animate_transition) = hooks::use_state(false);
    let (reset_generation, set_reset_generation) = hooks::use_state(0_u32);
    View::new()
      .name("tab-transition-harness")
      .style(Style::new().width(887).margin_top(24))
      .child((
        Flex::new()
          .direction(FlexDirection::Row)
          .gap(16.0)
          .style(Style::new().height(64).align_items(Align::Center))
          .child((
            ReviewButton::new()
              .label(if reduce_motion {
                tx("REDUCED MOTION", "Tab animation motion policy.")
              } else {
                tx("FULL MOTION", "Tab animation motion policy.")
              })
              .name("tab-transition-motion-policy")
              .on_press(set_reduce_motion.update_callback(|value| !value)),
            ReviewButton::new()
              .label(tx("RESET", "Reset tab animation specimen."))
              .name("tab-transition-reset")
              .on_press(
                set_active
                  .callback()
                  .map_input(|_| SettingsTab::Gameplay)
                  .then(set_direction.callback().map_input(|_| 1))
                  .then(set_reduce_motion.callback().map_input(|_| false))
                  .then(set_animate_transition.callback().map_input(|_| false))
                  .then(
                    set_reset_generation.update_callback(|generation| generation.wrapping_add(1)),
                  ),
              ),
          )),
        SettingsTabs::new()
          .active_tab(active)
          .on_select(EventCallback::new({
            let set_active = set_active.clone();
            let set_direction = set_direction.clone();
            let set_animate_transition = set_animate_transition.clone();
            move |next: SettingsTab| {
              if next == active {
                return;
              }
              set_direction.set(if next.index() > active.index() { 1 } else { -1 });
              set_animate_transition.set(true);
              set_active.set(next);
            }
          })),
        SettingsPanel::new().children(
          ArcadeTabTransition::new()
            .active_key(active)
            .children(TabContent::new().tab(active))
            .direction(direction)
            .reduce_motion(reduce_motion)
            .show_effects(animate_transition)
            .key(reset_generation),
        ),
      ))
  }
}

#[builder]
struct TabContent {
  #[builder(required)]
  tab: SettingsTab,
}

impl Component for TabContent {
  fn render(&self) -> impl Render {
    View::new()
      .name(format!("tab-content-{}", self.tab.slug()))
      .style(Style::new().full_size().center_content())
      .child(
        Text::new(ls(format!(
          "{} SETTINGS",
          self.tab.label_text().to_uppercase()
        )))
        .style(
          Style::new()
            .color(Color::hex(0xeef8ff))
            .unity_font_definition(VALUE_FONT)
            .font_size(72)
            .letter_spacing(3)
            .unity_text_align(TextAnchor::MiddleCenter),
        ),
      )
  }
}
