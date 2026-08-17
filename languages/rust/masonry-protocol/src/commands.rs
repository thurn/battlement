//! Core command envelopes and payloads.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    CameraClearMode, Color, CommandId, ConflictPolicy, HorizontalAlignment, ImageFit, KeyCode,
    LightType, ObjectId, PointerEvent, PreparedAsset, Quaternion, RgbColor, RuntimeObject, SceneId,
    ShadowMode, Tween, Vector3, VerticalAlignment, default_true, is_false, is_one_f64, is_true,
    is_zero_u32, is_zero_u64,
};

/// A fully typed Masonry core command.
///
/// `command_id` also identifies any asynchronous operation started by the
/// command. Commands are blocking by default; a nonblocking command lets its
/// batch advance while the operation continues.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    /// Session-unique identity of the command and any operation it starts.
    pub command_id: CommandId,
    /// Whether later groups wait for this command to finish.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub blocking: bool,
    /// Exact core command type, conflict behavior, and payload.
    #[serde(flatten)]
    pub body: CommandBody,
}

impl Command {
    /// Creates a blocking command.
    #[must_use]
    pub fn new(command_id: CommandId, body: CommandBody) -> Self {
        Self {
            command_id,
            blocking: true,
            body,
        }
    }

    /// Marks this command as nonblocking and returns it.
    #[must_use]
    pub fn nonblocking(mut self) -> Self {
        self.blocking = false;
        self
    }
}

/// A core-command body that does not participate in property conflict handling.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
pub struct CommandPayload<P> {
    /// Command-specific payload.
    pub payload: P,
}

impl<P> From<P> for CommandPayload<P> {
    fn from(payload: P) -> Self {
        Self { payload }
    }
}

/// A property-writing core-command body.
///
/// Omitted conflict behavior means [`ConflictPolicy::Cancel`].
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct PropertyCommand<P> {
    /// How to handle an operation already controlling the same canonical property.
    #[serde(default, skip_serializing_if = "is_cancel")]
    pub on_conflict: ConflictPolicy,
    /// Command-specific payload.
    pub payload: P,
}

impl<P> PropertyCommand<P> {
    /// Creates a property write that cancels conflicting work.
    #[must_use]
    pub fn canceling(payload: P) -> Self {
        Self {
            on_conflict: ConflictPolicy::Cancel,
            payload,
        }
    }

    /// Creates a property write that waits for conflicting work.
    #[must_use]
    pub fn waiting(payload: P) -> Self {
        Self {
            on_conflict: ConflictPolicy::Wait,
            payload,
        }
    }
}

fn is_cancel(value: &ConflictPolicy) -> bool {
    *value == ConflictPolicy::Cancel
}

/// The exact v1 union of built-in Masonry command bodies.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(tag = "type")]
pub enum CommandBody {
    /// Atomically replace the complete prepared asset set.
    #[serde(rename = "masonry.assets.replaceSet")]
    AssetsReplaceSet(CommandPayload<ReplaceAssetSetPayload>),
    /// Additively load one prepared content scene.
    #[serde(rename = "masonry.scene.load")]
    SceneLoad(CommandPayload<SceneLoadPayload>),
    /// Unload a non-primary content scene.
    #[serde(rename = "masonry.scene.unload")]
    SceneUnload(CommandPayload<SceneIdPayload>),
    /// Make a loaded scene primary and active.
    #[serde(rename = "masonry.scene.setPrimary")]
    SceneSetPrimary(CommandPayload<SceneIdPayload>),
    /// Create one complete runtime object root.
    #[serde(rename = "masonry.object.create")]
    ObjectCreate(Box<CommandPayload<ObjectCreatePayload>>),
    /// Destroy a runtime root and its runtime-object descendants.
    #[serde(rename = "masonry.object.destroy")]
    ObjectDestroy(CommandPayload<ObjectIdPayload>),
    /// Set whether a runtime root is active.
    #[serde(rename = "masonry.object.setActive")]
    ObjectSetActive(CommandPayload<ObjectSetActivePayload>),
    /// Reparent a runtime root within its current placement.
    #[serde(rename = "masonry.object.reparent")]
    ObjectReparent(CommandPayload<ObjectReparentPayload>),
    /// Set local position immediately.
    #[serde(rename = "masonry.transform.setLocalPosition")]
    TransformSetLocalPosition(PropertyCommand<PositionPayload>),
    /// Set world position immediately.
    #[serde(rename = "masonry.transform.setWorldPosition")]
    TransformSetWorldPosition(PropertyCommand<PositionPayload>),
    /// Tween local position.
    #[serde(rename = "masonry.transform.tweenLocalPosition")]
    TransformTweenLocalPosition(PropertyCommand<TweenPositionPayload>),
    /// Tween world position.
    #[serde(rename = "masonry.transform.tweenWorldPosition")]
    TransformTweenWorldPosition(PropertyCommand<TweenPositionPayload>),
    /// Set local rotation immediately.
    #[serde(rename = "masonry.transform.setLocalRotation")]
    TransformSetLocalRotation(PropertyCommand<RotationPayload>),
    /// Set world rotation immediately.
    #[serde(rename = "masonry.transform.setWorldRotation")]
    TransformSetWorldRotation(PropertyCommand<RotationPayload>),
    /// Tween local rotation along the normalized shortest arc.
    #[serde(rename = "masonry.transform.tweenLocalRotation")]
    TransformTweenLocalRotation(PropertyCommand<TweenRotationPayload>),
    /// Tween world rotation along the normalized shortest arc.
    #[serde(rename = "masonry.transform.tweenWorldRotation")]
    TransformTweenWorldRotation(PropertyCommand<TweenRotationPayload>),
    /// Set local scale immediately.
    #[serde(rename = "masonry.transform.setLocalScale")]
    TransformSetLocalScale(PropertyCommand<ScalePayload>),
    /// Tween local scale.
    #[serde(rename = "masonry.transform.tweenLocalScale")]
    TransformTweenLocalScale(PropertyCommand<TweenScalePayload>),
    /// Assign a prepared material to one or all root-renderer slots.
    #[serde(rename = "masonry.renderer.setMaterial")]
    RendererSetMaterial(PropertyCommand<SetMaterialPayload>),
    /// Enable or disable a camera component.
    #[serde(rename = "masonry.camera.setEnabled")]
    CameraSetEnabled(CommandPayload<ObjectEnabledPayload>),
    /// Switch a camera to perspective projection.
    #[serde(rename = "masonry.camera.setPerspective")]
    CameraSetPerspective(PropertyCommand<PerspectivePayload>),
    /// Tween a perspective camera's vertical field of view.
    #[serde(rename = "masonry.camera.tweenFieldOfView")]
    CameraTweenFieldOfView(PropertyCommand<TweenFieldOfViewPayload>),
    /// Switch a camera to orthographic projection.
    #[serde(rename = "masonry.camera.setOrthographic")]
    CameraSetOrthographic(PropertyCommand<OrthographicPayload>),
    /// Tween an orthographic camera's size.
    #[serde(rename = "masonry.camera.tweenOrthographicSize")]
    CameraTweenOrthographicSize(PropertyCommand<TweenOrthographicSizePayload>),
    /// Set a camera's near and far clipping distances.
    #[serde(rename = "masonry.camera.setClipping")]
    CameraSetClipping(CommandPayload<CameraClippingPayload>),
    /// Set a camera's clear mode and optional solid clear color.
    #[serde(rename = "masonry.camera.setClear")]
    CameraSetClear(CommandPayload<CameraClearPayload>),
    /// Enable or disable a light component.
    #[serde(rename = "masonry.light.setEnabled")]
    LightSetEnabled(CommandPayload<ObjectEnabledPayload>),
    /// Change a standard light's type.
    #[serde(rename = "masonry.light.setType")]
    LightSetType(CommandPayload<LightTypePayload>),
    /// Set a light's color immediately.
    #[serde(rename = "masonry.light.setColor")]
    LightSetColor(PropertyCommand<ColorPayload>),
    /// Tween a light's color.
    #[serde(rename = "masonry.light.tweenColor")]
    LightTweenColor(PropertyCommand<TweenColorPayload>),
    /// Set a light's intensity immediately.
    #[serde(rename = "masonry.light.setIntensity")]
    LightSetIntensity(PropertyCommand<IntensityPayload>),
    /// Tween a light's intensity.
    #[serde(rename = "masonry.light.tweenIntensity")]
    LightTweenIntensity(PropertyCommand<TweenIntensityPayload>),
    /// Set the range of a point or spot light.
    #[serde(rename = "masonry.light.setRange")]
    LightSetRange(CommandPayload<LightRangePayload>),
    /// Set a spot light's inner and outer angles.
    #[serde(rename = "masonry.light.setSpotAngle")]
    LightSetSpotAngle(CommandPayload<SpotAnglePayload>),
    /// Set a light's shadow mode.
    #[serde(rename = "masonry.light.setShadows")]
    LightSetShadows(CommandPayload<LightShadowsPayload>),
    /// Replace an image quad's prepared texture.
    #[serde(rename = "masonry.image.setTexture")]
    ImageSetTexture(CommandPayload<SetAddressPayload>),
    /// Resize an image quad and its generated collider.
    #[serde(rename = "masonry.image.setSize")]
    ImageSetSize(CommandPayload<ImageSizePayload>),
    /// Change an image quad's fitting mode.
    #[serde(rename = "masonry.image.setFit")]
    ImageSetFit(CommandPayload<ImageFitPayload>),
    /// Set image tint immediately.
    #[serde(rename = "masonry.image.setTint")]
    ImageSetTint(PropertyCommand<TintPayload>),
    /// Tween image tint.
    #[serde(rename = "masonry.image.tweenTint")]
    ImageTweenTint(PropertyCommand<TweenTintPayload>),
    /// Set image opacity immediately.
    #[serde(rename = "masonry.image.setOpacity")]
    ImageSetOpacity(PropertyCommand<OpacityPayload>),
    /// Tween image opacity.
    #[serde(rename = "masonry.image.tweenOpacity")]
    ImageTweenOpacity(PropertyCommand<TweenOpacityPayload>),
    /// Enable or disable image billboard behavior.
    #[serde(rename = "masonry.image.setFaceCamera")]
    ImageSetFaceCamera(CommandPayload<ObjectEnabledPayload>),
    /// Replace displayed world-text content.
    #[serde(rename = "masonry.text.setContent")]
    TextSetContent(CommandPayload<TextContentPayload>),
    /// Replace a world-text object's prepared font.
    #[serde(rename = "masonry.text.setFont")]
    TextSetFont(CommandPayload<SetAddressPayload>),
    /// Set world-text size immediately.
    #[serde(rename = "masonry.text.setSize")]
    TextSetSize(PropertyCommand<TextSizePayload>),
    /// Tween world-text size.
    #[serde(rename = "masonry.text.tweenSize")]
    TextTweenSize(PropertyCommand<TweenTextSizePayload>),
    /// Set world-text color immediately.
    #[serde(rename = "masonry.text.setColor")]
    TextSetColor(PropertyCommand<ColorPayload>),
    /// Tween world-text color.
    #[serde(rename = "masonry.text.tweenColor")]
    TextTweenColor(PropertyCommand<TweenColorPayload>),
    /// Set horizontal and vertical text alignment.
    #[serde(rename = "masonry.text.setAlignment")]
    TextSetAlignment(CommandPayload<TextAlignmentPayload>),
    /// Enable or disable text wrapping and set its width.
    #[serde(rename = "masonry.text.setWrapping")]
    TextSetWrapping(CommandPayload<TextWrappingPayload>),
    /// Enable or disable TextMesh Pro rich-text parsing.
    #[serde(rename = "masonry.text.setRichText")]
    TextSetRichText(CommandPayload<ObjectEnabledPayload>),
    /// Enable or disable text billboard behavior.
    #[serde(rename = "masonry.text.setFaceCamera")]
    TextSetFaceCamera(CommandPayload<ObjectEnabledPayload>),
    /// Play an Animator state directly.
    #[serde(rename = "masonry.animator.play")]
    AnimatorPlay(CommandPayload<AnimatorPlayPayload>),
    /// Cross-fade to an Animator state.
    #[serde(rename = "masonry.animator.crossFade")]
    AnimatorCrossFade(CommandPayload<AnimatorCrossFadePayload>),
    /// Set a persistent boolean Animator parameter.
    #[serde(rename = "masonry.animator.setBool")]
    AnimatorSetBool(CommandPayload<AnimatorBoolPayload>),
    /// Set a persistent integer Animator parameter.
    #[serde(rename = "masonry.animator.setInt")]
    AnimatorSetInt(CommandPayload<AnimatorIntPayload>),
    /// Set a persistent floating-point Animator parameter.
    #[serde(rename = "masonry.animator.setFloat")]
    AnimatorSetFloat(CommandPayload<AnimatorFloatPayload>),
    /// Fire an Animator trigger.
    #[serde(rename = "masonry.animator.setTrigger")]
    AnimatorSetTrigger(CommandPayload<AnimatorParameterPayload>),
    /// Set nonnegative Animator playback speed.
    #[serde(rename = "masonry.animator.setSpeed")]
    AnimatorSetSpeed(CommandPayload<AnimatorSpeedPayload>),
    /// Recursively play root and descendant particle systems.
    #[serde(rename = "masonry.particle.play")]
    ParticlePlay(CommandPayload<ParticlePlayPayload>),
    /// Recursively stop root and descendant particle systems.
    #[serde(rename = "masonry.particle.stop")]
    ParticleStop(CommandPayload<ParticleStopPayload>),
    /// Spawn a prepared temporary particle-effect prefab.
    #[serde(rename = "masonry.particle.spawn")]
    ParticleSpawn(CommandPayload<ParticleSpawnPayload>),
    /// Play a prepared audio clip.
    #[serde(rename = "masonry.audio.play")]
    AudioPlay(CommandPayload<AudioPlayPayload>),
    /// Stop audio started by a previous audio-play command.
    #[serde(rename = "masonry.audio.stop")]
    AudioStop(CommandPayload<AudioStopPayload>),
    /// Set a playing audio operation's volume immediately.
    #[serde(rename = "masonry.audio.setVolume")]
    AudioSetVolume(PropertyCommand<AudioVolumePayload>),
    /// Tween a playing audio operation's volume.
    #[serde(rename = "masonry.audio.tweenVolume")]
    AudioTweenVolume(PropertyCommand<TweenAudioVolumePayload>),
    /// Wait for a positive duration. This command must be blocking.
    #[serde(rename = "masonry.time.wait")]
    TimeWait(CommandPayload<WaitPayload>),
    /// Cancel a running operation, or no-op for an already executed command.
    #[serde(rename = "masonry.operation.cancel")]
    OperationCancel(CommandPayload<CancelOperationPayload>),
    /// Gate all pointer and key input.
    #[serde(rename = "masonry.input.setEnabled")]
    InputSetEnabled(CommandPayload<SetInputEnabledPayload>),
    /// Select the enabled camera used for input raycasting.
    #[serde(rename = "masonry.input.setCamera")]
    InputSetCamera(CommandPayload<ObjectIdPayload>),
    /// Replace the unique pointer-event set for an object.
    #[serde(rename = "masonry.input.setPointerEvents")]
    InputSetPointerEvents(CommandPayload<PointerEventsPayload>),
    /// Replace the unique set of enabled global physical keys.
    #[serde(rename = "masonry.input.setGlobalKeys")]
    InputSetGlobalKeys(CommandPayload<GlobalKeysPayload>),
}

impl CommandBody {
    /// Creates a `masonry.object.create` body without exposing its internal boxing.
    #[must_use]
    pub fn object_create(object: RuntimeObject) -> Self {
        Self::ObjectCreate(Box::new(CommandPayload::from(ObjectCreatePayload {
            object,
        })))
    }
}

/// A custom game command using Masonry's shared command envelope.
///
/// The namespaced type and payload contract belong to the game schema rather
/// than the Masonry core schema.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct CustomCommand<P = Value> {
    /// Session-unique command and operation identity.
    pub command_id: CommandId,
    /// Game-owned namespaced command discriminator.
    #[serde(rename = "type")]
    #[schemars(length(max = 65_536))]
    pub command_type: String,
    /// Whether later groups wait for the custom handler's operation.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub blocking: bool,
    /// Game-specific payload, raw JSON by default or a game-owned Rust type.
    pub payload: P,
}

impl<P> CustomCommand<P> {
    /// Creates a blocking custom command.
    #[must_use]
    pub fn new(command_id: CommandId, command_type: impl Into<String>, payload: P) -> Self {
        Self {
            command_id,
            command_type: command_type.into(),
            blocking: true,
            payload,
        }
    }

    /// Marks this custom command as nonblocking and returns it.
    #[must_use]
    pub fn nonblocking(mut self) -> Self {
        self.blocking = false;
        self
    }
}

/// A command list entry that may contain either core or game-specific work.
///
/// Use this as the command parameter of [`crate::Delivery`] when a rules engine
/// needs to mix core commands with registered custom commands. The custom
/// payload defaults to raw JSON but can be a game-owned Rust type.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(untagged)]
pub enum AnyCommand<P = Value> {
    /// A command implemented by Masonry itself.
    Core(Command),
    /// A command handled by registered game code.
    Custom(CustomCommand<P>),
}

impl<P> From<Command> for AnyCommand<P> {
    fn from(command: Command) -> Self {
        Self::Core(command)
    }
}

impl<P> From<CustomCommand<P>> for AnyCommand<P> {
    fn from(command: CustomCommand<P>) -> Self {
        Self::Custom(command)
    }
}

/// Atomically replaces the complete prepared asset set.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
pub struct ReplaceAssetSetPayload {
    /// Complete replacement set; addresses must be unique.
    #[schemars(length(max = 16_384))]
    pub assets: Vec<PreparedAsset>,
}

/// Loads one prepared scene additively.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct SceneLoadPayload {
    /// New session-unique scene instance identity.
    pub scene_id: SceneId,
    /// Prepared Addressables scene address.
    #[schemars(length(max = 65_536))]
    pub address: String,
    /// Whether to make the loaded scene primary after it is ready.
    #[serde(default, skip_serializing_if = "is_false")]
    pub make_primary: bool,
}

/// A payload that names one loaded content scene.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct SceneIdPayload {
    /// Target scene identity.
    pub scene_id: SceneId,
}

/// Creates one complete runtime object record.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
pub struct ObjectCreatePayload {
    /// Complete object to create.
    pub object: RuntimeObject,
}

/// A payload that names one runtime object root.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ObjectIdPayload {
    /// Target runtime object root.
    pub object_id: ObjectId,
}

/// Sets the active state of a runtime object root.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ObjectSetActivePayload {
    /// Target runtime object root.
    pub object_id: ObjectId,
    /// New active state.
    pub active: bool,
}

/// Reparents a runtime object root within its current placement.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ObjectReparentPayload {
    /// Runtime object root to reparent.
    pub object_id: ObjectId,
    /// New runtime-object parent, or `null` for the placement container.
    pub parent_id: Option<ObjectId>,
    /// Whether Unity preserves the object's current world transform.
    pub world_position_stays: bool,
}

/// Sets a runtime object's position immediately.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct PositionPayload {
    /// Target runtime object root.
    pub object_id: ObjectId,
    /// Requested local or world position, according to the command type.
    pub position: Vector3,
}

/// Tweens a runtime object's position.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TweenPositionPayload {
    /// Target runtime object root.
    pub object_id: ObjectId,
    /// Requested final local or world position.
    pub position: Vector3,
    /// Tween timing and repetition.
    #[serde(flatten)]
    pub tween: Tween,
}

/// Sets a runtime object's rotation immediately.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct RotationPayload {
    /// Target runtime object root.
    pub object_id: ObjectId,
    /// Requested local or world rotation, according to the command type.
    pub rotation: Quaternion,
}

/// Tweens a runtime object's rotation along the normalized shortest arc.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TweenRotationPayload {
    /// Target runtime object root.
    pub object_id: ObjectId,
    /// Requested final local or world rotation.
    pub rotation: Quaternion,
    /// Tween timing and repetition.
    #[serde(flatten)]
    pub tween: Tween,
}

/// Sets a runtime object's local scale immediately.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ScalePayload {
    /// Target runtime object root.
    pub object_id: ObjectId,
    /// Requested local scale.
    pub scale: Vector3,
}

/// Tweens a runtime object's local scale.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TweenScalePayload {
    /// Target runtime object root.
    pub object_id: ObjectId,
    /// Requested final local scale.
    pub scale: Vector3,
    /// Tween timing and repetition.
    #[serde(flatten)]
    pub tween: Tween,
}

/// Assigns one prepared material to a supported root renderer.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct SetMaterialPayload {
    /// Target primitive or prefab root.
    pub object_id: ObjectId,
    /// Prepared material address.
    #[schemars(length(max = 65_536))]
    pub address: String,
    /// Zero-based renderer slot, or every root-renderer slot when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<u32>,
}

/// Enables or disables a supported component or billboard behavior.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ObjectEnabledPayload {
    /// Target runtime object root.
    pub object_id: ObjectId,
    /// New enabled state.
    pub enabled: bool,
}

/// Switches a camera to perspective projection.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct PerspectivePayload {
    /// Target camera object.
    pub object_id: ObjectId,
    /// Vertical field of view in degrees, strictly between 1 and 179.
    #[schemars(range(min = 1.0, max = 179.0))]
    #[schemars(
        extend("exclusiveMinimum" = 1.0),
        extend("exclusiveMaximum" = 179.0)
    )]
    pub field_of_view: f64,
}

/// Tweens a perspective camera's vertical field of view.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TweenFieldOfViewPayload {
    /// Target perspective camera object.
    pub object_id: ObjectId,
    /// Final vertical field of view in degrees, strictly between 1 and 179.
    #[schemars(range(min = 1.0, max = 179.0))]
    #[schemars(
        extend("exclusiveMinimum" = 1.0),
        extend("exclusiveMaximum" = 179.0)
    )]
    pub field_of_view: f64,
    /// Tween timing and repetition.
    #[serde(flatten)]
    pub tween: Tween,
}

/// Switches a camera to orthographic projection.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct OrthographicPayload {
    /// Target camera object.
    pub object_id: ObjectId,
    /// Positive orthographic half-height.
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub size: f64,
}

/// Tweens an orthographic camera's size.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TweenOrthographicSizePayload {
    /// Target orthographic camera object.
    pub object_id: ObjectId,
    /// Positive final orthographic half-height.
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub size: f64,
    /// Tween timing and repetition.
    #[serde(flatten)]
    pub tween: Tween,
}

/// Sets a camera's clipping distances.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct CameraClippingPayload {
    /// Target camera object.
    pub object_id: ObjectId,
    /// Positive near clipping distance.
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub near: f64,
    /// Far clipping distance, which must be greater than `near`.
    #[schemars(range(min = 0.0))]
    pub far: f64,
}

/// Sets a camera's clear behavior.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct CameraClearPayload {
    /// Target camera object.
    pub object_id: ObjectId,
    /// Requested clear mode.
    pub clear_mode: CameraClearMode,
    /// Required for `solidColor`; otherwise omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clear_color: Option<Color>,
}

/// Changes a standard light's type.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct LightTypePayload {
    /// Target light object.
    pub object_id: ObjectId,
    /// Requested standard light type.
    pub light_type: LightType,
}

/// Sets a light or text object's linear RGBA color.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ColorPayload {
    /// Target light or world-text object.
    pub object_id: ObjectId,
    /// Requested linear color.
    pub color: Color,
}

/// Tweens a light or text object's linear RGBA color.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TweenColorPayload {
    /// Target light or world-text object.
    pub object_id: ObjectId,
    /// Requested final linear color.
    pub color: Color,
    /// Tween timing and repetition.
    #[serde(flatten)]
    pub tween: Tween,
}

/// Sets a light's nonnegative intensity.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct IntensityPayload {
    /// Target light object.
    pub object_id: ObjectId,
    /// Requested nonnegative intensity.
    #[schemars(range(min = 0.0))]
    pub intensity: f64,
}

/// Tweens a light's nonnegative intensity.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TweenIntensityPayload {
    /// Target light object.
    pub object_id: ObjectId,
    /// Requested final nonnegative intensity.
    #[schemars(range(min = 0.0))]
    pub intensity: f64,
    /// Tween timing and repetition.
    #[serde(flatten)]
    pub tween: Tween,
}

/// Sets the positive range of a point or spot light.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct LightRangePayload {
    /// Target point or spot light object.
    pub object_id: ObjectId,
    /// Positive range in world units.
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub range: f64,
}

/// Sets a spot light's cone angles.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct SpotAnglePayload {
    /// Target spot light object.
    pub object_id: ObjectId,
    /// Outer angle in degrees, strictly between zero and 179.
    #[schemars(range(min = 0.0, max = 179.0))]
    #[schemars(
        extend("exclusiveMinimum" = 0.0),
        extend("exclusiveMaximum" = 179.0)
    )]
    pub outer_spot_angle: f64,
    /// Inner angle in `[0, outer_spot_angle]`.
    #[schemars(range(min = 0.0, max = 179.0))]
    pub inner_spot_angle: f64,
}

/// Sets a standard light's shadow mode.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct LightShadowsPayload {
    /// Target light object.
    pub object_id: ObjectId,
    /// Requested shadow mode.
    pub shadows: ShadowMode,
}

/// Replaces a prepared asset address on an existing object.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct SetAddressPayload {
    /// Target runtime object root.
    pub object_id: ObjectId,
    /// Prepared texture or TMP font address, according to the command type.
    #[schemars(length(max = 65_536))]
    pub address: String,
}

/// Resizes a Masonry image quad.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ImageSizePayload {
    /// Target image object.
    pub object_id: ObjectId,
    /// Positive world-space width.
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub width: f64,
    /// Positive world-space height.
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub height: f64,
}

/// Changes an image quad's texture fitting behavior.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ImageFitPayload {
    /// Target image object.
    pub object_id: ObjectId,
    /// Requested fitting mode.
    pub fit: ImageFit,
}

/// Sets an image quad's linear RGB tint.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TintPayload {
    /// Target image object.
    pub object_id: ObjectId,
    /// Requested linear RGB tint.
    pub tint: RgbColor,
}

/// Tweens an image quad's linear RGB tint.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TweenTintPayload {
    /// Target image object.
    pub object_id: ObjectId,
    /// Requested final linear RGB tint.
    pub tint: RgbColor,
    /// Tween timing and repetition.
    #[serde(flatten)]
    pub tween: Tween,
}

/// Sets an image quad's opacity.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct OpacityPayload {
    /// Target image object.
    pub object_id: ObjectId,
    /// Requested opacity in the inclusive range `[0, 1]`.
    #[schemars(range(min = 0.0, max = 1.0))]
    pub opacity: f64,
}

/// Tweens an image quad's opacity.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TweenOpacityPayload {
    /// Target image object.
    pub object_id: ObjectId,
    /// Requested final opacity in the inclusive range `[0, 1]`.
    #[schemars(range(min = 0.0, max = 1.0))]
    pub opacity: f64,
    /// Tween timing and repetition.
    #[serde(flatten)]
    pub tween: Tween,
}

/// Replaces displayed world-text content.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TextContentPayload {
    /// Target world-text object.
    pub object_id: ObjectId,
    /// New text content.
    #[schemars(length(max = 65_536))]
    pub text: String,
}

/// Sets a world-text object's positive size.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TextSizePayload {
    /// Target world-text object.
    pub object_id: ObjectId,
    /// Positive world-space text size.
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub size: f64,
}

/// Tweens a world-text object's positive size.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TweenTextSizePayload {
    /// Target world-text object.
    pub object_id: ObjectId,
    /// Positive final world-space text size.
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub size: f64,
    /// Tween timing and repetition.
    #[serde(flatten)]
    pub tween: Tween,
}

/// Sets horizontal and vertical world-text alignment.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TextAlignmentPayload {
    /// Target world-text object.
    pub object_id: ObjectId,
    /// Horizontal alignment.
    pub horizontal: HorizontalAlignment,
    /// Vertical alignment.
    pub vertical: VerticalAlignment,
}

/// Enables or disables world-text wrapping.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TextWrappingPayload {
    /// Target world-text object.
    pub object_id: ObjectId,
    /// Whether wrapping is enabled.
    pub enabled: bool,
    /// Positive width required when wrapping is enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub wrap_width: Option<f64>,
}

/// Plays an Animator state with explicit scheduling time.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct AnimatorPlayPayload {
    /// Target prefab root with a supported Animator.
    pub object_id: ObjectId,
    /// Animator state name.
    #[schemars(length(max = 65_536))]
    pub state: String,
    /// Nonnegative Animator layer index.
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub layer: u32,
    /// Normalized starting time in the inclusive range `[0, 1]`.
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    #[schemars(range(min = 0.0, max = 1.0))]
    pub normalized_start_time: f64,
    /// Explicit operation duration for group scheduling; zero does not wait.
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    #[schemars(range(max = 86_400_000))]
    pub wait_ms: u64,
}

/// Cross-fades to an Animator state with explicit scheduling time.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct AnimatorCrossFadePayload {
    /// Target prefab root with a supported Animator.
    pub object_id: ObjectId,
    /// Animator state name.
    #[schemars(length(max = 65_536))]
    pub state: String,
    /// Nonnegative Animator layer index.
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub layer: u32,
    /// Normalized starting time in the inclusive range `[0, 1]`.
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    #[schemars(range(min = 0.0, max = 1.0))]
    pub normalized_start_time: f64,
    /// Explicit operation duration for group scheduling; zero does not wait.
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    #[schemars(range(max = 86_400_000))]
    pub wait_ms: u64,
    /// Positive cross-fade duration in milliseconds.
    #[schemars(range(min = 1, max = 86_400_000))]
    pub cross_fade_ms: u64,
}

/// Sets a persistent boolean Animator parameter.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct AnimatorBoolPayload {
    /// Target prefab root with a supported Animator.
    pub object_id: ObjectId,
    /// Parameter name.
    #[schemars(length(max = 65_536))]
    pub parameter: String,
    /// New boolean value.
    pub value: bool,
}

/// Sets a persistent signed 32-bit Animator parameter.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct AnimatorIntPayload {
    /// Target prefab root with a supported Animator.
    pub object_id: ObjectId,
    /// Parameter name.
    #[schemars(length(max = 65_536))]
    pub parameter: String,
    /// New signed 32-bit value.
    pub value: i32,
}

/// Sets a persistent finite floating-point Animator parameter.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct AnimatorFloatPayload {
    /// Target prefab root with a supported Animator.
    pub object_id: ObjectId,
    /// Parameter name.
    #[schemars(length(max = 65_536))]
    pub parameter: String,
    /// New finite floating-point value.
    pub value: f64,
}

/// Names an Animator parameter without an associated value.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct AnimatorParameterPayload {
    /// Target prefab root with a supported Animator.
    pub object_id: ObjectId,
    /// Parameter name.
    #[schemars(length(max = 65_536))]
    pub parameter: String,
}

/// Sets nonnegative Animator playback speed.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct AnimatorSpeedPayload {
    /// Target prefab root with a supported Animator.
    pub object_id: ObjectId,
    /// Nonnegative playback speed.
    #[schemars(range(min = 0.0))]
    pub speed: f64,
}

/// Recursively plays particle systems rooted at an object.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ParticlePlayPayload {
    /// Target object whose root or descendants contain particle systems.
    pub object_id: ObjectId,
    /// Whether to restart systems that are already playing.
    #[serde(default, skip_serializing_if = "is_false")]
    pub restart: bool,
}

/// Recursively stops particle systems rooted at an object.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ParticleStopPayload {
    /// Target object whose root or descendants contain particle systems.
    pub object_id: ObjectId,
    /// Whether to clear live particles after stopping.
    #[serde(default, skip_serializing_if = "is_false")]
    pub clear: bool,
}

/// Spawns a prepared temporary particle-effect prefab.
///
/// Exactly one of `at_object_id` and `world_position` must be present.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ParticleSpawnPayload {
    /// Prepared particle-effect-prefab address.
    #[schemars(length(max = 65_536))]
    pub address: String,
    /// Object whose current world position supplies the spawn position.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at_object_id: Option<ObjectId>,
    /// Explicit world-space spawn position.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_position: Option<Vector3>,
    /// Positive effect lifetime in milliseconds.
    #[schemars(range(min = 1, max = 86_400_000))]
    pub lifetime_ms: u64,
}

/// Plays a prepared audio clip through a Masonry-owned 2D audio source.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct AudioPlayPayload {
    /// Prepared audio-clip address.
    #[schemars(length(max = 65_536))]
    pub address: String,
    /// Initial volume in the inclusive range `[0, 1]`.
    #[serde(default = "one", skip_serializing_if = "is_one_f64")]
    #[schemars(range(min = 0.0, max = 1.0))]
    pub volume: f64,
    /// Playback pitch in the range `(0, 3]`.
    #[serde(default = "one", skip_serializing_if = "is_one_f64")]
    #[schemars(range(min = 0.0, max = 3.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub pitch: f64,
    /// Whether playback loops until explicitly stopped.
    #[serde(default, skip_serializing_if = "is_false")]
    pub r#loop: bool,
    /// Fade-in duration in milliseconds.
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    #[schemars(range(max = 86_400_000))]
    pub fade_in_ms: u64,
}

/// Stops audio started by an earlier audio-play command.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct AudioStopPayload {
    /// Command and operation identity of the audio playback.
    pub audio_command_id: CommandId,
    /// Fade-out duration in milliseconds.
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    #[schemars(range(max = 86_400_000))]
    pub fade_out_ms: u64,
}

/// Sets a playing audio operation's volume.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct AudioVolumePayload {
    /// Command and operation identity of the audio playback.
    pub audio_command_id: CommandId,
    /// Requested volume in the inclusive range `[0, 1]`.
    #[schemars(range(min = 0.0, max = 1.0))]
    pub volume: f64,
}

/// Tweens a playing audio operation's volume.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TweenAudioVolumePayload {
    /// Command and operation identity of the audio playback.
    pub audio_command_id: CommandId,
    /// Requested final volume in the inclusive range `[0, 1]`.
    #[schemars(range(min = 0.0, max = 1.0))]
    pub volume: f64,
    /// Tween timing and repetition.
    #[serde(flatten)]
    pub tween: Tween,
}

/// Waits for a fixed positive duration.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct WaitPayload {
    /// Positive wait duration in milliseconds.
    #[schemars(range(min = 1, max = 86_400_000))]
    pub duration_ms: u64,
}

/// Cancels an operation by the command identity that started it.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct CancelOperationPayload {
    /// Command and operation identity to cancel.
    pub command_id: CommandId,
}

/// Gates every pointer and key action.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
pub struct SetInputEnabledPayload {
    /// Whether Masonry accepts input actions.
    pub enabled: bool,
}

/// Replaces the enabled pointer-event set for one runtime object.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct PointerEventsPayload {
    /// Target runtime object root.
    pub object_id: ObjectId,
    /// Unique enabled pointer-event kinds.
    #[schemars(extend("uniqueItems" = true))]
    pub events: Vec<PointerEvent>,
}

/// Replaces the global physical-key set enabled for the session.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
pub struct GlobalKeysPayload {
    /// Unique enabled W3C physical key codes.
    #[schemars(extend("uniqueItems" = true))]
    pub keys: Vec<KeyCode>,
}

fn one() -> f64 {
    1.0
}

fn is_zero_f64(value: &f64) -> bool {
    *value == 0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn command_serializes_with_a_flat_namespaced_discriminator() {
        let command_id = "7bbcb27e-f75b-4c63-bf86-ad1b0f6ee2cd".parse().unwrap();
        let object_id = "cc847d6e-1468-42c6-9bec-9af5b5aa5c03".parse().unwrap();
        let command = Command::new(
            command_id,
            CommandBody::TransformTweenWorldPosition(PropertyCommand::canceling(
                TweenPositionPayload {
                    object_id,
                    position: Vector3::new(4.0, 0.0, 2.0),
                    tween: Tween {
                        duration_ms: 300,
                        ..Tween::default()
                    },
                },
            )),
        );

        assert_eq!(
            serde_json::to_value(command).unwrap(),
            json!({
                "commandId": "7bbcb27e-f75b-4c63-bf86-ad1b0f6ee2cd",
                "type": "masonry.transform.tweenWorldPosition",
                "payload": {
                    "objectId": "cc847d6e-1468-42c6-9bec-9af5b5aa5c03",
                    "position": { "x": 4.0, "y": 0.0, "z": 2.0 },
                    "durationMs": 300
                }
            })
        );
    }

    #[test]
    fn wait_conflict_policy_is_explicit_on_the_wire() {
        let command_id = "565e76aa-b480-43c2-900b-1cb9d90e4602".parse().unwrap();
        let object_id = "cc847d6e-1468-42c6-9bec-9af5b5aa5c03".parse().unwrap();
        let command = Command::new(
            command_id,
            CommandBody::TransformSetLocalScale(PropertyCommand::waiting(ScalePayload {
                object_id,
                scale: Vector3::ONE,
            })),
        );
        let value = serde_json::to_value(command).unwrap();

        assert_eq!(value["onConflict"], "wait");
    }
}
