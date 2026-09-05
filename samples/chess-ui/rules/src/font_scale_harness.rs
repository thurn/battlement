//! Interactive specimens for the complete text-size contract.

use battlement::{
  AccessibilityScrollAxis, Align, Color, FlexDirection, Overflow, Position, Style, TextAnchor,
  WhiteSpace,
};
use battlement_reactant::{control_behavior, hooks, portal::PortalTarget, prelude::*};
use trox::{ls, tx};

use crate::{
  action_button::{ActionButton, ActionLabel},
  arcade_modal::ArcadeModal,
  font_scale::{self, FontScale},
  input_settings::InputSettings,
  return_button::ReturnButton,
  screen_header::{HeaderVariant, ScreenHeader},
  select_control::{SelectControl, VALUE_FONT},
  settings_tabs::{SettingsTab, SettingsTabs},
  toggle_control::ToggleControl,
  volume_control::VolumeControl,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Specimen {
  #[default]
  Controls,
  Navigation,
  Headings,
  Dialog,
  Input,
}

impl Specimen {
  const ALL: [Self; 5] = [
    Self::Controls,
    Self::Navigation,
    Self::Headings,
    Self::Dialog,
    Self::Input,
  ];

  const fn label(self) -> &'static str {
    match self {
      Self::Controls => "Controls",
      Self::Navigation => "Navigation",
      Self::Headings => "Headings",
      Self::Dialog => "Dialog",
      Self::Input => "Input table",
    }
  }
}

/// Switches among source-sized specimens at 100%, 150%, and 200% text.
#[builder]
pub struct FontScaleHarness {
  #[builder(required)]
  overlay: PortalTarget,
}

impl Component for FontScaleHarness {
  fn render(&self) -> impl Render {
    let (scale, set_scale) = hooks::use_state(FontScale::Percent100);
    let (specimen, set_specimen) = hooks::use_state(Specimen::Controls);
    View::new()
      .name("font-scale-harness")
      .style(Style::new().width(896).margin_top(36))
      .child((
        self::selector_row(
          "Text size",
          FontScale::ALL.map(|option| {
            self::selector_button(
              option.label(),
              option == scale,
              format!("font-scale-{}", option.label().trim_end_matches('%')),
              set_scale.callback().map_input(move |_| option),
            )
          }),
        ),
        self::selector_row(
          "Specimen",
          Specimen::ALL.map(|option| {
            self::selector_button(
              option.label(),
              option == specimen,
              format!(
                "font-scale-{}",
                option.label().to_ascii_lowercase().replace(' ', "-")
              ),
              set_specimen.callback().map_input(move |_| option),
            )
          }),
        ),
        font_scale::provider(
          scale,
          View::new()
            .name("font-scale-specimen")
            .style(
              Style::new()
                .position(Position::Relative)
                .width(if specimen == Specimen::Headings {
                  1024
                } else {
                  896
                })
                .height(980)
                .margin_left(if specimen == Specimen::Headings {
                  -64
                } else {
                  0
                })
                .margin_top(24)
                .overflow(Overflow::Hidden)
                .background_color(Color::rgb8(4, 17, 38)),
            )
            .child((
              (specimen == Specimen::Controls).then(|| {
                ControlsSpecimen::new()
                  .scale(scale)
                  .overlay(self.overlay.clone())
              }),
              (specimen == Specimen::Navigation).then(NavigationSpecimen::new),
              (specimen == Specimen::Headings).then(HeadingsSpecimen::new),
              (specimen == Specimen::Dialog)
                .then(|| DialogSpecimen::new().overlay(self.overlay.clone())),
              (specimen == Specimen::Input)
                .then(|| InputSettings::new().overlay(self.overlay.clone())),
            )),
        ),
      ))
  }
}

#[builder]
struct ControlsSpecimen {
  #[builder(required)]
  overlay: PortalTarget,
  #[builder(required)]
  scale: FontScale,
}

impl Component for ControlsSpecimen {
  fn render(&self) -> impl Render {
    let (selected, set_selected) = hooks::use_state("Borderless".to_owned());
    let (checked, set_checked) = hooks::use_state(true);
    let (volume, set_volume) = hooks::use_state(80_u32);
    ScrollArea::new(
      Some(ls("Scaled control specimens")),
      AccessibilityScrollAxis::Vertical,
      false,
      false,
    )
    .on_scroll(|_| {})
    .host_name("font-scale-controls-scroll")
    .style(Style::new().width(896).height(980))
    .child(
      View::new()
        .style(Style::new().width(839).align_self(Align::Center))
        .child((
          SelectControl::new()
            .label(control_behavior::name_source_text(tx(
              "Display Mode",
              "Scaled selector label.",
            )))
            .value(selected.clone())
            .options(vec![
              "Fullscreen".to_owned(),
              "Borderless".to_owned(),
              "Windowed".to_owned(),
            ])
            .overlay(self.overlay.clone())
            .font_scale(self.scale)
            .on_change(set_selected)
            .first(true),
          ToggleControl::new()
            .label(control_behavior::name_source_text(tx(
              "Screenshake",
              "Scaled toggle label.",
            )))
            .checked(checked)
            .on_change(set_checked),
          VolumeControl::new()
            .label(tx("Master Volume", "Scaled volume label."))
            .value(volume)
            .on_change(set_volume),
          View::new()
            .style(Style::new().width(620).height(126).margin((34, 0, 34, 18)))
            .child(
              ActionButton::new()
                .artwork(ActionLabel::Play)
                .children(control_behavior::name_source_text(tx(
                  "PLAY",
                  "Scaled action label.",
                )))
                .on_press(|| {}),
            ),
        )),
    )
  }
}

#[builder]
struct NavigationSpecimen;

impl Component for NavigationSpecimen {
  fn render(&self) -> impl Render {
    let (active, set_active) = hooks::use_state(SettingsTab::Gameplay);
    View::new()
      .style(
        Style::new()
          .position(Position::Absolute)
          .inset(0)
          .padding_top(90),
      )
      .child((
        View::new()
          .style(Style::new().width(887).align_self(Align::Center))
          .child(SettingsTabs::new().active_tab(active).on_select(set_active)),
        View::new()
          .style(
            Style::new()
              .width(620)
              .height(126)
              .margin_top(130)
              .align_self(Align::Center),
          )
          .child(
            ActionButton::new()
              .artwork(ActionLabel::Settings)
              .children(control_behavior::name_source_text(tx(
                "SETTINGS",
                "Scaled navigation label.",
              )))
              .on_press(|| {}),
          ),
        ReturnButton::new().left(264.0).top(650.0).on_press(|| {}),
      ))
  }
}

#[builder]
struct HeadingsSpecimen;

impl Component for HeadingsSpecimen {
  fn render(&self) -> impl Render {
    View::new()
      .style(Style::new().position(Position::Absolute).inset(0))
      .child((
        View::new()
          .style(
            Style::new()
              .position(Position::Relative)
              .width(1024)
              .height(440),
          )
          .child(ScreenHeader::new().variant(HeaderVariant::Game)),
        View::new()
          .style(
            Style::new()
              .position(Position::Relative)
              .width(1024)
              .height(360),
          )
          .child(ScreenHeader::new().variant(HeaderVariant::Settings)),
      ))
  }
}

#[builder]
struct DialogSpecimen {
  #[builder(required)]
  overlay: PortalTarget,
}

impl Component for DialogSpecimen {
  fn render(&self) -> impl Render {
    let (open, set_open) = hooks::use_state(false);
    View::new().style(Style::new().padding(80)).child((
      self::selector_button(
        "Open dialog",
        open,
        "font-scale-open-dialog".to_owned(),
        set_open.callback().map_input(|_| true),
      ),
      ArcadeModal::new()
        .open(open)
        .title(tx("Pause", "Scaled dialog title."))
        .children(
          Text::new(tx(
            "Text size keeps every dialog action visible.",
            "Scaled dialog description.",
          ))
          .style(
            Style::new()
              .width(620)
              .font_size(47)
              .white_space(WhiteSpace::Normal)
              .unity_font_definition(VALUE_FONT)
              .unity_text_align(TextAnchor::MiddleCenter),
          ),
        )
        .confirm_label(tx("Resume", "Scaled dialog confirmation."))
        .cancel_label(tx("Cancel", "Scaled dialog cancellation."))
        .reduce_motion(false)
        .on_confirm(set_open.callback().map_input(|_| false))
        .on_close(set_open.callback().map_input(|_| false))
        .overlay(self.overlay.clone()),
    ))
  }
}

fn selector_row(label: &'static str, buttons: impl Render) -> impl Render {
  Flex::new()
    .direction(FlexDirection::Row)
    .gap(10.0)
    .style(Style::new().min_height(64).align_items(Align::Center))
    .child((
      control_behavior::static_label(ls(label)).style(
        Style::new()
          .width(130)
          .font_size(26)
          .color(Color::rgb8(190, 218, 240)),
      ),
      buttons,
    ))
}

fn selector_button(
  label: &'static str,
  selected: bool,
  name: String,
  on_press: EventCallback<()>,
) -> impl Render {
  Button::new(ls(label))
    .host_name(name)
    .on_press(on_press)
    .style(
      Style::new()
        .min_width(102)
        .height(54)
        .padding((8, 12))
        .border_width(1)
        .border_color(Color::hex(if selected { 0x61f0e6 } else { 0x31455d }))
        .border_radius(6)
        .background_color(Color::hex(if selected { 0x163d43 } else { 0x101a28 }))
        .color(Color::hex(if selected { 0x7ffcf2 } else { 0xd4e4f1 }))
        .font_size(24)
        .unity_text_align(TextAnchor::MiddleCenter),
    )
}
