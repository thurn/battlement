//! Value interpolation for deterministic fake-host operations.

use std::f64::consts::PI;

use battlement::{Easing, GameObjectKind, Quaternion, RgbColor, Vector3};

use crate::{transform, world::FakeWorld};

pub(crate) enum OperationValue {
  Wait,
  LocalPosition {
    object_id: battlement::ObjectId,
    start: Vector3,
    target: Vector3,
  },
  WorldPosition {
    object_id: battlement::ObjectId,
    start: Vector3,
    target: Vector3,
  },
  LocalRotation {
    object_id: battlement::ObjectId,
    start: Quaternion,
    target: Quaternion,
  },
  WorldRotation {
    object_id: battlement::ObjectId,
    start: Quaternion,
    target: Quaternion,
  },
  LocalScale {
    object_id: battlement::ObjectId,
    start: Vector3,
    target: Vector3,
  },
  CameraFieldOfView {
    object_id: battlement::ObjectId,
    start: f64,
    target: f64,
  },
  CameraOrthographicSize {
    object_id: battlement::ObjectId,
    start: f64,
    target: f64,
  },
  LightColor {
    object_id: battlement::ObjectId,
    start: battlement::Color,
    target: battlement::Color,
  },
  LightIntensity {
    object_id: battlement::ObjectId,
    start: f64,
    target: f64,
  },
  ImageTint {
    object_id: battlement::ObjectId,
    start: RgbColor,
    target: RgbColor,
  },
  ImageOpacity {
    object_id: battlement::ObjectId,
    start: f64,
    target: f64,
  },
  TextSize {
    object_id: battlement::ObjectId,
    start: f64,
    target: f64,
  },
  TextColor {
    object_id: battlement::ObjectId,
    start: battlement::Color,
    target: battlement::Color,
  },
  AudioVolume {
    command_id: battlement::CommandId,
    start: f64,
    target: f64,
  },
}

impl OperationValue {
  pub(crate) fn apply(&self, world: &mut FakeWorld, factor: f64) {
    match self {
      Self::Wait => {}
      Self::LocalPosition {
        object_id,
        start,
        target,
      } => world.set_local_position(*object_id, vector(*start, *target, factor)),
      Self::WorldPosition {
        object_id,
        start,
        target,
      } => world.set_world_position(*object_id, vector(*start, *target, factor)),
      Self::LocalRotation {
        object_id,
        start,
        target,
      } => world.set_local_rotation(*object_id, quaternion(*start, *target, factor)),
      Self::WorldRotation {
        object_id,
        start,
        target,
      } => world.set_world_rotation(*object_id, quaternion(*start, *target, factor)),
      Self::LocalScale {
        object_id,
        start,
        target,
      } => world.set_local_scale(*object_id, vector(*start, *target, factor)),
      Self::CameraFieldOfView {
        object_id,
        start,
        target,
      } => {
        let camera = world.camera_mut(*object_id);
        camera.projection = battlement::CameraProjection::Perspective;
        camera.field_of_view = scalar(*start, *target, factor);
      }
      Self::CameraOrthographicSize {
        object_id,
        start,
        target,
      } => {
        let camera = world.camera_mut(*object_id);
        camera.projection = battlement::CameraProjection::Orthographic;
        camera.orthographic_size = scalar(*start, *target, factor);
      }
      Self::LightColor {
        object_id,
        start,
        target,
      } => world.light_mut(*object_id).color = color(*start, *target, factor),
      Self::LightIntensity {
        object_id,
        start,
        target,
      } => world.light_mut(*object_id).intensity = scalar(*start, *target, factor),
      Self::ImageTint {
        object_id,
        start,
        target,
      } => match &mut world.object_mut(*object_id).kind {
        GameObjectKind::Image { image } => image.tint = rgb(*start, *target, factor),
        _ => panic!("object is not an image: {object_id}"),
      },
      Self::ImageOpacity {
        object_id,
        start,
        target,
      } => match &mut world.object_mut(*object_id).kind {
        GameObjectKind::Image { image } => image.opacity = scalar(*start, *target, factor),
        _ => panic!("object is not an image: {object_id}"),
      },
      Self::TextSize {
        object_id,
        start,
        target,
      } => match &mut world.object_mut(*object_id).kind {
        GameObjectKind::Text { text } => text.size = scalar(*start, *target, factor),
        _ => panic!("object is not text: {object_id}"),
      },
      Self::TextColor {
        object_id,
        start,
        target,
      } => match &mut world.object_mut(*object_id).kind {
        GameObjectKind::Text { text } => text.color = color(*start, *target, factor),
        _ => panic!("object is not text: {object_id}"),
      },
      Self::AudioVolume {
        command_id,
        start,
        target,
      } => world.audio_mut(*command_id).volume = scalar(*start, *target, factor),
    }
  }
}

fn scalar(start: f64, target: f64, factor: f64) -> f64 {
  start + (target - start) * factor
}

fn vector(start: Vector3, target: Vector3, factor: f64) -> Vector3 {
  Vector3::new(
    scalar(start.x, target.x, factor),
    scalar(start.y, target.y, factor),
    scalar(start.z, target.z, factor),
  )
}

fn rgb(start: RgbColor, target: RgbColor, factor: f64) -> RgbColor {
  RgbColor {
    r: scalar(start.r, target.r, factor),
    g: scalar(start.g, target.g, factor),
    b: scalar(start.b, target.b, factor),
  }
}

fn color(start: battlement::Color, target: battlement::Color, factor: f64) -> battlement::Color {
  battlement::Color {
    r: scalar(start.r, target.r, factor),
    g: scalar(start.g, target.g, factor),
    b: scalar(start.b, target.b, factor),
    a: scalar(start.a, target.a, factor),
  }
}

fn quaternion(start: Quaternion, mut target: Quaternion, factor: f64) -> Quaternion {
  let start = transform::normalize(start);
  target = transform::normalize(target);
  let mut dot = start.x * target.x + start.y * target.y + start.z * target.z + start.w * target.w;
  if dot < 0.0 {
    dot = -dot;
    target = Quaternion {
      x: -target.x,
      y: -target.y,
      z: -target.z,
      w: -target.w,
    };
  }
  if dot > 0.9995 {
    return transform::normalize(Quaternion {
      x: scalar(start.x, target.x, factor),
      y: scalar(start.y, target.y, factor),
      z: scalar(start.z, target.z, factor),
      w: scalar(start.w, target.w, factor),
    });
  }
  let theta = dot.clamp(-1.0, 1.0).acos();
  let sin_theta = theta.sin();
  let left = ((1.0 - factor) * theta).sin() / sin_theta;
  let right = (factor * theta).sin() / sin_theta;
  transform::normalize(Quaternion {
    x: start.x * left + target.x * right,
    y: start.y * left + target.y * right,
    z: start.z * left + target.z * right,
    w: start.w * left + target.w * right,
  })
}

pub(crate) fn ease(easing: Easing, value: f64) -> f64 {
  match easing {
    Easing::Linear => value,
    Easing::InSine => 1.0 - (value * PI / 2.0).cos(),
    Easing::OutSine => (value * PI / 2.0).sin(),
    Easing::InOutSine => -((PI * value).cos() - 1.0) / 2.0,
    Easing::InQuad => value * value,
    Easing::OutQuad => 1.0 - (1.0 - value).powi(2),
    Easing::InOutQuad => {
      if value < 0.5 {
        2.0 * value * value
      } else {
        1.0 - (-2.0 * value + 2.0).powi(2) / 2.0
      }
    }
    Easing::InCubic => value.powi(3),
    Easing::OutCubic => 1.0 - (1.0 - value).powi(3),
    Easing::InOutCubic => {
      if value < 0.5 {
        4.0 * value.powi(3)
      } else {
        1.0 - (-2.0 * value + 2.0).powi(3) / 2.0
      }
    }
    Easing::InQuart => value.powi(4),
    Easing::OutQuart => 1.0 - (1.0 - value).powi(4),
    Easing::InOutQuart => {
      if value < 0.5 {
        8.0 * value.powi(4)
      } else {
        1.0 - (-2.0 * value + 2.0).powi(4) / 2.0
      }
    }
    Easing::InQuint => value.powi(5),
    Easing::OutQuint => 1.0 - (1.0 - value).powi(5),
    Easing::InOutQuint => {
      if value < 0.5 {
        16.0 * value.powi(5)
      } else {
        1.0 - (-2.0 * value + 2.0).powi(5) / 2.0
      }
    }
    Easing::InExpo => {
      if value == 0.0 {
        0.0
      } else {
        2.0_f64.powf(10.0 * value - 10.0)
      }
    }
    Easing::OutExpo => {
      if value == 1.0 {
        1.0
      } else {
        1.0 - 2.0_f64.powf(-10.0 * value)
      }
    }
    Easing::InOutExpo => {
      if value == 0.0 {
        0.0
      } else if value == 1.0 {
        1.0
      } else if value < 0.5 {
        2.0_f64.powf(20.0 * value - 10.0) / 2.0
      } else {
        (2.0 - 2.0_f64.powf(-20.0 * value + 10.0)) / 2.0
      }
    }
    Easing::InCirc => 1.0 - (1.0 - value * value).sqrt(),
    Easing::OutCirc => (1.0 - (value - 1.0).powi(2)).sqrt(),
    Easing::InOutCirc => {
      if value < 0.5 {
        (1.0 - (1.0 - (2.0 * value).powi(2)).sqrt()) / 2.0
      } else {
        ((1.0 - (-2.0 * value + 2.0).powi(2)).sqrt() + 1.0) / 2.0
      }
    }
    Easing::InBack => 2.70158 * value.powi(3) - 1.70158 * value.powi(2),
    Easing::OutBack => 1.0 + 2.70158 * (value - 1.0).powi(3) + 1.70158 * (value - 1.0).powi(2),
    Easing::InOutBack => {
      if value < 0.5 {
        ((2.0 * value).powi(2) * (7.189819 * value - 2.5949095)) / 2.0
      } else {
        ((2.0 * value - 2.0).powi(2) * (3.5949095 * (value * 2.0 - 2.0) + 2.5949095) + 2.0) / 2.0
      }
    }
    Easing::InElastic => {
      if value == 0.0 || value == 1.0 {
        value
      } else {
        -2.0_f64.powf(10.0 * value - 10.0) * ((value * 10.0 - 10.75) * 2.0 * PI / 3.0).sin()
      }
    }
    Easing::OutElastic => {
      if value == 0.0 || value == 1.0 {
        value
      } else {
        2.0_f64.powf(-10.0 * value) * ((value * 10.0 - 0.75) * 2.0 * PI / 3.0).sin() + 1.0
      }
    }
    Easing::InOutElastic => {
      if value == 0.0 || value == 1.0 {
        value
      } else if value < 0.5 {
        -(2.0_f64.powf(20.0 * value - 10.0) * ((20.0 * value - 11.125) * 2.0 * PI / 4.5).sin())
          / 2.0
      } else {
        (2.0_f64.powf(-20.0 * value + 10.0) * ((20.0 * value - 11.125) * 2.0 * PI / 4.5).sin())
          / 2.0
          + 1.0
      }
    }
    Easing::InBounce => 1.0 - bounce(1.0 - value),
    Easing::OutBounce => bounce(value),
    Easing::InOutBounce => {
      if value < 0.5 {
        (1.0 - bounce(1.0 - 2.0 * value)) / 2.0
      } else {
        (1.0 + bounce(2.0 * value - 1.0)) / 2.0
      }
    }
  }
}

fn bounce(value: f64) -> f64 {
  if value < 1.0 / 2.75 {
    7.5625 * value * value
  } else if value < 2.0 / 2.75 {
    let value = value - 1.5 / 2.75;
    7.5625 * value * value + 0.75
  } else if value < 2.5 / 2.75 {
    let value = value - 2.25 / 2.75;
    7.5625 * value * value + 0.9375
  } else {
    let value = value - 2.625 / 2.75;
    7.5625 * value * value + 0.984375
  }
}
