//! Runtime-sized specimens for the settled rendering and generated-asset choices.

use battlement::{
  AccessibilityScrollAxis, Align, Color, FlexDirection, FlexWrap, ImageScaleMode, Overflow,
  PickingMode, Position, Style, TextAnchor,
};
use battlement_reactant::{control_behavior, hooks, prelude::*};
use trox::{ls, tx};

use crate::{
  action_button::{ActionButton, ActionLabel},
  assets,
  screen_frame::ScreenFrame,
  screen_header::{HeaderVariant, ScreenHeader},
  settings_tabs::{SettingsTab, SettingsTabs},
  toggle_control::ToggleControl,
  volume_control::VolumeControl,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Specimen {
  #[default]
  Actions,
  Tabs,
  Checkboxes,
  Sliders,
  Headings,
  Frame,
  Panel,
}

impl Specimen {
  const ALL: [Self; 7] = [
    Self::Actions,
    Self::Tabs,
    Self::Checkboxes,
    Self::Sliders,
    Self::Headings,
    Self::Frame,
    Self::Panel,
  ];

  const fn label(self) -> &'static str {
    match self {
      Self::Actions => "Action labels",
      Self::Tabs => "Tabs",
      Self::Checkboxes => "Checkboxes",
      Self::Sliders => "Sliders",
      Self::Headings => "Headings",
      Self::Frame => "Outer frame",
      Self::Panel => "Panel",
    }
  }
}

/// Selects every retained procedural or generated rendering treatment.
#[builder]
pub struct RenderingAudit;

impl Component for RenderingAudit {
  fn render(&self) -> impl Render {
    let (specimen, set_specimen) = hooks::use_state(Specimen::Actions);
    View::new()
      .name("rendering-audit")
      .style(Style::new().width(896).margin_top(36))
      .child((
        Flex::new()
          .direction(FlexDirection::Row)
          .wrap(FlexWrap::Wrap)
          .gap(10.0)
          .style(Style::new().min_height(116).align_items(Align::Center))
          .child(Specimen::ALL.map(|option| {
            self::selector_button(
              option,
              option == specimen,
              set_specimen.callback().map_input(move |_| option),
            )
          })),
        View::new()
          .name("rendering-audit-specimen")
          .style(
            Style::new()
              .position(Position::Relative)
              .width(
                if matches!(specimen, Specimen::Headings | Specimen::Frame) {
                  1024
                } else {
                  896
                },
              )
              .height(980)
              .margin_left(
                if matches!(specimen, Specimen::Headings | Specimen::Frame) {
                  -64
                } else {
                  0
                },
              )
              .margin_top(20)
              .overflow(Overflow::Hidden)
              .background_color(Color::rgb8(4, 17, 38)),
          )
          .child((
            (specimen == Specimen::Actions).then(ActionSpecimen::new),
            (specimen == Specimen::Tabs).then(TabSpecimen::new),
            (specimen == Specimen::Checkboxes).then(CheckboxSpecimen::new),
            (specimen == Specimen::Sliders).then(SliderSpecimen::new),
            (specimen == Specimen::Headings).then(HeadingSpecimen::new),
            (specimen == Specimen::Frame).then(FrameSpecimen::new),
            (specimen == Specimen::Panel).then(PanelSpecimen::new),
          )),
      ))
  }
}

#[builder]
struct ActionSpecimen;

impl Component for ActionSpecimen {
  fn render(&self) -> impl Render {
    ScrollArea::new(
      Some(ls("Generated action labels")),
      AccessibilityScrollAxis::Vertical,
      false,
      false,
    )
    .on_scroll(|_| {})
    .host_name("rendering-action-scroll")
    .style(Style::new().width(896).height(980))
    .child(
      View::new()
        .style(
          Style::new()
            .width(760)
            .padding_top(20)
            .align_self(Align::Center),
        )
        .child((
          self::action("PLAY", ActionLabel::Play),
          self::action("SETTINGS", ActionLabel::Settings),
          self::action("ABOUT", ActionLabel::About),
          self::action("QUIT", ActionLabel::Quit),
          self::action("RETURN", ActionLabel::Return),
        )),
    )
  }
}

#[builder]
struct TabSpecimen;

impl Component for TabSpecimen {
  fn render(&self) -> impl Render {
    View::new()
      .style(
        Style::new()
          .width(887)
          .padding_top(180)
          .align_self(Align::Center),
      )
      .child(
        SettingsTabs::new()
          .active_tab(SettingsTab::Gameplay)
          .on_select(|_| {}),
      )
  }
}

#[builder]
struct CheckboxSpecimen;

impl Component for CheckboxSpecimen {
  fn render(&self) -> impl Render {
    View::new()
      .style(
        Style::new()
          .width(839)
          .padding_top(100)
          .align_self(Align::Center),
      )
      .child((
        ToggleControl::new()
          .label(control_behavior::name_source_text(tx(
            "Unchecked",
            "Procedural checkbox state.",
          )))
          .checked(false)
          .on_change(|_| {})
          .first(true),
        ToggleControl::new()
          .label(control_behavior::name_source_text(tx(
            "Checked",
            "Procedural checkbox state.",
          )))
          .checked(true)
          .on_change(|_| {}),
      ))
  }
}

#[builder]
struct SliderSpecimen;

impl Component for SliderSpecimen {
  fn render(&self) -> impl Render {
    View::new()
      .style(
        Style::new()
          .width(839)
          .padding_top(100)
          .align_self(Align::Center),
      )
      .child((
        VolumeControl::new()
          .label(tx("Minimum", "Procedural slider endpoint."))
          .value(0)
          .on_change(|_| {})
          .first(true),
        VolumeControl::new()
          .label(tx("Maximum", "Procedural slider endpoint."))
          .value(100)
          .on_change(|_| {}),
      ))
  }
}

#[builder]
struct HeadingSpecimen;

impl Component for HeadingSpecimen {
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
struct FrameSpecimen;

impl Component for FrameSpecimen {
  fn render(&self) -> impl Render {
    ScrollArea::new(
      Some(ls("Procedural outer frame")),
      AccessibilityScrollAxis::Vertical,
      false,
      false,
    )
    .on_scroll(|_| {})
    .host_name("rendering-frame-scroll")
    .style(Style::new().width(1024).height(980))
    .child(ScreenFrame::new().children(View::new()))
  }
}

#[builder]
struct PanelSpecimen;

impl Component for PanelSpecimen {
  fn render(&self) -> impl Render {
    ScrollArea::new(
      Some(ls("Generated settings panel surround")),
      AccessibilityScrollAxis::Vertical,
      false,
      false,
    )
    .on_scroll(|_| {})
    .host_name("rendering-panel-scroll")
    .style(Style::new().width(896).height(980))
    .child(
      assets::SETTINGS_PANEL_FRAME
        .image()
        .name("settings-panel-artwork")
        .picking_mode(PickingMode::Ignore)
        .scale_mode(ImageScaleMode::ScaleToFit)
        .style(
          Style::new()
            .width(887)
            .height(1021)
            .align_self(Align::Center),
        ),
    )
  }
}

fn action(label: &'static str, artwork: ActionLabel) -> impl Render {
  View::new()
    .style(Style::new().width(760).height(140).margin_bottom(20))
    .child(
      ActionButton::new()
        .artwork(artwork)
        .children(control_behavior::name_source_text(ls(label)))
        .on_press(|| {}),
    )
}

fn selector_button(option: Specimen, selected: bool, on_press: EventCallback<()>) -> impl Render {
  Button::new(ls(option.label()))
    .host_name(format!(
      "rendering-specimen-{}",
      option.label().to_ascii_lowercase().replace(' ', "-")
    ))
    .on_press(on_press)
    .style(
      Style::new()
        .min_width(116)
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
