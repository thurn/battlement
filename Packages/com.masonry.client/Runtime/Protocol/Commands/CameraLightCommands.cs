#nullable enable

namespace Masonry
{
    public abstract partial record CommandBody
    {
        public static class Camera
        {
            /// <summary>Enable or disable a camera component.</summary>
            /// <param name="ObjectId">Target camera object.</param>
            /// <param name="IsEnabled">New enabled state.</param>
            public sealed record SetEnabled(ObjectId ObjectId, bool IsEnabled) : CommandBody;

            /// <summary>Switch a camera to perspective projection.</summary>
            /// <param name="ObjectId">Target camera object.</param>
            /// <param name="FieldOfView">Vertical field of view in degrees.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record SetPerspective(
                ObjectId ObjectId,
                double FieldOfView,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;

            /// <summary>Tween a perspective camera's vertical field of view.</summary>
            /// <param name="ObjectId">Target perspective camera object.</param>
            /// <param name="FieldOfView">Final vertical field of view in degrees.</param>
            /// <param name="Tween">Tween timing and repetition.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record TweenFieldOfView(
                ObjectId ObjectId,
                double FieldOfView,
                Tween Tween,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;

            /// <summary>Switch a camera to orthographic projection.</summary>
            /// <param name="ObjectId">Target camera object.</param>
            /// <param name="Size">Positive orthographic half-height.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record SetOrthographic(
                ObjectId ObjectId,
                double Size,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;

            /// <summary>Tween an orthographic camera's size.</summary>
            /// <param name="ObjectId">Target orthographic camera object.</param>
            /// <param name="Size">Positive final orthographic half-height.</param>
            /// <param name="Tween">Tween timing and repetition.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record TweenOrthographicSize(
                ObjectId ObjectId,
                double Size,
                Tween Tween,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;

            /// <summary>Set a camera's near and far clipping distances.</summary>
            /// <param name="ObjectId">Target camera object.</param>
            /// <param name="Near">Positive near clipping distance.</param>
            /// <param name="Far">Far clipping distance, greater than the near distance.</param>
            public sealed record SetClipping(ObjectId ObjectId, double Near, double Far)
                : CommandBody;

            /// <summary>Set a camera's clear mode and optional solid clear color.</summary>
            /// <param name="ObjectId">Target camera object.</param>
            /// <param name="ClearMode">Requested clear mode.</param>
            /// <param name="ClearColor">Required for solid color; otherwise null.</param>
            public sealed record SetClear(
                ObjectId ObjectId,
                CameraClearMode ClearMode,
                Color? ClearColor = null
            ) : CommandBody;
        }

        public static class Light
        {
            /// <summary>Enable or disable a light component.</summary>
            /// <param name="ObjectId">Target light object.</param>
            /// <param name="IsEnabled">New enabled state.</param>
            public sealed record SetEnabled(ObjectId ObjectId, bool IsEnabled) : CommandBody;

            /// <summary>Change a standard light's type.</summary>
            /// <param name="ObjectId">Target light object.</param>
            /// <param name="Type">Requested standard light type.</param>
            public sealed record SetType(ObjectId ObjectId, LightType Type) : CommandBody;

            /// <summary>Set a light's color immediately.</summary>
            /// <param name="ObjectId">Target light object.</param>
            /// <param name="Color">Requested linear color.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record SetColor(
                ObjectId ObjectId,
                Color Color,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;

            /// <summary>Tween a light's color.</summary>
            /// <param name="ObjectId">Target light object.</param>
            /// <param name="Color">Requested final linear color.</param>
            /// <param name="Tween">Tween timing and repetition.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record TweenColor(
                ObjectId ObjectId,
                Color Color,
                Tween Tween,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;

            /// <summary>Set a light's intensity immediately.</summary>
            /// <param name="ObjectId">Target light object.</param>
            /// <param name="Intensity">Requested nonnegative intensity.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record SetIntensity(
                ObjectId ObjectId,
                double Intensity,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;

            /// <summary>Tween a light's intensity.</summary>
            /// <param name="ObjectId">Target light object.</param>
            /// <param name="Intensity">Requested final nonnegative intensity.</param>
            /// <param name="Tween">Tween timing and repetition.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record TweenIntensity(
                ObjectId ObjectId,
                double Intensity,
                Tween Tween,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;

            /// <summary>Set the range of a point or spot light.</summary>
            /// <param name="ObjectId">Target point or spot light object.</param>
            /// <param name="Range">Positive range in world units.</param>
            public sealed record SetRange(ObjectId ObjectId, double Range) : CommandBody;

            /// <summary>Set a spot light's inner and outer angles.</summary>
            /// <param name="ObjectId">Target spot light object.</param>
            /// <param name="OuterSpotAngle">Outer angle in degrees.</param>
            /// <param name="InnerSpotAngle">Inner angle in degrees.</param>
            public sealed record SetSpotAngle(
                ObjectId ObjectId,
                double OuterSpotAngle,
                double InnerSpotAngle
            ) : CommandBody;

            /// <summary>Set a light's shadow mode.</summary>
            /// <param name="ObjectId">Target light object.</param>
            /// <param name="Shadows">Requested shadow mode.</param>
            public sealed record SetShadows(ObjectId ObjectId, ShadowMode Shadows) : CommandBody;
        }
    }
}
