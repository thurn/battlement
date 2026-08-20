#nullable enable

using UnityEngine;

namespace Masonry
{
    internal static class MasonryStandardComponents
    {
        public static GameObject CreateCamera(CameraState state)
        {
            var gameObject = new GameObject("Masonry Camera");
            try
            {
                Camera camera = gameObject.AddComponent<Camera>();
                camera.orthographic = state.Projection switch
                {
                    CameraProjection.Perspective => false,
                    CameraProjection.Orthographic => true,
                    _ => throw Invalid("Camera projection is unknown."),
                };
                camera.fieldOfView = RequireRange(
                    state.FieldOfView,
                    1,
                    179,
                    "Camera field of view"
                );
                camera.orthographicSize = RequirePositive(
                    state.OrthographicSize,
                    "Camera orthographic size"
                );
                camera.nearClipPlane = RequirePositive(state.NearClip, "Camera near clip");
                camera.farClipPlane = RequireFinite(state.FarClip, "Camera far clip");
                if (camera.farClipPlane <= camera.nearClipPlane)
                {
                    throw Invalid("Camera far clip must be greater than its near clip.");
                }

                camera.clearFlags = state.ClearMode switch
                {
                    CameraClearMode.Skybox => CameraClearFlags.Skybox,
                    CameraClearMode.SolidColor => CameraClearFlags.SolidColor,
                    CameraClearMode.Depth => CameraClearFlags.Depth,
                    CameraClearMode.Nothing => CameraClearFlags.Nothing,
                    _ => throw Invalid("Camera clear mode is unknown."),
                };
                camera.backgroundColor = ConvertColor(state.ClearColor, "Camera clear color");
                camera.enabled = state.IsEnabled;
                return gameObject;
            }
            catch
            {
                DestroyUnityObject(gameObject);
                throw;
            }
        }

        public static GameObject CreateLight(LightState state)
        {
            var gameObject = new GameObject("Masonry Light");
            try
            {
                Light light = gameObject.AddComponent<Light>();
                light.type = state.Type switch
                {
                    LightType.Directional => UnityEngine.LightType.Directional,
                    LightType.Point => UnityEngine.LightType.Point,
                    LightType.Spot => UnityEngine.LightType.Spot,
                    _ => throw Invalid("Light type is unknown."),
                };
                light.color = ConvertColor(state.Color, "Light color");
                light.intensity = RequireNonnegative(state.Intensity, "Light intensity");
                light.range = RequirePositive(state.Range, "Light range");
                float outerSpotAngle = RequireRange(
                    state.OuterSpotAngle,
                    0,
                    179,
                    "Light outer spot angle"
                );
                float innerSpotAngle = RequireNonnegative(
                    state.InnerSpotAngle,
                    "Light inner spot angle"
                );
                if (innerSpotAngle > outerSpotAngle)
                {
                    throw Invalid("Light inner spot angle cannot exceed its outer angle.");
                }

                light.spotAngle = outerSpotAngle;
                light.innerSpotAngle = innerSpotAngle;

                light.shadows = state.Shadows switch
                {
                    ShadowMode.None => LightShadows.None,
                    ShadowMode.Hard => LightShadows.Hard,
                    ShadowMode.Soft => LightShadows.Soft,
                    _ => throw Invalid("Light shadow mode is unknown."),
                };
                light.enabled = state.IsEnabled;
                return gameObject;
            }
            catch
            {
                DestroyUnityObject(gameObject);
                throw;
            }
        }

        internal static UnityEngine.Color ConvertColor(Color value, string name) =>
            new(
                RequireUnit(value.Red, $"{name} red"),
                RequireUnit(value.Green, $"{name} green"),
                RequireUnit(value.Blue, $"{name} blue"),
                RequireUnit(value.Alpha, $"{name} alpha")
            );

        internal static float RequirePositive(double value, string name)
        {
            float converted = RequireFinite(value, name);
            return converted > 0
                ? converted
                : throw Invalid($"{name} must be finite and positive.");
        }

        private static float RequireNonnegative(double value, string name)
        {
            float converted = RequireFinite(value, name);
            return converted >= 0
                ? converted
                : throw Invalid($"{name} must be finite and nonnegative.");
        }

        private static float RequireRange(double value, double minimum, double maximum, string name)
        {
            float converted = RequireFinite(value, name);
            return value > minimum && value < maximum
                ? converted
                : throw Invalid($"{name} must be strictly between {minimum} and {maximum}.");
        }

        private static float RequireUnit(double value, string name) =>
            double.IsFinite(value) && value is >= 0 and <= 1
                ? (float)value
                : throw Invalid($"{name} must be in the inclusive range [0, 1].");

        private static float RequireFinite(double value, string name)
        {
            float converted = (float)value;
            return double.IsFinite(value) && float.IsFinite(converted)
                ? converted
                : throw Invalid($"{name} must be finite.");
        }

        private static MasonryWorldException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);

        private static void DestroyUnityObject(Object value)
        {
            if (Application.isPlaying)
            {
                Object.Destroy(value);
            }
            else
            {
                Object.DestroyImmediate(value);
            }
        }
    }
}
