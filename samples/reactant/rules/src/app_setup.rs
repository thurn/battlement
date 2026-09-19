use crate::{
  animation_validation, design_system, model,
  preview_resource::Preview,
  sample_constants::{CONTENT_SCENE, GEOMETRY_TARGET_ID, ROOT_ID},
  sample_shell,
};
use battlement::{
  CameraClearMode, CameraProjection, CameraState, Color, GameObject, GameObjectKind, ParentScene,
  TextureAddress, Vector3,
};
use reactant::app::App;

/// The sample application with component-local display state.
pub type ReactantEngine = App;

/// Creates the Reactant sample application.
pub fn create_engine() -> ReactantEngine {
  create_engine_with_screen(None)
}

pub(crate) fn create_engine_with_screen(screen: Option<crate::Screen>) -> ReactantEngine {
  animation_validation::fixture_registry()
    .validate()
    .expect("valid animation registry");
  let mut game = model::new();
  if let Some(screen) = screen {
    game.screen = screen;
  }
  let mut app = App::new(CONTENT_SCENE);
  let overlay = app.create_portal_target();
  let preview = Preview::new();
  app
    .ui(
      sample_shell::Laboratory::new()
        .initial(game)
        .event_overlay(overlay.clone())
        .preview_resource(preview),
    )
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

fn create_native_engine() -> Result<ReactantEngine, battlement_native::EngineError> {
  Ok(
    std::env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").map_or_else(
      |_| create_engine(),
      |selector| crate::fixture_catalog::build(&selector),
    ),
  )
}

/// Returns linked generated textures used by the gallery.
pub fn generated_asset_addresses() -> Vec<TextureAddress> {
  reactant::asset_generator::registrations()
    .map(|asset| TextureAddress::from(asset.address))
    .collect()
}

battlement_native::export_deterministic_engine!(
  self::create_native_engine,
  clock = virtualized,
  randomness = seeded,
  external_state = isolated,
  persistent_state = reset,
  input = semantic,
  visible_output = flatbuffers,
);
