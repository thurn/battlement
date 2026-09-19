use crate::ROOT_ID;
use battlement::{ObjectId, ParentScene, Quaternion, Vector3, object_id};
use reactant::{hooks, native_host, prelude::*, world};
use trox::ls;

const CARD: ObjectId = object_id!("38110000-0000-4000-8000-000000000001");
const HIT: ObjectId = object_id!("38110000-0000-4000-8000-000000000002");
struct HitProof;
pub(crate) fn app() -> crate::ReactantEngine {
  reactant::app::App::new(crate::CONTENT_SCENE)
    .ui(HitProof)
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
impl Component for HitProof {
  fn render(&self) -> impl Render {
    let reference = native_host::use_object_ref();
    let anchor = reference.local_point(Vector3::new(0.9, 1.25, -0.15));
    let (expanded, expand) = hooks::use_state(false);
    let (moved, move_card) = hooks::use_state(false);
    let (hidden, hide) = hooks::use_state(false);
    let (destroyed, destroy) = hooks::use_state(false);
    let (count, increment) = hooks::use_state(0);
    let card = (!destroyed).then(|| {
      world::Group::new()
        .id(*CARD.as_uuid())
        .reference(reference)
        .active(!hidden)
        .rotation(if moved {
          Quaternion::new(0.0, 0.0, 0.3826834323650898, 0.9238795325112867)
        } else {
          Quaternion::new(0.0, 0.0, 0.0, 1.0)
        })
        .on_click(increment.update_callback(|v| v + 1))
        .child((
          world::Sprite::new()
            .texture("reactant/assets/texture")
            .size(if expanded { 3.0 } else { 1.5 }, 2.5),
          world::BoxHitRegion::new()
            .id(*HIT.as_uuid())
            .size(Vector3::new(if expanded { 3.0 } else { 1.5 }, 2.5, 0.2))
            .center(Vector3::new(if expanded { 0.5 } else { 0.0 }, 0.0, 0.0)),
          world::Text::new()
            .font("reactant/world/font")
            .text("+")
            .size(5.0)
            .color(Color::rgb(1.0, 0.3, 0.1))
            .layer(3)
            .position(anchor.offset()),
        ))
    });
    (
      View::new()
        .style(
          Style::new()
            .width(285.px())
            .padding(20.px())
            .color(Color::WHITE)
            .background_color(Color::rgb(0.06, 0.09, 0.15)),
        )
        .child((
          Heading::new(ls("Hit regions and local anchors"), 1),
          Label::new(ls("Orange cross follows the card's local offset.")),
          Heading::new(ls(format!("Card clicks: {count}")), 2),
          Button::new(ls("Resize hit region")).on_press(expand.update_callback(|v| !v)),
          Button::new(ls("Move and rotate parent")).on_press(move_card.update_callback(|v| !v)),
          Button::new(ls("Hide or show card")).on_press(hide.update_callback(|v| !v)),
          Button::new(ls("Destroy or restore card")).on_press(destroy.update_callback(|v| !v)),
        )),
      world::SceneRoot::new(ParentScene::PrimaryScene).child((
        world::Group::new()
          .position(Vector3::new(0.5, 0.5, 0.0))
          .child((!moved).then(|| card.clone())),
        world::Group::new()
          .position(Vector3::new(2.5, -0.3, 0.0))
          .child(moved.then_some(card)),
      )),
    )
  }
}
