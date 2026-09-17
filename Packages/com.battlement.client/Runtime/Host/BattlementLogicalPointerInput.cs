#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;

namespace Battlement
{
    /// <summary>Routes world pointers through the synchronous logical event transport.</summary>
    internal sealed class BattlementLogicalPointerInput
    {
        private readonly Func<UiEvent, UiEventDisposition?> emit;
        private readonly Dictionary<int, State> pointers = new();

        internal BattlementLogicalPointerInput(Func<UiEvent, UiEventDisposition?> emit) =>
            this.emit = emit;

        internal bool IsCaptured(int pointerId) =>
            pointers.TryGetValue(pointerId, out State value) && value.Captured != null;

        internal bool Process(
            int pointerId,
            BattlementPointerSample sample,
            BattlementIdentity? picked,
            bool modalBlocked,
            IEnumerable<PointerButton>? previousButtons = null
        )
        {
            pointers.TryGetValue(pointerId, out State state);
            BattlementIdentity? target =
                modalBlocked || picked == null || picked.WorldPointer is null ? null : picked;
            if (state == null && target == null)
                return false;
            if (state == null)
            {
                state = pointers[pointerId] = new State(sample.Position);
                if (previousButtons != null)
                    state.Buttons.UnionWith(previousButtons);
            }
            bool handled = target != null || state.Captured != null || state.Hovered != null;
            if (
                !ReferenceEquals(state.Captured, null)
                && (modalBlocked || !Available(state.Captured))
            )
                Lose(pointerId, state);
            target = state.Captured != null ? state.Captured : target;
            UnityEngine.Vector2 delta = sample.Position - state.Position;
            state.Position = sample.Position;
            if (!ReferenceEquals(state.Hovered, target))
            {
                Boundary(state.Hovered, target, pointerId, sample.Position, false);
                Boundary(target, state.Hovered, pointerId, sample.Position, true);
                state.Hovered = target;
            }
            if (sample.IsCancelled || !sample.IsPresent)
            {
                if (state.Captured != null)
                    Send(
                        state.Captured,
                        new UiEventBody.PointerCancel(
                            new UiPointerCancelEvent(
                                Point(sample.Position),
                                new Vector(delta.x, -delta.y),
                                pointerId,
                                PointerType: Type(pointerId)
                            )
                        ),
                        false
                    );
                CancelPressed(state, pointerId);
                Boundary(state.Hovered, null, pointerId, state.Position, false);
                state.Hovered = null;
                Lose(pointerId, state);
                state.Pressed.Clear();
                state.Buttons.Clear();
                if (!sample.IsPresent)
                    pointers.Remove(pointerId);
                return handled;
            }
            if (delta != UnityEngine.Vector2.zero && Available(target))
            {
                state.Moved |= state.Captured != null;
                Send(
                    target!,
                    new UiEventBody.PointerMove(
                        new UiPointerMoveEvent(
                            Point(sample.Position),
                            new Vector(delta.x, -delta.y),
                            pointerId,
                            Buttons: Buttons(sample.Buttons),
                            PointerType: Type(pointerId)
                        )
                    ),
                    true
                );
            }
            foreach (PointerButton button in Enum.GetValues(typeof(PointerButton)))
            {
                bool down = sample.Buttons.Contains(button);
                bool wasDown = state.Buttons.Contains(button);
                var value = new UiPointerButtonEvent(
                    Point(sample.Position),
                    new Vector(delta.x, -delta.y),
                    pointerId,
                    Button(button),
                    Buttons(sample.Buttons),
                    PointerType: Type(pointerId)
                );
                if (down && !wasDown)
                {
                    state.Pressed[button] = target;
                    state.Moved = false;
                    if (Available(target))
                    {
                        UiEventDisposition? disposition = Send(
                            target!,
                            new UiEventBody.PointerDown(value),
                            true
                        );
                        if (
                            button == PointerButton.Left
                            && disposition == UiEventDisposition.Continue
                            && target!.WorldPointer?.CaptureOnPress == true
                        )
                            Capture(pointerId, state, target);
                    }
                }
                else if (!down && wasDown)
                {
                    state.Pressed.Remove(button, out BattlementIdentity? pressed);
                    if (Available(target))
                    {
                        UiEventDisposition? disposition = Send(
                            target!,
                            new UiEventBody.PointerUp(value),
                            true
                        );
                        if (
                            ReferenceEquals(pressed, target)
                            && !state.Moved
                            && disposition == UiEventDisposition.Continue
                        )
                            Send(
                                target!,
                                new UiEventBody.Click(
                                    new ClickEvent.Pointer(
                                        value.Position,
                                        value.ClickCount,
                                        pointerId,
                                        value.Button
                                    )
                                ),
                                true
                            );
                    }
                    if (button == PointerButton.Left)
                        Lose(pointerId, state);
                }
            }
            state.Buttons.Clear();
            state.Buttons.UnionWith(sample.Buttons);
            return handled;
        }

        internal void Reset()
        {
            foreach ((int id, State state) in pointers.ToArray())
            {
                CancelPressed(state, id);
                Boundary(state.Hovered, null, id, state.Position, false);
                Lose(id, state);
            }
            pointers.Clear();
        }

        private void CancelPressed(State state, int pointerId)
        {
            foreach (BattlementIdentity? target in state.Pressed.Values.Distinct())
                if (target != null && !ReferenceEquals(target, state.Captured))
                    Send(
                        target,
                        new UiEventBody.PointerCancel(
                            new UiPointerCancelEvent(
                                Point(state.Position),
                                new Vector(0, 0),
                                pointerId,
                                PointerType: Type(pointerId)
                            )
                        ),
                        false
                    );
        }

        private void Capture(int id, State state, BattlementIdentity target)
        {
            Lose(id, state);
            state.Captured = target;
            state.Unavailable = () => Lose(id, state);
            target.PointerUnavailable += state.Unavailable;
            Send(target, new UiEventBody.PointerCapture(new UiPointerCaptureEvent(id)), false);
        }

        private void Lose(int id, State state)
        {
            BattlementIdentity? target = state.Captured;
            if (ReferenceEquals(target, null))
                return;
            state.Captured = null;
            if (target != null)
                target.PointerUnavailable -= state.Unavailable;
            state.Unavailable = null;
            state.Pressed.Clear();
            state.Moved = true;
            Send(target!, new UiEventBody.PointerCaptureOut(new UiPointerCaptureEvent(id)), false);
        }

        private void Boundary(
            BattlementIdentity? target,
            BattlementIdentity? related,
            int id,
            UnityEngine.Vector2 point,
            bool enter
        )
        {
            if (!Available(target))
                return;
            var crossing = new UiPointerCrossingEvent(
                Point(point),
                id,
                Type(id),
                related == null ? null : new ObjectId(related.Id)
            );
            Send(
                target!,
                enter
                    ? new UiEventBody.PointerOver(crossing)
                    : new UiEventBody.PointerOut(crossing),
                false
            );
            var value = new UiPointerBoundaryEvent(Point(point), id, Type(id));
            Send(
                target!,
                enter ? new UiEventBody.PointerEnter(value) : new UiEventBody.PointerLeave(value),
                false
            );
        }

        private UiEventDisposition? Send(
            BattlementIdentity target,
            UiEventBody body,
            bool cancelable
        ) => emit(new UiEvent(new ObjectId(target.Id), cancelable, false, body));

        private static bool Available(BattlementIdentity? target) =>
            target != null && target.IsAvailableForPointerInput && target.HasPointerEvents;

        private static UiPointerType Type(int id) =>
            id == 0 ? UiPointerType.Mouse : UiPointerType.Touch;

        private static PanelPoint Point(UnityEngine.Vector2 point) =>
            new(point.x, Screen.height - point.y);

        private static uint Buttons(HashSet<PointerButton> buttons) =>
            (uint)(
                (buttons.Contains(PointerButton.Left) ? 1 : 0)
                | (buttons.Contains(PointerButton.Right) ? 2 : 0)
                | (buttons.Contains(PointerButton.Middle) ? 4 : 0)
            );

        private static UiPointerButton Button(PointerButton button) =>
            button switch
            {
                PointerButton.Left => new UiPointerButton.Left(),
                PointerButton.Middle => new UiPointerButton.Middle(),
                _ => new UiPointerButton.Right(),
            };

        private sealed class State
        {
            internal State(UnityEngine.Vector2 position) => Position = position;

            internal UnityEngine.Vector2 Position;
            internal BattlementIdentity? Hovered;
            internal BattlementIdentity? Captured;
            internal System.Action? Unavailable;
            internal bool Moved;
            internal readonly HashSet<PointerButton> Buttons = new();
            internal readonly Dictionary<PointerButton, BattlementIdentity?> Pressed = new();
        }
    }
}
