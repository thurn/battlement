//! Deterministic review surface for control release effects.

use trox::{ls, tx};

use crate::{
  action_button::{ActionButton, ActionLabel},
  select_control::SelectControl,
  settings_tabs::{SettingsTab, SettingsTabs},
  toggle_control::ToggleControl,
  volume_control::VolumeControl,
};
use battlement::{Align, Color, FlexDirection, Position, Style, TextAnchor};
use battlement_reactant::{
  control_behavior, hooks,
  motion_config::{MotionConfig, ReducedMotion},
  portal::PortalTarget,
  prelude::*,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Specimen {
  #[default]
  Action,
  Tabs,
  Select,
  Checkbox,
  Slider,
}

/// Exercises every source control-effect family in one resettable stage.
#[builder]
pub struct EffectsHarness {
  overlay: Option<PortalTarget>,
}

impl Component for EffectsHarness {
  fn render(&self) -> impl Render {
    let (specimen, set_specimen) = hooks::use_state(Specimen::Action);
    let (shine, set_shine) = hooks::use_state(false);
    let (reduced_motion, set_reduced_motion) = hooks::use_state(false);
    let (checked, set_checked) = hooks::use_state(false);
    let (volume, set_volume) = hooks::use_state(55_u32);
    let (tab, set_tab) = hooks::use_state(SettingsTab::Gameplay);
    let (selection, set_selection) = hooks::use_state(String::from("Borderless"));
    let (reset_generation, set_reset_generation) = hooks::use_state(0_u32);
    MotionConfig::new(
      View::new()
        .name("control-effects-specimen")
        .style(
          Style::new()
            .position(Position::Absolute)
            .left(0)
            .top(0)
            .width(1024)
            .height(1536),
        )
        .child((
          Flex::new()
            .direction(FlexDirection::Row)
            .gap(8.0)
            .name("effect-family-selector")
            .style(
              Style::new()
                .position(Position::Absolute)
                .left(54)
                .top(380)
                .width(916)
                .height(68),
            )
            .child(
              [
                (Specimen::Action, "ACTION"),
                (Specimen::Tabs, "TABS"),
                (Specimen::Select, "SELECT"),
                (Specimen::Checkbox, "CHECKBOX"),
                (Specimen::Slider, "SLIDER"),
              ]
              .map(|(value, label)| {
                let set_specimen = set_specimen.clone();
                Button::new(ls(label))
                  .host_name(format!("effect-family-{}", label.to_ascii_lowercase()))
                  .style(self::selector_style(specimen == value, 174.0))
                  .on_press(move || set_specimen.set(value))
              }),
            ),
          Flex::new()
            .direction(FlexDirection::Row)
            .gap(10.0)
            .name("effect-options")
            .style(
              Style::new()
                .position(Position::Absolute)
                .left(190)
                .top(468)
                .width(644)
                .height(64),
            )
            .child(
              Button::new(ls(if shine { "SHINE ON" } else { "SHINE OFF" }))
                .host_name("effect-shine")
                .style(self::selector_style(shine, 184.0))
                .on_press(set_shine.update_callback(|value| !value)),
            )
            .child(
              Button::new(ls(if reduced_motion {
                "REDUCED"
              } else {
                "FULL MOTION"
              }))
              .host_name("effect-reduced-motion")
              .style(self::selector_style(reduced_motion, 210.0))
              .on_press(set_reduced_motion.update_callback(|value| !value)),
            )
            .child(
              Button::new(ls("RESET"))
                .host_name("effect-reset")
                .style(self::selector_style(false, 184.0))
                .on_press(EventCallback::new({
                  let set_checked = set_checked.clone();
                  let set_volume = set_volume.clone();
                  let set_tab = set_tab.clone();
                  let set_selection = set_selection.clone();
                  move |()| {
                    set_specimen.set(Specimen::Action);
                    set_shine.set(false);
                    set_reduced_motion.set(false);
                    set_checked.set(false);
                    set_volume.set(55);
                    set_tab.set(SettingsTab::Gameplay);
                    set_selection.set(String::from("Borderless"));
                    set_reset_generation.update(|generation| generation.wrapping_add(1));
                  }
                })),
            ),
          View::new()
            .key(reset_generation)
            .style(
              Style::new()
                .position(Position::Absolute)
                .left(68)
                .top(620)
                .width(887)
                .height(720)
                .align_items(Align::Center),
            )
            .child(self::specimen(
              specimen,
              shine,
              checked,
              set_checked,
              volume,
              set_volume,
              tab,
              set_tab,
              selection,
              set_selection,
              self.overlay.clone(),
            )),
        )),
    )
    .reduced_motion(if reduced_motion {
      ReducedMotion::Always
    } else {
      ReducedMotion::Never
    })
  }
}

#[allow(clippy::too_many_arguments)]
fn specimen(
  specimen: Specimen,
  shine: bool,
  checked: bool,
  set_checked: hooks::StateSetter<bool>,
  volume: u32,
  set_volume: hooks::StateSetter<u32>,
  tab: SettingsTab,
  set_tab: hooks::StateSetter<SettingsTab>,
  selection: String,
  set_selection: hooks::StateSetter<String>,
  overlay: Option<PortalTarget>,
) -> Node {
  match specimen {
    Specimen::Action => Node::new(
      View::new()
        .style(Style::new().width(760).height(140))
        .child(
          ActionButton::new()
            .artwork(ActionLabel::Play)
            .children(control_behavior::name_source_text(tx(
              "PLAY",
              "Control effects action label.",
            )))
            .shine_active(shine),
        ),
    ),
    Specimen::Tabs => Node::new(
      View::new()
        .style(Style::new().width(887).height(160))
        .child(SettingsTabs::new().active_tab(tab).on_select(set_tab)),
    ),
    Specimen::Select => Node::new(
      View::new().style(Style::new().width(839)).child(
        SelectControl::new()
          .label(control_behavior::name_source_text(tx(
            "Display Mode",
            "Control effects selector label.",
          )))
          .value(selection)
          .options(vec![
            String::from("Borderless"),
            String::from("Fullscreen"),
            String::from("Windowed"),
          ])
          .overlay(overlay)
          .on_change(set_selection)
          .first(true),
      ),
    ),
    Specimen::Checkbox => Node::new(
      View::new().style(Style::new().width(839)).child(
        ToggleControl::new()
          .label(control_behavior::name_source_text(tx(
            "VSync",
            "Control effects checkbox label.",
          )))
          .checked(checked)
          .on_change(set_checked)
          .first(true),
      ),
    ),
    Specimen::Slider => Node::new(
      View::new().style(Style::new().width(839)).child(
        VolumeControl::new()
          .label(tx("Master Volume", "Control effects slider label."))
          .value(volume)
          .on_change(set_volume)
          .first(true),
      ),
    ),
  }
}

fn selector_style(selected: bool, width: f32) -> Style {
  Style::new()
    .width(width)
    .height(62)
    .margin(0)
    .padding(0)
    .border_width(2)
    .border_color(if selected {
      Color::hex(0x8ff8ff)
    } else {
      Color::hex(0x315c78)
    })
    .border_radius(8)
    .background_color(if selected {
      Color::hex(0x123b55)
    } else {
      Color::hex(0x071321)
    })
    .color(Color::hex(0xe9fbff))
    .font_size(25)
    .unity_text_align(TextAnchor::MiddleCenter)
}
