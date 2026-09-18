#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using Battlement.UI;
using UnityEngine;
using UnityQuaternion = UnityEngine.Quaternion;
using UnityVector3 = UnityEngine.Vector3;

namespace Battlement
{
    internal interface IBattlementMotionAudio
    {
        bool HasMotionPlayback(ObjectId playbackId);
        float ReadMotionVolume(ObjectId playbackId);
        void WriteMotionVolume(ObjectId playbackId, double value);
    }

    /// <summary>Composes transform placement and local interaction channels.</summary>
    internal sealed class BattlementWorldMotionTarget : IBattlementMotionTarget
    {
        private readonly Transform transform;
        private readonly IBattlementMotionAudio? audioSources;
        private UnityVector3 position;
        private UnityVector3 rotation;
        private UnityVector3 scale;
        private UnityVector3 offset;
        private UnityVector3 tilt;
        private UnityVector3 scaleFactor = UnityVector3.one;
        private UnityVector3 displayedPosition;
        private UnityQuaternion displayedRotation;
        private UnityVector3 displayedScale;
        private BattlementMaterialInstances? materials;
        private MotionPropertyTarget.MaterialScalar? materialTarget;
        private Light? light;
        private ParticleSystem[] particles = Array.Empty<ParticleSystem>();
        private ObjectId? audioPlayback;
        private float lightOrigin;
        private float[] particleOrigins = Array.Empty<float>();
        private float audioOrigin;

        public BattlementWorldMotionTarget(
            Transform transform,
            IBattlementMotionAudio? audioSources = null
        )
        {
            this.transform = transform;
            this.audioSources = audioSources;
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
                SupportsCatalog,
                false
            );
            BattlementMotionGraph.ValidateDescriptor(descriptor, SupportsCatalog);
            foreach (
                MotionValueBinding binding in descriptor.ValueBindings
                    ?? Array.Empty<MotionValueBinding>()
            )
                if (binding.Composition == MotionBindingComposition.Compose)
                    throw Invalid(
                        "World Motion composes interaction through its local offset channels."
                    );
        }

        internal void Configure(MotionDescriptor descriptor)
        {
            MotionPropertyTarget.MaterialScalar? nextMaterial = null;
            ObjectId? nextAudio = null;
            bool usesLight = false;
            bool usesParticles = false;
            foreach (MotionPropertyTrack track in Tracks(descriptor))
            {
                switch (track.Target)
                {
                    case MotionPropertyTarget.Host:
                        break;
                    case MotionPropertyTarget.MaterialScalar target:
                        if (nextMaterial is not null && nextMaterial != target)
                            throw Invalid(
                                "One Motion host cannot target multiple material scalars."
                            );
                        nextMaterial = target;
                        break;
                    case MotionPropertyTarget.AudioVolume target:
                        if (nextAudio is ObjectId prior && prior != target.PlaybackId)
                            throw Invalid(
                                "One Motion host cannot target multiple audio playbacks."
                            );
                        nextAudio = target.PlaybackId;
                        break;
                    default:
                        throw Invalid("Unknown Motion property target.");
                }
                usesLight |= track.Property == MotionProperty.LightIntensity;
                usesParticles |= track.Property == MotionProperty.ParticleEmission;
            }

            if (nextMaterial != materialTarget)
                ReleaseMaterial();
            if (nextMaterial is not null)
            {
                if (
                    !transform.TryGetComponent(out BattlementMaterialInstances candidate)
                    || !candidate.SupportsMotionScalar(nextMaterial.Slot, nextMaterial.Parameter)
                )
                    throw Invalid("Material Motion requires the exact prepared float parameter.");
                materials = candidate;
                materialTarget = nextMaterial;
            }

            if (usesLight && light == null)
            {
                if (!transform.TryGetComponent(out Light candidate))
                    throw Invalid("Light-intensity Motion requires a Light host.");
                light = candidate;
                lightOrigin = candidate.intensity;
            }
            else if (!usesLight && light != null)
            {
                light.intensity = lightOrigin;
                light = null;
            }

            if (usesParticles && particles.Length == 0)
            {
                particles = transform.GetComponentsInChildren<ParticleSystem>(true);
                if (particles.Length == 0)
                    throw Invalid("Particle-emission Motion requires a ParticleSystem host.");
                particleOrigins = particles
                    .Select(system => system.emission.rateOverTimeMultiplier)
                    .ToArray();
            }
            else if (!usesParticles && particles.Length != 0)
            {
                RestoreParticles();
                particles = Array.Empty<ParticleSystem>();
                particleOrigins = Array.Empty<float>();
            }

            if (nextAudio != audioPlayback)
            {
                RestoreAudio();
                if (nextAudio is ObjectId playback)
                {
                    if (audioSources?.HasMotionPlayback(playback) != true)
                        throw Invalid(
                            "Audio-volume Motion requires a live Battlement audio playback."
                        );
                    audioPlayback = playback;
                    audioOrigin = audioSources.ReadMotionVolume(playback);
                }
            }
        }

        public bool Supports(MotionProperty property) =>
            SupportsTransform(property)
            || (property == MotionProperty.MaterialScalar && materialTarget is not null)
            || (property == MotionProperty.LightIntensity && light != null)
            || (property == MotionProperty.ParticleEmission && particles.Length != 0)
            || (property == MotionProperty.AudioVolume && audioPlayback is not null);

        private static bool SupportsTransform(MotionProperty property) =>
            property >= MotionProperty.LocalPositionX
            && property <= MotionProperty.LocalScaleFactorZ;

        private static bool SupportsCatalog(MotionProperty property) =>
            SupportsTransform(property)
            || property
                is MotionProperty.MaterialScalar
                    or MotionProperty.LightIntensity
                    or MotionProperty.ParticleEmission
                    or MotionProperty.AudioVolume;

        public bool IsLayout(MotionProperty property) => false;

        public bool IsSpatial(MotionProperty property) => SupportsTransform(property);

        public MotionValue Read(MotionProperty property)
        {
            Require(property);
            if (property == MotionProperty.MaterialScalar)
                return new MotionValue.Scalar(
                    materials!.ReadMotionScalar(materialTarget!.Slot, materialTarget.Parameter)
                );
            if (property == MotionProperty.LightIntensity)
                return new MotionValue.Scalar(light!.intensity);
            if (property == MotionProperty.ParticleEmission)
                return new MotionValue.Scalar(particles[0].emission.rateOverTimeMultiplier);
            if (property == MotionProperty.AudioVolume)
                return new MotionValue.Scalar(audioSources!.ReadMotionVolume(audioPlayback!.Value));
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
            float number = checked((float)value);
            if (!float.IsFinite(number))
                throw Invalid("World Motion channels must remain finite.");
            if (property == MotionProperty.MaterialScalar)
            {
                materials!.WriteMotionScalar(
                    materialTarget!.Slot,
                    materialTarget.Parameter,
                    number
                );
                return;
            }
            if (property == MotionProperty.LightIntensity)
            {
                light!.intensity = Math.Max(0, number);
                return;
            }
            if (property == MotionProperty.ParticleEmission)
            {
                foreach (ParticleSystem system in particles)
                {
                    ParticleSystem.EmissionModule emission = system.emission;
                    emission.rateOverTimeMultiplier = Math.Max(0, number);
                }
                return;
            }
            if (property == MotionProperty.AudioVolume)
            {
                audioSources!.WriteMotionVolume(audioPlayback!.Value, number);
                return;
            }
            SynchronizePresentation();
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

        public void Release()
        {
            ReleaseMaterial();
            if (light != null)
                light.intensity = lightOrigin;
            RestoreParticles();
            RestoreAudio();
            light = null;
            particles = Array.Empty<ParticleSystem>();
            particleOrigins = Array.Empty<float>();
        }

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

        private static IEnumerable<MotionPropertyTrack> Tracks(MotionDescriptor descriptor) =>
            (descriptor.Initial?.Tracks ?? Array.Empty<MotionPropertyTrack>())
                .Concat(descriptor.Slots.SelectMany(slot => slot.Target.Tracks))
                .Concat(
                    (descriptor.NamedTargets ?? Array.Empty<MotionNamedTarget>()).SelectMany(
                        target => target.Target.Tracks
                    )
                );

        private void ReleaseMaterial()
        {
            if (materials != null && materialTarget is not null)
                materials.ClearMotionScalar(materialTarget.Slot, materialTarget.Parameter);
            materials = null;
            materialTarget = null;
        }

        private void RestoreParticles()
        {
            for (int index = 0; index < Math.Min(particles.Length, particleOrigins.Length); index++)
            {
                if (particles[index] == null)
                    continue;
                ParticleSystem.EmissionModule emission = particles[index].emission;
                emission.rateOverTimeMultiplier = particleOrigins[index];
            }
        }

        private void RestoreAudio()
        {
            if (
                audioPlayback is ObjectId playback
                && audioSources?.HasMotionPlayback(playback) == true
            )
                audioSources.WriteMotionVolume(playback, audioOrigin);
            audioPlayback = null;
        }
    }
}
