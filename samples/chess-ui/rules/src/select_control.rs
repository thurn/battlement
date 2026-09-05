//! A controlled selector with an anchored pointer-operated popover.

use trox::{ls, tx};

use crate::{caret::Caret, check_mark::CheckMark, setting_row::SettingRow, use_interaction};
use battlement::{
  Align, Color, FlexDirection, Gradient, Justify, Length, LengthUnits, MotionProperty, PickingMode,
  PopoverPlacement, Position, Scale, Style, TextAnchor, TransformOrigin, Translate, UiFontAddress,
};
use battlement_reactant::{
  control_behavior, element_ref, geometry, hooks,
  host::ButtonHost,
  motion::{Easing, MotionTarget, StyleTarget, Transition},
  overlay::Overlay,
  paint::{PaintLayer, PaintStyle},
  portal::PortalTarget,
  prelude::*,
  prelude::{PaintDropShadow, PaintFilterList},
};

/// Native TextCore face for selected control values.
pub const VALUE_FONT: UiFontAddress = UiFontAddress::from_static("chess-ui/fonts/control");

/// A settings selector whose parent owns the accepted value.
#[builder]
pub struct SelectControl {
  #[builder(required, into)]
  label: Child,
  /// Omits the separator above the first row.
  first: bool,
  /// Offsets the control vertically without moving its row label.
  offset_y: f32,
  /// Sets the minimum row height in portrait design pixels.
  row_height: Option<f32>,
  #[builder(required)]
  value: String,
  options: Vec<String>,
  overlay: Option<PortalTarget>,
  #[builder(default = EventCallback::noop())]
  on_change: EventCallback<String>,
}

impl Component for SelectControl {
  fn render(&self) -> impl Render {
    let interaction = use_interaction::use_interaction();
    let (open, set_open) = hooks::use_state(false);
    let anchor = element_ref::use_element_ref();
    let measured_anchor = geometry::use_geometry(anchor.clone());
    let popover_scale = hooks::use_memo(
      move || {
        measured_anchor.measurements.latest.map_or(1.0, |value| {
          value.viewport_bound.width as f32 / value.layout.width as f32
        })
      },
      measured_anchor.measurements.latest,
    );
    let label = use_control_label();
    let value_label = use_label();
    let (label, trigger) = label.bind_with({
      let set_open = set_open.clone();
      let value_label = value_label.clone();
      let interactive = self.overlay.is_some() && !self.options.is_empty();
      move |name| {
        let SemanticName::LabelledBy(references) = name else {
          panic!("control labels must resolve through labelled-by references");
        };
        control_behavior::button(
          SemanticName::LabelledBy(
            references
              .into_iter()
              .chain([value_label.reference()])
              .collect(),
          ),
          None,
          false,
          set_open.update_callback(move |value| interactive && !value),
        )
        .map_semantic(move |mut semantic| {
          semantic.state.popup = Some(PopupKind::ListBox);
          semantic.state.expanded = Some(open);
          semantic
        })
      }
    });
    SettingRow::new()
      .label(self.label.render())
      .children(
        View::new()
          .name("select-control")
          .style(
            Style::new()
              .position(Position::Relative)
              .width(396)
              .height(106)
              .flex_shrink(0.0)
              .align_items(Align::Center)
              .translate(Translate::two_dimensional(
                Length::Px(0.0),
                Length::Px(self.offset_y),
              )),
          )
          .child((
            View::new()
              .name("select-frame")
              .element_ref(anchor.clone())
              .style(
                Style::new()
                  .position(Position::Relative)
                  .width(396)
                  .height(106),
              )
              .child(
                interaction
                  .button(
                    ButtonHost::new(tx("", "Resolution selector interface label."))
                      .name("select-trigger")
                      .associated_control(trigger),
                  )
                  .style(
                    Style::new()
                      .position(Position::Relative)
                      .width(100.pct())
                      .height(100.pct())
                      .flex_direction(FlexDirection::Row)
                      .align_items(Align::Center)
                      .margin(0)
                      .padding_top(0)
                      .padding_bottom(0)
                      .padding_left(39)
                      .padding_right(74)
                      .border_width(0)
                      .background_color(Color::TRANSPARENT)
                      .color(Color::rgb8(245, 246, 251))
                      .unity_font_definition(VALUE_FONT)
                      .font_size(60)
                      .unity_text_align(TextAnchor::MiddleLeft),
                  )
                  .paint(
                    PaintStyle::new()
                      .background(self::border(false))
                      .paint_filter(self::filter(false))
                      .clip_polygon(self::clip(10.0))
                      .layer(
                        PaintLayer::new(Color::hex(0x020611))
                          .bounds_inset(3.0)
                          .clip_polygon(self::clip(7.0)),
                      ),
                  )
                  .initial(false)
                  .animate(self::target(interaction.state))
                  .child((
                    control_behavior::name_source_text(ls(self.value.clone()))
                      .name("select-value")
                      .element_ref(value_label.reference()),
                    Caret::new().is_open(open),
                  )),
              ),
            (open && self.overlay.is_some()).then(|| {
              (
                Overlay::layer(self.overlay.clone().unwrap()).child(
                  View::new()
                    .name("select-dismiss-layer")
                    .style(Style::new().width(100.pct()).height(100.pct()))
                    .on_click(set_open.callback().map_input(|()| false)),
                ),
                Overlay::popover(self.overlay.clone().unwrap(), anchor)
                  .host_name("select-popover")
                  .placement(PopoverPlacement::bottom_start().offset(6.0))
                  .style(Style::new().width(396.0 * popover_scale).height(250))
                  .child(
                    ListBox::new(tx(
                      "Display Mode options",
                      "Display mode options interface label.",
                    ))
                    .host_name("select-listbox")
                    .style(
                      Style::new()
                        .width(396)
                        .height(250)
                        .padding_top(11)
                        .padding_bottom(11)
                        .padding_left(9)
                        .padding_right(9)
                        .scale(Scale::uniform(popover_scale))
                        .transform_origin(TransformOrigin::two_dimensional(
                          Length::Px(0.0),
                          Length::Px(0.0),
                        )),
                    )
                    .configure_host(|host| host.paint(self::popover_paint()))
                    .child(
                      self
                        .options
                        .iter()
                        .map(|option| {
                          ListBoxOption::new(ls(option.clone()), option == &self.value)
                            .host_name(format!("select-option-{}", option.to_ascii_lowercase()))
                            .style(self::option_style())
                            .hover_style(
                              Style::new().background_color(Color::rgba8(11, 113, 207, 128)),
                            )
                            .on_press(
                              self
                                .on_change
                                .clone()
                                .map_input({
                                  let option = option.clone();
                                  move |()| option.clone()
                                })
                                .then(set_open.callback().map_input(|()| false)),
                            )
                            .child(
                              View::decorative()
                                .name("select-option-mark")
                                .picking_mode(PickingMode::Ignore)
                                .style(
                                  Style::new()
                                    .position(Position::Absolute)
                                    .right(20)
                                    .top(16)
                                    .width(48)
                                    .height(44)
                                    .flex_shrink(0.0),
                                )
                                .child(
                                  (option == &self.value).then(|| CheckMark::new().scale(0.62)),
                                ),
                            )
                        })
                        .collect::<Vec<_>>(),
                    ),
                  ),
              )
            }),
          )),
      )
      .associated_label(label)
      .first(self.first)
      .row_height(self.row_height)
  }
}

fn option_style() -> Style {
  Style::new()
    .position(Position::Relative)
    .width(100.pct())
    .height(76)
    .min_height(76)
    .flex_direction(FlexDirection::Row)
    .align_items(Align::Center)
    .justify_content(Justify::SpaceBetween)
    .padding_top(6)
    .padding_bottom(6)
    .padding_left(25)
    .padding_right(20)
    .border_width(0)
    .background_color(Color::TRANSPARENT)
    .color(Color::hex(0xd9e1f2))
    .unity_font_definition(VALUE_FONT)
    .font_size(47)
    .unity_text_align(TextAnchor::MiddleLeft)
}

fn popover_paint() -> PaintStyle {
  PaintStyle::new()
    .background(
      Gradient::linear(145.0)
        .stop(0.0, Color::hex(0x5df5ff))
        .stop(0.48, Color::hex(0x718cff))
        .stop(1.0, Color::hex(0xff4bc9)),
    )
    .paint_filter(
      PaintFilterList::default()
        .drop_shadow(PaintDropShadow::new(
          0.0,
          10.0,
          14.0,
          0.0,
          Color::BLACK.with_alpha(0.72),
        ))
        .drop_shadow(PaintDropShadow::new(
          0.0,
          0.0,
          8.0,
          0.0,
          Color::hex(0x2b7eff).with_alpha(0.65),
        )),
    )
    .clip_polygon(self::clip(10.0))
    .layer(
      PaintLayer::new(
        Gradient::linear(180.0)
          .stop(0.0, Color::hex(0x07152e))
          .stop(1.0, Color::hex(0x020611)),
      )
      .bounds_inset(3.0)
      .clip_polygon(self::clip(7.0)),
    )
}

fn border(highlighted: bool) -> Gradient {
  if highlighted {
    Gradient::linear(16.0)
      .stop(0.0, Color::hex(0xb5ffff))
      .stop(0.48, Color::hex(0xd3ddff))
      .stop(1.0, Color::hex(0xff75dc))
  } else {
    Gradient::linear(16.0)
      .stop(0.0, Color::hex(0x5df5ff))
      .stop(0.48, Color::hex(0xa5cbff))
      .stop(1.0, Color::hex(0xff4bc9))
  }
}

fn filter(highlighted: bool) -> PaintFilterList {
  PaintFilterList::default()
    .brightness(if highlighted { 1.12 } else { 1.0 })
    .drop_shadow(PaintDropShadow::new(
      0.0,
      0.0,
      if highlighted { 13.0 } else { 6.0 },
      0.0,
      if highlighted {
        Color::hex(0x53e2ff).with_alpha(0.78)
      } else {
        Color::hex(0x2a67ff).with_alpha(0.38)
      },
    ))
}

fn target(state: use_interaction::InteractionState) -> MotionTarget {
  let highlighted = state.hovered || state.focus_visible;
  MotionTarget::new(
    StyleTarget::new()
      .background_gradient(if state.focus_visible {
        use_interaction::focus_gradient(110.0)
      } else {
        self::border(highlighted)
      })
      .paint_filter(if state.focus_visible {
        use_interaction::focus_filter()
      } else {
        self::filter(highlighted)
      })
      .scale(if state.pressed && !state.reduced_motion {
        0.965
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

fn clip(cut: f32) -> Vec<[Length; 2]> {
  let near = Length::px(cut);
  let far = Length::calc(-cut, 100.0);
  let zero = Length::px(0.0);
  let full = Length::percent(100.0);
  vec![
    [near, zero],
    [far, zero],
    [full, near],
    [full, far],
    [far, full],
    [near, full],
    [zero, far],
    [zero, near],
  ]
}
