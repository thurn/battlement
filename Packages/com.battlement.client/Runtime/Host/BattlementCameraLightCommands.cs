#nullable enable

using System;
using UnityEngine;

namespace Battlement
{
    internal static class BattlementCameraLightCommands
    {
        public static IBattlementCommandOperation? SetCameraEnabled(
            CommandBody.Camera.SetEnabled command,
            BattlementWorld world
        )
        {
            Camera camera = RequireComponent<Camera>(command.ObjectId, world);
            world.SetCameraEnabled(camera, command.IsEnabled);
            return null;
        }

        public static IBattlementCommandOperation? SetPerspective(
            CommandBody.Camera.SetPerspective command,
            BattlementWorld world
        )
        {
            Camera camera = RequireComponent<Camera>(command.ObjectId, world);
            float fieldOfView = BattlementStandardComponents.RequireRange(
                command.FieldOfView,
                1,
                179,
                "Camera field of view"
            );
            camera.orthographic = false;
            camera.fieldOfView = fieldOfView;
            return null;
        }

        public static IBattlementCommandOperation? TweenFieldOfView(
            CommandBody.Camera.TweenFieldOfView command,
            BattlementWorld world,
            BattlementTweenAdapter tweens,
            TimeSpan now
        )
        {
            Camera camera = RequireComponent<Camera>(command.ObjectId, world);
            if (camera.orthographic)
            {
                throw Invalid("Camera must be perspective to tween its field of view.");
            }

            float fieldOfView = BattlementStandardComponents.RequireRange(
                command.FieldOfView,
                1,
                179,
                "Camera field of view"
            );
            return tweens.Float(
                camera.transform,
                camera.fieldOfView,
                fieldOfView,
                command.Tween,
                now,
                value => camera.fieldOfView = value
            );
        }

        public static IBattlementCommandOperation? SetOrthographic(
            CommandBody.Camera.SetOrthographic command,
            BattlementWorld world
        )
        {
            Camera camera = RequireComponent<Camera>(command.ObjectId, world);
            float size = BattlementStandardComponents.RequirePositive(
                command.Size,
                "Camera orthographic size"
            );
            camera.orthographic = true;
            camera.orthographicSize = size;
            return null;
        }

        public static IBattlementCommandOperation? TweenOrthographicSize(
            CommandBody.Camera.TweenOrthographicSize command,
            BattlementWorld world,
            BattlementTweenAdapter tweens,
            TimeSpan now
        )
        {
            Camera camera = RequireComponent<Camera>(command.ObjectId, world);
            if (!camera.orthographic)
            {
                throw Invalid("Camera must be orthographic to tween its size.");
            }

            float size = BattlementStandardComponents.RequirePositive(
                command.Size,
                "Camera orthographic size"
            );
            return tweens.Float(
                camera.transform,
                camera.orthographicSize,
                size,
                command.Tween,
                now,
                value => camera.orthographicSize = value
            );
        }

        public static IBattlementCommandOperation? SetClipping(
            CommandBody.Camera.SetClipping command,
            BattlementWorld world
        )
        {
            Camera camera = RequireComponent<Camera>(command.ObjectId, world);
            float near = BattlementStandardComponents.RequirePositive(
                command.Near,
                "Camera near clip"
            );
            float far = RequireFinite(command.Far, "Camera far clip");
            if (far <= near)
            {
                throw Invalid("Camera far clip must be greater than its near clip.");
            }

            camera.nearClipPlane = near;
            camera.farClipPlane = far;
            return null;
        }

        public static IBattlementCommandOperation? SetClear(
            CommandBody.Camera.SetClear command,
            BattlementWorld world
        )
        {
            Camera camera = RequireComponent<Camera>(command.ObjectId, world);
            bool needsColor = command.ClearMode == CameraClearMode.SolidColor;
            if (needsColor != command.ClearColor.HasValue)
            {
                throw Invalid("Camera clear color must be present only for solid-color clearing.");
            }

            CameraClearFlags flags = command.ClearMode switch
            {
                CameraClearMode.Skybox => CameraClearFlags.Skybox,
                CameraClearMode.SolidColor => CameraClearFlags.SolidColor,
                CameraClearMode.Depth => CameraClearFlags.Depth,
                CameraClearMode.Nothing => CameraClearFlags.Nothing,
                _ => throw Invalid("Camera clear mode is unknown."),
            };
            UnityEngine.Color? color = command.ClearColor is Color value
                ? BattlementStandardComponents.ConvertColor(value, "Camera clear color")
                : null;
            camera.clearFlags = flags;
            if (color is UnityEngine.Color converted)
            {
                camera.backgroundColor = converted;
            }

            return null;
        }

        public static IBattlementCommandOperation? SetLightEnabled(
            CommandBody.Light.SetEnabled command,
            BattlementWorld world
        )
        {
            RequireComponent<Light>(command.ObjectId, world).enabled = command.IsEnabled;
            return null;
        }

        public static IBattlementCommandOperation? SetLightType(
            CommandBody.Light.SetType command,
            BattlementWorld world
        )
        {
            Light light = RequireComponent<Light>(command.ObjectId, world);
            light.type = command.Type switch
            {
                LightType.Directional => UnityEngine.LightType.Directional,
                LightType.Point => UnityEngine.LightType.Point,
                LightType.Spot => UnityEngine.LightType.Spot,
                _ => throw Invalid("Light type is unknown."),
            };
            return null;
        }

        public static IBattlementCommandOperation? SetLightColor(
            CommandBody.Light.SetColor command,
            BattlementWorld world
        )
        {
            RequireComponent<Light>(command.ObjectId, world).color =
                BattlementStandardComponents.ConvertColor(command.Color, "Light color");
            return null;
        }

        public static IBattlementCommandOperation? TweenLightColor(
            CommandBody.Light.TweenColor command,
            BattlementWorld world,
            BattlementTweenAdapter tweens,
            TimeSpan now
        )
        {
            Light light = RequireComponent<Light>(command.ObjectId, world);
            UnityEngine.Color color = BattlementStandardComponents.ConvertColor(
                command.Color,
                "Light color"
            );
            return tweens.Color(
                light.transform,
                light.color,
                color,
                command.Tween,
                now,
                value => light.color = value
            );
        }

        public static IBattlementCommandOperation? SetLightIntensity(
            CommandBody.Light.SetIntensity command,
            BattlementWorld world
        )
        {
            RequireComponent<Light>(command.ObjectId, world).intensity =
                BattlementStandardComponents.RequireNonnegative(
                    command.Intensity,
                    "Light intensity"
                );
            return null;
        }

        public static IBattlementCommandOperation? TweenLightIntensity(
            CommandBody.Light.TweenIntensity command,
            BattlementWorld world,
            BattlementTweenAdapter tweens,
            TimeSpan now
        )
        {
            Light light = RequireComponent<Light>(command.ObjectId, world);
            float intensity = BattlementStandardComponents.RequireNonnegative(
                command.Intensity,
                "Light intensity"
            );
            return tweens.Float(
                light.transform,
                light.intensity,
                intensity,
                command.Tween,
                now,
                value => light.intensity = value
            );
        }

        public static IBattlementCommandOperation? SetLightRange(
            CommandBody.Light.SetRange command,
            BattlementWorld world
        )
        {
            Light light = RequireComponent<Light>(command.ObjectId, world);
            if (light.type is not UnityEngine.LightType.Point and not UnityEngine.LightType.Spot)
            {
                throw Invalid("Light range is valid only for point and spot lights.");
            }

            light.range = BattlementStandardComponents.RequirePositive(
                command.Range,
                "Light range"
            );
            return null;
        }

        public static IBattlementCommandOperation? SetSpotAngle(
            CommandBody.Light.SetSpotAngle command,
            BattlementWorld world
        )
        {
            Light light = RequireComponent<Light>(command.ObjectId, world);
            if (light.type != UnityEngine.LightType.Spot)
            {
                throw Invalid("Spot angles are valid only for spot lights.");
            }

            float outer = BattlementStandardComponents.RequireRange(
                command.OuterSpotAngle,
                0,
                179,
                "Light outer spot angle"
            );
            float inner = BattlementStandardComponents.RequireNonnegative(
                command.InnerSpotAngle,
                "Light inner spot angle"
            );
            if (inner > outer)
            {
                throw Invalid("Light inner spot angle cannot exceed its outer angle.");
            }

            light.spotAngle = outer;
            light.innerSpotAngle = inner;
            return null;
        }

        public static IBattlementCommandOperation? SetShadows(
            CommandBody.Light.SetShadows command,
            BattlementWorld world
        )
        {
            Light light = RequireComponent<Light>(command.ObjectId, world);
            light.shadows = command.Shadows switch
            {
                ShadowMode.None => LightShadows.None,
                ShadowMode.Hard => LightShadows.Hard,
                ShadowMode.Soft => LightShadows.Soft,
                _ => throw Invalid("Light shadow mode is unknown."),
            };
            return null;
        }

        private static T RequireComponent<T>(ObjectId objectId, BattlementWorld world)
            where T : Component
        {
            T[] components = world.RequireObject(objectId).GetComponents<T>();
            if (components.Length == 1)
            {
                return components[0];
            }

            throw new BattlementWorldException(
                components.Length == 0
                    ? CoreErrorCode.ComponentMissing
                    : CoreErrorCode.InvalidComponentCount,
                $"Command requires exactly one root {typeof(T).Name}; found {components.Length}."
            );
        }

        private static float RequireFinite(double value, string name)
        {
            float converted = (float)value;
            if (!double.IsFinite(value) || !float.IsFinite(converted))
            {
                throw Invalid($"{name} must be finite.");
            }

            return converted;
        }

        private static BattlementWorldException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);
    }
}
