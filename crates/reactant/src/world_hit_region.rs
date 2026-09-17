//! Independent world pointer geometry.

use crate::world_object::WorldObject;
use battlement::{BoxHitRegionState, GameObjectKind, Vector3};

/// A renderer-free local box, independent of a card's visual children.
pub type BoxHitRegion = WorldObject<BoxHitRegionState>;

impl BoxHitRegion {
  /// Creates a centered unit box; handlers participate in the logical tree.
  pub fn new() -> Self {
    Self::with_properties(BoxHitRegionState::default(), |region| {
      assert!(region.is_valid(), "invalid box hit-region geometry");
      GameObjectKind::BoxHitRegion { region: *region }
    })
  }
  /// Sets positive local dimensions; parent transforms apply normally.
  pub fn size(mut self, size: Vector3) -> Self {
    self.properties.size = size;
    self
  }
  /// Sets the local collider center independently of visual geometry.
  pub fn center(mut self, center: Vector3) -> Self {
    self.properties.center = center;
    self
  }
}
impl Default for BoxHitRegion {
  fn default() -> Self {
    Self::new()
  }
}
