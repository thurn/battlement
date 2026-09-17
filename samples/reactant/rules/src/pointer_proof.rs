use crate::{CONTENT_SCENE, Game, ROOT_ID, model};
use battlement::{ObjectId, ParentScene, PickingMode, Position, Prop, Vector3, object_id};
use reactant::{
  app::App,
  hooks,
  overlay::{Overlay, OverlayHost},
  portal::PortalTarget,
  prelude::*,
  world,
};
use trox::ls;

const HIT: ObjectId = object_id!("39110000-0000-4000-8000-000000000001");
const CARD: ObjectId = object_id!("39110000-0000-4000-8000-000000000002");
struct PointerProof {
  target: PortalTarget,
}

pub(crate) fn app() -> App<Game> {
  let mut app = App::with_model(CONTENT_SCENE, model::new());
  let target = app.create_portal_target();
  app = app
    .ui(PointerProof { target })
    .document(|mut doc| {
      doc.root_id = ROOT_ID;
      doc.element.picking_mode = Prop::Set(PickingMode::Ignore);
      doc
    })
    .camera(|camera| {
      world::Camera::new()
        .orthographic(4.0)
        .clipping(0.1, 50.0)
        .background(Color::rgb(0.035, 0.05, 0.08))
        .position(Vector3::new(0.0, 0.0, -10.0))
        .into_object(camera.object_id)
    });
  app
}

impl Component for PointerProof {
  fn render(&self) -> impl Render {
    let (pass, passthrough) = hooks::use_state(false);
    let (moved, reparent) = hooks::use_state(false);
    let (hidden, hide) = hooks::use_state(false);
    let (gone, remove) = hooks::use_state(false);
    let (outer, outer_set) = hooks::use_state(false);
    let (inner, inner_set) = hooks::use_state(false);
    let (clicks, click) = hooks::use_state(0);
    let (captured, capture) = hooks::use_state(false);
    let move_drag = reparent.clone();
    let gain = capture.clone();
    let lose = capture.clone();
    let card = (!gone).then(|| {
      world::Group::new()
        .id(*CARD.as_uuid())
        .active(!hidden)
        .child((
          world::Sprite::new()
            .texture("reactant/assets/texture")
            .size(2.6, 2.0),
          world::Text::new()
            .font("reactant/world/font")
            .text("WORLD CARD")
            .size(3.0)
            .position(Vector3::new(0.0, 0.0, -0.2))
            .layer(3)
            .color(Color::WHITE),
          world::BoxHitRegion::new()
            .id(*HIT.as_uuid())
            .size(Vector3::new(2.6, 2.0, 0.2))
            .capture_on_press(true)
            .on_click(click.update_callback(|n| n + 1))
            .events(
              world::PointerHandlers::new()
                .on_pointer_capture(
                  move |_: reactant::event::ReactantEvent<battlement::PointerCaptureEvent>| {
                    gain.set(true)
                  },
                )
                .on_pointer_capture_out(
                  move |_: reactant::event::ReactantEvent<battlement::PointerCaptureEvent>| {
                    lose.set(false)
                  },
                )
                .on_pointer_move(
                  move |e: reactant::event::ReactantEvent<battlement::PointerMoveEvent>| {
                    if e.payload().buttons != 0 {
                      move_drag.set(true);
                    }
                  },
                ),
            ),
        ))
    });
    Stack::new()
      .picking_mode(PickingMode::Ignore)
      .style(Style::new().width(100.pct()).height(100.pct()))
      .child((
        View::new()
          .picking_mode(PickingMode::Ignore)
          .style(Style::new().width(100.pct()).height(100.pct()))
          .child((
            View::new()
              .style(
                Style::new()
                  .position(Position::Absolute)
                  .left(15.px())
                  .top(15.px())
                  .width(280.px())
                  .padding(15.px())
                  .background_color(Color::rgb(0.06, 0.09, 0.15))
                  .color(Color::WHITE),
              )
              .child((
                Heading::new(ls("Unified pointer routing"), 1),
                Heading::new(ls(format!("World clicks: {clicks}")), 2),
                Label::new(ls(format!("Captured: {captured} | Reparented: {moved}"))),
                Button::new(ls("Toggle passthrough")).on_press(passthrough.update_callback(|v| !v)),
                Button::new(ls("Open outer modal")).on_press(outer_set.update_callback(|_| true)),
                Button::new(ls("Reparent card")).on_press(reparent.update_callback(|v| !v)),
                Button::new(ls("Hide or show card")).on_press(hide.update_callback(|v| !v)),
                Button::new(ls("Remove or restore card")).on_press(remove.update_callback(|v| !v)),
              )),
            View::new()
              .picking_mode(if pass {
                PickingMode::Ignore
              } else {
                PickingMode::Position
              })
              .style(
                Style::new()
                  .position(Position::Absolute)
                  .left(42.pct())
                  .top(35.pct())
                  .width(24.pct())
                  .height(30.pct())
                  .background_color(if pass {
                    Color::rgba(0.1, 0.4, 0.5, 0.18)
                  } else {
                    Color::rgba(0.15, 0.3, 0.55, 0.85)
                  }),
              )
              .child(
                Label::new(ls(if pass {
                  "DECORATION: PASSTHROUGH"
                } else {
                  "UI BLOCKS WORLD"
                }))
                .picking_mode(PickingMode::Ignore)
                .style(Style::new().color(Color::WHITE).padding(12.px())),
              ),
            world::SceneRoot::new(ParentScene::PrimaryScene).child((
              world::Group::new().child((!moved).then(|| card.clone())),
              world::Group::new()
                .position(Vector3::new(2.8, -1.5, 0.0))
                .child(moved.then_some(card)),
            )),
            outer.then(|| {
              Overlay::modal(self.target.clone(), ls("Outer pointer scope")).child(
                View::new()
                  .style(
                    Style::new()
                      .padding(30.px())
                      .background_color(Color::rgb(0.12, 0.15, 0.23))
                      .color(Color::WHITE),
                  )
                  .child((
                    Heading::new(ls("Outer modal owns input"), 1),
                    Button::new(ls("Open inner modal"))
                      .on_press(inner_set.update_callback(|_| true)),
                    Button::new(ls("Close outer modal"))
                      .on_press(outer_set.update_callback(|_| false)),
                    inner.then(|| {
                      Overlay::modal(self.target.clone(), ls("Inner pointer scope")).child(
                        View::new()
                          .style(
                            Style::new()
                              .padding(60.px())
                              .background_color(Color::rgb(0.22, 0.12, 0.25))
                              .color(Color::WHITE),
                          )
                          .child((
                            Heading::new(ls("Inner modal owns input"), 1),
                            Button::new(ls("Close inner modal"))
                              .on_press(inner_set.update_callback(|_| false)),
                          )),
                      )
                    }),
                  )),
              )
            }),
          )),
        OverlayHost::new(self.target.clone()),
      ))
  }
}

#[cfg(test)]
mod tests {
  use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
  #[test]
  fn pointer_scene_mounts() {
    let mut assets = FakeAssetCatalog::new();
    assets.add_scene(crate::CONTENT_SCENE);
    assets.add_texture("reactant/assets/texture");
    assets.add_text_mesh_pro_font("reactant/world/font");
    assets.add_textures(crate::generated_asset_addresses());
    let _ = FakeClient::connect(crate::pointer_proof::app(), assets);
  }
}
