#nullable enable

using Google.FlatBuffers;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement.Tests
{
    internal static partial class BattlementFlatBufferResponseFixtures
    {
        private static Payload Perspective(
            FlatBufferBuilder builder,
            CommandBody.Camera.SetPerspective value
        )
        {
            Wire.PerspectivePayload.StartPerspectivePayload(builder);
            Wire.PerspectivePayload.AddFieldOfView(builder, value.FieldOfView);
            Wire.PerspectivePayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            Wire.PerspectivePayload.AddOnConflict(builder, Conflict(value.OnConflict));
            return new(
                Wire.CoreCommandKind.CameraSetPerspective,
                Wire.CoreCommandPayload.PerspectivePayload,
                Wire.PerspectivePayload.EndPerspectivePayload(builder).Value
            );
        }

        private static Payload TweenFieldOfView(
            FlatBufferBuilder builder,
            CommandBody.Camera.TweenFieldOfView value
        )
        {
            Wire.TweenFieldOfViewPayload.StartTweenFieldOfViewPayload(builder);
            Wire.TweenFieldOfViewPayload.AddTween(builder, Tween(builder, value.Tween));
            Wire.TweenFieldOfViewPayload.AddFieldOfView(builder, value.FieldOfView);
            Wire.TweenFieldOfViewPayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            Wire.TweenFieldOfViewPayload.AddOnConflict(builder, Conflict(value.OnConflict));
            return new(
                Wire.CoreCommandKind.CameraTweenFieldOfView,
                Wire.CoreCommandPayload.TweenFieldOfViewPayload,
                Wire.TweenFieldOfViewPayload.EndTweenFieldOfViewPayload(builder).Value
            );
        }

        private static Payload Orthographic(
            FlatBufferBuilder builder,
            CommandBody.Camera.SetOrthographic value
        )
        {
            Wire.OrthographicPayload.StartOrthographicPayload(builder);
            Wire.OrthographicPayload.AddSize(builder, value.Size);
            Wire.OrthographicPayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            Wire.OrthographicPayload.AddOnConflict(builder, Conflict(value.OnConflict));
            return new(
                Wire.CoreCommandKind.CameraSetOrthographic,
                Wire.CoreCommandPayload.OrthographicPayload,
                Wire.OrthographicPayload.EndOrthographicPayload(builder).Value
            );
        }

        private static Payload TweenOrthographic(
            FlatBufferBuilder builder,
            CommandBody.Camera.TweenOrthographicSize value
        )
        {
            Wire.TweenOrthographicSizePayload.StartTweenOrthographicSizePayload(builder);
            Wire.TweenOrthographicSizePayload.AddTween(builder, Tween(builder, value.Tween));
            Wire.TweenOrthographicSizePayload.AddSize(builder, value.Size);
            Wire.TweenOrthographicSizePayload.AddObjectId(
                builder,
                Uuid(builder, value.ObjectId.Value)
            );
            Wire.TweenOrthographicSizePayload.AddOnConflict(builder, Conflict(value.OnConflict));
            return new(
                Wire.CoreCommandKind.CameraTweenOrthographicSize,
                Wire.CoreCommandPayload.TweenOrthographicSizePayload,
                Wire.TweenOrthographicSizePayload.EndTweenOrthographicSizePayload(builder).Value
            );
        }

        private static Payload CameraClipping(
            FlatBufferBuilder builder,
            CommandBody.Camera.SetClipping value
        )
        {
            Wire.CameraClippingPayload.StartCameraClippingPayload(builder);
            Wire.CameraClippingPayload.AddFar(builder, value.Far);
            Wire.CameraClippingPayload.AddNear(builder, value.Near);
            Wire.CameraClippingPayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            return new(
                Wire.CoreCommandKind.CameraSetClipping,
                Wire.CoreCommandPayload.CameraClippingPayload,
                Wire.CameraClippingPayload.EndCameraClippingPayload(builder).Value
            );
        }

        private static Payload CameraClear(
            FlatBufferBuilder builder,
            CommandBody.Camera.SetClear value
        )
        {
            Wire.CameraClearPayload.StartCameraClearPayload(builder);
            if (value.ClearColor is Color color)
                Wire.CameraClearPayload.AddClearColor(builder, Rgba(builder, color));
            Wire.CameraClearPayload.AddClearMode(builder, (Wire.CameraClearMode)value.ClearMode);
            Wire.CameraClearPayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            return new(
                Wire.CoreCommandKind.CameraSetClear,
                Wire.CoreCommandPayload.CameraClearPayload,
                Wire.CameraClearPayload.EndCameraClearPayload(builder).Value
            );
        }

        private static Payload LightType(FlatBufferBuilder builder, CommandBody.Light.SetType value)
        {
            Wire.LightTypePayload.StartLightTypePayload(builder);
            Wire.LightTypePayload.AddLightType(builder, (Wire.LightType)value.Type);
            Wire.LightTypePayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            return new(
                Wire.CoreCommandKind.LightSetType,
                Wire.CoreCommandPayload.LightTypePayload,
                Wire.LightTypePayload.EndLightTypePayload(builder).Value
            );
        }

        private static Payload Color(
            FlatBufferBuilder builder,
            Battlement.ObjectId objectId,
            Battlement.Color value,
            ConflictPolicy conflict,
            Wire.CoreCommandKind kind
        )
        {
            Wire.ColorPayload.StartColorPayload(builder);
            Wire.ColorPayload.AddColor(builder, Rgba(builder, value));
            Wire.ColorPayload.AddObjectId(builder, Uuid(builder, objectId.Value));
            Wire.ColorPayload.AddOnConflict(builder, Conflict(conflict));
            return new(
                kind,
                Wire.CoreCommandPayload.ColorPayload,
                Wire.ColorPayload.EndColorPayload(builder).Value
            );
        }

        private static Payload TweenColor(
            FlatBufferBuilder builder,
            Battlement.ObjectId objectId,
            Battlement.Color value,
            Tween tween,
            ConflictPolicy conflict,
            Wire.CoreCommandKind kind
        )
        {
            Wire.TweenColorPayload.StartTweenColorPayload(builder);
            Wire.TweenColorPayload.AddTween(builder, Tween(builder, tween));
            Wire.TweenColorPayload.AddColor(builder, Rgba(builder, value));
            Wire.TweenColorPayload.AddObjectId(builder, Uuid(builder, objectId.Value));
            Wire.TweenColorPayload.AddOnConflict(builder, Conflict(conflict));
            return new(
                kind,
                Wire.CoreCommandPayload.TweenColorPayload,
                Wire.TweenColorPayload.EndTweenColorPayload(builder).Value
            );
        }

        private static Payload Intensity(
            FlatBufferBuilder builder,
            Battlement.ObjectId objectId,
            double value,
            ConflictPolicy conflict,
            Wire.CoreCommandKind kind
        )
        {
            Wire.IntensityPayload.StartIntensityPayload(builder);
            Wire.IntensityPayload.AddIntensity(builder, value);
            Wire.IntensityPayload.AddObjectId(builder, Uuid(builder, objectId.Value));
            Wire.IntensityPayload.AddOnConflict(builder, Conflict(conflict));
            return new(
                kind,
                Wire.CoreCommandPayload.IntensityPayload,
                Wire.IntensityPayload.EndIntensityPayload(builder).Value
            );
        }

        private static Payload TweenIntensity(
            FlatBufferBuilder builder,
            CommandBody.Light.TweenIntensity value
        )
        {
            Wire.TweenIntensityPayload.StartTweenIntensityPayload(builder);
            Wire.TweenIntensityPayload.AddTween(builder, Tween(builder, value.Tween));
            Wire.TweenIntensityPayload.AddIntensity(builder, value.Intensity);
            Wire.TweenIntensityPayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            Wire.TweenIntensityPayload.AddOnConflict(builder, Conflict(value.OnConflict));
            return new(
                Wire.CoreCommandKind.LightTweenIntensity,
                Wire.CoreCommandPayload.TweenIntensityPayload,
                Wire.TweenIntensityPayload.EndTweenIntensityPayload(builder).Value
            );
        }

        private static Payload LightRange(
            FlatBufferBuilder builder,
            CommandBody.Light.SetRange value
        )
        {
            Wire.LightRangePayload.StartLightRangePayload(builder);
            Wire.LightRangePayload.AddRange(builder, value.Range);
            Wire.LightRangePayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            return new(
                Wire.CoreCommandKind.LightSetRange,
                Wire.CoreCommandPayload.LightRangePayload,
                Wire.LightRangePayload.EndLightRangePayload(builder).Value
            );
        }

        private static Payload SpotAngle(
            FlatBufferBuilder builder,
            CommandBody.Light.SetSpotAngle value
        )
        {
            Wire.SpotAnglePayload.StartSpotAnglePayload(builder);
            Wire.SpotAnglePayload.AddInnerSpotAngle(builder, value.InnerSpotAngle);
            Wire.SpotAnglePayload.AddOuterSpotAngle(builder, value.OuterSpotAngle);
            Wire.SpotAnglePayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            return new(
                Wire.CoreCommandKind.LightSetSpotAngle,
                Wire.CoreCommandPayload.SpotAnglePayload,
                Wire.SpotAnglePayload.EndSpotAnglePayload(builder).Value
            );
        }

        private static Payload LightShadows(
            FlatBufferBuilder builder,
            CommandBody.Light.SetShadows value
        )
        {
            Wire.LightShadowsPayload.StartLightShadowsPayload(builder);
            Wire.LightShadowsPayload.AddShadows(builder, (Wire.ShadowMode)value.Shadows);
            Wire.LightShadowsPayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            return new(
                Wire.CoreCommandKind.LightSetShadows,
                Wire.CoreCommandPayload.LightShadowsPayload,
                Wire.LightShadowsPayload.EndLightShadowsPayload(builder).Value
            );
        }
    }
}
