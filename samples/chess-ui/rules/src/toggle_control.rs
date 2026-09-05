//! A controlled checkbox whose label activates and focuses its input.

use trox::{LocalizedString, tx};

use crate::{check_mark::CheckMark, setting_row::SettingRow, use_interaction};
use battlement::{
  Align, Color, FontStyle, Gradient, Justify, Length, MotionProperty, PickingMode, Position, Scale,
  Shadow, Style, TextAnchor, Translate,
};
use battlement_reactant::{
  control_behavior,
  host::ToggleHost,
  motion::{Easing, MotionTarget, StyleTarget, Transition},
  paint::{PaintLayer, PaintStyle},
  prelude::{PaintFilterList, *},
};

/// A controlled checkbox with an associated, clickable settings label.
#[builder]
pub struct ToggleControl {
  #[builder(required, into)]
  label: Child,
  /// Omits the top separator when this is the first row.
  first: bool,
  /// Offsets the control vertically without moving its row label.
  offset_y: f32,
  /// Sets the minimum row height in portrait design pixels.
  row_height: Option<f32>,
  #[builder(required)]
  checked: bool,
  /// Overrides the accessible name when the visible wording needs clarification.
  aria_label: Option<LocalizedString>,
  /// Adds assistive context without adding visible copy to the settings row.
  accessibility_description: Option<LocalizedString>,
  /// Shows crash-report context next to the visible label.
  with_info: bool,
  /// Handles activation of the optional crash-report information badge.
  #[builder(default = EventCallback::noop())]
  on_info_click: EventCallback<()>,
  #[builder(required)]
  on_change: EventCallback<bool>,
}

impl Component for ToggleControl {
  fn render(&self) -> impl Render {
    let interaction = use_interaction::use_interaction();
    let (label, checkbox) = use_control_label().bind_with(|label_name| {
      control_behavior::checkbox(
        self
          .aria_label
          .as_ref()
          .map_or(label_name, |name| SemanticName::text(name.clone())),
        self
          .accessibility_description
          .as_ref()
          .map(|description| SemanticDescription::text(description.clone()))
          .or_else(|| {
            self.with_info.then(|| {
              SemanticDescription::text(tx(
                "We upload crash reports to Unity Diagnostics.",
                "Crash report toggle accessibility description.",
              ))
            })
          }),
        self.checked,
        false,
        self.on_change.clone(),
      )
    });
    View::new()
      .name("toggle-control-label")
      .style(Style::new().height(self.row_height))
      .child(
        SettingRow::new()
          .label((
            self.label.render(),
            self
              .with_info
              .then(|| InfoBadge::new().on_click(self.on_info_click.clone())),
          ))
          .children(
            View::new()
              .name("toggle-control-box")
              .style(
                Style::new()
                  .position(Position::Relative)
                  .align_items(Align::Center)
                  .width(77)
                  .height(77)
                  .margin_left(8)
                  .translate(Translate::two_dimensional(
                    Length::Px(0.0),
                    Length::Px(self.offset_y),
                  )),
              )
              .child((
                View::new()
                  .name("toggle-control-surface")
                  .picking_mode(PickingMode::Ignore)
                  .style(Style::new().width(77).height(77).border_radius(11))
                  .paint(self::surface_paint())
                  .initial(false)
                  .animate(self::surface_target(interaction.state))
                  .child(self.checked.then_some(CheckMark::new())),
                interaction
                  .toggle(
                    ToggleHost::new()
                      .value(self.checked)
                      .name("toggle-control-input")
                      .associated_control(checkbox),
                  )
                  .on_change_value(self.on_change.clone())
                  .input_style(Style::new().opacity(0.0))
                  .style(
                    Style::new()
                      .position(Position::Absolute)
                      .left(0)
                      .top(0)
                      .width(77)
                      .height(77)
                      .margin(0)
                      .padding(0)
                      .border_width(0)
                      .background_color(Color::TRANSPARENT),
                  ),
              )),
          )
          .associated_label(label)
          .first(self.first)
          .row_height(self.row_height),
      )
  }
}

fn surface_paint() -> PaintStyle {
  PaintStyle::new()
    .background(self::flat_gradient(Color::rgb8(75, 163, 255)))
    .box_shadow(self::shadows(false, false))
    .clip_polygon(self::rounded_box())
    .layer(
      PaintLayer::new(
        Gradient::linear(90.0)
          .stop(0.0, Color::hex(0x06142b))
          .stop(1.0, Color::hex(0x02091a)),
      )
      .bounds_inset(4.0)
      .clip_polygon(self::rounded_box()),
    )
}

fn surface_target(state: use_interaction::InteractionState) -> MotionTarget {
  MotionTarget::new(
    StyleTarget::new()
      .background_gradient(if state.focus_visible {
        use_interaction::focus_gradient(110.0)
      } else {
        self::flat_gradient(if state.hovered {
          Color::hex(0x91faff)
        } else {
          Color::hex(0x4ba3ff)
        })
      })
      .box_shadow(self::shadows(state.hovered, state.focus_visible))
      .paint_filter(if state.focus_visible {
        use_interaction::focus_filter()
      } else {
        PaintFilterList::default().brightness(if state.pressed { 0.76 } else { 1.0 })
      })
      .scale(if state.pressed && !state.reduced_motion {
        0.88
      } else if state.hovered {
        1.045
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
      )
      .property(
        MotionProperty::PaintFilter,
        Transition::tween().duration_secs(0.09).ease(Easing::Ease),
      ),
  )
}

fn flat_gradient(color: Color) -> Gradient {
  Gradient::linear(0.0).stop(0.0, color).stop(1.0, color)
}

fn shadows(hovered: bool, focus_visible: bool) -> Vec<Shadow> {
  if focus_visible {
    vec![
      self::shadow(14.0, Color::BLACK, true),
      self::shadow(4.0, Color::WHITE, false),
      self::shadow(16.0, Color::hex(0xffd900), false),
    ]
  } else if hovered {
    vec![
      self::shadow(12.0, Color::BLACK, true),
      self::shadow(15.0, Color::hex(0x2acfff), false),
      self::shadow(8.0, Color::hex(0xb8ffff), false),
    ]
  } else {
    vec![
      self::shadow(14.0, Color::BLACK, true),
      self::shadow(10.0, Color::hex(0x166cff), false),
      self::shadow(5.0, Color::hex(0x6af6ff), false),
    ]
  }
}

fn shadow(blur: f32, color: Color, inset: bool) -> Shadow {
  Shadow {
    x: 0.0,
    y: 0.0,
    blur,
    spread: 0.0,
    color,
    inset,
  }
}

fn rounded_box() -> Vec<[Length; 2]> {
  vec![
    [Length::px(11.0), Length::px(0.0)],
    [Length::calc(-11.0, 100.0), Length::px(0.0)],
    [Length::percent(100.0), Length::px(11.0)],
    [Length::percent(100.0), Length::calc(-11.0, 100.0)],
    [Length::calc(-11.0, 100.0), Length::percent(100.0)],
    [Length::px(11.0), Length::percent(100.0)],
    [Length::px(0.0), Length::calc(-11.0, 100.0)],
    [Length::px(0.0), Length::px(11.0)],
  ]
}

#[builder]
/// A compact information action positioned beside a setting label.
pub struct InfoBadge {
  #[builder(required)]
  on_click: EventCallback<()>,
}

impl Component for InfoBadge {
  fn render(&self) -> impl Render {
    Button::content(Text::new(tx("i", "Crash report toggle interface label.")))
      .semantic_name(SemanticName::text(tx(
        "About crash report uploads",
        "Crash report toggle accessibility label.",
      )))
      .host_name("toggle-info")
      .on_press(self.on_click.clone())
      .style(
        Style::new()
          .position(Position::Absolute)
          .left(205)
          .bottom(37)
          .width(38)
          .height(38)
          .padding(0)
          .border_width(2)
          .border_color(Color::rgb8(85, 184, 255))
          .border_radius(19)
          .background_color(Color::TRANSPARENT)
          .color(Color::rgb8(188, 244, 255))
          .font_size(27)
          .unity_font_style_and_weight(FontStyle::Bold)
          .unity_text_align(TextAnchor::MiddleCenter)
          .align_items(Align::Center)
          .justify_content(Justify::Center)
          .translate(Translate::two_dimensional(Length::Px(0.0), Length::Px(1.0)))
          .scale(Scale::new(0.957, 1.0)),
      )
      .paint(
        PaintStyle::new()
          .background(Color::TRANSPARENT)
          .box_shadow([
            Shadow::outer(0.0, 0.0, 8.0, 0.0, Color::hex(0x155eff)),
            Shadow::inset(0.0, 0.0, 7.0, 0.0, Color::rgba8(13, 76, 180, 204)),
          ]),
      )
  }
}
