//! Renderer-independent pointer geometry.

use crate::Vector3;

/// An axis-aligned box in its owning object's local coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxHitRegionState {
  /// Strictly positive dimensions in world units before the object transform.
  pub size: Vector3,
  /// Local offset from the object's origin.
  pub center: Vector3,
}

impl Default for BoxHitRegionState {
  fn default() -> Self {
    Self {
      size: Vector3::new(1.0, 1.0, 1.0),
      center: Vector3::new(0.0, 0.0, 0.0),
    }
  }
}

impl BoxHitRegionState {
  /// Whether the geometry fits finite native collider coordinates.
  pub fn is_valid(&self) -> bool {
    let size = [self.size.x, self.size.y, self.size.z];
    let center = [self.center.x, self.center.y, self.center.z];
    size
      .iter()
      .all(|v| *v <= f64::from(f32::MAX) && (*v as f32) > 0.0)
      && center
        .iter()
        .all(|v| v.is_finite() && v.abs() <= f64::from(f32::MAX))
  }
}
