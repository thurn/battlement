//! Camera and light descriptions lowered to the existing standard components.

use battlement::{
  CameraClearMode, CameraProjection, CameraState, Color, GameObjectKind, LightState, LightType,
  ShadowMode,
};

use crate::world_object::WorldObject;

/// A standard camera; application input-camera selection remains explicit.
pub type Camera = WorldObject<CameraState>;
/// A standard directional, point, or spot light.
pub type Light = WorldObject<LightState>;

impl Camera {
  /// Creates a camera with protocol defaults.
  pub fn new() -> Self {
    Self::with_properties(CameraState::default(), |camera| GameObjectKind::Camera {
      camera: *camera,
    })
  }
  /// Sets the complete standard camera state.
  pub fn state(mut self, state: CameraState) -> Self {
    self.properties = state;
    self
  }
  /// Enables the Camera component independently of object activation.
  pub fn enabled(mut self, enabled: bool) -> Self {
    self.properties.enabled = enabled;
    self
  }
  /// Sets orthographic projection and its positive half-height in world units.
  pub fn orthographic(mut self, size: f64) -> Self {
    self.properties.projection = CameraProjection::Orthographic;
    self.properties.orthographic_size = size;
    self
  }
  /// Sets perspective projection and vertical field of view in degrees.
  pub fn perspective(mut self, field_of_view: f64) -> Self {
    self.properties.projection = CameraProjection::Perspective;
    self.properties.field_of_view = field_of_view;
    self
  }
  /// Sets the positive near and farther clipping distances.
  pub fn clipping(mut self, near: f64, far: f64) -> Self {
    self.properties.near = near;
    self.properties.far = far;
    self
  }
  /// Clears the camera background to the given linear color.
  pub fn background(mut self, color: Color) -> Self {
    self.properties.clear_mode = CameraClearMode::SolidColor;
    self.properties.clear_color = color;
    self
  }
}

impl Default for Camera {
  fn default() -> Self {
    Self::new()
  }
}

impl Light {
  /// Creates a light with protocol defaults.
  pub fn new() -> Self {
    Self::with_properties(LightState::default(), |light| GameObjectKind::Light {
      light: *light,
    })
  }
  /// Sets the complete standard light state.
  pub fn state(mut self, state: LightState) -> Self {
    self.properties = state;
    self
  }
  /// Enables the Light component independently of object activation.
  pub fn enabled(mut self, enabled: bool) -> Self {
    self.properties.enabled = enabled;
    self
  }
  /// Selects directional, point, or spot lighting.
  pub fn light_type(mut self, light_type: LightType) -> Self {
    self.properties.light_type = light_type;
    self
  }
  /// Sets linear light color.
  pub fn color(mut self, color: Color) -> Self {
    self.properties.color = color;
    self
  }
  /// Sets nonnegative light intensity.
  pub fn intensity(mut self, intensity: f64) -> Self {
    self.properties.intensity = intensity;
    self
  }
  /// Sets positive range for point or spot lighting.
  pub fn range(mut self, range: f64) -> Self {
    self.properties.range = range;
    self
  }
  /// Sets the spot cone's inner and outer angles in degrees.
  pub fn spot_angles(mut self, inner: f64, outer: f64) -> Self {
    self.properties.inner_spot_angle = inner;
    self.properties.outer_spot_angle = outer;
    self
  }
  /// Selects the standard shadow mode.
  pub fn shadows(mut self, shadows: ShadowMode) -> Self {
    self.properties.shadows = shadows;
    self
  }
}

impl Default for Light {
  fn default() -> Self {
    Self::new()
  }
}
