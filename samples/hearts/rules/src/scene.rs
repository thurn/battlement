use battlement::{
  LightType, MaterialAssignment, PickingMode, PrefabAddress, Quaternion, ShadowMode, Vector3,
};
use reactant::{prelude::*, world};

use crate::{HumanView, assets, domain::Seat, layout::Layout};

const CAMERA_SIZE: f64 = 5.7;
const CAMERA_TILT: f64 = 60.0;

pub(crate) fn camera() -> world::Camera {
  world::Camera::new()
    .orthographic(CAMERA_SIZE)
    .clipping(0.1, 70.0)
    .background(Color::rgb(0.18, 0.26, 0.12))
    .position(Vector3::new(0.0, 20.0, -11.547005))
    .rotation(self::pitch(CAMERA_TILT))
}

pub(crate) fn environment(aspect: f64) -> impl Render {
  let portrait = aspect < 1.0;
  let half_width = 5.7 * aspect;
  (
    world::Plane::new()
      .scale(Vector3::new(6.0, 1.0, 6.0))
      .materials([MaterialAssignment::new(
        0,
        assets::hearts::materials::CLEARING,
      )]),
    world::Cylinder::new()
      .position(Vector3::new(0.0, 0.005, 0.0))
      .scale(Vector3::new(half_width * 1.30, 0.005, 7.0))
      .materials([MaterialAssignment::new(0, assets::hearts::materials::SAND)]),
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
        .intensity(0.25)
        .shadows(ShadowMode::None),
    ),
    self::forest(half_width, portrait),
  )
}

pub(crate) fn seats(view: &HumanView, layout: Layout, larger_text: bool) -> impl Render {
  let portrait = layout.portrait;
  let safe = layout.safe;
  let w = f64::from(layout.viewport.size.width);
  let h = f64::from(layout.viewport.size.height);
  [Seat::North, Seat::West, Seat::East, Seat::South]
    .into_iter()
    .map(|seat| {
      let (left, top) = if portrait {
        if seat == Seat::South {
          (
            safe.x + safe.width / 2.0,
            layout.hand_top(larger_text) - 40.0,
          )
        } else {
          let (x, y) = layout.opponent_center(seat, larger_text);
          (x, y - if seat == Seat::North { 70.0 } else { 84.0 })
        }
      } else {
        match seat {
          Seat::North => (w * 0.5, h * 0.04),
          Seat::West => (w * 0.22, h * 0.18),
          Seat::East => (w * 0.78, h * 0.18),
          Seat::South => (w * 0.5, h * if larger_text { 0.42 } else { 0.57 }),
        }
      };
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
      .semantic(reactant::control_behavior::static_text_props(trox::ls(
        format!("{name}: {} total points", view.table.totals[seat.index()]),
      )))
      .picking_mode(PickingMode::Ignore)
      .style(
        Style::new()
          .position(Position::Absolute)
          .left((left as f32).px())
          .top((top as f32).px())
          .width(if portrait { 96.px() } else { 150.px() })
          .margin_left(if portrait { (-48).px() } else { (-75).px() })
          .height(32.px())
          .font_size(if larger_text {
            if portrait { 24.px() } else { 28.px() }
          } else {
            22.px()
          })
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
      } else {
        -1.15 + (index % 3) as f64 * 0.18
      };
      let x = side * (half_width + inset);
      let elevated = !portrait && index >= 3;
      if portrait || elevated {
        objects.push(self::prop(
          assets::hearts::forest::HILL_4X2X2_COLOR1,
          x,
          if portrait { -0.8 } else { -0.45 },
          z,
          scale,
          side * 90.0,
        ));
      }
      objects.push(self::prop(
        if index % 2 == 0 {
          assets::hearts::forest::TREE_1_A_COLOR1
        } else {
          assets::hearts::forest::TREE_2_A_COLOR1
        },
        x,
        if elevated { 0.55 } else { 0.0 },
        z + 0.3,
        scale,
        index as f64 * 73.0,
      ));
      objects.push(self::prop(
        assets::hearts::forest::ROCK_1_A_COLOR1,
        x - side * 0.7,
        0.0,
        z - 0.9,
        scale * if portrait { 1.4 } else { 1.8 },
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
  let count = (half_width * 2.0 / if portrait { 2.3 } else { 3.1 }).ceil() as usize;
  for index in 0..=count {
    let x = -half_width + index as f64 * (2.0 * half_width / count as f64);
    objects.push(self::prop(
      if !portrait && index % 2 == 0 {
        assets::hearts::forest::TREE_1_A_COLOR1
      } else {
        assets::hearts::forest::TREE_2_A_COLOR1
      },
      x,
      0.0,
      if portrait {
        5.8
      } else {
        5.1 + (index % 2) as f64 * 0.7
      },
      scale
        * if portrait {
          1.0
        } else {
          0.75 + (index % 3) as f64 * 0.12
        },
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
      side * (half_width + if portrait { 0.3 } else { -1.1 } - (index % 3) as f64 * 0.1),
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

pub(crate) fn table_drag_delta(dx: f64, dy: f64, viewport_height: u32) -> Vector3 {
  let units = 2.0 * CAMERA_SIZE / f64::from(viewport_height);
  Vector3::new(
    dx * units,
    0.0,
    -dy * units / CAMERA_TILT.to_radians().sin(),
  )
}
