//! Prepared textured surfaces and authored mesh geometry.

use battlement::RenderOrder;

use battlement::{
  GameObjectKind, ImageFit, ImageState, MaterialAssignment, MeshAddress, RgbColor, TextureAddress,
};

use crate::world_object::WorldObject;

/// A centered XY texture surface with explicit world-unit dimensions.
pub type Sprite = WorldObject<ImageState>;
/// A prepared mesh in its authored coordinates, with explicit local placement.
pub type Mesh = WorldObject<MeshProperties>;

/// Prepared geometry and material slots for a world mesh.
#[derive(Clone, Default)]
pub struct MeshProperties {
  address: Option<MeshAddress>,
  materials: Vec<MaterialAssignment>,
}

impl Sprite {
  /// Sets renderer order relative to the nearest enclosing sorting group.
  pub fn layer(mut self, order: i16) -> Self {
    self.group.render_order = Some(RenderOrder::Layer(order));
    self
  }

  /// Creates a unit XY surface; set its required texture before rendering.
  pub fn new() -> Self {
    Self::with_properties(ImageState::new("", 1.0, 1.0), |image| {
      assert!(
        !image.texture.as_str().is_empty(),
        "world sprite requires a texture"
      );
      GameObjectKind::Image {
        image: image.clone(),
      }
    })
  }
  /// Sets the prepared texture through the existing image host.
  pub fn texture(mut self, texture: impl Into<TextureAddress>) -> Self {
    self.properties.texture = texture.into();
    self
  }
  /// Sets positive width and height around a centered pivot in the XY plane.
  pub fn size(mut self, width: f64, height: f64) -> Self {
    self.properties.width = width;
    self.properties.height = height;
    self
  }
  /// Sets texture fitting within the declared dimensions.
  pub fn fit(mut self, fit: ImageFit) -> Self {
    self.properties.fit = fit;
    self
  }
  /// Sets linear RGB tint.
  pub fn tint(mut self, tint: RgbColor) -> Self {
    self.properties.tint = tint;
    self
  }
  /// Sets opacity in the inclusive range zero to one.
  pub fn opacity(mut self, opacity: f64) -> Self {
    self.properties.opacity = opacity;
    self
  }
  /// Rotates the surface toward the existing input camera when enabled.
  pub fn face_camera(mut self, enabled: bool) -> Self {
    self.properties.face_camera = enabled;
    self
  }
}

impl Default for Sprite {
  fn default() -> Self {
    Self::new()
  }
}

impl Mesh {
  /// Sets renderer order relative to the nearest enclosing sorting group.
  pub fn layer(mut self, order: i16) -> Self {
    self.group.render_order = Some(RenderOrder::Layer(order));
    self
  }

  /// Creates a mesh declaration; set its required prepared address before rendering.
  pub fn new() -> Self {
    Self::with_properties(MeshProperties::default(), |mesh| GameObjectKind::Mesh {
      address: mesh
        .address
        .clone()
        .expect("world mesh requires a prepared address"),
      materials: mesh.materials.clone(),
    })
  }
  /// Sets prepared geometry without implicit scale or orientation corrections.
  pub fn mesh(mut self, address: impl Into<MeshAddress>) -> Self {
    self.properties.address = Some(address.into());
    self
  }
  /// Sets prepared materials by authored submesh slot.
  pub fn materials(mut self, materials: impl IntoIterator<Item = MaterialAssignment>) -> Self {
    self.properties.materials = materials.into_iter().collect();
    self
  }
}

impl Default for Mesh {
  fn default() -> Self {
    Self::new()
  }
}
