//! Fluent configuration methods for objects and component states with defaults.

use crate::{
    AnimatorState, CameraClearMode, CameraProjection, CameraState, Color, DragMode, FontAddress,
    GameObject, GameObjectKind, HorizontalAlignment, ImageFit, ImageState, LightState, LightType,
    LocalTransform, ObjectId, ParentScene, PointerEvent, Quaternion, RgbColor, ShadowMode,
    TextState, Vector3, VerticalAlignment,
};

impl GameObject {
    /// Sets the owning scene and returns the updated object.
    #[must_use]
    pub fn parent_scene(mut self, value: ParentScene) -> Self {
        self.parent_scene = value;
        self
    }

    /// Sets the parent object and returns the updated object.
    #[must_use]
    pub fn parent_id(mut self, value: ObjectId) -> Self {
        self.parent_id = Some(value);
        self
    }

    /// Sets the active state and returns the updated object.
    #[must_use]
    pub fn active(mut self, value: bool) -> Self {
        self.active = value;
        self
    }

    /// Replaces the local transform and returns the updated object.
    #[must_use]
    pub fn local_transform(mut self, value: LocalTransform) -> Self {
        self.local_transform = value;
        self
    }

    /// Replaces the object kind and returns the updated object.
    #[must_use]
    pub fn kind(mut self, value: GameObjectKind) -> Self {
        self.kind = value;
        self
    }

    /// Replaces the pointer events and returns the updated object.
    #[must_use]
    pub fn pointer_events(mut self, values: impl IntoIterator<Item = PointerEvent>) -> Self {
        self.pointer_events = values.into_iter().collect();
        self
    }

    /// Makes the object draggable with the requested pickup behavior.
    #[must_use]
    pub fn draggable(mut self, mode: DragMode) -> Self {
        self.drag_mode = Some(mode);
        self
    }

    /// Sets the local position and returns the updated object.
    #[must_use]
    pub fn position(mut self, value: Vector3) -> Self {
        self.local_transform.position = value;
        self
    }

    /// Sets the local rotation and returns the updated object.
    #[must_use]
    pub fn rotation(mut self, value: Quaternion) -> Self {
        self.local_transform.rotation = value;
        self
    }

    /// Sets the local scale and returns the updated object.
    #[must_use]
    pub fn scale(mut self, value: Vector3) -> Self {
        self.local_transform.scale = value;
        self
    }
}

impl ImageState {
    /// Sets the image fitting mode and returns the updated state.
    #[must_use]
    pub fn fit(mut self, value: ImageFit) -> Self {
        self.fit = value;
        self
    }

    /// Sets the tint and returns the updated state.
    #[must_use]
    pub fn tint(mut self, value: RgbColor) -> Self {
        self.tint = value;
        self
    }

    /// Sets the opacity and returns the updated state.
    #[must_use]
    pub fn opacity(mut self, value: f64) -> Self {
        self.opacity = value;
        self
    }

    /// Sets billboard behavior and returns the updated state.
    #[must_use]
    pub fn face_camera(mut self, value: bool) -> Self {
        self.face_camera = value;
        self
    }
}

impl TextState {
    /// Replaces the displayed text and returns the updated state.
    #[must_use]
    pub fn text(mut self, value: impl Into<String>) -> Self {
        self.text = value.into();
        self
    }

    /// Replaces the font and returns the updated state.
    #[must_use]
    pub fn font(mut self, value: impl Into<FontAddress>) -> Self {
        self.font = value.into();
        self
    }

    /// Sets the text size and returns the updated state.
    #[must_use]
    pub fn size(mut self, value: f64) -> Self {
        self.size = value;
        self
    }

    /// Sets the text color and returns the updated state.
    #[must_use]
    pub fn color(mut self, value: Color) -> Self {
        self.color = value;
        self
    }

    /// Sets horizontal alignment and returns the updated state.
    #[must_use]
    pub fn horizontal(mut self, value: HorizontalAlignment) -> Self {
        self.horizontal = value;
        self
    }

    /// Sets vertical alignment and returns the updated state.
    #[must_use]
    pub fn vertical(mut self, value: VerticalAlignment) -> Self {
        self.vertical = value;
        self
    }

    /// Sets text wrapping and returns the updated state.
    #[must_use]
    pub fn wrap_width(mut self, value: f64) -> Self {
        self.wrap_width = Some(value);
        self
    }

    /// Sets rich-text parsing and returns the updated state.
    #[must_use]
    pub fn rich_text(mut self, value: bool) -> Self {
        self.rich_text = value;
        self
    }

    /// Sets billboard behavior and returns the updated state.
    #[must_use]
    pub fn face_camera(mut self, value: bool) -> Self {
        self.face_camera = value;
        self
    }
}

impl CameraState {
    /// Sets whether the camera is enabled and returns the updated state.
    #[must_use]
    pub fn enabled(mut self, value: bool) -> Self {
        self.enabled = value;
        self
    }

    /// Sets the projection and returns the updated state.
    #[must_use]
    pub fn projection(mut self, value: CameraProjection) -> Self {
        self.projection = value;
        self
    }

    /// Sets the field of view and returns the updated state.
    #[must_use]
    pub fn field_of_view(mut self, value: f64) -> Self {
        self.field_of_view = value;
        self
    }

    /// Sets the orthographic size and returns the updated state.
    #[must_use]
    pub fn orthographic_size(mut self, value: f64) -> Self {
        self.orthographic_size = value;
        self
    }

    /// Sets the near clipping distance and returns the updated state.
    #[must_use]
    pub fn near(mut self, value: f64) -> Self {
        self.near = value;
        self
    }

    /// Sets the far clipping distance and returns the updated state.
    #[must_use]
    pub fn far(mut self, value: f64) -> Self {
        self.far = value;
        self
    }

    /// Sets the clear mode and returns the updated state.
    #[must_use]
    pub fn clear_mode(mut self, value: CameraClearMode) -> Self {
        self.clear_mode = value;
        self
    }

    /// Sets the clear color and returns the updated state.
    #[must_use]
    pub fn clear_color(mut self, value: Color) -> Self {
        self.clear_color = value;
        self
    }
}

impl LightState {
    /// Sets whether the light is enabled and returns the updated state.
    #[must_use]
    pub fn enabled(mut self, value: bool) -> Self {
        self.enabled = value;
        self
    }

    /// Sets the light type and returns the updated state.
    #[must_use]
    pub fn light_type(mut self, value: LightType) -> Self {
        self.light_type = value;
        self
    }

    /// Sets the light color and returns the updated state.
    #[must_use]
    pub fn color(mut self, value: Color) -> Self {
        self.color = value;
        self
    }

    /// Sets the intensity and returns the updated state.
    #[must_use]
    pub fn intensity(mut self, value: f64) -> Self {
        self.intensity = value;
        self
    }

    /// Sets the range and returns the updated state.
    #[must_use]
    pub fn range(mut self, value: f64) -> Self {
        self.range = value;
        self
    }

    /// Sets the outer spot angle and returns the updated state.
    #[must_use]
    pub fn outer_spot_angle(mut self, value: f64) -> Self {
        self.outer_spot_angle = value;
        self
    }

    /// Sets the inner spot angle and returns the updated state.
    #[must_use]
    pub fn inner_spot_angle(mut self, value: f64) -> Self {
        self.inner_spot_angle = value;
        self
    }

    /// Sets the shadow mode and returns the updated state.
    #[must_use]
    pub fn shadows(mut self, value: ShadowMode) -> Self {
        self.shadows = value;
        self
    }
}

impl AnimatorState {
    /// Replaces the Animator state name and returns the updated state.
    #[must_use]
    pub fn state(mut self, value: impl Into<String>) -> Self {
        self.state = value.into();
        self
    }

    /// Sets the Animator layer and returns the updated state.
    #[must_use]
    pub fn layer(mut self, value: u32) -> Self {
        self.layer = value;
        self
    }

    /// Sets the normalized start time and returns the updated state.
    #[must_use]
    pub fn normalized_start_time(mut self, value: f64) -> Self {
        self.normalized_start_time = value;
        self
    }

    /// Sets the playback speed and returns the updated state.
    #[must_use]
    pub fn speed(mut self, value: f64) -> Self {
        self.speed = value;
        self
    }

    /// Replaces the persistent boolean parameters.
    #[must_use]
    pub fn bool_parameters(mut self, parameters: impl IntoIterator<Item = (String, bool)>) -> Self {
        self.bool_parameters = parameters.into_iter().collect();
        self
    }

    /// Replaces the persistent integer parameters.
    #[must_use]
    pub fn int_parameters(mut self, parameters: impl IntoIterator<Item = (String, i32)>) -> Self {
        self.int_parameters = parameters.into_iter().collect();
        self
    }

    /// Replaces the persistent floating-point parameters.
    #[must_use]
    pub fn float_parameters(mut self, parameters: impl IntoIterator<Item = (String, f64)>) -> Self {
        self.float_parameters = parameters.into_iter().collect();
        self
    }
}
