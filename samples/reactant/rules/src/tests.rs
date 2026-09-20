use crate::DITTO_VISUAL_STATE_REGISTRY;

fn activate(display: &mut reactant_testing::Display, label: &str) {
  let object_id = display
    .accessibility()
    .nodes
    .iter()
    .find(|node| node.label.as_deref() == Some(label))
    .unwrap_or_else(|| panic!("missing accessible control {label}"))
    .object_id;
  display.ui().deliver_event(battlement::UiEvent {
    target_id: object_id,
    cancelable: true,
    default_prevented: false,
    body: battlement::UiEventBody::AccessibilityAction(battlement::UiAccessibilityActionEvent {
      backend_generation: 1,
      action: battlement::UiAccessibilityAction::Activate,
    }),
  });
  display.poll();
}

fn contains_named(
  ui: &battlement_fake::client::ui::UiClient<'_, reactant::ApplicationEngine>,
  root: battlement::ObjectId,
  expected: &str,
) -> bool {
  let mut pending = vec![root];
  while let Some(object_id) = pending.pop() {
    let element = ui.element(object_id);
    if element.name() == Some(expected) {
      return true;
    }
    pending.extend(element.children());
  }
  false
}

#[test]
fn effect_occurrence_fixture_connects_through_the_public_display() {
  let mut assets = battlement_fake::assets::FakeAssetCatalog::new();
  assets.add_scene(crate::CONTENT_SCENE);
  assets.add_texture("reactant/assets/texture");
  assets.add_textures(crate::generated_asset_addresses());
  assets.add_audio_clip("reactant/assets/clock-pulse");
  assets.add_particle_effect("reactant/effect-burst");
  let mut display = reactant_testing::Display::mount(crate::effect_occurrence_proof::app, assets);
  display.poll();
}

#[test]
fn effect_exit_retention_fixture_connects_through_the_public_display() {
  let mut assets = battlement_fake::assets::FakeAssetCatalog::new();
  assets.add_scene(crate::CONTENT_SCENE);
  assets.add_texture("reactant/assets/texture");
  assets.add_textures(crate::generated_asset_addresses());
  assets.add_material_with_parameters(
    "reactant/world/card-material",
    [battlement::MaterialParameter::<f64>::new("_Clip").value(0.0)],
  );
  assets.add_text_mesh_pro_font("reactant/world/font");
  let mut display =
    reactant_testing::Display::mount(crate::effect_exit_retention_proof::app, assets);
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

#[test]
fn presentation_inspector_stays_open_as_the_game_advances() {
  let mut assets = battlement_fake::assets::FakeAssetCatalog::new();
  assets.add_scene(crate::CONTENT_SCENE);
  assets.add_material(crate::MOTION_MATERIAL);
  assets.add_textures(crate::generated_asset_addresses());
  let mut display = reactant_testing::Display::mount(crate::batch_proof::inspector_app, assets);
  display.poll();
  for _ in 0..8 {
    display.poll();
  }

  activate(&mut display, "Open inspector");
  assert!(
    contains_named(
      &display.ui(),
      crate::ROOT_ID,
      "presentation-inspector-panel"
    ),
    "inspector panel was not displayed after semantic activation"
  );
  activate(&mut display, "Begin ordered game");
  display.advance_frame();

  assert!(
    contains_named(
      &display.ui(),
      crate::ROOT_ID,
      "presentation-inspector-panel"
    ),
    "game advancement closed the inspector"
  );
}
