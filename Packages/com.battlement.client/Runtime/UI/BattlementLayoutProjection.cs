#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal interface IBattlementLayoutProjection
    {
        MotionLayoutDescriptor Descriptor { get; }
        BattlementLayoutDomain Domain { get; }
        ViewportRect VisibleBounds { get; }
        bool IsComplete { get; }
        bool IsPaused { get; }
        void CaptureDestination();
        void Sample(ulong clockMicros, bool reducedMotion = false);
        void Pause(ulong clockMicros);
        void Resume(ulong clockMicros);
        void Release();
        void Abort();
    }

    internal interface IBattlementLayoutProjectionTarget
    {
        BattlementLayoutDomain Domain { get; }
        ViewportRect VisibleBounds(MotionLayoutDescriptor descriptor);
        IBattlementLayoutProjection CreateProjection(
            MotionLayoutDescriptor descriptor,
            BattlementLayoutOrigin origin,
            ulong anchorMicros
        );
    }

    internal sealed class BattlementUiLayoutProjectionTarget : IBattlementLayoutProjectionTarget
    {
        private readonly VisualElement target;
        private readonly Func<VisualElement, BattlementUiProjectionSpace> projectionSpace;

        public BattlementUiLayoutProjectionTarget(
            VisualElement target,
            Func<VisualElement, BattlementUiProjectionSpace> projectionSpace
        ) => (this.target, this.projectionSpace) = (target, projectionSpace);

        public BattlementLayoutDomain Domain => BattlementLayoutDomain.Ui;

        public ViewportRect VisibleBounds(MotionLayoutDescriptor descriptor) =>
            target.panel is null
                ? new ViewportRect(0, 0, 0, 0, new DisplayId(0))
                : projectionSpace(target).ToViewport(target.worldBound);

        public IBattlementLayoutProjection CreateProjection(
            MotionLayoutDescriptor descriptor,
            BattlementLayoutOrigin origin,
            ulong anchorMicros
        ) =>
            new BattlementLayoutProjection(
                target,
                projectionSpace,
                descriptor,
                origin,
                anchorMicros
            );
    }

    internal sealed class BattlementLayoutProjection : IBattlementLayoutProjection
    {
        private readonly VisualElement target;
        private readonly Func<VisualElement, BattlementUiProjectionSpace> projectionSpace;
        private readonly MotionLayoutDescriptor descriptor;
        private ulong anchorMicros;
        private ulong pausedAtMicros;
        private bool paused;
        private readonly ViewportRect originViewport;
        private UnityEngine.Rect origin;
        private readonly Dictionary<VisualElement, ScaleCorrectionState> childCorrections = new();
        private UnityEngine.Rect destination;
        private Vector2 lastTranslation;
        private Vector2 lastScale = Vector2.one;
        private Vector2 lastPresentedTranslation;
        private Vector2 lastPresentedScale;
        private bool captured;
        private bool completed;
        private bool popped;
        private StyleEnum<Position> priorPosition;
        private StyleLength priorLeft;
        private StyleLength priorTop;
        private StyleLength priorWidth;
        private StyleLength priorHeight;

        public BattlementLayoutProjection(
            VisualElement target,
            Func<VisualElement, BattlementUiProjectionSpace> projectionSpace,
            MotionLayoutDescriptor descriptor,
            BattlementLayoutOrigin origin,
            ulong anchorMicros
        )
        {
            this.target = target;
            this.projectionSpace = projectionSpace;
            this.descriptor = descriptor;
            originViewport = origin.Bounds;
            this.anchorMicros = anchorMicros;
            lastPresentedTranslation = Translation(target.resolvedStyle.translate);
            lastPresentedScale = target.resolvedStyle.scale.value;
            if (descriptor.PopLayout)
                PopFromLayout();
        }

        public MotionLayoutDescriptor Descriptor => descriptor;

        public BattlementLayoutDomain Domain => BattlementLayoutDomain.Ui;

        public ViewportRect VisibleBounds =>
            target.panel is null
                ? originViewport
                : projectionSpace(target).ToViewport(target.worldBound);

        public bool IsComplete => completed;

        public bool IsPaused => paused;

        public void CaptureDestination()
        {
            if (captured)
                return;
            destination = target.worldBound;
            origin = Valid(originViewport)
                ? projectionSpace(target).FromViewport(originViewport)
                : destination;
            captured = Valid(origin) && Valid(destination);
            if (!captured || Approximately(origin, destination))
                completed = true;
        }

        public void Sample(ulong clockMicros, bool reducedMotion = false)
        {
            if (!captured || completed)
                return;
            if (reducedMotion)
            {
                Apply(Vector2.zero, Vector2.one);
                completed = true;
                return;
            }
            ulong sampleMicros = paused ? pausedAtMicros : clockMicros;
            MotionScalarSample progress = BattlementMotionScalarSampler.Sample(
                0,
                1,
                0,
                descriptor.Transition,
                sampleMicros >= anchorMicros ? sampleMicros - anchorMicros : 0
            );
            (Vector2 translation, Vector2 scale) = Resolve(
                origin,
                destination,
                descriptor.Mode,
                checked((float)progress.Value)
            );
            Apply(translation, scale);
            if (!progress.Done)
                return;
            Apply(Vector2.zero, Vector2.one);
            completed = true;
        }

        public void Pause(ulong clockMicros)
        {
            if (completed || paused)
                return;
            pausedAtMicros = clockMicros;
            paused = true;
        }

        public void Resume(ulong clockMicros)
        {
            if (!paused)
                return;
            anchorMicros = checked(anchorMicros + clockMicros - pausedAtMicros);
            paused = false;
        }

        public void Release()
        {
            Apply(Vector2.zero, Vector2.one);
            completed = true;
        }

        public void Abort()
        {
            Release();
            if (!popped)
                return;
            target.style.position = priorPosition;
            target.style.left = priorLeft;
            target.style.top = priorTop;
            target.style.width = priorWidth;
            target.style.height = priorHeight;
            popped = false;
        }

        private void Apply(Vector2 translation, Vector2 scale)
        {
            Translate currentTranslation = target.resolvedStyle.translate;
            Scale currentScale = target.resolvedStyle.scale;
            Vector2 current = Translation(currentTranslation);
            Vector2 currentScaleValue = currentScale.value;
            Vector2 baseTranslation = Approximately(current, lastPresentedTranslation)
                ? current - lastTranslation
                : current;
            Vector2 baseScale = Approximately(currentScaleValue, lastPresentedScale)
                ? Divide(currentScaleValue, lastScale)
                : currentScaleValue;
            Vector2 presentedTranslation = baseTranslation + translation;
            Vector2 presentedScale = Multiply(baseScale, scale);
            target.style.translate = new Translate(
                presentedTranslation.x,
                presentedTranslation.y,
                currentTranslation.z
            );
            target.style.scale = new Scale(presentedScale);
            CorrectChildren(scale);
            lastTranslation = translation;
            lastScale = scale;
            lastPresentedTranslation = presentedTranslation;
            lastPresentedScale = presentedScale;
        }

        private void CorrectChildren(Vector2 scale)
        {
            foreach (VisualElement child in target.Children())
            {
                Vector2 current = child.resolvedStyle.scale.value;
                ScaleCorrectionState prior = childCorrections.TryGetValue(
                    child,
                    out ScaleCorrectionState value
                )
                    ? value
                    : new ScaleCorrectionState(Vector2.one, current);
                Vector2 authored = Approximately(current, prior.Presented)
                    ? Divide(current, prior.Correction)
                    : current;
                Vector2 correction = Divide(Vector2.one, scale);
                Vector2 presented = Multiply(authored, correction);
                child.style.scale = new Scale(presented);
                childCorrections[child] = new ScaleCorrectionState(correction, presented);
            }
        }

        private void PopFromLayout()
        {
            UnityEngine.Rect local = target.layout;
            if (!Valid(local))
                return;
            priorPosition = target.style.position;
            priorLeft = target.style.left;
            priorTop = target.style.top;
            priorWidth = target.style.width;
            priorHeight = target.style.height;
            target.style.position = Position.Absolute;
            target.style.left = local.x;
            target.style.top = local.y;
            target.style.width = local.width;
            target.style.height = local.height;
            popped = true;
        }

        internal static (Vector2 Translation, Vector2 Scale) Resolve(
            UnityEngine.Rect origin,
            UnityEngine.Rect destination,
            MotionLayoutMode mode,
            float progress
        )
        {
            float remaining = 1 - progress;
            Vector2 translation = mode switch
            {
                MotionLayoutMode.Position => new Vector2(
                    origin.x - destination.x,
                    origin.y - destination.y
                ) * remaining,
                MotionLayoutMode.Size => Vector2.zero,
                MotionLayoutMode.Both => (origin.center - destination.center) * remaining,
                _ => throw new ArgumentOutOfRangeException(nameof(mode)),
            };
            Vector2 scale =
                mode == MotionLayoutMode.Position
                    ? Vector2.one
                    : new Vector2(
                        Mathf.Lerp(origin.width / destination.width, 1, progress),
                        Mathf.Lerp(origin.height / destination.height, 1, progress)
                    );
            return (translation, scale);
        }

        internal static UnityEngine.Rect ProjectedBounds(
            UnityEngine.Rect origin,
            UnityEngine.Rect destination,
            MotionLayoutMode mode,
            float progress
        )
        {
            (Vector2 translation, Vector2 scale) = Resolve(origin, destination, mode, progress);
            Vector2 size = Multiply(destination.size, scale);
            Vector2 center = destination.center + translation;
            return new UnityEngine.Rect(center - size / 2, size);
        }

        internal static Vector2 ComposeScaleCorrection(
            Vector2 current,
            Vector2 priorCorrection,
            Vector2 priorPresented,
            Vector2 parentScale
        )
        {
            Vector2 authored = Approximately(current, priorPresented)
                ? Divide(current, priorCorrection)
                : current;
            return Multiply(authored, Divide(Vector2.one, parentScale));
        }

        private static bool Valid(UnityEngine.Rect value) =>
            float.IsFinite(value.x)
            && float.IsFinite(value.y)
            && float.IsFinite(value.width)
            && float.IsFinite(value.height)
            && value.width > 0.001f
            && value.height > 0.001f;

        private static bool Valid(ViewportRect value) =>
            double.IsFinite(value.X)
            && double.IsFinite(value.Y)
            && double.IsFinite(value.Width)
            && double.IsFinite(value.Height)
            && value.Width > 0.001
            && value.Height > 0.001;

        private static bool Approximately(UnityEngine.Rect left, UnityEngine.Rect right) =>
            Mathf.Abs(left.x - right.x) < 0.01f
            && Mathf.Abs(left.y - right.y) < 0.01f
            && Mathf.Abs(left.width - right.width) < 0.01f
            && Mathf.Abs(left.height - right.height) < 0.01f;

        private static bool Approximately(Vector2 left, Vector2 right) =>
            Mathf.Abs(left.x - right.x) < 0.0001f && Mathf.Abs(left.y - right.y) < 0.0001f;

        private static Vector2 Divide(Vector2 left, Vector2 right) =>
            new(left.x / right.x, left.y / right.y);

        private static Vector2 Multiply(Vector2 left, Vector2 right) =>
            new(left.x * right.x, left.y * right.y);

        private static Vector2 Translation(Translate value) => new(value.x.value, value.y.value);

        private readonly struct ScaleCorrectionState
        {
            public ScaleCorrectionState(Vector2 correction, Vector2 presented) =>
                (Correction, Presented) = (correction, presented);

            public Vector2 Correction { get; }

            public Vector2 Presented { get; }
        }
    }
}
