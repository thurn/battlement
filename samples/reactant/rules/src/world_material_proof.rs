use crate::{CONTENT_SCENE, Game, ROOT_ID, model};
use battlement::{MaterialInstance, MaterialParameter, MaterialVector, ParentScene, Vector3};
use reactant::{app::App, hooks, prelude::*, world};
use trox::ls;

const CLIP: MaterialParameter<f64> = MaterialParameter::new("_Clip");
const ACCENT: MaterialParameter<Color> = MaterialParameter::new("_Accent");
const WARP: MaterialParameter<MaterialVector> = MaterialParameter::new("_Warp");
const MATERIAL: &str = "reactant/world/card-material";

struct MaterialProof;
pub(crate) fn app() -> App<Game> {
  App::with_model(CONTENT_SCENE, model::new())
    .ui(MaterialProof)
    .document(|mut document| {
      document.root_id = ROOT_ID;
      document
    })
    .camera(|camera| {
      world::Camera::new()
        .orthographic(4.0)
        .clipping(0.1, 50.0)
        .background(Color::rgb(0.035, 0.05, 0.08))
        .position(Vector3::new(0.0, 0.0, -10.0))
        .into_object(camera.object_id)
    })
}
impl Component for MaterialProof {
  fn render(&self) -> impl Render {
    let (changed, change) = hooks::use_state(false);
    let (cleared, clear) = hooks::use_state(false);
    let (faded, fade) = hooks::use_state(false);
    let mut left = world::Sprite::new()
      .texture("reactant/assets/texture")
      .size(2.3, 3.4)
      .opacity(if faded { 0.4 } else { 1.0 })
      .layer(0);
    if !cleared {
      left = left.material(
        MaterialInstance::new(MATERIAL)
          .parameter(CLIP, if changed { 0.7 } else { 0.12 })
          .parameter(
            ACCENT,
            if changed {
              Color::rgb(1.0, 0.45, 0.15)
            } else {
              Color::rgb(0.35, 0.7, 1.0)
            },
          )
          .parameter(WARP, MaterialVector([0.0; 4])),
      );
    }
    (
      View::new()
        .style(
          Style::new()
            .width(300.px())
            .padding(24.px())
            .color(Color::WHITE)
            .background_color(Color::rgb(0.06, 0.09, 0.15)),
        )
        .child((
          Heading::new(ls("Independent card materials"), 1),
          Label::new(ls(
            "Three cards share one material. Only the left card changes.",
          )),
          Button::new(ls("Change left card")).on_press(change.update_callback(|v| !v)),
          Button::new(ls("Fade artwork")).on_press(fade.update_callback(|v| !v)),
          Button::new(ls("Clear left overrides")).on_press(clear.update_callback(|v| !v)),
        )),
      world::SceneRoot::new(ParentScene::PrimaryScene).child((
        world::Group::new()
          .position(Vector3::new(-2.3, 0.3, 0.0))
          .child((left, self::label("LEFT", -2.1))),
        world::Group::new()
          .position(Vector3::new(0.8, 0.3, 0.0))
          .child((
            world::Sprite::new()
              .texture("reactant/assets/texture")
              .size(2.3, 3.4)
              .layer(0)
              .material(
                MaterialInstance::new(MATERIAL)
                  .parameter(CLIP, 0.35)
                  .parameter(ACCENT, Color::rgb(0.3, 1.0, 0.6)),
              ),
            self::label("RIGHT", -2.1),
          )),
        world::Group::new()
          .position(Vector3::new(3.9, 0.3, 0.0))
          .child((
            world::Sprite::new()
              .texture("reactant/assets/texture")
              .size(2.3, 3.4)
              .layer(0)
              .material(MaterialInstance::new(MATERIAL)),
            self::label("SOURCE", -2.1),
          )),
      )),
    )
  }
}
fn label(text: &str, y: f64) -> world::Text {
  world::Text::new()
    .font("reactant/world/font")
    .text(text)
    .size(4.5)
    .position(Vector3::new(0.0, y, 0.0))
    .layer(1)
}
