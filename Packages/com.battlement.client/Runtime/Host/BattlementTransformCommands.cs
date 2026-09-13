#nullable enable

using System;
using UnityEngine;
using ProtocolQuaternion = Battlement.Quaternion;
using ProtocolVector3 = Battlement.Vector3;

namespace Battlement
{
    internal static class BattlementTransformCommands
    {
        public static IBattlementCommandOperation? SetLocalPosition(
            CommandBody.Transform.SetLocalPosition command,
            BattlementWorld world
        )
        {
            world.RequireObject(command.ObjectId).transform.localPosition = ToUnity(
                command.Position
            );
            return null;
        }

        public static IBattlementCommandOperation? SetLocalPosition(
            BattlementDirectLocalPosition command,
            BattlementWorld world
        )
        {
            world.RequireObject(command.ObjectId).transform.localPosition = new UnityEngine.Vector3(
                RequireFinite(command.X, "Local position X"),
                RequireFinite(command.Y, "Local position Y"),
                RequireFinite(command.Z, "Local position Z")
            );
            return null;
        }

        public static IBattlementCommandOperation? SetWorldPosition(
            CommandBody.Transform.SetWorldPosition command,
            BattlementWorld world
        )
        {
            world.RequireObject(command.ObjectId).transform.position = ToUnity(
                command.Position,
                "World position"
            );
            return null;
        }

        public static IBattlementCommandOperation? SetWorldPosition(
            BattlementDirectWorldPosition command,
            BattlementWorld world
        )
        {
            world.RequireObject(command.ObjectId).transform.position = new UnityEngine.Vector3(
                RequireFinite(command.X, "World position X"),
                RequireFinite(command.Y, "World position Y"),
                RequireFinite(command.Z, "World position Z")
            );
            return null;
        }

        public static IBattlementCommandOperation? TweenLocalPosition(
            CommandBody.Transform.TweenLocalPosition command,
            BattlementWorld world,
            BattlementTweenAdapter tweens,
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

        public static IBattlementCommandOperation? TweenLocalPosition(
            BattlementDirectTweenLocalPosition command,
            BattlementWorld world,
            BattlementTweenAdapter tweens,
            TimeSpan now
        )
        {
            Transform target = world.RequireObject(command.ObjectId).transform;
            UnityEngine.Vector3 start = command.World ? target.position : target.localPosition;
            return tweens.Vector(
                target,
                start,
                new UnityEngine.Vector3(
                    RequireFinite(command.X, "Position X"),
                    RequireFinite(command.Y, "Position Y"),
                    RequireFinite(command.Z, "Position Z")
                ),
                command.Tween,
                now,
                command.World
                    ? (item, value) => item.position = value
                    : (item, value) => item.localPosition = value
            );
        }

        public static IBattlementCommandOperation? TweenWorldPosition(
            CommandBody.Transform.TweenWorldPosition command,
            BattlementWorld world,
            BattlementTweenAdapter tweens,
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

        public static IBattlementCommandOperation? TweenLocalRotation(
            CommandBody.Transform.TweenLocalRotation command,
            BattlementWorld world,
            BattlementTweenAdapter tweens,
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

        public static IBattlementCommandOperation? TweenWorldRotation(
            CommandBody.Transform.TweenWorldRotation command,
            BattlementWorld world,
            BattlementTweenAdapter tweens,
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

        public static IBattlementCommandOperation? TweenRotation(
            BattlementDirectTweenRotation command,
            BattlementWorld world,
            BattlementTweenAdapter tweens,
            TimeSpan now
        )
        {
            Transform target = RequireRotationTarget(command.ObjectId, world);
            UnityEngine.Quaternion end = ToUnity(command.X, command.Y, command.Z, command.W);
            return tweens.Rotation(
                target,
                command.World ? target.rotation : target.localRotation,
                end,
                command.Tween,
                now,
                command.World
                    ? (item, value) => item.rotation = value
                    : (item, value) => item.localRotation = value
            );
        }

        public static IBattlementCommandOperation? SetLocalRotation(
            CommandBody.Transform.SetLocalRotation command,
            BattlementWorld world
        )
        {
            RequireRotationTarget(command.ObjectId, world).localRotation = ToUnity(
                command.Rotation
            );
            return null;
        }

        public static IBattlementCommandOperation? SetWorldRotation(
            CommandBody.Transform.SetWorldRotation command,
            BattlementWorld world
        )
        {
            RequireRotationTarget(command.ObjectId, world).rotation = ToUnity(command.Rotation);
            return null;
        }

        public static IBattlementCommandOperation? SetRotation(
            BattlementDirectRotation command,
            BattlementWorld world
        )
        {
            Transform target = RequireRotationTarget(command.ObjectId, world);
            UnityEngine.Quaternion value = ToUnity(command.X, command.Y, command.Z, command.W);
            if (command.World)
                target.rotation = value;
            else
                target.localRotation = value;
            return null;
        }

        public static IBattlementCommandOperation? SetLocalScale(
            CommandBody.Transform.SetLocalScale command,
            BattlementWorld world
        )
        {
            world.RequireObject(command.ObjectId).transform.localScale = ToUnity(
                command.Scale,
                "Local scale"
            );
            return null;
        }

        public static IBattlementCommandOperation? SetLocalScale(
            BattlementDirectScale command,
            BattlementWorld world
        )
        {
            world.RequireObject(command.ObjectId).transform.localScale = new UnityEngine.Vector3(
                RequireFinite(command.X, "Local scale X"),
                RequireFinite(command.Y, "Local scale Y"),
                RequireFinite(command.Z, "Local scale Z")
            );
            return null;
        }

        public static IBattlementCommandOperation? TweenLocalScale(
            CommandBody.Transform.TweenLocalScale command,
            BattlementWorld world,
            BattlementTweenAdapter tweens,
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

        public static IBattlementCommandOperation? TweenLocalScale(
            BattlementDirectTweenScale command,
            BattlementWorld world,
            BattlementTweenAdapter tweens,
            TimeSpan now
        )
        {
            Transform target = world.RequireObject(command.ObjectId).transform;
            return tweens.Vector(
                target,
                target.localScale,
                new UnityEngine.Vector3(
                    RequireFinite(command.X, "Local scale X"),
                    RequireFinite(command.Y, "Local scale Y"),
                    RequireFinite(command.Z, "Local scale Z")
                ),
                command.Tween,
                now,
                (item, value) => item.localScale = value
            );
        }

        private static UnityEngine.Vector3 ToUnity(ProtocolVector3 value) =>
            ToUnity(value, "Local position");

        private static Transform RequireRotationTarget(ObjectId objectId, BattlementWorld world)
        {
            GameObject target = world.RequireObject(objectId);
            if (target.TryGetComponent(out BattlementImage image) && image.FacesCamera)
            {
                throw BillboardControlled(objectId);
            }
            if (target.TryGetComponent(out BattlementText text) && text.FacesCamera)
            {
                throw BillboardControlled(objectId);
            }

            return target.transform;
        }

        private static BattlementCommandException BillboardControlled(ObjectId objectId) =>
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

        private static UnityEngine.Quaternion ToUnity(ProtocolQuaternion value) =>
            ToUnity(value.X, value.Y, value.Z, value.W);

        private static UnityEngine.Quaternion ToUnity(double x, double y, double z, double w)
        {
            var converted = new UnityEngine.Quaternion(
                RequireFinite(x, "Rotation X"),
                RequireFinite(y, "Rotation Y"),
                RequireFinite(z, "Rotation Z"),
                RequireFinite(w, "Rotation W")
            );
            float magnitude = Mathf.Sqrt(
                converted.x * converted.x
                    + converted.y * converted.y
                    + converted.z * converted.z
                    + converted.w * converted.w
            );
            if (magnitude <= 0f)
            {
                throw new BattlementCommandException(
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
                throw new BattlementCommandException(
                    CoreErrorCode.InvalidProperty,
                    $"{name} must be finite."
                );
            }

            return converted;
        }
    }
}
