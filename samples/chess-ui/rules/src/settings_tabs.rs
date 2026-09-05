//! Controlled settings categories rendered through native tab headers.

use trox::{LocalizedString, tx};

use crate::{select_control, tabs_navigation, tabs_skin, use_interaction};
use battlement::{
  Align, Color, FlexDirection, MotionProperty, Overflow, PickingMode, Position, Style, TextAnchor,
  TextShadow, WhiteSpace,
};
use battlement_reactant::{
  host::ButtonHost,
  motion::{Easing, MotionTarget, StyleTarget, Transition},
  prelude::*,
};

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
    let references = self::use_references();
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
          .on_select(self.on_select.clone())
          .references(references.clone())
      }))
  }
}

#[builder]
struct SettingsTabButton {
  #[builder(required)]
  tab: SettingsTab,
  active: bool,
  #[builder(required)]
  on_select: EventCallback<SettingsTab>,
  #[builder(required)]
  references: [ElementRef; 4],
}

impl Component for SettingsTabButton {
  fn render(&self) -> impl Render {
    let interaction = use_interaction::use_interaction();
    TabButton::new()
      .label(self.tab.label())
      .index(self.tab as u32)
      .host(
        interaction
          .button(ButtonHost::new(self.tab.label()))
          .element_ref(self.references[self.tab as usize].clone())
          .on_key_down_event_callback(tabs_navigation::key_callback(
            self.tab,
            self.references.clone(),
            self.on_select.clone(),
          ))
          .on_navigation_move_event_callback(tabs_navigation::controller_callback(
            self.tab,
            self.references.clone(),
            self.on_select.clone(),
          ))
          .style(
            Style::new()
              .width(self.tab.width())
              .flex_shrink(0)
              .height(if self.active { 130 } else { 127 })
              .margin(0)
              .margin_right(if self.tab == SettingsTab::Input { 0 } else { 2 })
              .padding(0)
              .border_width(0)
              .background_color(Color::TRANSPARENT)
              .center_content()
              .overflow(Overflow::Visible)
              .unity_font_definition(select_control::VALUE_FONT)
              .font_size(if self.active { 55 } else { 51 })
              .letter_spacing(1)
              .color(Color::hex(0xf7f7fb))
              .white_space(WhiteSpace::NoWrap)
              .unity_text_align(TextAnchor::MiddleCenter)
              .text_shadow(TextShadow::new(0.0, 5.0, 7.0, Color::BLACK)),
          )
          .paint(tabs_skin::paint(self.active))
          .initial(false)
          .animate(self::target(self.active, interaction.state))
          .child(
            TextElement::new(self.tab.label())
              .picking_mode(PickingMode::Ignore)
              .style(
                Style::new()
                  .position(Position::Absolute)
                  .full_size()
                  .margin(0)
                  .padding(0)
                  .unity_text_align(TextAnchor::MiddleCenter)
                  .text_shadow(TextShadow::new(2.0, 4.0, 0.0, Color::hex(0x182b50))),
              ),
          ),
      )
  }
}

fn use_references() -> [ElementRef; 4] {
  [
    use_element_ref(),
    use_element_ref(),
    use_element_ref(),
    use_element_ref(),
  ]
}

fn target(active: bool, state: use_interaction::InteractionState) -> MotionTarget {
  let highlighted = state.hovered || state.focus_visible;
  MotionTarget::new(
    StyleTarget::new()
      .y(if active {
        0.0
      } else if state.hovered {
        -1.0
      } else {
        3.0
      })
      .scale(if state.pressed && !state.reduced_motion {
        0.955
      } else {
        1.0
      })
      .background_gradient(if state.focus_visible {
        use_interaction::focus_gradient(110.0)
      } else {
        tabs_skin::gradient(active, highlighted)
      })
      .paint_filter(if state.focus_visible {
        use_interaction::focus_filter()
      } else {
        tabs_skin::filter(active, highlighted)
      }),
  )
  .transition(
    Transition::spring()
      .stiffness(520.0)
      .damping(32.0)
      .mass(0.7)
      .property(
        MotionProperty::BackgroundGradient,
        Transition::tween().duration_secs(0.14).ease(Easing::Ease),
      )
      .property(
        MotionProperty::PaintFilter,
        Transition::tween().duration_secs(0.14).ease(Easing::Ease),
      ),
  )
}
