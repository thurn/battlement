//! Small `f64` transform helpers used by the fake world.

use masonry::{LocalTransform, Quaternion, Vector3};

use crate::world::WorldTransform;

pub(crate) fn normalize(value: Quaternion) -> Quaternion {
    let length =
        (value.x * value.x + value.y * value.y + value.z * value.z + value.w * value.w).sqrt();
    assert!(
        length > 0.0 && length.is_finite(),
        "cannot normalize quaternion"
    );
    Quaternion {
        x: value.x / length,
        y: value.y / length,
        z: value.z / length,
        w: value.w / length,
    }
}

pub(crate) fn multiply(left: Quaternion, right: Quaternion) -> Quaternion {
    normalize(multiply_raw(left, right))
}

fn multiply_raw(left: Quaternion, right: Quaternion) -> Quaternion {
    Quaternion {
        x: left.w * right.x + left.x * right.w + left.y * right.z - left.z * right.y,
        y: left.w * right.y - left.x * right.z + left.y * right.w + left.z * right.x,
        z: left.w * right.z + left.x * right.y - left.y * right.x + left.z * right.w,
        w: left.w * right.w - left.x * right.x - left.y * right.y - left.z * right.z,
    }
}

pub(crate) fn inverse(value: Quaternion) -> Quaternion {
    let normalized = normalize(value);
    Quaternion {
        x: -normalized.x,
        y: -normalized.y,
        z: -normalized.z,
        w: normalized.w,
    }
}

pub(crate) fn rotate(rotation: Quaternion, vector: Vector3) -> Vector3 {
    let rotation = normalize(rotation);
    let quaternion = Quaternion {
        x: vector.x,
        y: vector.y,
        z: vector.z,
        w: 0.0,
    };
    let rotated = multiply_raw(multiply_raw(rotation, quaternion), inverse(rotation));
    Vector3::new(rotated.x, rotated.y, rotated.z)
}

pub(crate) fn compose(parent: WorldTransform, local: LocalTransform) -> WorldTransform {
    WorldTransform {
        position: add(
            parent.position,
            rotate(
                parent.rotation,
                multiply_vector(parent.scale, local.position),
            ),
        ),
        rotation: multiply(parent.rotation, local.rotation),
        scale: multiply_vector(parent.scale, local.scale),
    }
}

pub(crate) fn relative(parent: Option<WorldTransform>, world: WorldTransform) -> LocalTransform {
    let Some(parent) = parent else {
        return LocalTransform {
            position: world.position,
            rotation: normalize(world.rotation),
            scale: world.scale,
        };
    };
    let delta = subtract(world.position, parent.position);
    LocalTransform {
        position: divide_vector(rotate(inverse(parent.rotation), delta), parent.scale),
        rotation: multiply(inverse(parent.rotation), world.rotation),
        scale: divide_vector(world.scale, parent.scale),
    }
}

fn add(left: Vector3, right: Vector3) -> Vector3 {
    Vector3::new(left.x + right.x, left.y + right.y, left.z + right.z)
}

fn subtract(left: Vector3, right: Vector3) -> Vector3 {
    Vector3::new(left.x - right.x, left.y - right.y, left.z - right.z)
}

fn multiply_vector(left: Vector3, right: Vector3) -> Vector3 {
    Vector3::new(left.x * right.x, left.y * right.y, left.z * right.z)
}

fn divide_vector(left: Vector3, right: Vector3) -> Vector3 {
    assert!(right.x != 0.0, "cannot invert zero parent scale on x");
    assert!(right.y != 0.0, "cannot invert zero parent scale on y");
    assert!(right.z != 0.0, "cannot invert zero parent scale on z");
    Vector3::new(left.x / right.x, left.y / right.y, left.z / right.z)
}
