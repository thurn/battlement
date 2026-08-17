//! Serde defaults and omission predicates used by protocol records.

use crate::{Color, LocalTransform, Vector3};

pub(crate) fn default_true() -> bool {
    true
}

pub(crate) fn default_one() -> f64 {
    1.0
}

pub(crate) fn default_vector_one() -> Vector3 {
    Vector3::ONE
}

pub(crate) fn default_field_of_view() -> f64 {
    60.0
}

pub(crate) fn default_orthographic_size() -> f64 {
    5.0
}

pub(crate) fn default_near_clip() -> f64 {
    0.3
}

pub(crate) fn default_far_clip() -> f64 {
    1000.0
}

pub(crate) fn default_black() -> Color {
    Color::BLACK
}

pub(crate) fn default_light_range() -> f64 {
    10.0
}

pub(crate) fn default_outer_spot_angle() -> f64 {
    30.0
}

pub(crate) fn is_default<T>(value: &T) -> bool
where
    T: Default + PartialEq,
{
    *value == T::default()
}

pub(crate) fn is_true(value: &bool) -> bool {
    *value
}

pub(crate) fn is_one(value: &f64) -> bool {
    *value == 1.0
}

pub(crate) fn is_vector_one(value: &Vector3) -> bool {
    *value == Vector3::ONE
}

pub(crate) fn is_default_transform(value: &LocalTransform) -> bool {
    *value == LocalTransform::default()
}

pub(crate) fn is_default_field_of_view(value: &f64) -> bool {
    *value == default_field_of_view()
}

pub(crate) fn is_default_orthographic_size(value: &f64) -> bool {
    *value == default_orthographic_size()
}

pub(crate) fn is_default_near_clip(value: &f64) -> bool {
    *value == default_near_clip()
}

pub(crate) fn is_default_far_clip(value: &f64) -> bool {
    *value == default_far_clip()
}

pub(crate) fn is_default_black(value: &Color) -> bool {
    *value == Color::BLACK
}

pub(crate) fn is_default_light_range(value: &f64) -> bool {
    *value == default_light_range()
}

pub(crate) fn is_default_outer_spot_angle(value: &f64) -> bool {
    *value == default_outer_spot_angle()
}
