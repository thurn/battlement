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
use reactant::{
  Application,
  prelude::{Component, Render},
};

/// The sample application with component-local display state.
pub type ReactantApplication = Application;

/// Creates the Reactant sample application.
pub fn application() -> ReactantApplication {
  application_with_screen(None)
}

pub(crate) fn application_with_screen(screen: Option<crate::Screen>) -> ReactantApplication {
  animation_validation::fixture_registry()
    .validate()
    .expect("valid animation registry");
  let mut game = model::new();
  if let Some(screen) = screen {
    game.screen = screen;
  }
  Application::new(CONTENT_SCENE)
    .child(GalleryRoot { game })
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

struct GalleryRoot {
  game: crate::Game,
}

impl Component for GalleryRoot {
  fn render(&self) -> impl Render {
    let overlay = reactant::use_portal_target();
    let preview = reactant::hooks::use_memo(Preview::new, ());
    sample_shell::Laboratory::new()
      .initial(self.game.clone())
      .event_overlay(overlay)
      .preview_resource(preview)
  }
}

fn exported_application() -> ReactantApplication {
  std::env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").map_or_else(
    |_| application(),
    |selector| crate::fixture_catalog::build(&selector),
  )
}

/// Returns linked generated textures used by the gallery.
pub fn generated_asset_addresses() -> Vec<TextureAddress> {
  reactant::asset_generator::registrations()
    .map(|asset| TextureAddress::from(asset.address))
    .collect()
}

reactant::export_application!(self::exported_application);
