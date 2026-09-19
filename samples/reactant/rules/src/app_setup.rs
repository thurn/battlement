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
use reactant::app::App;
use std::env;

/// The sample's application with its game-owned demonstration state.
pub type ReactantEngine = App<Game>;

/// Creates the Reactant sample application.
pub fn create_engine() -> ReactantEngine {
  create_engine_with_screen(None)
}

fn create_engine_with_screen(screen: Option<crate::Screen>) -> ReactantEngine {
  animation_validation::fixture_registry()
    .validate()
    .expect("valid animation registry");
  let mut game = model::new();
  if let Some(screen) = screen {
    game.screen = screen;
  }
  let mut app = App::with_model(CONTENT_SCENE, game);
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

fn create_native_engine() -> Result<ReactantEngine, battlement_native::EngineError> {
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() == Ok("motion-ui") {
    return Ok(create_engine_with_screen(Some(
      crate::Screen::TargetsTimelines,
    )));
  }
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() == Ok("motion-reduced") {
    return Ok(create_engine_with_screen(Some(
      crate::Screen::ComposedEffects,
    )));
  }
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() == Ok("motion-sequence") {
    return Ok(create_engine_with_screen(Some(
      crate::Screen::ValuesTimeControls,
    )));
  }
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() == Ok("shared-motion") {
    return Ok(crate::world_motion_proof::app());
  }
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() == Ok("effect-occurrences") {
    return Ok(crate::effect_occurrence_proof::app());
  }
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() == Ok("effect-exit-retention") {
    return Ok(crate::effect_exit_retention_proof::app());
  }
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() == Ok("gameplay-pause") {
    return Ok(crate::gameplay_pause_proof::app());
  }
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() == Ok("world-navigation") {
    return Ok(crate::navigation_proof::app());
  }
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() == Ok("pointer-routing") {
    return Ok(crate::pointer_proof::app());
  }
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() == Ok("world-hits") {
    return Ok(crate::world_hit_proof::app());
  }
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() == Ok("world-layout") {
    return Ok(crate::world_layout_proof::app());
  }
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() == Ok("draw-reflow") {
    return Ok(crate::draw_reflow_proof::app());
  }
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() == Ok("world-materials") {
    return Ok(crate::world_material_proof::app());
  }
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() == Ok("world-text") {
    return Ok(crate::world_text_proof::app());
  }
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() == Ok("world-primitives") {
    return Ok(crate::world_proof::app());
  }
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() == Ok("stable-selectors") {
    return Ok(crate::selector_proof::app(false));
  }
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() == Ok("stable-props") {
    return Ok(crate::selector_proof::app(true));
  }
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() == Ok("destruction-queue") {
    return Ok(crate::destruction_queue_proof::app());
  }
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() == Ok("presentation-lifetime") {
    return Ok(crate::lifetime_proof::app());
  }
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() == Ok("presentation-identity") {
    return Ok(crate::identity_proof::app());
  }
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref()
    == Ok("presentation-identity-perspective")
  {
    return Ok(crate::identity_proof::perspective_app());
  }
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() == Ok("mixed-tree") {
    return Ok(crate::mixed_proof::app());
  }
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() == Ok("rules-prompts") {
    return Ok(crate::prompt_proof::app());
  }
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() == Ok("rules-session") {
    return Ok(crate::session_proof::app());
  }
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() == Ok("rules-batches") {
    return Ok(crate::batch_proof::app());
  }
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() == Ok("rules-batches-auto") {
    return Ok(crate::batch_proof::automatic_app());
  }
  if env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() == Ok("presentation-inspector") {
    return Ok(crate::batch_proof::inspector_app());
  }
  Ok(create_engine())
}

/// Returns linked generated textures used by the gallery.
pub fn generated_asset_addresses() -> Vec<TextureAddress> {
  assets::addresses()
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
