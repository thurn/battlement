//! Controlled settings categories rendered through native tab headers.

use trox::{LocalizedString, tx};

use crate::select_control;
use battlement::{
  Align, Color, FlexDirection, Length, Overflow, Style, TextAnchor, Translate, WhiteSpace,
};
use battlement_reactant::{host::ButtonHost, prelude::*};

/// The settings categories in their display order.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SettingsTab {
  #[default]
  Gameplay,
  Graphics,
  Sound,
  Input,
}

/// Four horizontal categories whose selected value belongs to the caller.
#[builder]
pub struct SettingsTabs {
  #[builder(required)]
  active_tab: SettingsTab,
  #[builder(required)]
  on_select: EventCallback<SettingsTab>,
}

impl SettingsTab {
  /// Categories in left-to-right order.
  pub const ALL: [Self; 4] = [Self::Gameplay, Self::Graphics, Self::Sound, Self::Input];

  pub fn label(self) -> LocalizedString {
    match self {
      Self::Gameplay => tx("Gameplay", "Settings category."),
      Self::Graphics => tx("Graphics", "Settings category."),
      Self::Sound => tx("Sound", "Settings category."),
      Self::Input => tx("Input", "Settings category."),
    }
  }

  fn width(self) -> f32 {
    match self {
      Self::Gameplay => 264.0,
      Self::Graphics => 212.0,
      Self::Sound => 205.0,
      Self::Input => 200.0,
    }
  }
}

impl Component for SettingsTabs {
  fn render(&self) -> impl Render {
    TabStrip::new()
      .label(tx("Settings categories", "Settings category list."))
      .selected_index(self.active_tab as u32)
      .on_select(
        self
          .on_select
          .clone()
          .map_input(|index| SettingsTab::ALL[index as usize]),
      )
      .host(
        View::new().name("settings-tabs").style(
          Style::new()
            .width(887)
            .height(129)
            .flex_shrink(0)
            .flex_direction(FlexDirection::Row)
            .align_items(Align::FlexEnd)
            .overflow(Overflow::Visible),
        ),
      )
      .children(SettingsTab::ALL.map(|tab| {
        SettingsTabButton::new()
          .tab(tab)
          .active(tab == self.active_tab)
      }))
  }
}

#[builder]
struct SettingsTabButton {
  #[builder(required)]
  tab: SettingsTab,
  active: bool,
}

impl Component for SettingsTabButton {
  fn render(&self) -> impl Render {
    TabButton::new()
      .label(self.tab.label())
      .index(self.tab as u32)
      .host(
        ButtonHost::new(self.tab.label()).style(
          Style::new()
            .width(self.tab.width())
            .flex_shrink(0)
            .height(if self.active { 130 } else { 127 })
            .margin(0)
            .margin_right(if self.tab == SettingsTab::Input { 0 } else { 2 })
            .padding(0)
            .border_width(4)
            .border_color(if self.active {
              Color::hex(0x53afff)
            } else {
              Color::hex(0x657287)
            })
            .background_color(if self.active {
              Color::hex(0x071831)
            } else {
              Color::hex(0x071328)
            })
            .center_content()
            .overflow(Overflow::Visible)
            .translate(Translate::two_dimensional(
              Length::px(0.0),
              Length::px(if self.active { 0.0 } else { 3.0 }),
            ))
            .unity_font_definition(select_control::VALUE_FONT)
            .font_size(if self.active { 55 } else { 51 })
            .letter_spacing(1)
            .color(Color::hex(0xf7f7fb))
            .white_space(WhiteSpace::NoWrap)
            .unity_text_align(TextAnchor::MiddleCenter),
        ),
      )
  }
}
