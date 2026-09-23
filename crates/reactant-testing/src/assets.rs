//! Generated address lists supply asset identity; Unity validates imported contents.
use battlement::PreparedAsset;
use battlement_fake::assets::{FakeAssetCatalog, FakePrefab};

/// Builds an opaque asset catalog from generated Addressables constants.
/// Prefabs can be instantiated or emitted as temporary effects. This does not
/// invent imported colliders or renderer components: author hit regions in the
/// application, and use explicit `FakePrefab` fixtures for component-level tests.
pub fn catalog(assets: &[PreparedAsset]) -> FakeAssetCatalog {
  let mut catalog = FakeAssetCatalog::new();
  for asset in assets {
    match asset {
      PreparedAsset::Scene(a) => catalog.add_scene(a.clone()),
      PreparedAsset::Prefab(a) => {
        catalog.add_prefab(a.clone(), FakePrefab::new());
        catalog.add_particle_effect(a.clone());
      }
      PreparedAsset::ParticleEffect(a) => catalog.add_particle_effect(a.clone()),
      PreparedAsset::Mesh(_) | PreparedAsset::MaterialParameters { .. } => {
        panic!("asset requires an explicit fake capability description: {asset:?}");
      }
      PreparedAsset::Material(a) => catalog.add_material(a.clone()),
      PreparedAsset::Texture(a) => catalog.add_texture(a.clone()),
      PreparedAsset::Sprite(a) => catalog.add_sprite(a.clone()),
      PreparedAsset::VectorImage(a) => catalog.add_vector_image(a.clone()),
      PreparedAsset::RenderTexture(a) => catalog.add_render_texture(a.clone()),
      PreparedAsset::AudioClip(a) => catalog.add_audio_clip(a.clone()),
      PreparedAsset::TextMeshProFont(a) => catalog.add_text_mesh_pro_font(a.clone()),
      PreparedAsset::UiFont(a) => catalog.add_ui_font(a.clone()),
    }
  }
  catalog
}
