//! Resting paint for the clipped settings category headers.

use battlement::{Color, Gradient, Length, Shadow};
use battlement_reactant::prelude::{PaintDropShadow, PaintFilterList, PaintLayer, PaintStyle};

/// Paints a category outline and its inset without changing the tab's layout.
pub fn paint(active: bool) -> PaintStyle {
  PaintStyle::new()
    .background(if active {
      Gradient::linear(22.0)
        .stop(0.0, Color::hex(0x72f5ff))
        .stop(0.44, Color::hex(0x53afff))
        .stop(0.68, Color::hex(0x9a83ff))
        .stop(1.0, Color::hex(0xff4ed3))
    } else {
      Gradient::linear(20.0)
        .stop(0.0, Color::hex(0x657287))
        .stop(0.52, Color::hex(0x454f64))
        .stop(1.0, Color::hex(0x6f6577))
    })
    .clip_polygon(self::clip(18.0))
    .paint_filter(if active {
      PaintFilterList::default().drop_shadow(PaintDropShadow::new(
        0.0,
        0.0,
        10.0,
        0.0,
        Color::hex(0x2385ff).with_alpha(0.86),
      ))
    } else {
      PaintFilterList::default()
    })
    .layer(
      PaintLayer::new(
        Gradient::linear(90.0)
          .stop(0.0, Color::hex(if active { 0x071831 } else { 0x071328 }))
          .stop(1.0, Color::hex(if active { 0x030b1d } else { 0x020817 })),
      )
      .bounds_inset(4.0)
      .clip_polygon(self::clip(15.0))
      .box_shadow(if active {
        vec![
          self::inset_shadow(0.0, 34.0, Color::hex(0x1462e2).with_alpha(0.52)),
          self::inset_shadow(-3.0, 0.0, Color::hex(0xf14dd7)),
        ]
      } else {
        vec![self::inset_shadow(0.0, 24.0, Color::BLACK.with_alpha(0.5))]
      }),
    )
}

fn inset_shadow(y: f32, blur: f32, color: Color) -> Shadow {
  Shadow {
    x: 0.0,
    y,
    blur,
    spread: 0.0,
    color,
    inset: true,
  }
}

fn clip(cut: f32) -> [[Length; 2]; 6] {
  let zero = Length::px(0.0);
  let full = Length::percent(100.0);
  [
    [Length::px(cut), zero],
    [Length::calc(-cut, 100.0), zero],
    [full, Length::px(cut)],
    [full, full],
    [zero, full],
    [zero, Length::px(cut)],
  ]
}
