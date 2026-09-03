//! Static decorative paint for host backgrounds and clipping.

use battlement::{
  MotionColor, MotionGradient, MotionLength, MotionProperty, MotionPropertyValue, MotionShadow,
  MotionValue,
};

/// A solid or gradient background painted inside a host's clip geometry.
#[derive(Clone, Debug, PartialEq)]
pub enum PaintFill {
  /// A uniform color.
  Color(MotionColor),
  /// A typed linear or radial gradient.
  Gradient(MotionGradient),
}

/// Static paint in host border-box coordinates, below animated presentation.
/// Fills follow the host's resolved corner radii unless an explicit polygon is supplied.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaintStyle {
  background: Option<PaintFill>,
  clip_polygon: Option<Vec<[MotionLength; 2]>>,
  box_shadow: Option<Vec<MotionShadow>>,
}

impl PaintStyle {
  /// Creates an empty static paint value.
  #[must_use]
  pub const fn new() -> Self {
    Self {
      background: None,
      clip_polygon: None,
      box_shadow: None,
    }
  }

  /// Sets the background fill.
  #[must_use]
  pub fn background(mut self, value: PaintFill) -> Self {
    self.background = Some(value);
    self
  }

  /// Clips the painted surface to a polygon without changing child layout.
  #[must_use]
  pub fn clip_polygon(mut self, value: impl IntoIterator<Item = [MotionLength; 2]>) -> Self {
    self.clip_polygon = Some(value.into_iter().collect());
    self
  }

  /// Sets outer or inset shadows.
  #[must_use]
  pub fn box_shadow(mut self, value: impl IntoIterator<Item = MotionShadow>) -> Self {
    self.box_shadow = Some(value.into_iter().collect());
    self
  }

  pub(crate) fn values(&self) -> Vec<MotionPropertyValue> {
    let mut values = Vec::new();
    if let Some(value) = &self.background {
      values.push(match value {
        PaintFill::Color(color) => MotionPropertyValue {
          property: MotionProperty::BackgroundColor,
          value: MotionValue::Color(*color),
        },
        PaintFill::Gradient(gradient) => MotionPropertyValue {
          property: MotionProperty::BackgroundGradient,
          value: MotionValue::Gradient(gradient.clone()),
        },
      });
    }
    if let Some(value) = &self.clip_polygon {
      values.push(MotionPropertyValue {
        property: MotionProperty::ClipPolygon,
        value: MotionValue::ClipPolygon(value.clone()),
      });
    }
    if let Some(value) = &self.box_shadow {
      values.push(MotionPropertyValue {
        property: MotionProperty::BoxShadow,
        value: MotionValue::ShadowList(value.clone()),
      });
    }
    values
  }
}
