use battlement::{HorizontalAlignment, ParentScene, RgbColor, Vector3, VerticalAlignment};
use reactant::{hooks, prelude::*, world};
use trox::ls;

use crate::ROOT_ID;

struct WorldTextProof;

pub(crate) fn app() -> crate::ReactantEngine {
  reactant::app::App::new(crate::CONTENT_SCENE)
    .ui(WorldTextProof)
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

impl Component for WorldTextProof {
  fn render(&self) -> impl Render {
    let (covered, cover) = hooks::use_state(false);
    let (raised, raise) = hooks::use_state(false);
    let (narrow, narrow_text) = hooks::use_state(false);
    let (faded, fade) = hooks::use_state(false);
    (
      View::new()
        .style(
          Style::new()
            .width(310.px())
            .padding(24.px())
            .color(Color::WHITE)
            .background_color(Color::rgb(0.06, 0.09, 0.15)),
        )
        .child((
          Heading::new(ls("World text and layers"), 1),
          Label::new(ls(
            "Each card is a sorting group. Glyphs and the stripe share its local order.",
          )),
          Button::new(ls("Cover glyphs")).on_press(cover.update_callback(|v| !v)),
          Button::new(ls("Raise text card")).on_press(raise.update_callback(|v| !v)),
          Button::new(ls("Change wrapping")).on_press(narrow_text.update_callback(|v| !v)),
          Button::new(ls("Tint and fade")).on_press(fade.update_callback(|v| !v)),
        )),
      world::SceneRoot::new(ParentScene::PrimaryScene).child((
        world::Group::new()
          .sort_order(if raised { 3 } else { 1 })
          .position(Vector3::new(0.0, 0.4, 0.0))
          .child((
            world::Sprite::new()
              .texture("reactant/assets/texture")
              .size(3.4, 3.6)
              .tint(RgbColor::rgb(0.25, 0.35, 0.55))
              .layer(0),
            world::Sprite::new()
              .texture("reactant/assets/texture")
              .size(3.4, 0.45)
              .tint(RgbColor::rgb(0.9, 0.2, 0.12))
              .position(Vector3::new(0.0, 0.15, 0.0))
              .layer(if covered { 3 } else { 1 }),
            world::Text::new()
              .font("reactant/world/font")
              .text("<b>RICH</b> WORLD TEXT")
              .rich_text(true)
              .size(6.0)
              .wrapping(Some(if narrow { 1.5 } else { 3.1 }))
              .alignment(
                if narrow {
                  HorizontalAlignment::Right
                } else {
                  HorizontalAlignment::Center
                },
                VerticalAlignment::Middle,
              )
              .tint(if faded {
                RgbColor::rgb(1.0, 0.65, 0.15)
              } else {
                RgbColor::WHITE
              })
              .opacity(if faded { 0.45 } else { 1.0 })
              .layer(2),
          )),
        world::Group::new()
          .sort_order(2)
          .position(Vector3::new(2.0, -0.6, 0.0))
          .child((
            world::Sprite::new()
              .texture("reactant/assets/texture")
              .size(2.4, 2.7)
              .tint(RgbColor::rgb(0.1, 0.7, 0.5))
              .layer(-10),
            world::Text::new()
              .font("reactant/world/font")
              .text("SECOND")
              .size(5.0)
              .wrapping(Some(2.3))
              .layer(-9),
          )),
      )),
    )
  }
}
