use serde::{Deserialize, Serialize};

use crate::{
  GameObject, GeometryObservationUpdate, ObjectId, VisualElementCreate, VisualElementDestroy,
  VisualElementPerformAction, VisualElementUpdate,
};

use super::*;

/// The exact union of built-in Battlement command bodies.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum CommandBody {
  /// Atomically replace the complete prepared asset set.
  AssetsReplaceSet(ReplaceAssetSetPayload),
  /// Additively load one prepared content scene.
  SceneLoad(SceneLoadPayload),
  /// Unload a non-primary content scene.
  SceneUnload(SceneIdPayload),
  /// Make a loaded scene primary and call Unity's `SetActiveScene` for it.
  SceneSetPrimary(SceneIdPayload),
  /// Create one complete game object.
  ObjectCreate(Box<ObjectCreatePayload>),
  /// Destroy a game object and its game-object descendants.
  ObjectDestroy(ObjectIdPayload),
  /// Set a game object's Unity `activeSelf` value.
  ObjectSetActive(ObjectSetActivePayload),
  /// Reparent a game object within its current placement.
  ObjectReparent(ObjectReparentPayload),
  /// Set local position immediately.
  TransformSetLocalPosition(PropertyCommand<PositionPayload>),
  /// Set world position immediately.
  TransformSetWorldPosition(PropertyCommand<PositionPayload>),
  /// Tween local position.
  TransformTweenLocalPosition(PropertyCommand<TweenPositionPayload>),
  /// Tween world position.
  TransformTweenWorldPosition(PropertyCommand<TweenPositionPayload>),
  /// Set local rotation immediately.
  TransformSetLocalRotation(PropertyCommand<RotationPayload>),
  /// Set world rotation immediately.
  TransformSetWorldRotation(PropertyCommand<RotationPayload>),
  /// Tween local rotation along the normalized shortest arc.
  TransformTweenLocalRotation(PropertyCommand<TweenRotationPayload>),
  /// Tween world rotation along the normalized shortest arc.
  TransformTweenWorldRotation(PropertyCommand<TweenRotationPayload>),
  /// Set local scale immediately.
  TransformSetLocalScale(PropertyCommand<ScalePayload>),
  /// Tween local scale.
  TransformTweenLocalScale(PropertyCommand<TweenScalePayload>),
  /// Assign a prepared material to one or all renderer slots.
  RendererSetMaterial(PropertyCommand<SetMaterialPayload>),
  /// Enable or disable a camera component.
  CameraSetEnabled(ObjectEnabledPayload),
  /// Switch a camera to perspective projection.
  CameraSetPerspective(PropertyCommand<PerspectivePayload>),
  /// Tween a perspective camera's vertical field of view.
  CameraTweenFieldOfView(PropertyCommand<TweenFieldOfViewPayload>),
  /// Switch a camera to orthographic projection.
  CameraSetOrthographic(PropertyCommand<OrthographicPayload>),
  /// Tween an orthographic camera's size.
  CameraTweenOrthographicSize(PropertyCommand<TweenOrthographicSizePayload>),
  /// Set a camera's near and far clipping distances.
  CameraSetClipping(CameraClippingPayload),
  /// Set a camera's clear mode and optional solid clear color.
  CameraSetClear(CameraClearPayload),
  /// Enable or disable a light component.
  LightSetEnabled(ObjectEnabledPayload),
  /// Change a standard light's type.
  LightSetType(LightTypePayload),
  /// Set a light's color immediately.
  LightSetColor(PropertyCommand<ColorPayload>),
  /// Tween a light's color.
  LightTweenColor(PropertyCommand<TweenColorPayload>),
  /// Set a light's intensity immediately.
  LightSetIntensity(PropertyCommand<IntensityPayload>),
  /// Tween a light's intensity.
  LightTweenIntensity(PropertyCommand<TweenIntensityPayload>),
  /// Set the range of a point or spot light.
  LightSetRange(LightRangePayload),
  /// Set a spot light's inner and outer angles.
  LightSetSpotAngle(SpotAnglePayload),
  /// Set a light's shadow mode.
  LightSetShadows(LightShadowsPayload),
  /// Replace an image quad's prepared texture.
  ImageSetTexture(SetTexturePayload),
  /// Resize an image quad and its generated collider.
  ImageSetSize(ImageSizePayload),
  /// Change an image quad's fitting mode.
  ImageSetFit(ImageFitPayload),
  /// Set image tint immediately.
  ImageSetTint(PropertyCommand<TintPayload>),
  /// Tween image tint.
  ImageTweenTint(PropertyCommand<TweenTintPayload>),
  /// Set image opacity immediately.
  ImageSetOpacity(PropertyCommand<OpacityPayload>),
  /// Tween image opacity.
  ImageTweenOpacity(PropertyCommand<TweenOpacityPayload>),
  /// Enable or disable image billboard behavior.
  ImageSetFaceCamera(ObjectEnabledPayload),
  /// Replace displayed world-text content.
  TextSetContent(TextContentPayload),
  /// Replace a world-text object's prepared font.
  TextSetFont(SetFontPayload),
  /// Set world-text size immediately.
  TextSetSize(PropertyCommand<TextSizePayload>),
  /// Tween world-text size.
  TextTweenSize(PropertyCommand<TweenTextSizePayload>),
  /// Set world-text color immediately.
  TextSetColor(PropertyCommand<ColorPayload>),
  /// Tween world-text color.
  TextTweenColor(PropertyCommand<TweenColorPayload>),
  /// Set horizontal and vertical text alignment.
  TextSetAlignment(TextAlignmentPayload),
  /// Set text wrapping width, or disable wrapping with [`None`].
  TextSetWrapping(TextWrappingPayload),
  /// Enable or disable TextMesh Pro rich-text parsing.
  TextSetRichText(ObjectEnabledPayload),
  /// Enable or disable text billboard behavior.
  TextSetFaceCamera(ObjectEnabledPayload),
  /// Play an Animator state directly.
  AnimatorPlay(AnimatorPlayPayload),
  /// Cross-fade to an Animator state.
  AnimatorCrossFade(AnimatorCrossFadePayload),
  /// Set a persistent boolean Animator parameter.
  AnimatorSetBool(AnimatorBoolPayload),
  /// Set a persistent integer Animator parameter.
  AnimatorSetInt(AnimatorIntPayload),
  /// Set a persistent floating-point Animator parameter.
  AnimatorSetFloat(AnimatorFloatPayload),
  /// Fire an Animator trigger.
  AnimatorSetTrigger(AnimatorParameterPayload),
  /// Set nonnegative Animator playback speed.
  AnimatorSetSpeed(AnimatorSpeedPayload),
  /// Recursively play particle systems on the game object and its descendants.
  ParticlePlay(ParticlePlayPayload),
  /// Recursively stop particle systems on the game object and its descendants.
  ParticleStop(ParticleStopPayload),
  /// Spawn a prepared temporary particle-effect prefab.
  ParticleSpawn(ParticleSpawnPayload),
  /// Play a prepared audio clip.
  AudioPlay(AudioPlayPayload),
  /// Stop audio started by a previous audio-play command.
  AudioStop(AudioStopPayload),
  /// Set a playing audio operation's volume immediately.
  AudioSetVolume(PropertyCommand<AudioVolumePayload>),
  /// Tween a playing audio operation's volume.
  AudioTweenVolume(PropertyCommand<TweenAudioVolumePayload>),
  /// Wait for a positive duration. This command must be blocking.
  TimeWait(WaitPayload),
  /// Cancel a running operation, or no-op for an already executed command.
  OperationCancel(CancelOperationPayload),
  /// Gate all pointer and key input.
  InputSetEnabled(SetInputEnabledPayload),
  /// Select the enabled camera used for input raycasting.
  InputSetCamera(ObjectIdPayload),
  /// Replace the unique pointer-event set for an object.
  InputSetPointerEvents(PointerEventsPayload),
  /// Replace the unique set of enabled global physical keys.
  InputSetGlobalKeys(GlobalKeysPayload),
  /// Replace controller-button and discrete-navigation settings.
  InputSetController(ControllerInputSettings),
  /// Run controller vibration motors for a bounded duration.
  ControllerVibrate(ControllerVibrationPayload),
  /// Set whether one Battlement developer interface surface is visible.
  DebugUi(DebugUiPayload),
  /// Create and attach one UI element subtree.
  VisualElementCreate(Box<VisualElementCreate>),
  /// Apply one sparse property or hierarchy update to a live UI element.
  VisualElementUpdate(Box<VisualElementUpdate>),
  /// Destroy a UI element and its descendants.
  VisualElementDestroy(VisualElementDestroy),
  /// Perform one transient UI operation.
  VisualElementPerformAction(VisualElementPerformAction),
  /// Atomically update the native geometry observation registry.
  GeometryObservationUpdate(GeometryObservationUpdate),
}

impl CommandBody {
  /// Creates a `battlement.object.create` body without exposing its internal boxing.
  #[must_use]
  pub fn object_create(object: GameObject) -> Self {
    Self::ObjectCreate(Box::new(ObjectCreatePayload { object }))
  }

  /// Creates a `battlement.object.destroy` body.
  #[must_use]
  pub fn object_destroy(object_id: ObjectId) -> Self {
    Self::ObjectDestroy(ObjectIdPayload { object_id })
  }

  /// Creates a `battlement.text.setContent` body.
  #[must_use]
  pub fn set_text(object_id: ObjectId, text: impl Into<String>) -> Self {
    Self::TextSetContent(TextContentPayload {
      object_id,
      text: text.into(),
    })
  }

  /// Creates a `battlement.input.setEnabled` body.
  #[must_use]
  pub fn set_input_enabled(enabled: bool) -> Self {
    Self::InputSetEnabled(SetInputEnabledPayload { enabled })
  }
}
