use battlement_types::Color;
use serde::{Deserialize, Serialize};

use crate::{FilterFunction, FilterList, Length};

/// A solid or gradient background painted inside an element's clip geometry.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum PaintFill {
  /// A uniform color.
  Color(Color),
  /// A typed linear or radial gradient.
  Gradient(Gradient),
}

impl From<Color> for PaintFill {
  fn from(value: Color) -> Self {
    Self::Color(value)
  }
}

impl From<Gradient> for PaintFill {
  fn from(value: Gradient) -> Self {
    Self::Gradient(value)
  }
}

/// Static decorative paint in element border-box coordinates.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct PaintStyle {
  background: Option<PaintFill>,
  paint_filter: Option<FilterList>,
  clip_polygon: Option<Vec<[Length; 2]>>,
  box_shadow: Option<Vec<Shadow>>,
  clip_inset: Option<[Length; 4]>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  layers: Vec<PaintLayer>,
}

/// One additional static paint layer drawn over a host's primary paint.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PaintLayer {
  background: PaintFill,
  paint_filter: Option<FilterList>,
  clip_polygon: Option<Vec<[Length; 2]>>,
  box_shadow: Option<Vec<Shadow>>,
  clip_inset: Option<[Length; 4]>,
  bounds_inset: Option<[Length; 4]>,
}

impl PaintStyle {
  /// Creates an empty paint value.
  #[must_use]
  pub const fn new() -> Self {
    Self {
      background: None,
      paint_filter: None,
      clip_polygon: None,
      box_shadow: None,
      clip_inset: None,
      layers: Vec::new(),
    }
  }

  /// Creates paint with one background fill.
  #[must_use]
  pub fn fill(value: impl Into<PaintFill>) -> Self {
    Self::new().background(value)
  }

  /// Sets the background fill.
  #[must_use]
  pub fn background(mut self, value: impl Into<PaintFill>) -> Self {
    self.background = Some(value.into());
    self
  }

  /// Filters only this host's owned decorative paint.
  #[must_use]
  pub fn paint_filter(mut self, value: impl Into<FilterList>) -> Self {
    self.paint_filter = Some(value.into());
    self
  }

  /// Clips the painted surface to a polygon.
  #[must_use]
  pub fn clip_polygon(mut self, value: impl IntoIterator<Item = [Length; 2]>) -> Self {
    self.clip_polygon = Some(value.into_iter().collect());
    self
  }

  /// Sets outer or inset decorative-surface shadows.
  #[must_use]
  pub fn box_shadow(mut self, value: impl IntoIterator<Item = Shadow>) -> Self {
    self.box_shadow = Some(value.into_iter().collect());
    self
  }

  /// Clips the painted surface to border-box insets.
  #[must_use]
  pub fn clip_inset(mut self, value: impl IntoPaintInsets) -> Self {
    self.clip_inset = Some(value.into_paint_insets());
    self
  }

  /// Appends one static foreground layer in authored order.
  #[must_use]
  pub fn layer(mut self, value: PaintLayer) -> Self {
    self.layers.push(value);
    self
  }

  /// Appends static foreground layers in authored order.
  #[must_use]
  pub fn layers(mut self, values: impl IntoIterator<Item = PaintLayer>) -> Self {
    self.layers.extend(values);
    self
  }

  /// Returns the configured background fill.
  #[must_use]
  pub fn background_fill(&self) -> Option<&PaintFill> {
    self.background.as_ref()
  }

  /// Returns filters applied to the owned decorative paint.
  #[must_use]
  pub fn paint_filters(&self) -> Option<&FilterList> {
    self.paint_filter.as_ref()
  }

  /// Returns the configured clip polygon.
  #[must_use]
  pub fn clip_polygon_value(&self) -> Option<&[[Length; 2]]> {
    self.clip_polygon.as_deref()
  }

  /// Returns the configured box shadows.
  #[must_use]
  pub fn box_shadows(&self) -> Option<&[Shadow]> {
    self.box_shadow.as_deref()
  }

  /// Returns the configured clip insets.
  #[must_use]
  pub const fn clip_insets(&self) -> Option<&[Length; 4]> {
    self.clip_inset.as_ref()
  }

  /// Returns the additional static foreground layers.
  #[must_use]
  pub fn paint_layers(&self) -> &[PaintLayer] {
    &self.layers
  }

  pub(crate) fn is_valid(&self) -> bool {
    let background_valid = self.background.as_ref().is_none_or(|value| match value {
      PaintFill::Color(value) => [value.r, value.g, value.b, value.a]
        .into_iter()
        .all(f64::is_finite),
      PaintFill::Gradient(value) => gradient_is_valid(value),
    });
    let clip_valid = self.clip_polygon.as_ref().is_none_or(|value| {
      value.len() >= 3 && value.iter().flatten().all(|value| value.is_finite())
    });
    let filters_valid = self.paint_filter.as_ref().is_none_or(|filters| {
      let mut drop_shadows = 0;
      filters.as_slice().iter().all(|filter| match filter {
        FilterFunction::Brightness(value) => value.is_finite() && *value >= 0.0,
        FilterFunction::DropShadow(value) => {
          drop_shadows += 1;
          if !shadow_is_finite(*value) || value.inset {
            return false;
          }
          drop_shadows == 1
        }
        _ => false,
      })
    });
    if !background_valid || !clip_valid {
      return false;
    }
    if !filters_valid || (self.paint_filter.is_some() && self.background.is_none()) {
      return false;
    }
    if self
      .clip_inset
      .is_some_and(|values| values.iter().any(|value| !value.is_finite()))
    {
      return false;
    }
    let layers_valid = self.layers.iter().all(PaintLayer::is_valid);
    layers_valid
      && self
        .box_shadow
        .as_ref()
        .is_none_or(|values| values.iter().all(|value| shadow_is_finite(*value)))
  }
}

impl PaintLayer {
  /// Creates one layer with its required fill.
  #[must_use]
  pub fn new(background: impl Into<PaintFill>) -> Self {
    Self {
      background: background.into(),
      paint_filter: None,
      clip_polygon: None,
      box_shadow: None,
      clip_inset: None,
      bounds_inset: None,
    }
  }

  /// Filters this layer's fill.
  #[must_use]
  pub fn paint_filter(mut self, value: impl Into<FilterList>) -> Self {
    self.paint_filter = Some(value.into());
    self
  }

  /// Clips this layer to a polygon in its inset bounds.
  #[must_use]
  pub fn clip_polygon(mut self, value: impl IntoIterator<Item = [Length; 2]>) -> Self {
    self.clip_polygon = Some(value.into_iter().collect());
    self
  }

  /// Sets outer or inset shadows for this layer.
  #[must_use]
  pub fn box_shadow(mut self, value: impl IntoIterator<Item = Shadow>) -> Self {
    self.box_shadow = Some(value.into_iter().collect());
    self
  }

  /// Insets this layer's rounded clip without changing its coordinate space.
  #[must_use]
  pub fn clip_inset(mut self, value: impl IntoPaintInsets) -> Self {
    self.clip_inset = Some(value.into_paint_insets());
    self
  }

  /// Insets the layer's coordinate space inside the host border box.
  #[must_use]
  pub fn bounds_inset(mut self, value: impl IntoPaintInsets) -> Self {
    self.bounds_inset = Some(value.into_paint_insets());
    self
  }

  fn is_valid(&self) -> bool {
    let background_valid = match &self.background {
      PaintFill::Color(value) => [value.r, value.g, value.b, value.a]
        .into_iter()
        .all(f64::is_finite),
      PaintFill::Gradient(value) => gradient_is_valid(value),
    };
    let polygon_valid = self.clip_polygon.as_ref().is_none_or(|value| {
      value.len() >= 3 && value.iter().flatten().all(|value| value.is_finite())
    });
    let filters_valid = self.paint_filter.as_ref().is_none_or(|filters| {
      let mut drop_shadows = 0;
      filters.as_slice().iter().all(|filter| match filter {
        FilterFunction::Brightness(value) => value.is_finite() && *value >= 0.0,
        FilterFunction::DropShadow(value) => {
          drop_shadows += 1;
          !value.inset && shadow_is_finite(*value) && drop_shadows == 1
        }
        _ => false,
      })
    });
    let insets_valid = self
      .clip_inset
      .iter()
      .chain(self.bounds_inset.iter())
      .flatten()
      .all(|value| value.is_finite());
    let shadows_valid = self
      .box_shadow
      .as_ref()
      .is_none_or(|values| values.iter().all(|value| shadow_is_finite(*value)));
    background_valid && polygon_valid && filters_valid && insets_valid && shadows_valid
  }
}

/// Converts CSS-order shorthand values to paint insets.
pub trait IntoPaintInsets {
  /// Expands the shorthand to top, right, bottom, and left lengths.
  fn into_paint_insets(self) -> [Length; 4];
}

macro_rules! paint_insets {
  ($source:ty) => {
    impl IntoPaintInsets for $source {
      fn into_paint_insets(self) -> [Length; 4] {
        let value = self.into();
        [value; 4]
      }
    }

    impl IntoPaintInsets for ($source, $source) {
      fn into_paint_insets(self) -> [Length; 4] {
        [self.0.into(), self.1.into(), self.0.into(), self.1.into()]
      }
    }

    impl IntoPaintInsets for ($source, $source, $source) {
      fn into_paint_insets(self) -> [Length; 4] {
        [self.0.into(), self.1.into(), self.2.into(), self.1.into()]
      }
    }

    impl IntoPaintInsets for ($source, $source, $source, $source) {
      fn into_paint_insets(self) -> [Length; 4] {
        [self.0.into(), self.1.into(), self.2.into(), self.3.into()]
      }
    }
  };
}

paint_insets!(i32);
paint_insets!(u32);
paint_insets!(f32);
paint_insets!(Length);

/// One typed operation in an authored transform list.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum TransformOperation {
  /// Three-axis translation.
  Translate([Length; 3]),
  /// Three-axis rotation in degrees.
  Rotate([f32; 3]),
  /// Two-axis skew in degrees.
  Skew([f32; 2]),
  /// Three-axis scale.
  Scale([f32; 3]),
}

/// Fluent ordered transform operations for motion targets.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TransformList(Vec<TransformOperation>);

/// One outer or inset painted shadow.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Shadow {
  /// Horizontal offset in pixels.
  pub x: f32,
  /// Vertical offset in pixels.
  pub y: f32,
  /// Blur radius in pixels.
  pub blur: f32,
  /// Spread radius in pixels.
  pub spread: f32,
  /// Shadow color.
  pub color: Color,
  /// Whether the shadow paints inward.
  pub inset: bool,
}

/// One gradient color stop.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct GradientStop {
  /// Stop color.
  pub color: Color,
  /// Normalized stop position.
  pub position: f32,
}

/// A linear or radial painted gradient.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum Gradient {
  /// Linear gradient with an angle in degrees.
  Linear {
    /// Gradient direction.
    angle: f32,
    /// Ordered normalized color stops.
    stops: Vec<GradientStop>,
  },
  /// Radial gradient with normalized center and radii.
  Radial {
    /// Normalized center point.
    center: [f32; 2],
    /// Normalized horizontal and vertical radii.
    radius: [f32; 2],
    /// Ordered normalized color stops.
    stops: Vec<GradientStop>,
  },
}

impl TransformOperation {
  /// Creates a three-axis translation.
  #[must_use]
  pub const fn translate(x: Length, y: Length, z: Length) -> Self {
    Self::Translate([x, y, z])
  }

  /// Creates a two-dimensional translation with zero depth.
  #[must_use]
  pub const fn translate_2d(x: Length, y: Length) -> Self {
    Self::translate(x, y, Length::Px(0.0))
  }

  /// Creates a three-axis rotation in degrees.
  #[must_use]
  pub const fn rotate(x: f32, y: f32, z: f32) -> Self {
    Self::Rotate([x, y, z])
  }

  /// Creates a rotation around the z axis.
  #[must_use]
  pub const fn rotate_z(degrees: f32) -> Self {
    Self::rotate(0.0, 0.0, degrees)
  }

  /// Creates a two-axis skew in degrees.
  #[must_use]
  pub const fn skew(x: f32, y: f32) -> Self {
    Self::Skew([x, y])
  }

  /// Creates a three-axis scale.
  #[must_use]
  pub const fn scale(x: f32, y: f32, z: f32) -> Self {
    Self::Scale([x, y, z])
  }

  /// Creates a uniform three-axis scale.
  #[must_use]
  pub const fn scale_uniform(value: f32) -> Self {
    Self::scale(value, value, value)
  }
}

impl TransformList {
  /// Creates an empty transform list.
  #[must_use]
  pub const fn new() -> Self {
    Self(Vec::new())
  }

  /// Returns transform operations in evaluation order.
  #[must_use]
  pub fn as_slice(&self) -> &[TransformOperation] {
    &self.0
  }

  /// Appends one transform operation.
  #[must_use]
  pub fn operation(mut self, value: TransformOperation) -> Self {
    self.0.push(value);
    self
  }

  /// Appends a two-dimensional translation.
  #[must_use]
  pub fn translate(self, x: Length, y: Length) -> Self {
    self.operation(TransformOperation::translate_2d(x, y))
  }

  /// Appends a rotation around the z axis.
  #[must_use]
  pub fn rotate(self, degrees: f32) -> Self {
    self.operation(TransformOperation::rotate_z(degrees))
  }

  /// Appends a two-axis skew.
  #[must_use]
  pub fn skew(self, x: f32, y: f32) -> Self {
    self.operation(TransformOperation::skew(x, y))
  }

  /// Appends a uniform three-axis scale.
  #[must_use]
  pub fn scale(self, value: f32) -> Self {
    self.operation(TransformOperation::scale_uniform(value))
  }

  /// Appends another ordered transform list.
  #[must_use]
  pub fn then(mut self, value: Self) -> Self {
    self.0.extend(value.0);
    self
  }
}

impl IntoIterator for TransformList {
  type Item = TransformOperation;
  type IntoIter = std::vec::IntoIter<TransformOperation>;

  fn into_iter(self) -> Self::IntoIter {
    self.0.into_iter()
  }
}

impl FromIterator<TransformOperation> for TransformList {
  fn from_iter<T: IntoIterator<Item = TransformOperation>>(iter: T) -> Self {
    Self(iter.into_iter().collect())
  }
}

impl Shadow {
  /// Creates an outer painted shadow.
  #[must_use]
  pub const fn outer(x: f32, y: f32, blur: f32, spread: f32, color: Color) -> Self {
    Self {
      x,
      y,
      blur,
      spread,
      color,
      inset: false,
    }
  }

  /// Creates an inset painted shadow.
  #[must_use]
  pub const fn inset(x: f32, y: f32, blur: f32, spread: f32, color: Color) -> Self {
    Self {
      inset: true,
      ..Self::outer(x, y, blur, spread, color)
    }
  }
}

impl GradientStop {
  /// Creates one normalized gradient stop.
  #[must_use]
  pub const fn new(position: f32, color: Color) -> Self {
    Self { color, position }
  }
}

impl Gradient {
  /// Starts a linear gradient at the supplied angle in degrees.
  #[must_use]
  pub const fn linear(angle: f32) -> Self {
    Self::Linear {
      angle,
      stops: Vec::new(),
    }
  }

  /// Starts a radial gradient with normalized center and radii.
  #[must_use]
  pub const fn radial(center: [f32; 2], radius: [f32; 2]) -> Self {
    Self::Radial {
      center,
      radius,
      stops: Vec::new(),
    }
  }

  /// Appends one color stop in authored order.
  #[must_use]
  pub fn stop(mut self, position: f32, color: Color) -> Self {
    match &mut self {
      Self::Linear { stops, .. } | Self::Radial { stops, .. } => {
        stops.push(GradientStop::new(position, color));
      }
    }
    self
  }

  /// Appends color stops in authored order.
  #[must_use]
  pub fn stops(mut self, values: impl IntoIterator<Item = GradientStop>) -> Self {
    match &mut self {
      Self::Linear { stops, .. } | Self::Radial { stops, .. } => stops.extend(values),
    }
    self
  }
}

fn gradient_is_valid(value: &Gradient) -> bool {
  let (geometry, stops) = match value {
    Gradient::Linear { angle, stops } => (vec![*angle], stops),
    Gradient::Radial {
      center,
      radius,
      stops,
    } => ([center.as_slice(), radius.as_slice()].concat(), stops),
  };
  !stops.is_empty()
    && geometry.into_iter().all(f32::is_finite)
    && stops.iter().all(|stop| {
      stop.position.is_finite()
        && [stop.color.r, stop.color.g, stop.color.b, stop.color.a]
          .into_iter()
          .all(f64::is_finite)
    })
}

fn shadow_is_finite(value: Shadow) -> bool {
  value.blur >= 0.0
    && [value.x, value.y, value.blur, value.spread]
      .into_iter()
      .all(f32::is_finite)
    && [value.color.r, value.color.g, value.color.b, value.color.a]
      .into_iter()
      .all(f64::is_finite)
}
