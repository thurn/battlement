#nullable enable

using System;
using System.Linq;
using UnityEngine;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal sealed class BattlementGestureState : IDisposable
    {
        private readonly MotionDescriptor descriptor;
        private readonly MotionGestureDescriptor gestures;
        private readonly VisualElement target;
        private readonly Func<ObjectId, VisualElement?> resolve;
        private readonly Func<TimeSpan> now;
        private readonly Func<bool> reducedMotion;
        private readonly Action<MotionLayer, bool> setLayer;
        private readonly Action<ObjectId, MotionValue> setValue;
        private readonly Action<MotionGestureEvent, bool> emit;
        private readonly EventCallback<PointerEnterEvent> pointerEnter;
        private readonly EventCallback<PointerLeaveEvent> pointerLeave;
        private readonly EventCallback<PointerDownEvent> pointerDown;
        private readonly EventCallback<PointerMoveEvent> pointerMove;
        private readonly EventCallback<PointerUpEvent> pointerUp;
        private readonly EventCallback<PointerCancelEvent> pointerCancel;
        private readonly EventCallback<PointerCaptureOutEvent> captureOut;
        private readonly EventCallback<FocusInEvent> focusIn;
        private readonly EventCallback<FocusOutEvent> focusOut;
        private readonly EventCallback<NavigationSubmitEvent> navigationSubmit;
        private readonly EventCallback<KeyDownEvent> keyDown;
        private int pointerId = -1;
        private MotionPointerDevice device = MotionPointerDevice.Mouse;
        private Vector2 start;
        private Vector2 point;
        private Vector2 priorPoint;
        private Vector2 offset;
        private Vector2 velocity;
        private Vector2 baseTranslation;
        private TimeSpan priorTime;
        private MotionGestureAxis? lockedAxis;
        private MotionDragBounds? measuredBounds;
        private Vector2 priorScroll;
        private bool panStarted;
        private bool dragStarted;
        private bool recognizesPanAndDrag;
        private bool momentum;
        private bool inView;
        private bool focusVisible;
        private bool disposed;
        private bool ownsCapture;
        private uint momentumGeneration;

        public BattlementGestureState(
            MotionDescriptor descriptor,
            VisualElement target,
            Func<ObjectId, VisualElement?> resolve,
            Func<TimeSpan> now,
            Func<bool> reducedMotion,
            Action<MotionLayer, bool> setLayer,
            Action<ObjectId, MotionValue> setValue,
            Action<MotionGestureEvent, bool> emit
        )
        {
            this.descriptor = descriptor;
            gestures = descriptor.Gestures!;
            this.target = target;
            this.resolve = resolve;
            this.now = now;
            this.reducedMotion = reducedMotion;
            this.setLayer = setLayer;
            this.setValue = setValue;
            this.emit = emit;
            pointerEnter = value => Hover(value.pointerType, true);
            pointerLeave = value => Hover(value.pointerType, false);
            pointerDown = Begin;
            pointerMove = Move;
            pointerUp = End;
            pointerCancel = _ => Cancel();
            captureOut = value =>
            {
                if (value.pointerId == pointerId)
                    Cancel();
            };
            focusIn = value => Focus(value.target, true);
            focusOut = value => Focus(value.target, false);
            navigationSubmit = _ => Submit(MotionPointerDevice.Gamepad);
            keyDown = SubmitKeyboard;
            target.RegisterCallback(pointerEnter);
            target.RegisterCallback(pointerLeave);
            target.RegisterCallback(pointerDown, TrickleDown.TrickleDown);
            target.RegisterCallback(pointerMove, TrickleDown.TrickleDown);
            target.RegisterCallback(pointerUp, TrickleDown.TrickleDown);
            target.RegisterCallback(pointerCancel, TrickleDown.TrickleDown);
            target.RegisterCallback(captureOut);
            target.RegisterCallback(focusIn);
            target.RegisterCallback(focusOut);
            target.RegisterCallback(navigationSubmit, TrickleDown.TrickleDown);
            target.RegisterCallback(keyDown, TrickleDown.TrickleDown);
            priorScroll = ScrollOffset();
        }

        public bool HasActiveWork => pointerId >= 0 || momentum;

        public ObjectId? ControlId => gestures.Drag?.ControlId;

        public void Sample()
        {
            if (disposed)
                return;
            if (!target.enabledInHierarchy && HasActiveWork)
                Cancel();
            if (gestures.Drag?.Constraints is MotionDragConstraint.Element)
                ResolveBounds();
            if (momentum && reducedMotion())
            {
                momentum = false;
                velocity = Vector2.zero;
                SnapToOrigin();
                setLayer(MotionLayer.Drag, false);
            }
            else if (momentum)
                SampleMomentum();
            ApplyPresentation();
            SampleScroll();
            SampleInView();
        }

        public void Cancel()
        {
            if (pointerId < 0 && !momentum)
                return;
            if (dragStarted || momentum)
                Emit(MotionGestureEventKind.DragCancel);
            else if (panStarted)
                Emit(MotionGestureEventKind.PanCancel);
            else
                Emit(MotionGestureEventKind.TapCancel);
            ReleasePointer();
            momentum = false;
            panStarted = false;
            dragStarted = false;
            velocity = Vector2.zero;
            setLayer(MotionLayer.Tap, false);
            setLayer(MotionLayer.Drag, false);
        }

        public void StartExternal(MotionDragControlOperation operation)
        {
            MotionDragDescriptor? drag = gestures.Drag;
            if (disposed || drag is null || operation.PointerId < 0)
                return;
            if (!target.enabledInHierarchy)
                return;
            Cancel();
            device = operation.Device;
            pointerId = operation.PointerId;
            start = point = priorPoint = new Vector2(operation.Point.X, operation.Point.Y);
            offset = Vector2.zero;
            velocity = Vector2.zero;
            priorTime = now();
            lockedAxis = null;
            panStarted = false;
            dragStarted = true;
            recognizesPanAndDrag = true;
            baseTranslation = new Vector2(
                ReadPixels(MotionProperty.X),
                ReadPixels(MotionProperty.Y)
            );
            ResolveBounds();
            if (operation.SnapToCursor)
                offset = Constrain(point - SnapCenter(), elastic: true);
            target.CapturePointer(pointerId);
            ownsCapture = true;
            setLayer(MotionLayer.Drag, true);
            ApplyPresentation();
            if (gestures.Subscriptions.Drag)
                Emit(MotionGestureEventKind.DragStart);
        }

        public void Dispose()
        {
            if (disposed)
                return;
            Cancel();
            disposed = true;
            target.UnregisterCallback(pointerEnter);
            target.UnregisterCallback(pointerLeave);
            target.UnregisterCallback(pointerDown, TrickleDown.TrickleDown);
            target.UnregisterCallback(pointerMove, TrickleDown.TrickleDown);
            target.UnregisterCallback(pointerUp, TrickleDown.TrickleDown);
            target.UnregisterCallback(pointerCancel, TrickleDown.TrickleDown);
            target.UnregisterCallback(captureOut);
            target.UnregisterCallback(focusIn);
            target.UnregisterCallback(focusOut);
            target.UnregisterCallback(navigationSubmit, TrickleDown.TrickleDown);
            target.UnregisterCallback(keyDown, TrickleDown.TrickleDown);
        }

        private void Hover(string pointerType, bool active)
        {
            if (Device(pointerType) == MotionPointerDevice.Touch)
                return;
            setLayer(MotionLayer.Hover, active);
            if (gestures.Subscriptions.Hover)
                Emit(active ? MotionGestureEventKind.HoverStart : MotionGestureEventKind.HoverEnd);
        }

        private void Focus(object eventTarget, bool active)
        {
            if (!ReferenceEquals(eventTarget, target))
                return;
            setLayer(MotionLayer.Focus, active);
            if (gestures.Subscriptions.Focus)
                Emit(active ? MotionGestureEventKind.FocusStart : MotionGestureEventKind.FocusEnd);
        }

        public void SetFocusVisible(bool value)
        {
            if (focusVisible == value)
                return;
            focusVisible = value;
            if (gestures.Subscriptions.FocusVisible)
                Emit(
                    value
                        ? MotionGestureEventKind.FocusVisibleStart
                        : MotionGestureEventKind.FocusVisibleEnd
                );
        }

        private void SubmitKeyboard(KeyDownEvent value)
        {
            if (
                value.keyCode
                is not KeyCode.Return
                    and not KeyCode.KeypadEnter
                    and not KeyCode.Space
            )
                return;
            Submit(MotionPointerDevice.Keyboard);
            value.StopPropagation();
        }

        private void Submit(MotionPointerDevice submittingDevice)
        {
            if (!target.enabledInHierarchy)
                return;
            device = submittingDevice;
            pointerId = -1;
            point = target.worldBound.center;
            setLayer(MotionLayer.Tap, true);
            if (gestures.Subscriptions.Tap)
            {
                Emit(MotionGestureEventKind.TapStart);
                Emit(MotionGestureEventKind.Tap);
            }
            setLayer(MotionLayer.Tap, false);
        }

        private void Begin(PointerDownEvent value)
        {
            if (pointerId >= 0 || !value.isPrimary || value.button != 0)
                return;
            MotionDragDescriptor? drag = gestures.Drag;
            bool directTarget = ReferenceEquals(value.target, target);
            bool tap = gestures.Subscriptions.Tap || HasLayer(MotionLayer.Tap);
            bool descendantTap = tap && OwnsTapTarget(value.target, value.pointerId);
            recognizesPanAndDrag = directTarget || drag?.Propagation == true;
            if (!recognizesPanAndDrag && !descendantTap)
                return;
            if (!tap && !gestures.Pan && (drag is null || !drag.Listener))
                return;
            if (momentum)
                Cancel();
            momentum = false;
            device = Device(value.pointerType);
            pointerId = value.pointerId;
            start = point = priorPoint = value.position;
            offset = Vector2.zero;
            velocity = Vector2.zero;
            priorTime = now();
            lockedAxis = null;
            panStarted = false;
            dragStarted = false;
            baseTranslation = new Vector2(
                ReadPixels(MotionProperty.X),
                ReadPixels(MotionProperty.Y)
            );
            ResolveBounds();
            if (directTarget || descendantTap)
            {
                target.CapturePointer(pointerId);
                ownsCapture = true;
            }
            setLayer(MotionLayer.Tap, true);
            if (gestures.Subscriptions.Tap)
                Emit(MotionGestureEventKind.TapStart);
            if (recognizesPanAndDrag && gestures.Pan && gestures.Subscriptions.Pan)
                Emit(MotionGestureEventKind.PanSessionStart);
        }

        private bool OwnsTapTarget(object eventTarget, int eventPointerId)
        {
            for (
                VisualElement? element = eventTarget as VisualElement;
                element is not null;
                element = element.parent
            )
            {
                if (ReferenceEquals(element, target))
                    return true;
                if (element.focusable || element.HasPointerCapture(eventPointerId))
                    return false;
            }
            return false;
        }

        private void Move(PointerMoveEvent value)
        {
            if (value.pointerId != pointerId)
                return;
            TimeSpan current = now();
            priorPoint = point;
            point = value.position;
            double seconds = Math.Max((current - priorTime).TotalSeconds, 0.000001);
            velocity = (point - priorPoint) / (float)seconds;
            priorTime = current;
            Vector2 raw = point - start;
            MaybeLock(raw);
            raw = ApplyAxis(raw);
            if (!panStarted && raw.magnitude >= gestures.PanThreshold)
            {
                panStarted = recognizesPanAndDrag && gestures.Pan;
                dragStarted = recognizesPanAndDrag && gestures.Drag is not null;
                setLayer(MotionLayer.Tap, false);
                if (gestures.Subscriptions.Tap)
                    Emit(MotionGestureEventKind.TapCancel);
                if (panStarted && gestures.Subscriptions.Pan)
                    Emit(MotionGestureEventKind.PanStart);
                if (dragStarted)
                {
                    if (gestures.Drag?.Propagation != true)
                        value.StopPropagation();
                    setLayer(MotionLayer.Drag, true);
                    if (gestures.Subscriptions.Drag)
                        Emit(MotionGestureEventKind.DragStart);
                }
            }
            offset = Constrain(raw, elastic: true);
            if (panStarted && gestures.Subscriptions.PanUpdate)
                Emit(MotionGestureEventKind.Pan, replaceable: true);
            if (dragStarted && gestures.Subscriptions.DragUpdate)
                Emit(MotionGestureEventKind.Drag, replaceable: true);
        }

        private void End(PointerUpEvent value)
        {
            if (value.pointerId != pointerId)
                return;
            point = value.position;
            if (!panStarted && !dragStarted)
            {
                float slop =
                    device == MotionPointerDevice.Touch
                        ? gestures.TouchTapSlop
                        : gestures.PointerTapSlop;
                if ((point - start).magnitude <= slop && gestures.Subscriptions.Tap)
                    Emit(MotionGestureEventKind.Tap);
                else if (gestures.Subscriptions.Tap)
                    Emit(MotionGestureEventKind.TapCancel);
            }
            if (panStarted && gestures.Subscriptions.Pan)
                Emit(MotionGestureEventKind.PanEnd);
            MotionDragDescriptor? drag = gestures.Drag;
            SuppressSnappedVelocity();
            bool continueMomentum =
                drag is not null
                && drag.Momentum
                && !reducedMotion()
                && velocity.magnitude >= drag.Transition.RestSpeed;
            if (dragStarted && continueMomentum)
                momentumGeneration++;
            if (dragStarted && gestures.Subscriptions.Drag)
                Emit(MotionGestureEventKind.DragEnd);
            ReleasePointer();
            setLayer(MotionLayer.Tap, false);
            panStarted = false;
            if (dragStarted && continueMomentum)
            {
                momentum = true;
            }
            else
            {
                momentum = false;
                SnapToOrigin();
                setLayer(MotionLayer.Drag, false);
            }
            dragStarted = false;
        }

        private void SampleMomentum()
        {
            TimeSpan current = now();
            float seconds = (float)Math.Max((current - priorTime).TotalSeconds, 1.0 / 120.0);
            priorTime = current;
            offset = Constrain(offset + velocity * seconds, elastic: false);
            velocity *= MathF.Pow(gestures.Drag!.Transition.VelocityRetention, seconds);
            if (velocity.magnitude >= gestures.Drag.Transition.RestSpeed)
                return;
            momentum = false;
            velocity = Vector2.zero;
            SnapToOrigin();
            if (gestures.Subscriptions.MomentumComplete)
                Emit(MotionGestureEventKind.DragMomentumComplete);
            setLayer(MotionLayer.Drag, false);
        }

        private void ApplyPresentation()
        {
            if (!dragStarted && !momentum)
                return;
            if (gestures.Drag?.XValue is ObjectId xValue)
                setValue(xValue, new MotionValue.Scalar(offset.x));
            if (gestures.Drag?.YValue is ObjectId yValue)
                setValue(yValue, new MotionValue.Scalar(offset.y));
            bool reduced = reducedMotion();
            BattlementMotionPropertyWriter.WriteTranslation(
                target,
                baseTranslation.x + (reduced ? 0 : offset.x),
                baseTranslation.y + (reduced ? 0 : offset.y)
            );
        }

        private void SampleScroll()
        {
            if (!gestures.Scroll)
                return;
            Vector2 current = ScrollOffset();
            if (current == priorScroll)
                return;
            Vector2 delta = current - priorScroll;
            priorScroll = current;
            point = current;
            priorPoint = current - delta;
            offset = current;
            velocity = delta;
            if (gestures.ScrollXValue is ObjectId xValue)
                setValue(xValue, new MotionValue.Scalar(current.x));
            if (gestures.ScrollYValue is ObjectId yValue)
                setValue(yValue, new MotionValue.Scalar(current.y));
            if (gestures.Subscriptions.Scroll)
                Emit(MotionGestureEventKind.Scroll, replaceable: true);
        }

        private void SampleInView()
        {
            if (!gestures.InView || target.panel is null)
                return;
            bool current = target.worldBound.Overlaps(target.panel.visualTree.worldBound);
            if (current == inView)
                return;
            inView = current;
            if (gestures.InViewValue is ObjectId value)
                setValue(value, new MotionValue.Scalar(current ? 1 : 0));
            setLayer(MotionLayer.InView, current);
            if (gestures.Subscriptions.InView)
                Emit(
                    current
                        ? MotionGestureEventKind.InViewEnter
                        : MotionGestureEventKind.InViewLeave
                );
        }

        private void ResolveBounds()
        {
            MotionDragBounds? resolved = gestures.Drag?.Constraints switch
            {
                MotionDragConstraint.Bounds value => value.Value,
                MotionDragConstraint.Element value => Measure(value.Value),
                _ => null,
            };
            if (resolved == measuredBounds)
                return;
            measuredBounds = resolved;
            if (resolved is not null && gestures.Subscriptions.ConstraintsMeasured)
                Emit(MotionGestureEventKind.DragConstraintsMeasured);
        }

        private MotionDragBounds? Measure(ObjectId id)
        {
            VisualElement? owner = resolve(id);
            if (owner is null || !ReferenceEquals(owner.panel, target.panel))
                return null;
            UnityEngine.Rect bounds = owner.worldBound;
            UnityEngine.Rect item = target.worldBound;
            var result = new MotionDragBounds(
                bounds.xMin - item.xMin,
                bounds.xMax - item.xMax,
                bounds.yMin - item.yMin,
                bounds.yMax - item.yMax
            );
            return result;
        }

        private Vector2 Constrain(Vector2 value, bool elastic)
        {
            if (measuredBounds is not MotionDragBounds bounds)
                return value;
            MotionDragElastic factors = gestures.Drag!.Elastic;
            float x = Elastic(
                value.x,
                bounds.MinX,
                bounds.MaxX,
                factors.Left,
                factors.Right,
                elastic
            );
            float y = Elastic(
                value.y,
                bounds.MinY,
                bounds.MaxY,
                factors.Top,
                factors.Bottom,
                elastic
            );
            return new Vector2(x, y);
        }

        private static float Elastic(
            float value,
            float minimum,
            float maximum,
            float before,
            float after,
            bool elastic
        )
        {
            if (value < minimum)
                return minimum + (value - minimum) * (elastic ? before : 0);
            if (value > maximum)
                return maximum + (value - maximum) * (elastic ? after : 0);
            return value;
        }

        private void MaybeLock(Vector2 raw)
        {
            if (lockedAxis is not null || gestures.Drag?.DirectionLock != true)
                return;
            if (Math.Abs(raw.y) >= gestures.DirectionLockThreshold)
                lockedAxis = MotionGestureAxis.Y;
            else if (Math.Abs(raw.x) >= gestures.DirectionLockThreshold)
                lockedAxis = MotionGestureAxis.X;
            if (lockedAxis is not null && gestures.Subscriptions.Drag)
                Emit(MotionGestureEventKind.DragDirectionLock);
        }

        private Vector2 ApplyAxis(Vector2 value)
        {
            MotionGestureAxis axis = lockedAxis ?? gestures.Drag?.Axis ?? MotionGestureAxis.Both;
            return axis switch
            {
                MotionGestureAxis.X => new Vector2(value.x, 0),
                MotionGestureAxis.Y => new Vector2(0, value.y),
                _ => value,
            };
        }

        private void SnapToOrigin()
        {
            switch (gestures.Drag?.SnapToOrigin)
            {
                case MotionGestureAxis.X:
                    offset.x = 0;
                    break;
                case MotionGestureAxis.Y:
                    offset.y = 0;
                    break;
                case MotionGestureAxis.Both:
                    offset = Vector2.zero;
                    break;
                default:
                    break;
            }
        }

        private void SuppressSnappedVelocity()
        {
            switch (gestures.Drag?.SnapToOrigin)
            {
                case MotionGestureAxis.X:
                    velocity.x = 0;
                    break;
                case MotionGestureAxis.Y:
                    velocity.y = 0;
                    break;
                case MotionGestureAxis.Both:
                    velocity = Vector2.zero;
                    break;
                default:
                    break;
            }
        }

        private void ReleasePointer()
        {
            int released = pointerId;
            pointerId = -1;
            if (ownsCapture && released >= 0 && target.HasPointerCapture(released))
                target.ReleasePointer(released);
            ownsCapture = false;
        }

        private void Emit(MotionGestureEventKind kind, bool replaceable = false) =>
            emit(
                new MotionGestureEvent(
                    descriptor.DescriptorId,
                    descriptor.Generation,
                    kind,
                    pointerId,
                    device,
                    new MotionGestureVector(point.x, point.y),
                    new MotionGestureVector(point.x - priorPoint.x, point.y - priorPoint.y),
                    new MotionGestureVector(offset.x, offset.y),
                    new MotionGestureVector(velocity.x, velocity.y),
                    lockedAxis,
                    momentumGeneration,
                    measuredBounds is not null
                ),
                replaceable
            );

        private float ReadPixels(MotionProperty property) =>
            BattlementMotionPropertyWriter.Read(target, property) is MotionValue.Length value
                ? (float)value.Value.Px
                : 0;

        private Vector2 ScrollOffset() =>
            target is ScrollView scroll ? scroll.scrollOffset : Vector2.zero;

        private Vector2 SnapCenter()
        {
            Vector2 center = target.worldBound.center;
            return IsFinite(center.x) && IsFinite(center.y) ? center : Vector2.zero;
        }

        private static bool IsFinite(float value) =>
            !float.IsNaN(value) && !float.IsInfinity(value);

        private bool HasLayer(MotionLayer layer) =>
            Array.Exists(descriptor.Slots.ToArray(), slot => slot.Layer == layer);

        internal static MotionPointerDevice Device(string value) =>
            value == UnityEngine.UIElements.PointerType.touch ? MotionPointerDevice.Touch
            : value == UnityEngine.UIElements.PointerType.pen ? MotionPointerDevice.Pen
            : MotionPointerDevice.Mouse;
    }
}
