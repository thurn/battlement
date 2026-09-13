#nullable enable

using System;
using UnityEngine;

namespace Battlement
{
    internal static class BattlementCameraLightCommands
    {
        public static IBattlementCommandOperation? Launch(
            BattlementDirectComponentCommand command,
            BattlementWorld world,
            BattlementTweenAdapter tweens,
            TimeSpan now
        ) =>
            command.Kind switch
            {
                BattlementDirectComponentCommandKind.CameraSetEnabled => SetCameraEnabled(
                    command,
                    world
                ),
                BattlementDirectComponentCommandKind.CameraSetPerspective => SetPerspective(
                    command,
                    world
                ),
                BattlementDirectComponentCommandKind.CameraTweenFieldOfView => TweenFieldOfView(
                    command,
                    world,
                    tweens,
                    now
                ),
                BattlementDirectComponentCommandKind.CameraSetOrthographic => SetOrthographic(
                    command,
                    world
                ),
                BattlementDirectComponentCommandKind.CameraTweenOrthographicSize =>
                    TweenOrthographicSize(command, world, tweens, now),
                BattlementDirectComponentCommandKind.CameraSetClipping => SetClipping(
                    command,
                    world
                ),
                BattlementDirectComponentCommandKind.CameraSetClear => SetClear(command, world),
                BattlementDirectComponentCommandKind.LightSetEnabled => SetLightEnabled(
                    command,
                    world
                ),
                BattlementDirectComponentCommandKind.LightSetType => SetLightType(command, world),
                BattlementDirectComponentCommandKind.LightSetColor => SetLightColor(command, world),
                BattlementDirectComponentCommandKind.LightTweenColor => TweenLightColor(
                    command,
                    world,
                    tweens,
                    now
                ),
                BattlementDirectComponentCommandKind.LightSetIntensity => SetLightIntensity(
                    command,
                    world
                ),
                BattlementDirectComponentCommandKind.LightTweenIntensity => TweenLightIntensity(
                    command,
                    world,
                    tweens,
                    now
                ),
                BattlementDirectComponentCommandKind.LightSetRange => SetLightRange(command, world),
                BattlementDirectComponentCommandKind.LightSetSpotAngle => SetSpotAngle(
                    command,
                    world
                ),
                BattlementDirectComponentCommandKind.LightSetShadows => SetShadows(command, world),
                _ => throw Invalid("A camera or light command kind is unknown."),
            };

        private static IBattlementCommandOperation? SetCameraEnabled(
            BattlementDirectComponentCommand command,
            BattlementWorld world
        )
        {
            world.SetCameraEnabled(
                RequireComponent<Camera>(command.ObjectId, world),
                command.Enabled
            );
            return null;
        }

        private static IBattlementCommandOperation? SetPerspective(
            BattlementDirectComponentCommand command,
            BattlementWorld world
        )
        {
            Camera camera = RequireComponent<Camera>(command.ObjectId, world);
            camera.orthographic = false;
            camera.fieldOfView = BattlementStandardComponents.RequireRange(
                command.First,
                1,
                179,
                "Camera field of view"
            );
            return null;
        }

        private static IBattlementCommandOperation? TweenFieldOfView(
            BattlementDirectComponentCommand command,
            BattlementWorld world,
            BattlementTweenAdapter tweens,
            TimeSpan now
        )
        {
            Camera camera = RequireComponent<Camera>(command.ObjectId, world);
            if (camera.orthographic)
                throw Invalid("Camera must be perspective to tween its field of view.");
            float value = BattlementStandardComponents.RequireRange(
                command.First,
                1,
                179,
                "Camera field of view"
            );
            return tweens.Float(
                camera.transform,
                camera.fieldOfView,
                value,
                RequireTween(command),
                now,
                result => camera.fieldOfView = result
            );
        }

        private static IBattlementCommandOperation? SetOrthographic(
            BattlementDirectComponentCommand command,
            BattlementWorld world
        )
        {
            Camera camera = RequireComponent<Camera>(command.ObjectId, world);
            camera.orthographic = true;
            camera.orthographicSize = BattlementStandardComponents.RequirePositive(
                command.First,
                "Camera orthographic size"
            );
            return null;
        }

        private static IBattlementCommandOperation? TweenOrthographicSize(
            BattlementDirectComponentCommand command,
            BattlementWorld world,
            BattlementTweenAdapter tweens,
            TimeSpan now
        )
        {
            Camera camera = RequireComponent<Camera>(command.ObjectId, world);
            if (!camera.orthographic)
                throw Invalid("Camera must be orthographic to tween its size.");
            float value = BattlementStandardComponents.RequirePositive(
                command.First,
                "Camera orthographic size"
            );
            return tweens.Float(
                camera.transform,
                camera.orthographicSize,
                value,
                RequireTween(command),
                now,
                result => camera.orthographicSize = result
            );
        }

        private static IBattlementCommandOperation? SetClipping(
            BattlementDirectComponentCommand command,
            BattlementWorld world
        )
        {
            Camera camera = RequireComponent<Camera>(command.ObjectId, world);
            float near = BattlementStandardComponents.RequirePositive(
                command.First,
                "Camera near clip"
            );
            float far = RequireFinite(command.Second, "Camera far clip");
            if (far <= near)
                throw Invalid("Camera far clip must be greater than its near clip.");
            camera.nearClipPlane = near;
            camera.farClipPlane = far;
            return null;
        }

        private static IBattlementCommandOperation? SetClear(
            BattlementDirectComponentCommand command,
            BattlementWorld world
        )
        {
            bool needsColor = command.Option == 1;
            if (needsColor != command.HasValue)
                throw Invalid("Camera clear color must be present only for solid-color clearing.");
            Camera camera = RequireComponent<Camera>(command.ObjectId, world);
            camera.clearFlags = command.Option switch
            {
                0 => CameraClearFlags.Skybox,
                1 => CameraClearFlags.SolidColor,
                2 => CameraClearFlags.Depth,
                3 => CameraClearFlags.Nothing,
                _ => throw Invalid("Camera clear mode is unknown."),
            };
            if (command.HasValue)
                camera.backgroundColor = DirectColor(command, "Camera clear color");
            return null;
        }

        private static IBattlementCommandOperation? SetLightEnabled(
            BattlementDirectComponentCommand command,
            BattlementWorld world
        )
        {
            RequireComponent<Light>(command.ObjectId, world).enabled = command.Enabled;
            return null;
        }

        private static IBattlementCommandOperation? SetLightType(
            BattlementDirectComponentCommand command,
            BattlementWorld world
        )
        {
            RequireComponent<Light>(command.ObjectId, world).type = command.Option switch
            {
                0 => UnityEngine.LightType.Directional,
                1 => UnityEngine.LightType.Point,
                2 => UnityEngine.LightType.Spot,
                _ => throw Invalid("Light type is unknown."),
            };
            return null;
        }

        private static IBattlementCommandOperation? SetLightColor(
            BattlementDirectComponentCommand command,
            BattlementWorld world
        )
        {
            RequireComponent<Light>(command.ObjectId, world).color = DirectColor(
                command,
                "Light color"
            );
            return null;
        }

        private static IBattlementCommandOperation? TweenLightColor(
            BattlementDirectComponentCommand command,
            BattlementWorld world,
            BattlementTweenAdapter tweens,
            TimeSpan now
        )
        {
            Light light = RequireComponent<Light>(command.ObjectId, world);
            return tweens.Color(
                light.transform,
                light.color,
                DirectColor(command, "Light color"),
                RequireTween(command),
                now,
                value => light.color = value
            );
        }

        private static IBattlementCommandOperation? SetLightIntensity(
            BattlementDirectComponentCommand command,
            BattlementWorld world
        )
        {
            RequireComponent<Light>(command.ObjectId, world).intensity =
                BattlementStandardComponents.RequireNonnegative(command.First, "Light intensity");
            return null;
        }

        private static IBattlementCommandOperation? TweenLightIntensity(
            BattlementDirectComponentCommand command,
            BattlementWorld world,
            BattlementTweenAdapter tweens,
            TimeSpan now
        )
        {
            Light light = RequireComponent<Light>(command.ObjectId, world);
            float value = BattlementStandardComponents.RequireNonnegative(
                command.First,
                "Light intensity"
            );
            return tweens.Float(
                light.transform,
                light.intensity,
                value,
                RequireTween(command),
                now,
                result => light.intensity = result
            );
        }

        private static IBattlementCommandOperation? SetLightRange(
            BattlementDirectComponentCommand command,
            BattlementWorld world
        )
        {
            Light light = RequireComponent<Light>(command.ObjectId, world);
            if (light.type is not UnityEngine.LightType.Point and not UnityEngine.LightType.Spot)
                throw Invalid("Light range is valid only for point and spot lights.");
            light.range = BattlementStandardComponents.RequirePositive(
                command.First,
                "Light range"
            );
            return null;
        }

        private static IBattlementCommandOperation? SetSpotAngle(
            BattlementDirectComponentCommand command,
            BattlementWorld world
        )
        {
            Light light = RequireComponent<Light>(command.ObjectId, world);
            if (light.type != UnityEngine.LightType.Spot)
                throw Invalid("Spot angles are valid only for spot lights.");
            float outer = BattlementStandardComponents.RequireRange(
                command.First,
                0,
                179,
                "Light outer spot angle"
            );
            float inner = BattlementStandardComponents.RequireNonnegative(
                command.Second,
                "Light inner spot angle"
            );
            if (inner > outer)
                throw Invalid("Light inner spot angle cannot exceed its outer angle.");
            light.spotAngle = outer;
            light.innerSpotAngle = inner;
            return null;
        }

        private static IBattlementCommandOperation? SetShadows(
            BattlementDirectComponentCommand command,
            BattlementWorld world
        )
        {
            RequireComponent<Light>(command.ObjectId, world).shadows = command.Option switch
            {
                0 => LightShadows.None,
                1 => LightShadows.Hard,
                2 => LightShadows.Soft,
                _ => throw Invalid("Light shadow mode is unknown."),
            };
            return null;
        }

        private static UnityEngine.Color DirectColor(
            BattlementDirectComponentCommand command,
            string name
        ) =>
            BattlementStandardComponents.ConvertColor(
                command.First,
                command.Second,
                command.Third,
                command.Fourth,
                name
            );

        private static BattlementDirectTweenSettings RequireTween(
            BattlementDirectComponentCommand command
        ) => command.Tween ?? throw Invalid("A tween command has no tween settings.");

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
