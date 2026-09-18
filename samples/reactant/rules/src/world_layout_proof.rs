use battlement::{ObjectId, ParentScene, PickingMode, Prop, Vector3, object_id};
use reactant::{app::App, prelude::*, world};
use trox::ls;

use crate::{CONTENT_SCENE, Game, ROOT_ID, model};

const FAN_A: ObjectId = object_id!("383a0000-0000-4000-8000-000000000001");
const FAN_B: ObjectId = object_id!("383a0000-0000-4000-8000-000000000002");
const FAN_C: ObjectId = object_id!("383a0000-0000-4000-8000-000000000003");
const GRID_A: ObjectId = object_id!("383a0000-0000-4000-8000-000000000004");
const GRID_B: ObjectId = object_id!("383a0000-0000-4000-8000-000000000005");
const NESTED: ObjectId = object_id!("383a0000-0000-4000-8000-000000000006");
const PILE_A: ObjectId = object_id!("383a0000-0000-4000-8000-000000000007");
const PILE_B: ObjectId = object_id!("383a0000-0000-4000-8000-000000000008");

struct WorldLayoutProof;

pub(crate) fn app() -> App<Game> {
  App::with_model(CONTENT_SCENE, model::new())
    .ui(WorldLayoutProof)
    .document(|mut document| {
      document.root_id = ROOT_ID;
      document.element.picking_mode = Prop::Set(PickingMode::Ignore);
      document
    })
    .camera(|camera| {
      world::Camera::new()
        .orthographic(5.0)
        .clipping(0.1, 50.0)
        .background(Color::rgb(0.035, 0.05, 0.08))
        .position(Vector3::new(0.0, 0.0, -10.0))
        .into_object(camera.object_id)
    })
}

impl Component for WorldLayoutProof {
  fn render(&self) -> impl Render {
    let fan = world::Fan::new()
      .plane(world::LayoutPlane::xy(Vector3::new(-1.0, 0.6, 0.0)))
      .extent((5.0, 3.0))
      .curve(3.2, 1.0)
      .angle(0.55)
      .children([
        card(FAN_A, "reactant/assets/texture"),
        card(FAN_B, "reactant/assets/cursor"),
        card(FAN_C, "reactant/assets/texture"),
      ]);
    let nested = world::Pile::new()
      .extent((2.6, 2.2))
      .step(0.35, 0.2)
      .children([
        card(PILE_A, "reactant/assets/cursor"),
        card(PILE_B, "reactant/assets/texture"),
      ]);
    let grid = world::Grid::new()
      .plane(world::LayoutPlane::xy(Vector3::new(-1.0, -3.2, 0.0)))
      .extent((5.0, 2.4))
      .columns(3)
      .gaps(0.25, 0.0)
      .children([
        card(GRID_A, "reactant/assets/cursor"),
        card(GRID_B, "reactant/assets/texture"),
        world::LayoutChild::new(
          world::LayoutDestination::new(*NESTED.as_uuid()),
          world::LayoutBox::new(2.6, 2.2),
          nested,
        ),
      ]);
    (
      View::new()
        .style(
          Style::new()
            .width(320.px())
            .padding(22.px())
            .color(Color::WHITE)
            .background_color(Color::rgb(0.06, 0.09, 0.15)),
        )
        .child((
          Heading::new(ls("World rest layout"), 1),
          Label::new(ls("Fan above. Grid and nested pile below.")),
          Label::new(ls("Explicit boxes preserve card scale and facing.")),
        )),
      world::SceneRoot::new(ParentScene::PrimaryScene).child((fan, grid)),
    )
  }
}

fn card(id: ObjectId, texture: &'static str) -> world::LayoutChild {
  world::LayoutChild::new(
    world::LayoutDestination::new(*id.as_uuid()),
    world::LayoutBox::new(1.15, 1.65),
    world::Sprite::new().texture(texture).size(1.15, 1.65),
  )
  .item(
    world::LayoutItem::new(*id.as_uuid(), world::LayoutBox::new(1.15, 1.65))
      .orientation(world::LayoutOrientation::Arrangement),
  )
}
