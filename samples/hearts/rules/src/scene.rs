use battlement::{
  LightType, MaterialAssignment, PickingMode, PrefabAddress, Quaternion, ShadowMode, Vector3,
};
use reactant::{prelude::*, world};

use crate::{HumanView, assets, card_assets, domain::Seat};

const CARD_WIDTH: f64 = 4.31462;

pub(crate) fn camera() -> world::Camera {
  world::Camera::new()
    .orthographic(5.7)
    .clipping(0.1, 70.0)
    .background(Color::rgb(0.18, 0.26, 0.12))
    .position(Vector3::new(0.0, 20.0, -11.547005))
    .rotation(self::pitch(60.0))
}

pub(crate) fn table(view: &HumanView, aspect: f64) -> impl Render {
  let portrait = aspect < 1.0;
  let half_width = 5.7 * aspect;
  let faces: Vec<_> = view.hands[Seat::South.index()]
    .iter()
    .enumerate()
    .map(|(index, card)| {
      let (x, z, width, angle) = if portrait {
        let column = index % 7;
        let count = if index < 7 {
          view.hands[Seat::South.index()].len().min(7)
        } else {
          view.hands[Seat::South.index()].len() - 7
        };
        let spacing = (half_width * 2.0 - 0.65) / 7.0;
        (
          (column as f64 - (count as f64 - 1.0) / 2.0) * spacing,
          -2.5 - (index / 7) as f64 * 1.55,
          spacing * 1.12,
          0.0,
        )
      } else {
        let offset = index as f64 - (view.hands[Seat::South.index()].len() as f64 - 1.0) / 2.0;
        let spacing = ((half_width * 2.0 - 3.0) / 13.0).min(1.16);
        (
          offset * spacing,
          -3.5 - offset * offset * 0.01,
          spacing * 2.0,
          offset * 2.3,
        )
      };
      let scale = width / CARD_WIDTH;
      world::Group::new()
        .position(Vector3::new(x, 0.12 + index as f64 * 0.008, z))
        .rotation(self::yaw(angle + 180.0))
        .child(
          world::Prefab::at(card_assets::model(card.face.expect("own card face")))
            .rotation(self::pitch(-90.0))
            .scale(Vector3::new(scale, scale, scale)),
        )
    })
    .collect();
  let opponents: Vec<_> = [Seat::North, Seat::West, Seat::East]
    .into_iter()
    .map(|seat| {
      let (x, z, angle, width, spacing) = match seat {
        Seat::North => (
          0.0,
          3.6,
          180.0,
          if portrait { 0.72 } else { 1.65 },
          if portrait { 0.22 } else { 0.56 },
        ),
        Seat::West => (
          -half_width * if portrait { 0.7 } else { 0.60 },
          0.6,
          90.0,
          if portrait { 0.72 } else { 1.5 },
          if portrait { 0.17 } else { 0.32 },
        ),
        Seat::East => (
          half_width * if portrait { 0.7 } else { 0.60 },
          0.6,
          -90.0,
          if portrait { 0.72 } else { 1.5 },
          if portrait { 0.17 } else { 0.32 },
        ),
        Seat::South => unreachable!(),
      };
      let cards: Vec<_> = (0..view.hands[seat.index()].len())
        .map(|index| {
          let offset = index as f64 - (view.hands[seat.index()].len() as f64 - 1.0) / 2.0;
          world::Group::new()
            .position(Vector3::new(
              offset * spacing,
              0.12 + index as f64 * 0.008,
              -offset * offset * 0.01,
            ))
            .rotation(self::yaw(-offset * 2.5))
            .child(
              world::Sprite::new()
                .texture(assets::hearts::cards::BACK)
                .size(width, width * 6.0 / CARD_WIDTH)
                .rotation(self::pitch(90.0)),
            )
        })
        .collect();
      world::Group::new()
        .position(Vector3::new(x, 0.0, z))
        .rotation(self::yaw(angle))
        .child(cards)
    })
    .collect();
  (
    world::Plane::new()
      .scale(Vector3::new(6.0, 1.0, 6.0))
      .materials([MaterialAssignment::new(
        0,
        assets::hearts::materials::CLEARING,
      )]),
    world::Group::new().rotation(self::yaw(-35.0)).child(
      world::Light::new()
        .light_type(LightType::Directional)
        .rotation(self::pitch(55.0))
        .color(Color::rgb(1.0, 0.96, 0.84))
        .intensity(0.9)
        .shadows(ShadowMode::Soft),
    ),
    world::Group::new().rotation(self::yaw(145.0)).child(
      world::Light::new()
        .light_type(LightType::Directional)
        .rotation(self::pitch(35.0))
        .color(Color::rgb(0.75, 0.84, 1.0))
        .intensity(0.15)
        .shadows(ShadowMode::None),
    ),
    self::forest(half_width, portrait),
    faces,
    opponents,
  )
}

pub(crate) fn seats(view: &HumanView, portrait: bool) -> impl Render {
  [
    (Seat::North, 50.0, if portrait { 13.0 } else { 4.0 }),
    (
      Seat::West,
      if portrait { 16.0 } else { 22.0 },
      if portrait { 28.0 } else { 18.0 },
    ),
    (
      Seat::East,
      if portrait { 84.0 } else { 78.0 },
      if portrait { 28.0 } else { 18.0 },
    ),
    (Seat::South, 50.0, if portrait { 60.0 } else { 57.0 }),
  ]
  .into_iter()
  .map(|(seat, left, top)| {
    let name = match seat {
      Seat::North => "North",
      Seat::West => "West",
      Seat::East => "East",
      Seat::South => "You",
    };
    Label::new(trox::ls(format!(
      "{name}  ·  {}",
      view.table.totals[seat.index()]
    )))
    .picking_mode(PickingMode::Ignore)
    .style(
      Style::new()
        .position(Position::Absolute)
        .left(left.pct())
        .top(top.pct())
        .width(120.px())
        .margin_left((-60).px())
        .height(32.px())
        .font_size(22.px())
        .unity_text_align(TextAnchor::MiddleCenter)
        .color(Color::rgb(0.08, 0.14, 0.06)),
    )
  })
  .collect::<Vec<_>>()
}

fn forest(half_width: f64, portrait: bool) -> Vec<world::Group> {
  let mut objects = Vec::new();
  let scale = if portrait { 0.48 } else { 0.60 };
  for side in [-1.0, 1.0] {
    for (index, z) in [-5.8, -2.9, 0.5, 3.6, 6.4].into_iter().enumerate() {
      let inset = if portrait {
        0.85
      } else if index == 0 {
        0.2
      } else {
        -0.25
      };
      let x = side * (half_width + inset);
      objects.push(self::prop(
        assets::hearts::forest::HILL_4X2X2_COLOR1,
        x,
        -0.8,
        z,
        scale,
        side * 90.0,
      ));
      objects.push(self::prop(
        if index % 2 == 0 {
          assets::hearts::forest::TREE_1_A_COLOR1
        } else {
          assets::hearts::forest::TREE_2_A_COLOR1
        },
        x,
        0.0,
        z + 0.3,
        scale,
        index as f64 * 73.0,
      ));
      objects.push(self::prop(
        assets::hearts::forest::ROCK_1_A_COLOR1,
        x - side * 0.7,
        0.0,
        z - 0.9,
        scale * 1.4,
        index as f64 * 42.0,
      ));
      objects.push(self::prop(
        assets::hearts::forest::BUSH_1_A_COLOR1,
        x - side * 0.45,
        0.0,
        z + 1.0,
        scale * 3.5,
        0.0,
      ));
    }
  }
  let count = (half_width * 2.0 / 2.3).ceil() as usize;
  for index in 0..=count {
    let x = -half_width + index as f64 * (2.0 * half_width / count as f64);
    objects.push(self::prop(
      assets::hearts::forest::TREE_2_A_COLOR1,
      x,
      0.0,
      5.8,
      scale,
      index as f64 * 51.0,
    ));
    if x.abs() > half_width * 0.72 {
      objects.push(self::prop(
        assets::hearts::forest::BUSH_1_A_COLOR1,
        x,
        0.0,
        -6.1,
        scale * 4.0,
        0.0,
      ));
    }
  }
  for index in 0..24 {
    let side = if index % 2 == 0 { -1.0 } else { 1.0 };
    objects.push(self::prop(
      assets::hearts::forest::GRASS_1_A_COLOR1,
      side * (half_width + if portrait { 0.3 } else { -0.2 } - (index % 3) as f64 * 0.1),
      0.0,
      -5.3 + (index / 2) as f64 * 0.92,
      scale,
      index as f64 * 37.0,
    ));
  }
  objects
}

fn prop(address: PrefabAddress, x: f64, y: f64, z: f64, scale: f64, angle: f64) -> world::Group {
  world::Prefab::at(address)
    .position(Vector3::new(x, y, z))
    .scale(Vector3::new(scale, scale, scale))
    .rotation(self::yaw(angle))
}

fn pitch(degrees: f64) -> Quaternion {
  let (sin, cos) = (degrees.to_radians() / 2.0).sin_cos();
  Quaternion::new(sin, 0.0, 0.0, cos)
}

fn yaw(degrees: f64) -> Quaternion {
  let (sin, cos) = (degrees.to_radians() / 2.0).sin_cos();
  Quaternion::new(0.0, sin, 0.0, cos)
}
