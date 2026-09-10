#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;

namespace Battlement
{
    internal static class BattlementUnionCaseCatalog
    {
        public static readonly IReadOnlyDictionary<Type, IReadOnlyDictionary<string, Type>> Cases =
            CreateCases();

        private static IReadOnlyDictionary<Type, IReadOnlyDictionary<string, Type>> CreateCases()
        {
            var cases = new Dictionary<Type, IReadOnlyDictionary<string, Type>>
            {
                [typeof(PreparedAsset)] = Fixed(
                    ("Scene", typeof(PreparedAsset.Scene)),
                    ("Prefab", typeof(PreparedAsset.Prefab)),
                    ("ParticleEffect", typeof(PreparedAsset.ParticleEffect)),
                    ("Material", typeof(PreparedAsset.Material)),
                    ("Texture", typeof(PreparedAsset.Texture)),
                    ("Sprite", typeof(PreparedAsset.Sprite)),
                    ("VectorImage", typeof(PreparedAsset.VectorImage)),
                    ("RenderTexture", typeof(PreparedAsset.RenderTexture)),
                    ("AudioClip", typeof(PreparedAsset.AudioClip)),
                    ("TextMeshProFont", typeof(PreparedAsset.TextMeshProFont)),
                    ("UiFont", typeof(PreparedAsset.UiFont))
                ),
                [typeof(BackgroundSource)] = Fixed(
                    ("Texture", typeof(BackgroundSource.Texture)),
                    ("Sprite", typeof(BackgroundSource.Sprite)),
                    ("VectorImage", typeof(BackgroundSource.VectorImage)),
                    ("RenderTexture", typeof(BackgroundSource.RenderTexture))
                ),
                [typeof(UiBackgroundSize)] = Fixed(
                    ("Auto", typeof(UiBackgroundSize.Auto)),
                    ("Cover", typeof(UiBackgroundSize.Cover)),
                    ("Contain", typeof(UiBackgroundSize.Contain)),
                    ("Axes", typeof(UiBackgroundSize.Axes))
                ),
                [typeof(UiTextAutoSize)] = Fixed(
                    ("None", typeof(UiTextAutoSize.None)),
                    ("BestFit", typeof(UiTextAutoSize.BestFit))
                ),
                [typeof(UiCursor)] = Fixed(
                    ("Default", typeof(UiCursor.Default)),
                    ("Texture", typeof(UiCursor.Texture))
                ),
                [typeof(ImageSource)] = Fixed(
                    ("Texture", typeof(ImageSource.Texture)),
                    ("Sprite", typeof(ImageSource.Sprite)),
                    ("VectorImage", typeof(ImageSource.VectorImage)),
                    ("RenderTexture", typeof(ImageSource.RenderTexture))
                ),
                [typeof(IconSource)] = Fixed(
                    ("Texture", typeof(IconSource.Texture)),
                    ("Sprite", typeof(IconSource.Sprite)),
                    ("VectorImage", typeof(IconSource.VectorImage)),
                    ("RenderTexture", typeof(IconSource.RenderTexture))
                ),
                [typeof(UiLength)] = Fixed(
                    ("Px", typeof(UiLength.Px)),
                    ("Percent", typeof(UiLength.Percent)),
                    ("Calc", typeof(UiLength.Calc))
                ),
                [typeof(UiLengthOrAuto)] = Fixed(
                    ("Px", typeof(UiLengthOrAuto.Px)),
                    ("Percent", typeof(UiLengthOrAuto.Percent)),
                    ("Auto", typeof(UiLengthOrAuto.Auto))
                ),
                [typeof(UiAspectRatio)] = Fixed(
                    ("Auto", typeof(UiAspectRatio.Auto)),
                    ("Ratio", typeof(UiAspectRatio.Ratio))
                ),
                [typeof(UiFilterFunction)] = Fixed(
                    ("Brightness", typeof(UiFilterFunction.Brightness)),
                    ("DropShadow", typeof(UiFilterFunction.DropShadow))
                ),
                [typeof(ParentScene)] = Fixed(
                    ("PrimaryScene", typeof(ParentScene.Primary)),
                    ("Scene", typeof(ParentScene.Specific)),
                    ("Persistent", typeof(ParentScene.Persistent))
                ),
                [typeof(GameObjectKind)] = Fixed(
                    ("UiDocument", typeof(GameObjectKind.UiDocumentState)),
                    ("Empty", typeof(GameObjectKind.Empty)),
                    ("Cube", typeof(GameObjectKind.Cube)),
                    ("Sphere", typeof(GameObjectKind.Sphere)),
                    ("Capsule", typeof(GameObjectKind.Capsule)),
                    ("Cylinder", typeof(GameObjectKind.Cylinder)),
                    ("Plane", typeof(GameObjectKind.Plane)),
                    ("Quad", typeof(GameObjectKind.Quad)),
                    ("Image", typeof(GameObjectKind.Image)),
                    ("Text", typeof(GameObjectKind.Text)),
                    ("Camera", typeof(GameObjectKind.Camera)),
                    ("Light", typeof(GameObjectKind.Light)),
                    ("Prefab", typeof(GameObjectKind.Prefab))
                ),
                [typeof(TransformOperation)] = Fixed(
                    ("Translate", typeof(TransformOperation.Translate)),
                    ("Rotate", typeof(TransformOperation.Rotate)),
                    ("Skew", typeof(TransformOperation.Skew)),
                    ("Scale", typeof(TransformOperation.Scale))
                ),
                [typeof(Gradient)] = Fixed(
                    ("Linear", typeof(Gradient.Linear)),
                    ("Radial", typeof(Gradient.Radial))
                ),
                [typeof(PaintFill)] = Fixed(
                    ("Color", typeof(PaintFill.Color)),
                    ("Gradient", typeof(PaintFill.Gradient))
                ),
                [typeof(MotionValue)] = Fixed(
                    ("Scalar", typeof(MotionValue.Scalar)),
                    ("Length", typeof(MotionValue.Length)),
                    ("Color", typeof(MotionValue.Color)),
                    ("Vector2", typeof(MotionValue.Vector2)),
                    ("Vector3", typeof(MotionValue.Vector3)),
                    ("Angle", typeof(MotionValue.Angle)),
                    ("TransformList", typeof(MotionValue.TransformList)),
                    ("FilterList", typeof(MotionValue.FilterList)),
                    ("ShadowList", typeof(MotionValue.ShadowList)),
                    ("Gradient", typeof(MotionValue.Gradient)),
                    ("ClipInset", typeof(MotionValue.ClipInset)),
                    ("ClipPolygon", typeof(MotionValue.ClipPolygon)),
                    ("Discrete", typeof(MotionValue.Discrete))
                ),
                [typeof(MotionExpressionOperation)] = Fixed(
                    ("Add", typeof(MotionExpressionOperation.Add)),
                    ("Subtract", typeof(MotionExpressionOperation.Subtract)),
                    ("Multiply", typeof(MotionExpressionOperation.Multiply)),
                    ("Divide", typeof(MotionExpressionOperation.Divide)),
                    ("Power", typeof(MotionExpressionOperation.Power)),
                    ("SquareRoot", typeof(MotionExpressionOperation.SquareRoot)),
                    ("Absolute", typeof(MotionExpressionOperation.Absolute)),
                    ("Minimum", typeof(MotionExpressionOperation.Minimum)),
                    ("Maximum", typeof(MotionExpressionOperation.Maximum)),
                    ("Clamp", typeof(MotionExpressionOperation.Clamp)),
                    ("Modulo", typeof(MotionExpressionOperation.Modulo)),
                    ("Wrap", typeof(MotionExpressionOperation.Wrap)),
                    ("ExponentialDecay", typeof(MotionExpressionOperation.ExponentialDecay)),
                    ("Mix", typeof(MotionExpressionOperation.Mix))
                ),
                [typeof(MotionValueSource)] = Fixed(
                    ("Mutable", typeof(MotionValueSource.Mutable)),
                    ("Time", typeof(MotionValueSource.Time)),
                    ("Velocity", typeof(MotionValueSource.Velocity)),
                    ("Range", typeof(MotionValueSource.Range)),
                    ("Spring", typeof(MotionValueSource.Spring)),
                    ("Expression", typeof(MotionValueSource.Expression))
                ),
                [typeof(MotionValueCommand)] = Fixed(
                    ("Set", typeof(MotionValueCommand.Set)),
                    ("Jump", typeof(MotionValueCommand.Jump)),
                    ("Stop", typeof(MotionValueCommand.Stop)),
                    ("Animate", typeof(MotionValueCommand.Animate))
                ),
                [typeof(MotionControlTarget)] = Fixed(
                    ("Target", typeof(MotionControlTarget.Target)),
                    ("Variant", typeof(MotionControlTarget.Variant))
                ),
                [typeof(MotionControlCommand)] = Fixed(
                    ("Start", typeof(MotionControlCommand.Start)),
                    ("Set", typeof(MotionControlCommand.Set)),
                    ("Stop", typeof(MotionControlCommand.Stop)),
                    ("Clear", typeof(MotionControlCommand.Clear))
                ),
                [typeof(MotionSelector)] = Fixed(
                    ("Element", typeof(MotionSelector.Element)),
                    ("Name", typeof(MotionSelector.Name)),
                    ("ScopeRoot", typeof(MotionSelector.ScopeRoot)),
                    ("Children", typeof(MotionSelector.Children)),
                    ("Descendants", typeof(MotionSelector.Descendants))
                ),
                [typeof(MotionScopeCommand)] = Fixed(
                    ("Start", typeof(MotionScopeCommand.Start)),
                    ("Set", typeof(MotionScopeCommand.Set)),
                    ("Stop", typeof(MotionScopeCommand.Stop))
                ),
                [typeof(MotionEasing)] = Fixed(
                    ("Linear", typeof(MotionEasing.Linear)),
                    ("EaseIn", typeof(MotionEasing.EaseIn)),
                    ("EaseOut", typeof(MotionEasing.EaseOut)),
                    ("EaseInOut", typeof(MotionEasing.EaseInOut)),
                    ("CubicBezier", typeof(MotionEasing.CubicBezier)),
                    ("Steps", typeof(MotionEasing.Steps))
                ),
                [typeof(MotionRepeat)] = Fixed(
                    ("None", typeof(MotionRepeat.None)),
                    ("Count", typeof(MotionRepeat.Count)),
                    ("Forever", typeof(MotionRepeat.Forever))
                ),
                [typeof(InertiaTarget)] = Fixed(
                    ("Identity", typeof(InertiaTarget.Identity)),
                    ("NearestMultiple", typeof(InertiaTarget.NearestMultiple)),
                    ("FloorMultiple", typeof(InertiaTarget.FloorMultiple)),
                    ("CeilingMultiple", typeof(InertiaTarget.CeilingMultiple)),
                    ("Clamp", typeof(InertiaTarget.Clamp))
                ),
                [typeof(SpringConfiguration)] = Fixed(
                    ("Physical", typeof(SpringConfiguration.Physical)),
                    ("Duration", typeof(SpringConfiguration.Duration)),
                    ("VisualDuration", typeof(SpringConfiguration.VisualDuration))
                ),
                [typeof(TransitionGenerator)] = Fixed(
                    ("Immediate", typeof(TransitionGenerator.Immediate)),
                    ("Tween", typeof(TransitionGenerator.Tween)),
                    ("Spring", typeof(TransitionGenerator.Spring)),
                    ("Inertia", typeof(TransitionGenerator.Inertia))
                ),
                [typeof(MotionClockSource)] = Fixed(
                    ("Unscaled", typeof(MotionClockSource.Unscaled)),
                    ("Scaled", typeof(MotionClockSource.Scaled)),
                    ("Controlled", typeof(MotionClockSource.Controlled)),
                    ("Audio", typeof(MotionClockSource.Audio))
                ),
                [typeof(MotionDragConstraint)] = Fixed(
                    ("Bounds", typeof(MotionDragConstraint.Bounds)),
                    ("Element", typeof(MotionDragConstraint.Element))
                ),
                [typeof(MotionEventKind)] = Fixed(
                    ("Activated", typeof(MotionEventKind.Activated)),
                    ("Started", typeof(MotionEventKind.Started)),
                    ("Repeated", typeof(MotionEventKind.Repeated)),
                    ("Completed", typeof(MotionEventKind.Completed)),
                    ("Stopped", typeof(MotionEventKind.Stopped)),
                    ("Cancelled", typeof(MotionEventKind.Cancelled))
                ),
                [typeof(MotionPlaybackCommand)] = Fixed(
                    ("Play", typeof(MotionPlaybackCommand.Play)),
                    ("Pause", typeof(MotionPlaybackCommand.Pause)),
                    ("Replay", typeof(MotionPlaybackCommand.Replay)),
                    ("Stop", typeof(MotionPlaybackCommand.Stop)),
                    ("Cancel", typeof(MotionPlaybackCommand.Cancel)),
                    ("Complete", typeof(MotionPlaybackCommand.Complete)),
                    ("Seek", typeof(MotionPlaybackCommand.Seek)),
                    ("SetSpeed", typeof(MotionPlaybackCommand.SetSpeed)),
                    ("SetDirection", typeof(MotionPlaybackCommand.SetDirection))
                ),
                [typeof(MotionControlledClockCommand)] = Fixed(
                    ("Set", typeof(MotionControlledClockCommand.Set)),
                    ("Advance", typeof(MotionControlledClockCommand.Advance))
                ),
                [typeof(GridTrack)] = Fixed(
                    ("Px", typeof(GridTrack.Px)),
                    ("Fraction", typeof(GridTrack.Fraction)),
                    ("Auto", typeof(GridTrack.Auto))
                ),
                [typeof(OverlayPlacement)] = Fixed(
                    ("Layer", typeof(OverlayPlacement.Layer)),
                    ("Popover", typeof(OverlayPlacement.Popover)),
                    ("Modal", typeof(OverlayPlacement.Modal))
                ),
                [typeof(UiElement)] = Fixed(
                    ("VisualElement", typeof(UiElement.VisualElement)),
                    ("Flex", typeof(UiElement.Flex)),
                    ("Grid", typeof(UiElement.Grid)),
                    ("Stack", typeof(UiElement.Stack)),
                    ("Box", typeof(UiElement.Box)),
                    ("Label", typeof(UiElement.Label)),
                    ("TextElement", typeof(UiElement.TextElement)),
                    ("TextField", typeof(UiElement.TextField)),
                    ("Toggle", typeof(UiElement.Toggle)),
                    ("RadioButton", typeof(UiElement.RadioButton)),
                    ("RadioButtonGroup", typeof(UiElement.RadioButtonGroup)),
                    ("ToggleButtonGroup", typeof(UiElement.ToggleButtonGroup)),
                    ("DropdownField", typeof(UiElement.DropdownField)),
                    ("Button", typeof(UiElement.Button)),
                    ("RepeatButton", typeof(UiElement.RepeatButton)),
                    ("GroupBox", typeof(UiElement.GroupBox)),
                    ("PopupWindow", typeof(UiElement.PopupWindow)),
                    ("ScrollView", typeof(UiElement.ScrollView)),
                    ("Scroller", typeof(UiElement.Scroller)),
                    ("Slider", typeof(UiElement.Slider)),
                    ("SliderInt", typeof(UiElement.SliderInt)),
                    ("MinMaxSlider", typeof(UiElement.MinMaxSlider)),
                    ("ProgressBar", typeof(UiElement.ProgressBar)),
                    ("Tab", typeof(UiElement.Tab)),
                    ("TabView", typeof(UiElement.TabView)),
                    ("Image", typeof(UiElement.Image))
                ),
                [typeof(UiEventBody)] = Fixed(
                    ("AccessibilityAction", typeof(UiEventBody.AccessibilityAction)),
                    ("PointerDown", typeof(UiEventBody.PointerDown)),
                    ("PointerMove", typeof(UiEventBody.PointerMove)),
                    ("PointerUp", typeof(UiEventBody.PointerUp)),
                    ("PointerCancel", typeof(UiEventBody.PointerCancel)),
                    ("Click", typeof(UiEventBody.Click)),
                    ("PointerEnter", typeof(UiEventBody.PointerEnter)),
                    ("PointerLeave", typeof(UiEventBody.PointerLeave)),
                    ("PointerOver", typeof(UiEventBody.PointerOver)),
                    ("PointerOut", typeof(UiEventBody.PointerOut)),
                    ("Wheel", typeof(UiEventBody.Wheel)),
                    ("PointerCapture", typeof(UiEventBody.PointerCapture)),
                    ("PointerCaptureOut", typeof(UiEventBody.PointerCaptureOut)),
                    ("KeyDown", typeof(UiEventBody.KeyDown)),
                    ("KeyUp", typeof(UiEventBody.KeyUp)),
                    ("NavigationMove", typeof(UiEventBody.NavigationMove)),
                    ("NavigationCancel", typeof(UiEventBody.NavigationCancel)),
                    ("FocusIn", typeof(UiEventBody.FocusIn)),
                    ("Focus", typeof(UiEventBody.Focus)),
                    ("FocusOut", typeof(UiEventBody.FocusOut)),
                    ("Blur", typeof(UiEventBody.Blur)),
                    ("GeometryChanged", typeof(UiEventBody.GeometryChanged)),
                    ("AttachToPanel", typeof(UiEventBody.AttachToPanel)),
                    ("DetachFromPanel", typeof(UiEventBody.DetachFromPanel)),
                    ("TransitionStart", typeof(UiEventBody.TransitionStart)),
                    ("TransitionEnd", typeof(UiEventBody.TransitionEnd)),
                    ("TransitionCancel", typeof(UiEventBody.TransitionCancel)),
                    ("ValueChanging", typeof(UiEventBody.ValueChanging)),
                    ("ValueCommitted", typeof(UiEventBody.ValueCommitted)),
                    ("Input", typeof(UiEventBody.Input)),
                    ("SelectionChanged", typeof(UiEventBody.SelectionChanged)),
                    ("LinkEnter", typeof(UiEventBody.LinkEnter)),
                    ("LinkLeave", typeof(UiEventBody.LinkLeave)),
                    ("LinkDown", typeof(UiEventBody.LinkDown)),
                    ("LinkUp", typeof(UiEventBody.LinkUp)),
                    ("ScrollSettled", typeof(UiEventBody.ScrollSettled)),
                    ("ScrollChanged", typeof(UiEventBody.ScrollChanged)),
                    ("TabSelectionRequested", typeof(UiEventBody.TabSelectionRequested)),
                    ("TabCloseRequested", typeof(UiEventBody.TabCloseRequested)),
                    ("TabReorderRequested", typeof(UiEventBody.TabReorderRequested))
                ),
                [typeof(UiPointerButton)] = Fixed(
                    ("Left", typeof(UiPointerButton.Left)),
                    ("Middle", typeof(UiPointerButton.Middle)),
                    ("Right", typeof(UiPointerButton.Right)),
                    ("Other", typeof(UiPointerButton.Other))
                ),
                [typeof(UiFocusDirection)] = Fixed(
                    ("None", typeof(UiFocusDirection.None)),
                    ("Unspecified", typeof(UiFocusDirection.Unspecified)),
                    ("Left", typeof(UiFocusDirection.Left)),
                    ("Right", typeof(UiFocusDirection.Right)),
                    ("Other", typeof(UiFocusDirection.Other))
                ),
                [typeof(UiValue)] = Fixed(
                    ("Bool", typeof(UiValue.Bool)),
                    ("Index", typeof(UiValue.Index)),
                    ("Indices", typeof(UiValue.Indices)),
                    ("Choice", typeof(UiValue.Choice)),
                    ("F32", typeof(UiValue.F32)),
                    ("I32", typeof(UiValue.I32)),
                    ("F32Range", typeof(UiValue.F32Range)),
                    ("String", typeof(UiValue.String))
                ),
                [typeof(LowerLimit)] = Fixed(
                    ("Unbounded", typeof(LowerLimit.Unbounded)),
                    ("Inclusive", typeof(LowerLimit.Inclusive))
                ),
                [typeof(UpperLimit)] = Fixed(
                    ("Unbounded", typeof(UpperLimit.Unbounded)),
                    ("Inclusive", typeof(UpperLimit.Inclusive))
                ),
                [typeof(InteractionDistance)] = Fixed(
                    ("Unbounded", typeof(InteractionDistance.Unbounded)),
                    ("Inclusive", typeof(InteractionDistance.Inclusive))
                ),
                [typeof(ClickEvent)] = Fixed(
                    ("Pointer", typeof(ClickEvent.Pointer)),
                    ("NavigationSubmit", typeof(ClickEvent.NavigationSubmit)),
                    ("Repeat", typeof(ClickEvent.Repeat))
                ),
                [typeof(VisualElementUpdate)] = Fixed(
                    ("Properties", typeof(VisualElementUpdate.Properties)),
                    ("Parent", typeof(VisualElementUpdate.Parent)),
                    ("Index", typeof(VisualElementUpdate.Index))
                ),
                [typeof(VisualElementAction)] = Fixed(
                    ("ParticleStreaks", typeof(VisualElementAction.ParticleStreaks)),
                    ("Focus", typeof(VisualElementAction.Focus)),
                    ("Blur", typeof(VisualElementAction.Blur)),
                    ("CapturePointer", typeof(VisualElementAction.CapturePointer)),
                    ("ReleasePointer", typeof(VisualElementAction.ReleasePointer)),
                    ("ScrollTo", typeof(VisualElementAction.ScrollTo)),
                    ("SelectText", typeof(VisualElementAction.SelectText))
                ),
                [typeof(ActionBody)] = Nested<ActionBody>(
                    "Activate",
                    "PointerEnter",
                    "PointerExit",
                    "PointerDown",
                    "PointerUp",
                    "PointerClick",
                    "DragStart",
                    "DragEnd",
                    "KeyDown",
                    "KeyUp",
                    "ControllerButtonDown",
                    "ControllerButtonUp",
                    "ControllerNavigate",
                    "GeometryObservations",
                    "MotionEvents",
                    "ApplicationStateChanged",
                    "ReducedMotionPreferenceChanged"
                ),
                [typeof(DiagnosticsCommand)] = Nested<DiagnosticsCommand>("SetMetadata"),
                [typeof(CameraTarget)] = Nested<CameraTarget>("Input", "Object"),
                [typeof(GeometryObservationTarget)] = Nested<GeometryObservationTarget>(
                    "UiElement",
                    "Viewport",
                    "WorldOrigin",
                    "WorldAnchor",
                    "WorldRenderedBounds"
                ),
                [typeof(GeometryValue)] = Nested<GeometryValue>(
                    "Element",
                    "Viewport",
                    "WorldPoint",
                    "WorldBounds"
                ),
                [typeof(GeometryObservationResult)] = Nested<GeometryObservationResult>(
                    "Current",
                    "Unavailable"
                ),
                [typeof(AccessibilityAction)] = Nested<AccessibilityAction>(
                    "Activate",
                    "Increment",
                    "Decrement",
                    "Dismiss",
                    "Scroll"
                ),
                [typeof(UiAccessibilityAction)] = Nested<UiAccessibilityAction>(
                    "Activate",
                    "Increment",
                    "Decrement",
                    "Dismiss",
                    "ScrollForward",
                    "ScrollBackward"
                ),
                [typeof(ParticleSpawnLocation)] = Fixed(
                    ("GameObject", typeof(ParticleSpawnLocation.AtGameObject)),
                    ("WorldPosition", typeof(ParticleSpawnLocation.AtWorldPosition))
                ),
                [typeof(TweenRepeat)] = Fixed(
                    ("Once", typeof(TweenRepeat.Once)),
                    ("Count", typeof(TweenRepeat.Count)),
                    ("Forever", typeof(TweenRepeat.Forever))
                ),
                [typeof(CommandBody)] = CommandCases(),
            };
            return cases;
        }

        private static IReadOnlyDictionary<string, Type> CommandCases()
        {
            return Fixed(
                ("AssetsReplaceSet", typeof(CommandBody.Assets.ReplaceSet)),
                ("SceneLoad", typeof(CommandBody.Scene.Load)),
                ("SceneUnload", typeof(CommandBody.Scene.Unload)),
                ("SceneSetPrimary", typeof(CommandBody.Scene.SetPrimary)),
                ("ObjectCreate", typeof(CommandBody.Object.Create)),
                ("ObjectDestroy", typeof(CommandBody.Object.Destroy)),
                ("ObjectSetActive", typeof(CommandBody.Object.SetActive)),
                ("ObjectReparent", typeof(CommandBody.Object.Reparent)),
                ("TransformSetLocalPosition", typeof(CommandBody.Transform.SetLocalPosition)),
                ("TransformSetWorldPosition", typeof(CommandBody.Transform.SetWorldPosition)),
                ("TransformTweenLocalPosition", typeof(CommandBody.Transform.TweenLocalPosition)),
                ("TransformTweenWorldPosition", typeof(CommandBody.Transform.TweenWorldPosition)),
                ("TransformSetLocalRotation", typeof(CommandBody.Transform.SetLocalRotation)),
                ("TransformSetWorldRotation", typeof(CommandBody.Transform.SetWorldRotation)),
                ("TransformTweenLocalRotation", typeof(CommandBody.Transform.TweenLocalRotation)),
                ("TransformTweenWorldRotation", typeof(CommandBody.Transform.TweenWorldRotation)),
                ("TransformSetLocalScale", typeof(CommandBody.Transform.SetLocalScale)),
                ("TransformTweenLocalScale", typeof(CommandBody.Transform.TweenLocalScale)),
                ("RendererSetMaterial", typeof(CommandBody.Renderer.SetMaterial)),
                ("CameraSetEnabled", typeof(CommandBody.Camera.SetEnabled)),
                ("CameraSetPerspective", typeof(CommandBody.Camera.SetPerspective)),
                ("CameraTweenFieldOfView", typeof(CommandBody.Camera.TweenFieldOfView)),
                ("CameraSetOrthographic", typeof(CommandBody.Camera.SetOrthographic)),
                ("CameraTweenOrthographicSize", typeof(CommandBody.Camera.TweenOrthographicSize)),
                ("CameraSetClipping", typeof(CommandBody.Camera.SetClipping)),
                ("CameraSetClear", typeof(CommandBody.Camera.SetClear)),
                ("LightSetEnabled", typeof(CommandBody.Light.SetEnabled)),
                ("LightSetType", typeof(CommandBody.Light.SetType)),
                ("LightSetColor", typeof(CommandBody.Light.SetColor)),
                ("LightTweenColor", typeof(CommandBody.Light.TweenColor)),
                ("LightSetIntensity", typeof(CommandBody.Light.SetIntensity)),
                ("LightTweenIntensity", typeof(CommandBody.Light.TweenIntensity)),
                ("LightSetRange", typeof(CommandBody.Light.SetRange)),
                ("LightSetSpotAngle", typeof(CommandBody.Light.SetSpotAngle)),
                ("LightSetShadows", typeof(CommandBody.Light.SetShadows)),
                ("ImageSetTexture", typeof(CommandBody.Image.SetTexture)),
                ("ImageSetSize", typeof(CommandBody.Image.SetSize)),
                ("ImageSetFit", typeof(CommandBody.Image.SetFit)),
                ("ImageSetTint", typeof(CommandBody.Image.SetTint)),
                ("ImageTweenTint", typeof(CommandBody.Image.TweenTint)),
                ("ImageSetOpacity", typeof(CommandBody.Image.SetOpacity)),
                ("ImageTweenOpacity", typeof(CommandBody.Image.TweenOpacity)),
                ("ImageSetFaceCamera", typeof(CommandBody.Image.SetFaceCamera)),
                ("TextSetContent", typeof(CommandBody.Text.SetContent)),
                ("TextSetFont", typeof(CommandBody.Text.SetFont)),
                ("TextSetSize", typeof(CommandBody.Text.SetSize)),
                ("TextTweenSize", typeof(CommandBody.Text.TweenSize)),
                ("TextSetColor", typeof(CommandBody.Text.SetColor)),
                ("TextTweenColor", typeof(CommandBody.Text.TweenColor)),
                ("TextSetAlignment", typeof(CommandBody.Text.SetAlignment)),
                ("TextSetWrapping", typeof(CommandBody.Text.SetWrapping)),
                ("TextSetRichText", typeof(CommandBody.Text.SetRichText)),
                ("TextSetFaceCamera", typeof(CommandBody.Text.SetFaceCamera)),
                ("AnimatorPlay", typeof(CommandBody.Animator.Play)),
                ("AnimatorCrossFade", typeof(CommandBody.Animator.CrossFade)),
                ("AnimatorSetBool", typeof(CommandBody.Animator.SetBool)),
                ("AnimatorSetInt", typeof(CommandBody.Animator.SetInt)),
                ("AnimatorSetFloat", typeof(CommandBody.Animator.SetFloat)),
                ("AnimatorSetTrigger", typeof(CommandBody.Animator.SetTrigger)),
                ("AnimatorSetSpeed", typeof(CommandBody.Animator.SetSpeed)),
                ("ParticlePlay", typeof(CommandBody.Particle.Play)),
                ("ParticleStop", typeof(CommandBody.Particle.Stop)),
                ("ParticleSpawn", typeof(CommandBody.Particle.Spawn)),
                ("AudioPlay", typeof(CommandBody.Audio.Play)),
                ("AudioStop", typeof(CommandBody.Audio.Stop)),
                ("AudioPause", typeof(CommandBody.Audio.Pause)),
                ("AudioResume", typeof(CommandBody.Audio.Resume)),
                ("AudioSeek", typeof(CommandBody.Audio.Seek)),
                ("AudioSetBuffering", typeof(CommandBody.Audio.SetBuffering)),
                ("AudioReplace", typeof(CommandBody.Audio.Replace)),
                ("AudioSetVolume", typeof(CommandBody.Audio.SetVolume)),
                ("AudioTweenVolume", typeof(CommandBody.Audio.TweenVolume)),
                ("TimeWait", typeof(CommandBody.Time.Wait)),
                ("OperationCancel", typeof(CommandBody.Operation.Cancel)),
                ("InputSetEnabled", typeof(CommandBody.Input.SetEnabled)),
                ("InputSetCamera", typeof(CommandBody.Input.SetCamera)),
                ("InputSetPointerEvents", typeof(CommandBody.Input.SetPointerEvents)),
                ("InputSetGlobalKeys", typeof(CommandBody.Input.SetGlobalKeys)),
                ("InputSetController", typeof(CommandBody.Input.SetController)),
                ("ControllerVibrate", typeof(CommandBody.Controller.Vibrate)),
                ("DebugUi", typeof(CommandBody.DebugUi)),
                ("VisualElementCreate", typeof(CommandBody.VisualElement.Create)),
                ("VisualElementUpdate", typeof(CommandBody.VisualElement.Update)),
                ("VisualElementDestroy", typeof(CommandBody.VisualElement.Destroy)),
                ("VisualElementPerformAction", typeof(CommandBody.VisualElement.PerformAction)),
                ("MotionValue", typeof(CommandBody.Motion.ValueCommand)),
                ("MotionValuePlayback", typeof(CommandBody.Motion.ValuePlayback)),
                ("MotionPlayback", typeof(CommandBody.Motion.Playback)),
                ("MotionControlledClock", typeof(CommandBody.Motion.ControlledClock)),
                ("MotionControl", typeof(CommandBody.Motion.Control)),
                ("MotionScope", typeof(CommandBody.Motion.Scope)),
                ("MotionDragControl", typeof(CommandBody.Motion.DragControl)),
                ("GeometryObservationUpdate", typeof(CommandBody.GeometryObservation)),
                ("AccessibilityUpdate", typeof(CommandBody.AccessibilityUpdate)),
                ("Diagnostics", typeof(CommandBody.Diagnostics)),
                ("ApplicationOpenUrl", typeof(CommandBody.ApplicationOpenUrl))
            );
        }

        private static IReadOnlyDictionary<string, Type> Nested<T>(params string[] names)
        {
            var result = new Dictionary<string, Type>();
            foreach (string name in names)
            {
                result[name] =
                    typeof(T).GetNestedType(name)
                    ?? throw new InvalidOperationException(
                        $"Union case {typeof(T).Name}.{name} is missing."
                    );
            }

            return result;
        }

        private static IReadOnlyDictionary<string, Type> Fixed(
            params (string Tag, Type Type)[] values
        ) => values.ToDictionary(value => value.Tag, value => value.Type, StringComparer.Ordinal);
    }
}
