//! Arcade actions with composed labels and parent-owned callbacks.

use crate::{
  action_skin, assets,
  font_scale::{self, FontScale},
  use_interaction,
};
use battlement::{
  Align, Color, FlexDirection, ImageScaleMode, Length, LengthUnits, MotionProperty, PickingMode,
  Position, Style, TextAnchor, Translate, UiFontAddress, WhiteSpace,
};
use battlement_reactant::prelude::{Children, EventCallback, builder};
use battlement_reactant::{
  component::Component,
  components::Button,
  host::View,
  motion::{Easing, MotionTarget, StyleTarget, Transition},
  paint::{PaintLayer, PaintStyle},
  prelude::{PaintDropShadow, PaintFilterList},
  render::Render,
  semantics::SemanticName,
};

/// Native TextCore face for action labels.
pub const ACTION_FONT: UiFontAddress = UiFontAddress::from_static("chess-ui/fonts/action");

/// Fixed decorative words with generated arcade lettering.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionLabel {
  Play,
  Settings,
  About,
  Quit,
  Return,
}

impl ActionLabel {
  fn asset(self) -> battlement_reactant::asset_generator::TextImageAsset {
    match self {
      Self::Play => assets::ACTION_LABEL_PLAY,
      Self::Settings => assets::ACTION_LABEL_SETTINGS,
      Self::About => assets::ACTION_LABEL_ABOUT,
      Self::Quit => assets::ACTION_LABEL_QUIT,
      Self::Return => assets::ACTION_LABEL_RETURN,
    }
  }

  fn slug(self) -> &'static str {
    match self {
      Self::Play => "play",
      Self::Settings => "settings",
      Self::About => "about",
      Self::Quit => "quit",
      Self::Return => "return",
    }
  }
}

/// A parent-sized arcade button with arbitrary non-interactive label content.
#[builder]
pub struct ActionButton {
  #[builder(required, into)]
  children: Children,
  /// Uses generated artwork for one fixed label while retaining live children.
  artwork: Option<ActionLabel>,
  /// Disables activation while retaining the control’s place in the layout.
  disabled: bool,
  /// Caps the label size relative to its authored arcade typography.
  max_text_scale: Option<f32>,
  /// Handles an accepted button activation.
  #[builder(default = EventCallback::noop())]
  on_press: EventCallback<()>,
}

impl ActionButton {
  fn label_style(&self, font_scale: FontScale) -> Style {
    let scale = font_scale
      .dynamic(font_scale::FontScaleRole::Navigation)
      .min(self.max_text_scale.unwrap_or(f32::INFINITY));
    Style::new()
      .position(Position::Relative)
      .flex_direction(FlexDirection::Row)
      .align_items(Align::Center)
      .height(81.9 * scale)
      .padding_right(10.92 * scale)
      .color(if self.artwork.is_some() {
        Color::TRANSPARENT
      } else {
        Color::rgb(0.97, 1.0, 1.0)
      })
      .unity_font_definition(ACTION_FONT)
      .font_size(91.0 * scale)
      .white_space(WhiteSpace::NoWrap)
      .letter_spacing(-2)
      .unity_text_align(TextAnchor::MiddleCenter)
      .translate(Translate::two_dimensional(
        Length::Px(0.0),
        Length::Px(-1.0),
      ))
  }

  fn artwork_style(&self, font_scale: FontScale) -> Style {
    let scale = font_scale
      .dynamic(font_scale::FontScaleRole::Navigation)
      .min(self.max_text_scale.unwrap_or(f32::INFINITY));
    Style::new()
      .position(Position::Absolute)
      .left(50.pct())
      .top(50.pct())
      .width(480.0 * scale)
      .height(146.0 * scale)
      .translate(Translate::two_dimensional(
        Length::Percent(-50.0),
        Length::Percent(-50.0),
      ))
  }
}

impl Component for ActionButton {
  fn render(&self) -> impl Render {
    let interaction = use_interaction::use_interaction();
    let font_scale = font_scale::use_font_scale();
    View::new()
      .style(
        Style::new()
          .position(Position::Relative)
          .width(100.pct())
          .height(100.pct()),
      )
      .child(
        Button::content(
          View::new()
            .name("action-label")
            .style(self.label_style(font_scale))
            .child((
              self.artwork.map(|artwork| {
                artwork
                  .asset()
                  .image()
                  .name(format!("action-label-artwork-{}", artwork.slug()))
                  .picking_mode(PickingMode::Ignore)
                  .scale_mode(ImageScaleMode::ScaleToFit)
                  .style(self.artwork_style(font_scale))
              }),
              self.children.render(),
            )),
        )
        .semantic_name(SemanticName::Contents)
        .host_name("action-button")
        .disabled(self.disabled)
        .on_press(self.on_press.clone())
        .configure_host(|host| interaction.button(host))
        .style(
          Style::new()
            .position(Position::Relative)
            .full_size()
            .margin(0)
            .padding(0)
            .border_width(0)
            .center_content()
            .background_color(Color::TRANSPARENT),
        )
        .disabled_style(
          Style::new()
            .opacity(1.0)
            .background_color(Color::TRANSPARENT),
        )
        .paint(
          PaintStyle::new()
            .background(action_skin::border())
            .paint_filter(self::filter(1.0, 10.0, 0.65))
            .clip_polygon(action_skin::clip(18.0, 17.0))
            .layer(
              PaintLayer::new(action_skin::INTERIOR)
                .bounds_inset(6.0)
                .clip_polygon(action_skin::clip(14.0, 13.0)),
            ),
        )
        .initial(false)
        .animate(self::target(interaction.state)),
      )
  }
}

fn filter(brightness: f32, blur: f32, alpha: f64) -> PaintFilterList {
  PaintFilterList::default()
    .brightness(brightness)
    .drop_shadow(PaintDropShadow::new(
      0.0,
      0.0,
      blur,
      0.0,
      Color::hex(0x3a9aff).with_alpha(alpha),
    ))
}

fn target(state: use_interaction::InteractionState) -> MotionTarget {
  let highlighted = state.hovered || state.focus_visible;
  let filter = if state.focus_visible {
    use_interaction::focus_filter()
  } else if state.pressed {
    self::filter(0.82, 8.0, 0.65)
  } else if state.hovered {
    self::filter(1.12, 16.0, 0.88)
  } else {
    self::filter(1.0, 10.0, 0.65)
  };
  MotionTarget::new(
    StyleTarget::new()
      .background_gradient(if state.focus_visible {
        use_interaction::focus_gradient(110.0)
      } else {
        action_skin::border_gradient(highlighted)
      })
      .paint_filter(filter)
      .scale(if state.pressed && !state.reduced_motion {
        0.955
      } else {
        1.0
      }),
  )
  .transition(
    Transition::tween()
      .duration_secs(0.14)
      .ease(Easing::Ease)
      .property(
        MotionProperty::Scale,
        Transition::tween()
          .duration_secs(0.09)
          .ease(Easing::CubicBezier([0.2, 0.8, 0.2, 1.0])),
      ),
  )
}
