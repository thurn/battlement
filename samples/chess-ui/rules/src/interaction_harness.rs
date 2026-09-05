use trox::{ls, tx, tx_args, txa};

use crate::{
  action_button::ActionButton,
  return_button::ReturnButton,
  select_control::SelectControl,
  settings_tabs::{SettingsTab, SettingsTabs},
  toggle_control::ToggleControl,
  volume_control::VolumeControl,
};
use battlement::{Align, Color, FlexDirection, Position, Style, TextAnchor};
use battlement_reactant::{components::Button, control_behavior, hooks, prelude::*};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Specimen {
  #[default]
  Checkbox,
  Select,
  Slider,
  Actions,
  Tabs,
}

/// Selects each control family and exposes its pointer-feedback states.
#[builder]
pub struct InteractionHarness;

impl Component for InteractionHarness {
  fn render(&self) -> impl Render {
    let (specimen, set_specimen) = hooks::use_state(Specimen::Checkbox);
    let (checked, set_checked) = hooks::use_state(false);
    let (volume, set_volume) = hooks::use_state(80_u32);
    let (tab, set_tab) = hooks::use_state(SettingsTab::Gameplay);
    let (activations, set_activations) = hooks::use_state(0_u32);
    View::new()
      .name("interaction-specimen")
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
          .gap(10.0)
          .name("interaction-selector")
          .style(
            Style::new()
              .position(Position::Absolute)
              .left(54)
              .top(390)
              .width(916)
              .height(74),
          )
          .child(
            [
              (Specimen::Checkbox, "CHECKBOX"),
              (Specimen::Select, "SELECT"),
              (Specimen::Slider, "SLIDER"),
              (Specimen::Actions, "ACTIONS"),
              (Specimen::Tabs, "TABS"),
            ]
            .map(|(value, label)| {
              let set_specimen = set_specimen.clone();
              Button::new(ls(label))
                .host_name(format!("interaction-selector-{label}"))
                .style(self::selector_style(specimen == value))
                .on_press(move || set_specimen.set(value))
            }),
          ),
        View::new()
          .style(
            Style::new()
              .position(Position::Absolute)
              .left(88)
              .top(600)
              .width(848)
              .height(1080)
              .align_items(Align::Center),
          )
          .child(self::specimen(
            specimen,
            checked,
            set_checked,
            volume,
            set_volume,
            tab,
            set_tab,
            activations,
            set_activations.clone(),
          )),
        (specimen == Specimen::Actions).then(|| {
          ReturnButton::new().on_press(set_activations.update_callback(|count| count + 1))
        }),
      ))
  }
}

#[allow(clippy::too_many_arguments)]
fn specimen(
  specimen: Specimen,
  checked: bool,
  set_checked: hooks::StateSetter<bool>,
  volume: u32,
  set_volume: hooks::StateSetter<u32>,
  tab: SettingsTab,
  set_tab: hooks::StateSetter<SettingsTab>,
  activations: u32,
  set_activations: hooks::StateSetter<u32>,
) -> Node {
  match specimen {
    Specimen::Checkbox => Node::new(
      View::new().style(Style::new().width(839)).child((
        ToggleControl::new()
          .label(control_behavior::name_source_text(tx(
            "VSync",
            "Interaction specimen checkbox label.",
          )))
          .checked(checked)
          .on_change(set_checked),
        self::status(if checked { "VSync: On" } else { "VSync: Off" }),
      )),
    ),
    Specimen::Select => Node::new(
      View::new().style(Style::new().width(839)).child((
        SelectControl::new()
          .label(control_behavior::name_source_text(tx(
            "Resolution",
            "Interaction specimen selector label.",
          )))
          .value("1920 × 1080")
          .first(true),
        self::status("Popover behavior begins on Task 14"),
      )),
    ),
    Specimen::Slider => Node::new(
      View::new().style(Style::new().width(839)).child((
        VolumeControl::new()
          .label(tx("Master Volume", "Interaction specimen slider label."))
          .value(volume)
          .on_change(set_volume)
          .first(true),
        control_behavior::static_label(txa(
          "Master Volume: {volume}%",
          tx_args![volume],
          "Interaction specimen slider status.",
        ))
        .style(self::status_style()),
      )),
    ),
    Specimen::Actions => Node::new(
      View::new()
        .style(Style::new().width(760).height(140))
        .child((
          ActionButton::new()
            .children(control_behavior::name_source_text(tx(
              "PLAY",
              "Interaction specimen action label.",
            )))
            .on_press(set_activations.update_callback(|count| count + 1)),
          control_behavior::static_label(txa(
            "Activations: {activations}",
            tx_args![activations],
            "Interaction specimen action count.",
          ))
          .style(self::status_style()),
        )),
    ),
    Specimen::Tabs => Node::new(View::new().style(Style::new().width(887)).child((
      SettingsTabs::new().active_tab(tab).on_select(set_tab),
      self::status("Hover an inactive tab, then press and release to select it"),
    ))),
  }
}

fn selector_style(selected: bool) -> Style {
  Style::new()
    .width(175)
    .height(68)
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
    .font_size(27)
    .unity_text_align(TextAnchor::MiddleCenter)
}

fn status(value: &str) -> Label {
  control_behavior::static_label(ls(value)).style(self::status_style())
}

fn status_style() -> Style {
  Style::new()
    .margin_top(34)
    .font_size(30)
    .color(Color::rgb(0.75, 0.86, 0.97))
    .unity_text_align(TextAnchor::MiddleCenter)
}
