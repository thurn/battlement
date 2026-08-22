//! Fluent configuration methods for reusable values with defaults.

use crate::{Easing, LocalTransform, Quaternion, Tween, TweenRepeat, Vector3};

impl LocalTransform {
    /// Sets the local position and returns the updated transform.
    #[must_use]
    pub fn position(mut self, value: Vector3) -> Self {
        self.position = value;
        self
    }

    /// Sets the local rotation and returns the updated transform.
    #[must_use]
    pub fn rotation(mut self, value: Quaternion) -> Self {
        self.rotation = value;
        self
    }

    /// Sets the local scale and returns the updated transform.
    #[must_use]
    pub fn scale(mut self, value: Vector3) -> Self {
        self.scale = value;
        self
    }
}

impl Tween {
    /// Sets the initial delay and returns the updated tween.
    #[must_use]
    pub fn delay_ms(mut self, value: u64) -> Self {
        self.delay_ms = value;
        self
    }

    /// Sets the traversal duration and returns the updated tween.
    #[must_use]
    pub fn duration_ms(mut self, value: u64) -> Self {
        self.duration_ms = value;
        self
    }

    /// Sets the easing curve and returns the updated tween.
    #[must_use]
    pub fn easing(mut self, value: Easing) -> Self {
        self.easing = value;
        self
    }

    /// Sets the repetition behavior and returns the updated tween.
    #[must_use]
    pub fn repeat(mut self, value: TweenRepeat) -> Self {
        self.repeat = value;
        self
    }
}
