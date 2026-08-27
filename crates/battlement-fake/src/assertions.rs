use battlement::Vector3;

use crate::client::PointerInput;

pub(crate) fn validate_pointer_input(input: PointerInput) {
  assert!(input.pointer_id >= 0, "pointer ID must be nonnegative");
  assert!(
    input.screen_position.x.is_finite(),
    "pointer screen x must be finite"
  );
  assert!(
    input.screen_position.y.is_finite(),
    "pointer screen y must be finite"
  );
  assert!(
    input.world_hit.x.is_finite(),
    "pointer world x must be finite"
  );
  assert!(
    input.world_hit.y.is_finite(),
    "pointer world y must be finite"
  );
  assert!(
    input.world_hit.z.is_finite(),
    "pointer world z must be finite"
  );
}

pub(crate) fn validate_world_position(value: Vector3) {
  assert!(value.x.is_finite(), "drag world x must be finite");
  assert!(value.y.is_finite(), "drag world y must be finite");
  assert!(value.z.is_finite(), "drag world z must be finite");
}

pub(crate) fn assert_transform_close(
  actual: battlement::LocalTransform,
  expected: battlement::LocalTransform,
  tolerance: f64,
  label: &str,
) {
  assert_vector_close(actual.position, expected.position, tolerance, label);
  assert_vector_close(actual.scale, expected.scale, tolerance, label);
  assert_quaternion_close(actual.rotation, expected.rotation, tolerance, label);
}

pub(crate) fn assert_transform_close_world(
  actual: crate::world::WorldTransform,
  expected: crate::world::WorldTransform,
  tolerance: f64,
  label: &str,
) {
  assert_vector_close(actual.position, expected.position, tolerance, label);
  assert_vector_close(actual.scale, expected.scale, tolerance, label);
  assert_quaternion_close(actual.rotation, expected.rotation, tolerance, label);
}

pub(crate) fn assert_vector_close(actual: Vector3, expected: Vector3, tolerance: f64, label: &str) {
  assert!(tolerance >= 0.0, "tolerance must be nonnegative");
  assert!(
    (actual.x - expected.x).abs() <= tolerance,
    "{label} x mismatch"
  );
  assert!(
    (actual.y - expected.y).abs() <= tolerance,
    "{label} y mismatch"
  );
  assert!(
    (actual.z - expected.z).abs() <= tolerance,
    "{label} z mismatch"
  );
}

fn assert_quaternion_close(
  actual: battlement::Quaternion,
  expected: battlement::Quaternion,
  tolerance: f64,
  label: &str,
) {
  let direct = (actual.x - expected.x).abs() <= tolerance
    && (actual.y - expected.y).abs() <= tolerance
    && (actual.z - expected.z).abs() <= tolerance
    && (actual.w - expected.w).abs() <= tolerance;
  let negated = (actual.x + expected.x).abs() <= tolerance
    && (actual.y + expected.y).abs() <= tolerance
    && (actual.z + expected.z).abs() <= tolerance
    && (actual.w + expected.w).abs() <= tolerance;
  assert!(direct || negated, "{label} rotation mismatch");
}
