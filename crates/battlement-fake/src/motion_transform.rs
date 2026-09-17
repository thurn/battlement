//! Transform channels composed after base placement by the shared Motion sampler.

use battlement::{LocalTransform, MotionProperty, MotionValue, ObjectId, Quaternion, Vector3};

use crate::{transform, world::FakeWorld};

#[derive(Clone)]
pub(crate) struct TransformWriter {
  host: ObjectId,
  position: [f64; 3],
  rotation: [f64; 3],
  scale: [f64; 3],
  offset: [f64; 3],
  tilt: [f64; 3],
  factor: [f64; 3],
  displayed: LocalTransform,
}

impl TransformWriter {
  pub(crate) fn new(host: ObjectId, world: &FakeWorld) -> Self {
    let pose = world.require_object(host).local_transform();
    Self {
      host,
      position: channels(pose.position),
      rotation: angles(pose.rotation),
      scale: channels(pose.scale),
      offset: [0.0; 3],
      tilt: [0.0; 3],
      factor: [1.0; 3],
      displayed: pose,
    }
  }

  pub(crate) fn read(&mut self, property: MotionProperty, world: &FakeWorld) -> MotionValue {
    self.synchronize(world);
    let (group, axis) = channel(property);
    let values = match group {
      0 => self.position,
      1 => self.rotation,
      2 => self.scale,
      3 => self.offset,
      4 => self.tilt,
      5 => self.factor,
      _ => unreachable!(),
    };
    MotionValue::Scalar(values[axis] as f32)
  }

  pub(crate) fn write(
    &mut self,
    property: MotionProperty,
    value: &MotionValue,
    world: &mut FakeWorld,
  ) {
    self.synchronize(world);
    let MotionValue::Scalar(value) = value else {
      panic!("world Motion requires a scalar value");
    };
    assert!(
      value.is_finite(),
      "world Motion produced a non-finite value"
    );
    let (group, axis) = channel(property);
    let values = match group {
      0 => &mut self.position,
      1 => &mut self.rotation,
      2 => &mut self.scale,
      3 => &mut self.offset,
      4 => &mut self.tilt,
      5 => &mut self.factor,
      _ => unreachable!(),
    };
    values[axis] = f64::from(*value);
    let rotation = quaternion(self.rotation);
    let delta = transform::rotate(rotation, vector(product(self.scale, self.offset)));
    world.set_local_position(
      self.host,
      Vector3::new(
        self.position[0] + delta.x,
        self.position[1] + delta.y,
        self.position[2] + delta.z,
      ),
    );
    world.set_local_rotation(
      self.host,
      transform::multiply(rotation, quaternion(self.tilt)),
    );
    world.set_local_scale(self.host, vector(product(self.scale, self.factor)));
    self.displayed = world.require_object(self.host).local_transform();
  }

  fn synchronize(&mut self, world: &FakeWorld) {
    let pose = world.require_object(self.host).local_transform();
    if pose.rotation != self.displayed.rotation {
      self.rotation = angles(transform::multiply(
        pose.rotation,
        transform::inverse(quaternion(self.tilt)),
      ));
    }
    if pose.scale != self.displayed.scale {
      for (axis, value) in channels(pose.scale).into_iter().enumerate() {
        if self.factor[axis] != 0.0 {
          self.scale[axis] = value / self.factor[axis];
        }
      }
    }
    if pose.position != self.displayed.position {
      let delta = channels(transform::rotate(
        quaternion(self.rotation),
        vector(product(self.scale, self.offset)),
      ));
      self.position = std::array::from_fn(|axis| channels(pose.position)[axis] - delta[axis]);
    }
  }
}

fn channel(property: MotionProperty) -> (usize, usize) {
  assert!(
    property.is_world_transform(),
    "a UI property cannot animate a world transform"
  );
  let index = property as usize - MotionProperty::LocalPositionX as usize;
  (index / 3, index % 3)
}

fn channels(value: Vector3) -> [f64; 3] {
  [value.x, value.y, value.z]
}
fn vector(value: [f64; 3]) -> Vector3 {
  Vector3::new(value[0], value[1], value[2])
}
fn product(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
  std::array::from_fn(|axis| left[axis] * right[axis])
}

fn quaternion(degrees: [f64; 3]) -> Quaternion {
  let [(sx, cx), (sy, cy), (sz, cz)] = degrees.map(|angle| (angle.to_radians() * 0.5).sin_cos());
  transform::multiply(
    transform::multiply(
      Quaternion {
        x: 0.0,
        y: sy,
        z: 0.0,
        w: cy,
      },
      Quaternion {
        x: sx,
        y: 0.0,
        z: 0.0,
        w: cx,
      },
    ),
    Quaternion {
      x: 0.0,
      y: 0.0,
      z: sz,
      w: cz,
    },
  )
}

fn angles(value: Quaternion) -> [f64; 3] {
  let q = transform::normalize(value);
  let sine = (2.0 * (q.w * q.x - q.y * q.z)).clamp(-1.0, 1.0);
  let x = sine.asin();
  let (y, z) = if sine.abs() < 0.9999999 {
    (
      (2.0 * (q.x * q.z + q.w * q.y)).atan2(1.0 - 2.0 * (q.x * q.x + q.y * q.y)),
      (2.0 * (q.x * q.y + q.w * q.z)).atan2(1.0 - 2.0 * (q.x * q.x + q.z * q.z)),
    )
  } else {
    (
      (2.0 * (q.w * q.y - q.x * q.z)).atan2(1.0 - 2.0 * (q.y * q.y + q.z * q.z)),
      0.0,
    )
  };
  [x, y, z].map(|angle| angle.to_degrees().rem_euclid(360.0))
}
