use battlement::{ObjectId, ParentScene, PickingMode, Prop, UiFontAddress, Vector3, object_id};
use reactant::{Application, app_context, hooks, prelude::*, world};
use trox::{SourceLocale, ls};

use crate::{
  assets, card_assets, controller,
  domain::{HeartsState, Seat, cards},
  scene,
};

pub(crate) const ROOT: ObjectId = object_id!("6644ed66-12dc-4590-9af8-19d174a47000");

struct HeartsRoot {
  initial: HeartsState,
  gallery: bool,
}

/// Opens a new match through the same root used for restored state.
pub fn application() -> Application {
  self::application_from_state(HeartsState::new(43))
}

/// Mounts an already validated logical match without replaying its history.
pub fn application_from_state(initial: HeartsState) -> Application {
  self::configured(initial, false)
}

pub(crate) fn exported_application() -> Application {
  match std::env::var("BATTLEMENT_DITTO_SEMANTIC_FIXTURE").as_deref() {
    Ok("layout") => crate::layout_fixture::application(),
    Ok("cards") => self::configured(HeartsState::new(43), true),
    Ok("restored") => self::application_from_state(HeartsState::new(73)),
    Ok("shell") | Err(_) => self::application(),
    Ok(name) => panic!("unknown Hearts fixture {name:?}"),
  }
}

fn configured(initial: HeartsState, gallery: bool) -> Application {
  Application::new(assets::hearts::CONTENT)
    .source_locale(SourceLocale::new("en-US").expect("source locale"))
    .child(HeartsRoot { initial, gallery })
    .document(|mut document| {
      document.root_id = ROOT;
      document.element.picking_mode = Prop::Set(PickingMode::Ignore);
      document
        .name("hearts")
        .style(Style::new().color(Color::rgb(0.97, 0.94, 0.83)))
    })
    .camera(move |camera| {
      if !gallery {
        return scene::camera().into_object(camera.object_id);
      }
      world::Camera::new()
        .orthographic(4.8)
        .clipping(0.1, 50.0)
        .background(Color::rgb(0.12, 0.23, 0.15))
        .position(Vector3::new(0.0, 0.0, -10.0))
        .into_object(camera.object_id)
    })
}

impl Component for HeartsRoot {
  fn render(&self) -> impl Render {
    let (generation, reset) = hooks::use_state(0_u64);
    let initial = if generation == 0 {
      self.initial.clone()
    } else {
      HeartsState::new(43)
    };
    let game = controller::use_hearts(generation, move || initial, Seat::South, true);
    let viewport = app_context::use_viewport_size();
    let aspect = f64::from(viewport.width) / f64::from(viewport.height);
    let faces = if self.gallery {
      cards::deck()
    } else {
      Vec::new()
    };
    let surfaces: Vec<_> = faces
      .into_iter()
      .enumerate()
      .map(|(index, card)| {
        world::Sprite::new()
          .texture(card_assets::face(card))
          .size(0.95, 1.4)
          .position(Vector3::new(
            (index % 13) as f64 * 1.05 - 6.3,
            1.5 - (index / 13) as f64 * 1.65,
            0.0,
          ))
      })
      .collect();
    (
      View::new()
        .picking_mode(PickingMode::Ignore)
        .style(
          Style::new()
            .padding(18.px())
            .color(if self.gallery {
              Color::rgb(0.97, 0.94, 0.83)
            } else {
              Color::rgb(0.06, 0.12, 0.03)
            })
            .unity_font_definition(UiFontAddress::from(assets::hearts::fonts::CONTROL)),
        )
        .child((
          Heading::new(ls("Hearts"), 1).style(Style::new().font_size(32.px())),
          Label::new(ls(format!("Hand {}", game.view.table.hand_index + 1)))
            .style(Style::new().font_size(24.px())),
          Button::new(ls("New game"))
            .host_name("new-game")
            .style(
              Style::new()
                .position(Position::Absolute)
                .right(18.px())
                .top(18.px())
                .width(120.px())
                .height(44.px())
                .font_size(20.px())
                .color(Color::rgb(0.08, 0.14, 0.06))
                .background_color(Color::rgb(0.96, 0.92, 0.77))
                .border_radius(6.px()),
            )
            .on_press(
              reset.update_callback(|value| value.checked_add(1).expect("session generation")),
            ),
        )),
      world::SceneRoot::new(ParentScene::PrimaryScene).child((
        surfaces,
        (!self.gallery).then(|| scene::table(&game.view, aspect)),
      )),
      (!self.gallery).then(|| {
        View::new()
          .picking_mode(PickingMode::Ignore)
          .style(
            Style::new()
              .position(Position::Absolute)
              .left(0)
              .top(0)
              .width(100.pct())
              .height(100.pct())
              .unity_font_definition(UiFontAddress::from(assets::hearts::fonts::CONTROL)),
          )
          .child(scene::seats(&game.view, aspect < 1.0))
      }),
    )
  }
}
