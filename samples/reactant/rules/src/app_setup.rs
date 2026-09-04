use crate::{
  Game, animation_validation, assets, design_system, model,
  preview_resource::Preview,
  sample_constants::{CONTENT_SCENE, GEOMETRY_TARGET_ID, ROOT_ID},
  sample_shell,
};
use battlement::{
  CameraClearMode, CameraProjection, CameraState, Color, GameObject, GameObjectKind, ParentScene,
  TextureAddress, Vector3,
};
use battlement_reactant::app::App;
use trox::Bundle;

/// The sample's application with its game-owned demonstration state.
pub type ReactantEngine = App<Game>;

/// Creates the Reactant sample application.
pub fn create_engine() -> ReactantEngine {
  animation_validation::fixture_registry()
    .validate()
    .expect("valid animation registry");
  let source = Bundle::from_canonical_json(include_str!("../../localization/en-US.trox.json"))
    .expect("valid embedded English trox bundle");
  let mut app = App::with_model(CONTENT_SCENE, model::new()).source_bundle(source);
  let overlay = app.create_portal_target();
  let preview = Preview::new();
  app
    .root(move |game| sample_shell::view(game, overlay.clone(), preview.clone()))
    .document(|mut document| {
      document.root_id = ROOT_ID;
      document
        .name("battlement-reactant")
        .style(design_system::root(false))
    })
    .camera(|camera| {
      GameObject::new(
        camera.object_id,
        CameraState::new()
          .projection(CameraProjection::Perspective)
          .field_of_view(50.0)
          .clear_mode(CameraClearMode::SolidColor)
          .clear_color(Color::rgb(0.012, 0.025, 0.045)),
      )
      .parent_scene(ParentScene::Persistent)
      .position(Vector3::new(0.0, 0.0, -10.0))
    })
    .object(
      GameObject::new(
        GEOMETRY_TARGET_ID,
        GameObjectKind::Cube {
          materials: Vec::new(),
        },
      )
      .parent_scene(ParentScene::Persistent)
      .position(Vector3::new(3.2, -1.8, 0.0))
      .scale(Vector3::new(1.4, 1.4, 1.4)),
    )
}

/// Returns linked generated textures used by the gallery.
pub fn generated_asset_addresses() -> Vec<TextureAddress> {
  assets::addresses()
}

battlement_native::export_engine!(self::create_engine);
