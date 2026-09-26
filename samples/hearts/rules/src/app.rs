use battlement::{ObjectId, ParentScene, TextureAddress, UiFontAddress, Vector3, object_id};
use reactant::{Application, hooks, prelude::*, world};
use trox::{SourceLocale, ls};

use crate::{
  assets, card_assets, controller,
  domain::{HeartsState, Seat, cards},
};

const ROOT: ObjectId = object_id!("6644ed66-12dc-4590-9af8-19d174a47000");

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
      document
        .name("hearts")
        .style(Style::new().color(Color::rgb(0.97, 0.94, 0.83)))
    })
    .camera(|camera| {
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
    let faces = if self.gallery {
      cards::deck()
    } else {
      game.view.hands[Seat::South.index()]
        .iter()
        .filter_map(|card| card.face)
        .collect()
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
        .style(
          Style::new()
            .padding(24.px())
            .unity_font_definition(UiFontAddress::from(assets::hearts::fonts::CONTROL)),
        )
        .child((
          Heading::new(ls("Hearts"), 1).style(Style::new().font_size(44.px())),
          Label::new(ls(format!("Hand {}", game.view.table.hand_index + 1)))
            .style(Style::new().font_size(24.px())),
          Button::new(ls("New game"))
            .host_name("new-game")
            .style(
              Style::new()
                .width(140.px())
                .height(44.px())
                .font_size(20.px()),
            )
            .on_press(
              reset.update_callback(|value| value.checked_add(1).expect("session generation")),
            ),
        )),
      world::SceneRoot::new(ParentScene::PrimaryScene).child((
        surfaces,
        (!self.gallery).then(|| {
          world::Sprite::new()
            .texture(TextureAddress::from(assets::hearts::cards::BACK))
            .size(0.95, 1.4)
            .position(Vector3::new(0.0, -1.0, 0.0))
        }),
      )),
    )
  }
}
