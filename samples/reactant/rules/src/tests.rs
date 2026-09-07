use crate::DITTO_VISUAL_STATE_REGISTRY;

#[test]
fn deterministic_registry_matches_the_static_composition_scenario() {
  let suite = include_str!("../../ditto.toml");
  assert_eq!(DITTO_VISUAL_STATE_REGISTRY.matches("[[states]]").count(), 1);
  assert!(DITTO_VISUAL_STATE_REGISTRY.contains("key = \"composition.initial\""));
  assert!(DITTO_VISUAL_STATE_REGISTRY.contains("screen = \"composition\""));
  assert_eq!(suite.matches("[[scenarios]]").count(), 1);
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
