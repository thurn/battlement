#nullable enable
using System;
using System.Collections.Generic;
using System.Linq;
using Battlement.UI;
using UnityEngine;

namespace Battlement
{
    /// <summary>Semantic world focus with native UI modal handoff.</summary>
    internal sealed class BattlementWorldFocusInput
    {
        private readonly BattlementWorld world;
        private readonly BattlementUiNavigation ui;
        private readonly Func<bool> modal;
        private readonly Func<UiEvent, UiEventDisposition?> emit;
        private BattlementIdentity? focused;
        private BattlementIdentity? invoker;
        private bool inModal;
        private bool inputAvailable = true;

        internal BattlementWorldFocusInput(
            BattlementWorld world,
            BattlementUiNavigation ui,
            Func<bool> modal,
            Func<UiEvent, UiEventDisposition?> emit
        ) => (this.world, this.ui, this.modal, this.emit) = (world, ui, modal, emit);

        internal BattlementIdentity? Focused => focused;
        private bool HasTargets => world.Identities.Any(Eligible);
        private bool OwnsNavigation => focused != null || invoker != null || HasTargets;

        internal bool Enables(PhysicalKey key) => OwnsNavigation && IsSemanticKey(key);

        internal ControllerInputSettings? Settings(ControllerInputSettings? settings)
        {
            if (!OwnsNavigation)
                return settings;
            return new ControllerInputSettings(
                (settings?.Buttons ?? Array.Empty<ControllerButton>())
                    .Union(new[] { ControllerButton.South, ControllerButton.East })
                    .ToArray(),
                true,
                settings?.StickDeadZone,
                settings?.RepeatDelay,
                settings?.RepeatInterval
            );
        }

        internal void Refresh(bool inputAvailable)
        {
            this.inputAvailable = inputAvailable;
            if (!inputAvailable)
            {
                invoker = null;
                inModal = false;
                SetFocus(null);
                return;
            }
            bool nextModal = modal();
            if (nextModal && !inModal)
            {
                invoker = focused;
                inModal = true;
                SetFocus(null);
                ui.ShowFocus();
            }
            else if (!nextModal && inModal)
            {
                inModal = false;
                BattlementIdentity? restore = invoker;
                invoker = null;
                if (Eligible(restore))
                    SetFocus(restore);
                else if (!ReferenceEquals(restore, null))
                    SetFocus(Candidates().FirstOrDefault());
            }
            if (!ReferenceEquals(focused, null) && !Eligible(focused))
                SetFocus(Candidates().FirstOrDefault());
            if (!inModal && focused != null && ui.Focused != null)
                SetFocus(null);
        }

        internal bool TryHandle(ActionBody body, bool includeUi = false)
        {
            Refresh(inputAvailable);
            if (!OwnsNavigation)
                return false;
            if (inModal || (focused == null && ui.Focused != null))
            {
                if (!includeUi)
                    return false;
                if (Direction(body) is UiNavigationDirection move)
                    ui.Navigate(move);
                else if (IsActivate(body))
                    ui.Activate();
                else if (IsCancel(body))
                    ui.Cancel();
                else
                    return false;
                Refresh(inputAvailable);
                return true;
            }
            if (Direction(body) is UiNavigationDirection direction)
                Navigate(direction);
            else if (IsActivate(body))
                Activate();
            else if (IsCancel(body))
                Cancel();
            else
                return false;
            return true;
        }

        internal bool Dispatch(DittoNavigationAction action)
        {
            Refresh(inputAvailable);
            if (!inputAvailable)
                return false;
            bool uiRoute = inModal || (focused == null && ui.Focused != null);
            if (uiRoute && ui.Focused == null)
                return false;
            if (!uiRoute && !Candidates().Any())
                return false;
            if (action == DittoNavigationAction.Activate || action == DittoNavigationAction.Cancel)
            {
                if (!uiRoute && focused == null)
                    return false;
                if (action == DittoNavigationAction.Activate)
                {
                    if (uiRoute)
                        ui.Activate();
                    else
                        Activate();
                }
                else
                {
                    if (uiRoute)
                        ui.Cancel();
                    else
                        Cancel();
                }
            }
            else
            {
                UiNavigationDirection direction = action switch
                {
                    DittoNavigationAction.Left => UiNavigationDirection.Left,
                    DittoNavigationAction.Right => UiNavigationDirection.Right,
                    DittoNavigationAction.Up => UiNavigationDirection.Up,
                    DittoNavigationAction.Down => UiNavigationDirection.Down,
                    DittoNavigationAction.Next => UiNavigationDirection.Next,
                    DittoNavigationAction.Previous => UiNavigationDirection.Previous,
                    _ => throw new ArgumentOutOfRangeException(nameof(action)),
                };
                if (uiRoute)
                    ui.Navigate(direction);
                else
                    Navigate(direction);
            }
            Refresh(inputAvailable);
            return true;
        }

        internal void Navigate(UiNavigationDirection direction)
        {
            Refresh(inputAvailable);
            if (inModal)
            {
                ui.Navigate(direction);
                return;
            }
            if (
                focused != null
                && Send(
                    focused,
                    new UiEventBody.NavigationMove(
                        new UiNavigationMoveEvent(
                            direction,
                            direction switch
                            {
                                UiNavigationDirection.Left => new Vector(-1, 0),
                                UiNavigationDirection.Right => new Vector(1, 0),
                                UiNavigationDirection.Up => new Vector(0, 1),
                                UiNavigationDirection.Down => new Vector(0, -1),
                                _ => new Vector(0, 0),
                            }
                        )
                    )
                ) != UiEventDisposition.Continue
            )
                return;
            BattlementIdentity[] candidates = Candidates().ToArray();
            int index = Array.IndexOf(candidates, focused);
            if (index < 0)
            {
                SetFocus(candidates.FirstOrDefault());
                return;
            }
            if (direction is UiNavigationDirection.Next or UiNavigationDirection.Previous)
            {
                int offset = direction == UiNavigationDirection.Next ? 1 : candidates.Length - 1;
                SetFocus(candidates[(index + offset) % candidates.Length]);
                return;
            }
            UnityEngine.Vector2 axis = direction switch
            {
                UiNavigationDirection.Left => UnityEngine.Vector2.left,
                UiNavigationDirection.Right => UnityEngine.Vector2.right,
                UiNavigationDirection.Up => UnityEngine.Vector2.up,
                UiNavigationDirection.Down => UnityEngine.Vector2.down,
                _ => UnityEngine.Vector2.zero,
            };
            UnityEngine.Vector3 origin = Position(focused!);
            BattlementIdentity? next = null;
            float score = float.PositiveInfinity;
            foreach (BattlementIdentity candidate in candidates)
            {
                UnityEngine.Vector3 delta = Position(candidate) - origin;
                float forward = delta.x * axis.x + delta.y * axis.y;
                if (forward <= 0.001f)
                    continue;
                float distance = forward + Mathf.Abs(delta.x * axis.y - delta.y * axis.x) * 2;
                if (distance < score)
                {
                    next = candidate;
                    score = distance;
                }
            }
            if (next != null)
                SetFocus(next);
        }

        internal void Activate()
        {
            Refresh(inputAvailable);
            if (inModal)
                ui.Activate();
            else if (Eligible(focused))
                Send(focused!, new UiEventBody.Click(new ClickEvent.NavigationSubmit()));
            Refresh(inputAvailable);
        }

        internal void Cancel()
        {
            Refresh(inputAvailable);
            if (inModal)
                ui.Cancel();
            else if (Eligible(focused))
                Send(focused!, new UiEventBody.NavigationCancel(new UiNavigationEvent()));
            Refresh(inputAvailable);
        }

        private IEnumerable<BattlementIdentity> Candidates() =>
            world.Identities.Where(Eligible).OrderBy(v => v.WorldPointer!.Order).ThenBy(v => v.Id);

        private bool Eligible(BattlementIdentity? value)
        {
            if (!inputAvailable || inModal)
                return false;
            if (value == null || !value.IsAvailableForPointerInput)
                return false;
            if (value.WorldPointer?.Focusable != true || !value.HasPointerEvents)
                return false;
            Camera? camera = world.InputCamera;
            if (camera == null)
                return false;
            float depth = Position(value).z;
            return depth >= camera.nearClipPlane && depth <= camera.farClipPlane;
        }

        private UnityEngine.Vector3 Position(BattlementIdentity value) =>
            world.InputCamera!.WorldToScreenPoint(
                value.TryGetComponent(out Collider collider)
                    ? collider.bounds.center
                    : value.transform.position
            );

        private void SetFocus(BattlementIdentity? next)
        {
            if (ReferenceEquals(focused, next))
                return;
            BattlementIdentity? previous = focused;
            focused = next;
            if (next != null)
                ui.Blur();
            if (previous != null)
            {
                ObjectId? related = next == null ? null : new ObjectId(next.Id);
                Send(previous, new UiEventBody.FocusOut(new UiFocusEvent(related)));
                Send(previous, new UiEventBody.Blur(new UiFocusEvent(related)));
            }
            if (next != null)
            {
                ObjectId? related = previous == null ? null : new ObjectId(previous.Id);
                Send(next, new UiEventBody.FocusIn(new UiFocusEvent(related)));
                Send(next, new UiEventBody.Focus(new UiFocusEvent(related)));
            }
        }

        private UiEventDisposition? Send(BattlementIdentity target, UiEventBody body) =>
            emit(new UiEvent(new ObjectId(target.Id), true, false, body));

        private static bool IsSemanticKey(PhysicalKey key) =>
            key
                is PhysicalKey.ArrowLeft
                    or PhysicalKey.ArrowRight
                    or PhysicalKey.ArrowUp
                    or PhysicalKey.ArrowDown
                    or PhysicalKey.Tab
                    or PhysicalKey.Enter
                    or PhysicalKey.Space
                    or PhysicalKey.Escape;

        private static bool IsActivate(ActionBody body) =>
            body
                is ActionBody.KeyDown { Key: PhysicalKey.Enter or PhysicalKey.Space }
                    or ActionBody.ControllerButtonDown { Button: ControllerButton.South };

        private static bool IsCancel(ActionBody body) =>
            body
                is ActionBody.KeyDown { Key: PhysicalKey.Escape }
                    or ActionBody.ControllerButtonDown { Button: ControllerButton.East };

        private static UiNavigationDirection? Direction(ActionBody body) =>
            body switch
            {
                ActionBody.KeyDown { Key: PhysicalKey.ArrowLeft } => UiNavigationDirection.Left,
                ActionBody.KeyDown { Key: PhysicalKey.ArrowRight } => UiNavigationDirection.Right,
                ActionBody.KeyDown { Key: PhysicalKey.ArrowUp } => UiNavigationDirection.Up,
                ActionBody.KeyDown { Key: PhysicalKey.ArrowDown } => UiNavigationDirection.Down,
                ActionBody.KeyDown { Key: PhysicalKey.Tab } => UiNavigationDirection.Next,
                ActionBody.ControllerNavigate value => value.Direction switch
                {
                    ControllerDirection.Left => UiNavigationDirection.Left,
                    ControllerDirection.Right => UiNavigationDirection.Right,
                    ControllerDirection.Up => UiNavigationDirection.Up,
                    ControllerDirection.Down => UiNavigationDirection.Down,
                    _ => null,
                },
                _ => null,
            };
    }
}
