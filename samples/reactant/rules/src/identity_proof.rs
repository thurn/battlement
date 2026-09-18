use battlement::{
  CameraProjection, CameraState, GameObject, ObjectId, ParentScene, Rect, UiDocument, Vector3,
  object_id,
};
use reactant::{
  app::App,
  callback::Callback,
  hooks,
  portal::{self, PortalTarget},
  prelude::*,
};
use trox::ls;

use crate::{CONTENT_SCENE, Game, ROOT_ID, model};

const CARD_ID: ObjectId = object_id!("25300000-0000-4000-8000-000000000081");
const FACE_ID: ObjectId = object_id!("25300000-0000-4000-8000-000000000082");
const VISUAL_ID: ObjectId = object_id!("25300000-0000-4000-8000-000000000083");
const SECOND_ROOT: ObjectId = object_id!("25300000-0000-4000-8000-000000000084");

#[derive(Clone, Default, PartialEq)]
struct Location(&'static str);

struct Card {
  portal: Option<PortalTarget>,
}

pub(crate) fn app() -> App<Game> {
  app_with_projection(CameraProjection::Orthographic)
}

pub(crate) fn perspective_app() -> App<Game> {
  app_with_projection(CameraProjection::Perspective)
}

fn app_with_projection(camera_projection: CameraProjection) -> App<Game> {
  let mut app = App::with_model(CONTENT_SCENE, model::new());
  let portal = app.create_portal_target();
  let destination = portal.clone();
  app
    .root(move |game| {
      View::new().style(self::panel()).child((
        Heading::new(ls("One identity across roots"), 1),
        Label::new(ls(
          "Increment the card or click its world visual, then move it.",
        )),
        Button::new(ls("Move card")).on_press(|game: &mut Game| {
          game.identity_location = (game.identity_location + 1) % 3;
        }),
        (game.identity_location == 0).then(|| self::card("Source", None)),
        (game.identity_location == 2).then(|| self::card("Portal", Some(portal.clone()))),
      ))
    })
    .document(|mut document| {
      document.root_id = ROOT_ID;
      document
    })
    .additional_root(
      UiDocument::new(SECOND_ROOT).style(Style::new().left(470.px()).width(410.px())),
      move |game| {
        View::new().style(self::panel()).child((
          Heading::new(ls("Destination root"), 1),
          (game.identity_location == 1).then(|| self::card("Destination", None)),
          View::new().portal_target(destination.clone()),
        ))
      },
    )
    .camera(|camera| {
      GameObject::new(
        camera.object_id,
        CameraState::new()
          .projection(camera_projection)
          .field_of_view(50.0)
          .orthographic_size(5.0),
      )
      .parent_scene(ParentScene::Persistent)
      .position(Vector3::new(0.0, 0.0, -10.0))
    })
}

fn card(location: &'static str, portal: Option<PortalTarget>) -> impl Render {
  ContextProvider::new()
    .context(Location(location))
    .child(Card { portal }.id(*CARD_ID.as_uuid()))
}

fn panel() -> Style {
  Style::new()
    .width(410.px())
    .padding(24.px())
    .color(Color::WHITE)
    .background_color(Color::rgb(0.06, 0.09, 0.15))
}

impl Component for Card {
  fn render(&self) -> impl Render {
    let (count, set_count) = hooks::use_state(0_u32);
    let location = hooks::use_context::<Location>().0;
    let reference = reactant::element_ref::use_element_ref();
    let increment = set_count.update_callback(|value| value + 1);
    let face = View::new()
      .id(*FACE_ID.as_uuid())
      .element_ref(reference)
      .layout(Layout::Both)
      .layout_id(CARD_ID)
      .ui_world_projection(self::projection())
      .child((
        Heading::new(ls(format!("Card count: {count} at {location}")), 2),
        Button::new(ls("Increment card")).on_press(increment.clone()),
      ));
    let face = match &self.portal {
      Some(target) => Node::new(portal::create_portal(face, target.clone())),
      None => Node::new(face),
    };
    (
      face,
      SceneRoot::new(if location == "Destination" {
        ParentScene::Persistent
      } else {
        ParentScene::PrimaryScene
      })
      .child(
        WorldGroup::new().on_click(increment).child(
          Prefab::at("reactant/mixed-visual")
            .id(*VISUAL_ID.as_uuid())
            .position(Vector3::new(4.5 + f64::from(count) * 0.12, -2.0, 0.0))
            .scale(Vector3::new(1.3, 1.3, 1.3))
            .on_click(Callback::noop())
            .motion(
              MotionProps::new()
                .layout(Layout::Both)
                .layout_id(VISUAL_ID)
                .ui_world_projection(self::projection()),
            ),
        ),
      ),
    )
  }
}

fn projection() -> UiWorldProjection {
  UiWorldProjection::new(
    ProjectionCamera::Input,
    Vector3::ZERO,
    Vector3::new(1.0, 0.0, 0.0),
    Vector3::new(0.0, 1.0, 0.0),
    Rect::new(3.2, -3.3, 2.6, 2.6),
  )
}
