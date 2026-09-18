use crate::DITTO_VISUAL_STATE_REGISTRY;

#[test]
fn effect_occurrence_fixture_connects_through_the_public_display() {
  let app = crate::effect_occurrence_proof::app();
  let mut assets = battlement_fake::assets::FakeAssetCatalog::new();
  assets.add_scene(crate::CONTENT_SCENE);
  assets.add_texture("reactant/assets/texture");
  assets.add_textures(crate::generated_asset_addresses());
  assets.add_audio_clip("reactant/assets/clock-pulse");
  assets.add_particle_effect("reactant/effect-burst");
  let mut display = battlement_fake::client::FakeClient::connect(app, assets);
  display.poll();
}

#[test]
fn deterministic_registry_matches_the_static_composition_scenario() {
  let suite = include_str!("../../ditto.toml");
  assert!(DITTO_VISUAL_STATE_REGISTRY.contains("key = \"composition.initial\""));
  assert!(DITTO_VISUAL_STATE_REGISTRY.contains("screen = \"composition\""));
  let suite = suite
    .split("[[scenarios]]")
    .find(|scenario| scenario.contains("name = \"composition\""))
    .expect("composition scenario");
  assert!(suite.contains("name = \"composition\""));
  assert_eq!(suite.matches("screenshot =").count(), 1);
  assert!(suite.contains("screenshot = { name = \"initial\" }"));
}

#[test]
fn assets_are_generated_without_runtime_images() {
  let source = include_str!("assets.rs");
  assert_eq!(source.matches("asset_generator::generate!").count(), 18);
  assert!(
    !source.contains("unity-url("),
    "asset declarations must not load reference images"
  );
  assert!(
    !source.contains("rules/assets/mockup"),
    "asset declarations must not refer to the mockup image directory"
  );
}
