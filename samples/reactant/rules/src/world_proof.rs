use battlement::{LightType, MaterialAssignment, ParentScene, Quaternion, Vector3};
use reactant::{app::App, hooks, prelude::*, world};
use trox::ls;

use crate::{CONTENT_SCENE, Game, ROOT_ID, model};

struct WorldProof;

pub(crate) fn app() -> App<Game> {
  App::with_model(CONTENT_SCENE, model::new())
    .ui(WorldProof)
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

impl Component for WorldProof {
  fn render(&self) -> impl Render {
    let (orb, set_orb) = hooks::use_state(false);
    let (front, set_front) = hooks::use_state(true);
    let (back, set_back) = hooks::use_state(false);
    let (bright, set_bright) = hooks::use_state(true);
    (
      View::new()
        .style(
          Style::new()
            .width(320.px())
            .padding(24.px())
            .color(Color::WHITE)
            .background_color(Color::rgb(0.06, 0.09, 0.15)),
        )
        .child((
          Heading::new(ls("Typed world geometry"), 1),
          Label::new(ls(
            "Centered XY surfaces. Authored mesh units. Explicit camera and light.",
          )),
          Heading::new(ls(if orb { "Mesh: orb" } else { "Mesh: block" }), 2),
          Heading::new(
            ls(if front {
              "Front: present"
            } else {
              "Front: removed"
            }),
            2,
          ),
          Button::new(ls("Switch mesh")).on_press(set_orb.update_callback(|value| !value)),
          Button::new(ls("Toggle front")).on_press(set_front.update_callback(|value| !value)),
          Button::new(ls("Flip card")).on_press(set_back.update_callback(|value| !value)),
          Button::new(ls("Change light")).on_press(set_bright.update_callback(|value| !value)),
        )),
      world::SceneRoot::new(ParentScene::PrimaryScene).child((
        world::Group::new()
          .position(Vector3::new(-0.3, 0.0, 0.0))
          .rotation(if back {
            Quaternion::new(0.0, 1.0, 0.0, 0.0)
          } else {
            Quaternion::IDENTITY
          })
          .child((
            front.then(|| {
              world::Sprite::new()
                .texture("reactant/assets/texture")
                .size(2.1, 3.0)
            }),
            world::Sprite::new()
              .texture("reactant/assets/cursor")
              .size(2.1, 3.0)
              .position(Vector3::new(0.0, 0.0, 0.02))
              .rotation(Quaternion::new(0.0, 1.0, 0.0, 0.0)),
          )),
        world::Mesh::new()
          .mesh(if orb {
            "reactant/world/orb"
          } else {
            "reactant/world/block"
          })
          .materials([MaterialAssignment::new(0, "reactant/world/material")])
          .position(Vector3::new(3.2, 0.0, 0.0))
          .scale(Vector3::new(1.3, 2.0, 1.0))
          .rotation(Quaternion::new(0.0, 0.382683432365, 0.0, 0.923879532511)),
        world::Light::new()
          .light_type(LightType::Directional)
          .rotation(Quaternion::new(0.2, -0.3, 0.0, 0.932737905309))
          .intensity(if bright { 2.0 } else { 0.3 }),
      )),
    )
  }
}
