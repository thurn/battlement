#nullable enable

using System;
using Battlement.UI;
using UnityEngine;
using UnityQuaternion = UnityEngine.Quaternion;
using UnityVector3 = UnityEngine.Vector3;

namespace Battlement
{
    /// <summary>Composes transform placement and local interaction channels.</summary>
    internal sealed class BattlementWorldMotionTarget : IBattlementMotionTarget
    {
        private readonly Transform transform;
        private UnityVector3 position;
        private UnityVector3 rotation;
        private UnityVector3 scale;
        private UnityVector3 offset;
        private UnityVector3 tilt;
        private UnityVector3 scaleFactor = UnityVector3.one;
        private UnityVector3 displayedPosition;
        private UnityQuaternion displayedRotation;
        private UnityVector3 displayedScale;

        public BattlementWorldMotionTarget(Transform transform)
        {
            this.transform = transform;
            position = transform.localPosition;
            rotation = transform.localEulerAngles;
            scale = transform.localScale;
            RememberPresentation();
        }

        internal static void Validate(ObjectId host, MotionDescriptor? descriptor)
        {
            if (descriptor is null)
                return;
            BattlementMotionValidator.Validate(descriptor, host);
            BattlementMotionDescriptorValidator.ValidateCapabilities(
                descriptor,
                SupportsWorld,
                false
            );
            BattlementMotionGraph.ValidateDescriptor(descriptor, SupportsWorld);
            foreach (
                MotionValueBinding binding in descriptor.ValueBindings
                    ?? Array.Empty<MotionValueBinding>()
            )
                if (binding.Composition == MotionBindingComposition.Compose)
                    throw Invalid(
                        "World Motion composes interaction through its local offset channels."
                    );
        }

        public bool Supports(MotionProperty property) => SupportsWorld(property);

        private static bool SupportsWorld(MotionProperty property) =>
            property >= MotionProperty.LocalPositionX
            && property <= MotionProperty.LocalScaleFactorZ;

        public bool IsLayout(MotionProperty property) => false;

        public bool IsSpatial(MotionProperty property) => Supports(property);

        public MotionValue Read(MotionProperty property)
        {
            Require(property);
            if (transform != null)
                SynchronizePresentation();
            int channel = (int)property - (int)MotionProperty.LocalPositionX;
            int group = channel / 3;
            UnityVector3 value = group switch
            {
                0 => position,
                1 => rotation,
                2 => scale,
                3 => offset,
                4 => tilt,
                5 => scaleFactor,
                _ => throw Invalid("Unknown transform Motion channel."),
            };
            return new MotionValue.Scalar(value[channel % 3]);
        }

        public void Write(MotionProperty property, MotionValue value)
        {
            if (value is not MotionValue.Scalar scalar)
                throw Invalid("Transform Motion channels require scalar values.");
            WriteScalar(property, scalar.Value);
        }

        public void WriteScalar(MotionProperty property, double value)
        {
            Require(property);
            SynchronizePresentation();
            float number = checked((float)value);
            if (!float.IsFinite(number))
                throw Invalid("Transform Motion channels must remain finite.");
            int channel = (int)property - (int)MotionProperty.LocalPositionX;
            int axis = channel % 3;
            switch (channel / 3)
            {
                case 0:
                    position[axis] = number;
                    break;
                case 1:
                    rotation[axis] = number;
                    break;
                case 2:
                    scale[axis] = number;
                    break;
                case 3:
                    offset[axis] = number;
                    break;
                case 4:
                    tilt[axis] = number;
                    break;
                case 5:
                    scaleFactor[axis] = number;
                    break;
                default:
                    throw Invalid("Unknown transform Motion channel.");
            }
            UnityQuaternion placement = UnityQuaternion.Euler(rotation);
            transform.SetLocalPositionAndRotation(
                position + placement * UnityVector3.Scale(scale, offset),
                placement * UnityQuaternion.Euler(tilt)
            );
            transform.localScale = UnityVector3.Scale(scale, scaleFactor);
            RememberPresentation();
        }

        public void WriteAdaptedScalar(MotionProperty property, double value) =>
            WriteScalar(property, value);

        public void SetContribution(MotionProperty property, MotionValue value) =>
            throw Invalid(
                "World Motion uses explicit local offset, tilt, and scale-factor channels."
            );

        public void RemoveContribution(MotionProperty property) { }

        public bool Contains(IBattlementMotionTarget target) =>
            target is BattlementWorldMotionTarget world && world.transform.IsChildOf(transform);

        public bool IsParentOf(IBattlementMotionTarget target) =>
            target is BattlementWorldMotionTarget world && world.transform.parent == transform;

        public void Release() { }

        internal void CapturePresentation()
        {
            if (transform != null)
                SynchronizePresentation();
        }

        private void SynchronizePresentation()
        {
            if (transform == null)
                throw Invalid("The Motion transform no longer exists.");
            if (transform.localRotation != displayedRotation)
                rotation = (
                    transform.localRotation * UnityQuaternion.Inverse(UnityQuaternion.Euler(tilt))
                ).eulerAngles;
            if (transform.localScale != displayedScale)
            {
                for (int axis = 0; axis < 3; axis++)
                    if (scaleFactor[axis] != 0)
                        scale[axis] = transform.localScale[axis] / scaleFactor[axis];
            }
            if (transform.localPosition != displayedPosition)
                position =
                    transform.localPosition
                    - UnityQuaternion.Euler(rotation) * UnityVector3.Scale(scale, offset);
        }

        private void RememberPresentation()
        {
            displayedPosition = transform.localPosition;
            displayedRotation = transform.localRotation;
            displayedScale = transform.localScale;
        }

        private void Require(MotionProperty property)
        {
            if (!Supports(property))
                throw Invalid($"Motion property {property} has no world transform writer.");
        }

        private static BattlementUiException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);
    }
}
