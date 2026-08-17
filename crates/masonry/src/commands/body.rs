use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::GameObject;

use super::*;

/// The exact union of built-in Masonry command bodies.
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
    /// Make a loaded scene primary and call Unity's `SetActiveScene` for it.
    #[serde(rename = "masonry.scene.setPrimary")]
    SceneSetPrimary(CommandPayload<SceneIdPayload>),
    /// Create one complete game object.
    #[serde(rename = "masonry.object.create")]
    ObjectCreate(Box<CommandPayload<ObjectCreatePayload>>),
    /// Destroy a game object and its game-object descendants.
    #[serde(rename = "masonry.object.destroy")]
    ObjectDestroy(CommandPayload<ObjectIdPayload>),
    /// Set a game object's Unity `activeSelf` value.
    #[serde(rename = "masonry.object.setActive")]
    ObjectSetActive(CommandPayload<ObjectSetActivePayload>),
    /// Reparent a game object within its current placement.
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
    /// Assign a prepared material to one or all renderer slots.
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
    ImageSetTexture(CommandPayload<SetTexturePayload>),
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
    TextSetFont(CommandPayload<SetFontPayload>),
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
    /// Recursively play particle systems on the game object and its descendants.
    #[serde(rename = "masonry.particle.play")]
    ParticlePlay(CommandPayload<ParticlePlayPayload>),
    /// Recursively stop particle systems on the game object and its descendants.
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
    pub fn object_create(object: GameObject) -> Self {
        Self::ObjectCreate(Box::new(CommandPayload::from(ObjectCreatePayload {
            object,
        })))
    }
}
