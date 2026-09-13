//! Verified retained decoding for static UI paint values.

use battlement::{FilterFunction, FilterList, PaintFill, PaintLayer, PaintStyle, Prop};
use battlement_flatbuffers::schema_generated::common_generated::battlement::flat_buffers::generated::RgbaColor;
use battlement_flatbuffers::schema_generated::ui_generated::battlement::flat_buffers::generated as wire;

pub(crate) fn paint(value: wire::UiProperty<'_>) -> Result<Prop<PaintStyle>, String> {
  Ok(match value.state() {
    wire::PropState::Unset => Prop::Unset,
    wire::PropState::Reset => Prop::Reset,
    wire::PropState::Set => {
      let value = value
        .value_as_paint_style_property_value()
        .ok_or_else(|| "UI paint payload is missing".to_owned())?;
      let mut result = PaintStyle::new();
      if let Some(value) = value.subtree_clip() {
        result = result.subtree_clip(clip_path(value)?);
      }
      if value.has_blend_mode() {
        result = result.blend_mode(blend_mode(value.blend_mode())?);
      }
      if let Some(value) = value.background() {
        result = result.background(fill(value)?);
      }
      if let Some(values) = value.filters() {
        result = result.paint_filter(filters(values)?);
      }
      if let Some(values) = value.clip_polygon() {
        result = result.clip_polygon(points(values)?);
      }
      if let Some(values) = value.box_shadows() {
        result = result.box_shadow(values.iter().map(shadow).collect::<Result<Vec<_>, _>>()?);
      }
      if let Some(value) = value.clip_insets() {
        result = result.clip_inset(inset_tuple(value)?);
      }
      for value in value.layers() {
        result = result.layer(layer(value)?);
      }
      Prop::Set(result)
    }
    _ => return Err("UI paint property state is unknown".to_owned()),
  })
}

fn layer(value: wire::PaintLayerValue<'_>) -> Result<PaintLayer, String> {
  let mut result = PaintLayer::new(fill(value.background())?);
  if let Some(values) = value.filters() {
    result = result.paint_filter(filters(values)?);
  }
  if let Some(values) = value.clip_polygon() {
    result = result.clip_polygon(points(values)?);
  }
  if let Some(values) = value.box_shadows() {
    result = result.box_shadow(values.iter().map(shadow).collect::<Result<Vec<_>, _>>()?);
  }
  if let Some(value) = value.clip_insets() {
    result = result.clip_inset(inset_tuple(value)?);
  }
  if let Some(value) = value.bounds_insets() {
    result = result.bounds_inset(inset_tuple(value)?);
  }
  Ok(result)
}

fn fill(value: wire::PaintFillValue<'_>) -> Result<PaintFill, String> {
  Ok(match value.kind() {
    wire::PaintFillKind::Color => PaintFill::Color(color(
      value
        .color()
        .ok_or_else(|| "UI paint color is missing".to_owned())?,
    )),
    wire::PaintFillKind::Gradient => PaintFill::Gradient(gradient(
      value
        .gradient()
        .ok_or_else(|| "UI paint gradient is missing".to_owned())?,
    )?),
    _ => return Err("UI paint fill kind is unknown".to_owned()),
  })
}

fn gradient(value: wire::PaintGradientValue<'_>) -> Result<battlement::Gradient, String> {
  let stops = value
    .stops()
    .iter()
    .map(|value| battlement::GradientStop {
      color: color(value.color()),
      position: value.position(),
    })
    .collect();
  Ok(match value.kind() {
    wire::PaintGradientKind::Linear => battlement::Gradient::Linear {
      angle: value.angle(),
      stops,
    },
    wire::PaintGradientKind::Radial => {
      let center = value
        .center()
        .ok_or_else(|| "UI radial paint center is missing".to_owned())?;
      let radius = value
        .radius()
        .ok_or_else(|| "UI radial paint radius is missing".to_owned())?;
      battlement::Gradient::Radial {
        center: [center.x(), center.y()],
        radius: [radius.x(), radius.y()],
        stops,
      }
    }
    _ => return Err("UI paint gradient kind is unknown".to_owned()),
  })
}

fn filters(
  values: flatbuffers::Vector<'_, flatbuffers::ForwardsUOffset<wire::PaintFilterValue<'_>>>,
) -> Result<FilterList, String> {
  Ok(FilterList::new(
    values
      .iter()
      .map(|value| match value.kind() {
        wire::PaintFilterKind::Brightness => Ok(FilterFunction::Brightness(value.amount())),
        wire::PaintFilterKind::DropShadow => Ok(FilterFunction::DropShadow(shadow(
          value
            .shadow()
            .ok_or_else(|| "UI paint drop shadow is missing".to_owned())?,
        )?)),
        _ => Err("UI paint filter kind is unknown".to_owned()),
      })
      .collect::<Result<Vec<_>, _>>()?,
  ))
}

fn clip_path(value: wire::PaintClipPathValue<'_>) -> Result<battlement::PaintClipPath, String> {
  let mut result = battlement::PaintClipPath::new(match value.fill_rule() {
    wire::PaintFillRule::NonZero => battlement::PaintFillRule::NonZero,
    wire::PaintFillRule::EvenOdd => battlement::PaintFillRule::EvenOdd,
    _ => return Err("UI paint fill rule is unknown".to_owned()),
  });
  for value in value.contours() {
    result = result.contour(points(value.points())?);
  }
  Ok(result)
}

fn points(
  values: flatbuffers::Vector<'_, flatbuffers::ForwardsUOffset<wire::PaintLengthPoint<'_>>>,
) -> Result<Vec<[battlement::Length; 2]>, String> {
  values
    .iter()
    .map(|value| Ok([length(value.x())?, length(value.y())?]))
    .collect()
}

fn inset_tuple(
  value: wire::PaintInsetsValue<'_>,
) -> Result<
  (
    battlement::Length,
    battlement::Length,
    battlement::Length,
    battlement::Length,
  ),
  String,
> {
  Ok((
    length(value.top())?,
    length(value.right())?,
    length(value.bottom())?,
    length(value.left())?,
  ))
}

fn length(value: wire::LengthPropertyValue<'_>) -> Result<battlement::Length, String> {
  Ok(match value.kind() {
    wire::LengthKind::Pixels => battlement::Length::px(value.pixels()),
    wire::LengthKind::Percent => battlement::Length::percent(value.percentage()),
    wire::LengthKind::Calc => battlement::Length::calc(value.pixels(), value.percentage()),
    _ => return Err("UI paint length kind is unknown".to_owned()),
  })
}

fn shadow(value: wire::PaintShadowValue<'_>) -> Result<battlement::Shadow, String> {
  Ok(battlement::Shadow {
    x: value.x(),
    y: value.y(),
    blur: value.blur(),
    spread: value.spread(),
    color: color(value.color()),
    inset: value.inset(),
  })
}

fn color(value: &RgbaColor) -> battlement::Color {
  battlement::Color {
    r: value.r(),
    g: value.g(),
    b: value.b(),
    a: value.a(),
  }
}

fn blend_mode(value: wire::PaintBlendMode) -> Result<battlement::PaintBlendMode, String> {
  Ok(match value {
    wire::PaintBlendMode::Normal => battlement::PaintBlendMode::Normal,
    wire::PaintBlendMode::Screen => battlement::PaintBlendMode::Screen,
    wire::PaintBlendMode::Additive => battlement::PaintBlendMode::Additive,
    _ => return Err("UI paint blend mode is unknown".to_owned()),
  })
}
