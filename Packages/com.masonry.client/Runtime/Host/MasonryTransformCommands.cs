#nullable enable

using System;
using UnityEngine;
using ProtocolQuaternion = Masonry.Quaternion;
using ProtocolVector3 = Masonry.Vector3;

namespace Masonry
{
    internal static class MasonryTransformCommands
    {
        public static IMasonryCommandOperation? SetLocalPosition(
            CommandBody.Transform.SetLocalPosition command,
            MasonryWorld world
        )
        {
            world.RequireObject(command.ObjectId).transform.localPosition = ToUnity(
                command.Position
            );
            return null;
        }

        public static IMasonryCommandOperation? SetWorldPosition(
            CommandBody.Transform.SetWorldPosition command,
            MasonryWorld world
        )
        {
            world.RequireObject(command.ObjectId).transform.position = ToUnity(
                command.Position,
                "World position"
            );
            return null;
        }

        public static IMasonryCommandOperation? TweenLocalPosition(
            CommandBody.Transform.TweenLocalPosition command,
            MasonryWorld world,
            MasonryTweenAdapter tweens,
            TimeSpan now
        )
        {
            Transform target = world.RequireObject(command.ObjectId).transform;
            return tweens.Vector(
                target,
                target.localPosition,
                ToUnity(command.Position, "Local position"),
                command.Tween,
                now,
                (item, value) => item.localPosition = value
            );
        }

        public static IMasonryCommandOperation? TweenWorldPosition(
            CommandBody.Transform.TweenWorldPosition command,
            MasonryWorld world,
            MasonryTweenAdapter tweens,
            TimeSpan now
        )
        {
            Transform target = world.RequireObject(command.ObjectId).transform;
            return tweens.Vector(
                target,
                target.position,
                ToUnity(command.Position, "World position"),
                command.Tween,
                now,
                (item, value) => item.position = value
            );
        }

        public static IMasonryCommandOperation? TweenLocalRotation(
            CommandBody.Transform.TweenLocalRotation command,
            MasonryWorld world,
            MasonryTweenAdapter tweens,
            TimeSpan now
        )
        {
            Transform target = RequireRotationTarget(command.ObjectId, world);
            return tweens.Rotation(
                target,
                target.localRotation,
                ToUnity(command.Rotation),
                command.Tween,
                now,
                (item, value) => item.localRotation = value
            );
        }

        public static IMasonryCommandOperation? TweenWorldRotation(
            CommandBody.Transform.TweenWorldRotation command,
            MasonryWorld world,
            MasonryTweenAdapter tweens,
            TimeSpan now
        )
        {
            Transform target = RequireRotationTarget(command.ObjectId, world);
            return tweens.Rotation(
                target,
                target.rotation,
                ToUnity(command.Rotation),
                command.Tween,
                now,
                (item, value) => item.rotation = value
            );
        }

        public static IMasonryCommandOperation? SetLocalRotation(
            CommandBody.Transform.SetLocalRotation command,
            MasonryWorld world
        )
        {
            RequireRotationTarget(command.ObjectId, world).localRotation = ToUnity(
                command.Rotation
            );
            return null;
        }

        public static IMasonryCommandOperation? SetWorldRotation(
            CommandBody.Transform.SetWorldRotation command,
            MasonryWorld world
        )
        {
            RequireRotationTarget(command.ObjectId, world).rotation = ToUnity(command.Rotation);
            return null;
        }

        public static IMasonryCommandOperation? SetLocalScale(
            CommandBody.Transform.SetLocalScale command,
            MasonryWorld world
        )
        {
            world.RequireObject(command.ObjectId).transform.localScale = ToUnity(
                command.Scale,
                "Local scale"
            );
            return null;
        }

        public static IMasonryCommandOperation? TweenLocalScale(
            CommandBody.Transform.TweenLocalScale command,
            MasonryWorld world,
            MasonryTweenAdapter tweens,
            TimeSpan now
        )
        {
            Transform target = world.RequireObject(command.ObjectId).transform;
            return tweens.Vector(
                target,
                target.localScale,
                ToUnity(command.Scale, "Local scale"),
                command.Tween,
                now,
                (item, value) => item.localScale = value
            );
        }

        private static UnityEngine.Vector3 ToUnity(ProtocolVector3 value) =>
            ToUnity(value, "Local position");

        private static Transform RequireRotationTarget(ObjectId objectId, MasonryWorld world)
        {
            GameObject target = world.RequireObject(objectId);
            if (target.TryGetComponent(out MasonryImage image) && image.FacesCamera)
            {
                throw BillboardControlled(objectId);
            }
            if (target.TryGetComponent(out MasonryText text) && text.FacesCamera)
            {
                throw BillboardControlled(objectId);
            }

            return target.transform;
        }

        private static MasonryCommandException BillboardControlled(ObjectId objectId) =>
            new(
                CoreErrorCode.PropertyControlledByBillboard,
                $"Object {objectId} rotation is controlled by face-camera behavior."
            );

        private static UnityEngine.Vector3 ToUnity(ProtocolVector3 value, string name) =>
            new(
                RequireFinite(value.X, $"{name} X"),
                RequireFinite(value.Y, $"{name} Y"),
                RequireFinite(value.Z, $"{name} Z")
            );

        private static UnityEngine.Quaternion ToUnity(ProtocolQuaternion value)
        {
            var converted = new UnityEngine.Quaternion(
                RequireFinite(value.X, "Rotation X"),
                RequireFinite(value.Y, "Rotation Y"),
                RequireFinite(value.Z, "Rotation Z"),
                RequireFinite(value.W, "Rotation W")
            );
            float magnitude = Mathf.Sqrt(
                converted.x * converted.x
                    + converted.y * converted.y
                    + converted.z * converted.z
                    + converted.w * converted.w
            );
            if (magnitude <= 0f)
            {
                throw new MasonryCommandException(
                    CoreErrorCode.InvalidProperty,
                    "Rotation must have nonzero length."
                );
            }

            return new UnityEngine.Quaternion(
                converted.x / magnitude,
                converted.y / magnitude,
                converted.z / magnitude,
                converted.w / magnitude
            );
        }

        private static float RequireFinite(double value, string name)
        {
            float converted = (float)value;
            if (!double.IsFinite(value) || !float.IsFinite(converted))
            {
                throw new MasonryCommandException(
                    CoreErrorCode.InvalidProperty,
                    $"{name} must be finite."
                );
            }

            return converted;
        }
    }
}
