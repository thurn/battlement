use battlement::{
  LightType, MaterialAssignment, PickingMode, PrefabAddress, Quaternion, ShadowMode, Vector3,
};
use reactant::{prelude::*, world};

use crate::{HumanView, assets, domain::Seat, layout::Layout, scenery};

/// Orthographic half-height of the table camera in world units.
pub(crate) const CAMERA_SIZE: f64 = 5.7;
/// Downward pitch of the table camera in degrees.
pub(crate) const CAMERA_TILT: f64 = 60.0;

pub(crate) fn camera() -> world::Camera {
  world::Camera::new()
    .orthographic(CAMERA_SIZE)
    .clipping(0.1, 70.0)
    .background(Color::rgb(0.18, 0.26, 0.12))
    .position(Vector3::new(0.0, 20.0, -11.547005))
    .rotation(self::pitch(CAMERA_TILT))
}

/// Table surroundings; `raised_hand` clears the lower interior for the raised larger-text hand.
pub(crate) fn environment(layout: Layout, raised_hand: bool) -> impl Render {
  let size = layout.viewport.size;
  let aspect = f64::from(size.width) / f64::from(size.height);
  let half_width = CAMERA_SIZE * aspect;
  let portrait = aspect < 1.0;
  // Portrait cards and labels are placed in pixels, so their ground footprint
  // depends on the viewport's resolution as well as its shape.
  let clearance = scenery::Clearance::new(if portrait {
    layout
      .portrait_obstacles()
      .into_iter()
      .map(|rect| self::ground_rect(layout, rect))
      .collect()
  } else {
    Vec::new()
  });
  (
    world::Plane::new()
      .scale(Vector3::new(6.0, 1.0, 6.0))
      .materials([MaterialAssignment::new(
        0,
        assets::hearts::materials::CLEARING,
      )]),
    scenery::ground(half_width, portrait, raised_hand, clearance.clone()),
    world::Group::new().rotation(self::yaw(-35.0)).child(
      world::Light::new()
        .light_type(LightType::Directional)
        .rotation(self::pitch(66.0))
        .color(Color::rgb(1.0, 0.93, 0.78))
        .intensity(1.0)
        .shadows(ShadowMode::Soft),
    ),
    world::Group::new().rotation(self::yaw(-20.0)).child(
      world::Light::new()
        .light_type(LightType::Directional)
        .rotation(self::pitch(35.0))
        .color(Color::rgb(0.75, 0.84, 1.0))
        .intensity(0.3)
        .shadows(ShadowMode::None),
    ),
    world::Group::new().child(scenery::forest(
      half_width,
      portrait,
      raised_hand,
      &clearance,
    )),
  )
}

pub(crate) fn seats(view: &HumanView, layout: Layout, larger_text: bool) -> impl Render {
  let portrait = layout.portrait;
  let w = f64::from(layout.viewport.size.width);
  let h = f64::from(layout.viewport.size.height);
  [Seat::North, Seat::West, Seat::East, Seat::South]
    .into_iter()
    .map(|seat| {
      let (left, top) = if portrait {
        layout.label_origin(seat, larger_text)
      } else {
        match seat {
          Seat::North => (w * 0.5, h * 0.215),
          Seat::West => (w * 0.315, h * 0.36),
          Seat::East => (w * 0.685, h * 0.36),
          Seat::South => (w * 0.5, h * if larger_text { 0.42 } else { 0.635 }),
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

pub(crate) fn prop(
  address: PrefabAddress,
  x: f64,
  y: f64,
  z: f64,
  scale: f64,
  angle: f64,
) -> world::Group {
  world::Prefab::at(address)
    .position(Vector3::new(x, y, z))
    .scale(Vector3::new(scale, scale, scale))
    .rotation(self::yaw(angle))
}

fn pitch(degrees: f64) -> Quaternion {
  let (sin, cos) = (degrees.to_radians() / 2.0).sin_cos();
  Quaternion::new(sin, 0.0, 0.0, cos)
}

pub(crate) fn yaw(degrees: f64) -> Quaternion {
  let (sin, cos) = (degrees.to_radians() / 2.0).sin_cos();
  Quaternion::new(0.0, sin, 0.0, cos)
}

/// Ground rectangle `[left, right, near, far]` beneath screen rectangle `[left, top, right, bottom]`.
fn ground_rect(layout: Layout, [left, top, right, bottom]: [f64; 4]) -> [f64; 4] {
  let (left, near) = layout.ground_point(left, bottom);
  let (right, far) = layout.ground_point(right, top);
  [left, right, near, far]
}

pub(crate) fn table_drag_delta(dx: f64, dy: f64, viewport_height: u32) -> Vector3 {
  let units = 2.0 * CAMERA_SIZE / f64::from(viewport_height);
  Vector3::new(
    dx * units,
    0.0,
    -dy * units / CAMERA_TILT.to_radians().sin(),
  )
}
