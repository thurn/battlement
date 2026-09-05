//! Review specimens for input glyphs and the generated settings surround.

use battlement::{
  AccessibilityScrollAxis, Align, Color, FlexDirection, FlexWrap, Style, TextAnchor,
};
use battlement_reactant::{hooks, prelude::*};
use trox::ls;

use crate::{
  font_scale::{self, FontScale},
  input_settings::{InputBindingVariant, InputSettings},
  setting_row::DISPLAY_FONT,
  settings_panel::SettingsPanel,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Specimen {
  #[default]
  Input,
  Panel,
}

impl Specimen {
  fn label(self) -> &'static str {
    match self {
      Self::Input => "Input table",
      Self::Panel => "Panel surround",
    }
  }
}

/// Switches between the final input table and plain settings-panel specimens.
#[builder]
pub struct InputSkinHarness;

impl Component for InputSkinHarness {
  fn render(&self) -> impl Render {
    let (specimen, set_specimen) = hooks::use_state(Specimen::Input);
    let (variant, set_variant) = hooks::use_state(InputBindingVariant::Default);
    let (scale, set_scale) = hooks::use_state(FontScale::Percent100);
    View::new()
      .name("input-skin-harness")
      .style(Style::new().width(896).margin_top(24))
      .child((
        self::selector_row(
          "Specimen",
          [Specimen::Input, Specimen::Panel].map(|option| {
            self::selector_button(
              option.label(),
              option == specimen,
              format!(
                "input-skin-{}",
                option.label().to_ascii_lowercase().replace(' ', "-")
              ),
              set_specimen.callback().map_input(move |_| option),
            )
          }),
        ),
        self::selector_row(
          "Bindings",
          [InputBindingVariant::Default, InputBindingVariant::Custom].map(|option| {
            self::selector_button(
              option.label(),
              option == variant,
              format!(
                "input-skin-{}",
                option.label().to_ascii_lowercase().replace(['/', ' '], "-")
              ),
              set_variant.callback().map_input(move |_| option),
            )
          }),
        ),
        self::selector_row(
          "Text size",
          FontScale::ALL.map(|option| {
            self::selector_button(
              option.label(),
              option == scale,
              format!("input-skin-scale-{}", option.label().trim_end_matches('%')),
              set_scale.callback().map_input(move |_| option),
            )
          }),
        ),
        Button::new(ls("Reset specimen"))
          .host_name("input-skin-reset")
          .on_press(
            set_specimen
              .callback()
              .map_input(|_| Specimen::Input)
              .then(
                set_variant
                  .callback()
                  .map_input(|_| InputBindingVariant::Default),
              )
              .then(set_scale.callback().map_input(|_| FontScale::Percent100)),
          )
          .style(self::button_style(false)),
        View::new()
          .name("input-skin-specimen")
          .style(
            Style::new()
              .width(896)
              .height(980)
              .margin_top(18)
              .background_color(Color::rgb8(4, 17, 38)),
          )
          .child(font_scale::provider(
            scale,
            (
              (specimen == Specimen::Input).then(|| {
                InputSettings::new()
                  .variant(variant)
                  .key((scale.label(), variant.label()))
              }),
              (specimen == Specimen::Panel).then(PanelSpecimen::new),
            ),
          )),
      ))
  }
}

#[builder]
struct PanelSpecimen;

impl Component for PanelSpecimen {
  fn render(&self) -> impl Render {
    ScrollArea::new(
      Some(ls("Settings panel surround")),
      AccessibilityScrollAxis::Vertical,
      false,
      false,
    )
    .on_scroll(|_| {})
    .host_name("settings-panel-scroll")
    .style(Style::new().width(896).height(980))
    .child(
      SettingsPanel::new().children(
        View::new()
          .name("settings-panel-padding-guide")
          .style(
            Style::new()
              .full_size()
              .border_width(2)
              .border_color(Color::rgba8(92, 236, 255, 105))
              .background_color(Color::rgba8(25, 92, 148, 24))
              .center_content(),
          )
          .child(
            Text::new(ls("18 px top · 24 px sides · 32 px bottom")).style(
              Style::new()
                .color(Color::hex(0x95ddec))
                .unity_font_definition(DISPLAY_FONT)
                .font_size(34)
                .unity_text_align(TextAnchor::MiddleCenter),
            ),
          ),
      ),
    )
  }
}

fn selector_row(label: &'static str, children: impl Render) -> impl Render {
  Flex::new()
    .direction(FlexDirection::Row)
    .wrap(FlexWrap::Wrap)
    .gap(10.0)
    .style(Style::new().min_height(66).align_items(Align::Center))
    .child((
      Text::new(ls(label)).style(
        Style::new()
          .width(122)
          .color(Color::hex(0x95a9bd))
          .font_size(22)
          .unity_text_align(TextAnchor::MiddleLeft),
      ),
      children,
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
    .style(self::button_style(selected))
}

fn button_style(selected: bool) -> Style {
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
    .unity_text_align(TextAnchor::MiddleCenter)
}
