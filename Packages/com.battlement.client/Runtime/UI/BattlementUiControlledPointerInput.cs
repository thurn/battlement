#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using UnityEngine;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal sealed record BattlementUiControlledPointerResult(
        bool Handled,
        ObjectId? Hit,
        ObjectId? Capture
    );

    /// <summary>Feeds owned raw samples through Unity's production UI input provider.</summary>
    internal sealed class BattlementUiControlledPointerInput
    {
        private readonly Func<IEnumerable<UIDocument>> documents;
        private readonly Func<VisualElement, ObjectId?> identity;
        private readonly Dictionary<int, PointerState> pointers = new();

        internal BattlementUiControlledPointerInput(
            Func<IEnumerable<UIDocument>> inputDocuments,
            Func<VisualElement, ObjectId?> objectIdentity
        ) => (documents, identity) = (inputDocuments, objectIdentity);

        internal BattlementUiControlledPointerResult Process(
            int pointerId,
            Vector2 position,
            int buttons,
            bool isPresent,
            bool isCancelled
        )
        {
            pointers.TryGetValue(pointerId, out PointerState state);
            state ??= new PointerState(new Vector2(-1, Screen.height + 1));
            if (isCancelled)
            {
                RawUiPointerProvider.Dispatch(
                    pointerId,
                    position,
                    Vector2.zero,
                    buttons: 0,
                    RawUiPointerProvider.EventType.Cancel,
                    changedButton: 1
                );
            }
            else if (!isPresent)
            {
                ReleaseButtons(pointerId, position, state.Buttons);
                RawUiPointerProvider.Dispatch(
                    pointerId,
                    new Vector2(-1, Screen.height + 1),
                    Vector2.zero,
                    buttons: 0,
                    RawUiPointerProvider.EventType.Move,
                    changedButton: 0
                );
            }
            else
            {
                Vector2 delta = position - state.ScreenPosition;
                if (delta == Vector2.zero)
                    delta.x = 0.0000001f;
                RawUiPointerProvider.Dispatch(
                    pointerId,
                    position,
                    delta,
                    buttons,
                    RawUiPointerProvider.EventType.Move,
                    changedButton: 0
                );
                DispatchButtonChanges(pointerId, position, state.Buttons, buttons);
            }

            state.ScreenPosition = position;
            state.Buttons = buttons;
            if (isPresent && !isCancelled)
                pointers[pointerId] = state;
            else
                pointers.Remove(pointerId);

            int uiPointerId = UiPointerId(pointerId);
            PanelHit hit = Pick(position);
            VisualElement? captured =
                CapturedPanel(uiPointerId)?.GetCapturingElement(uiPointerId) as VisualElement;
            return new BattlementUiControlledPointerResult(
                hit.Element is not null || captured is not null,
                ObjectIdentity(hit.Element),
                ObjectIdentity(captured)
            );
        }

        internal void Reset()
        {
            foreach ((int pointerId, PointerState state) in pointers)
            {
                RawUiPointerProvider.Dispatch(
                    pointerId,
                    state.ScreenPosition,
                    Vector2.zero,
                    buttons: 0,
                    RawUiPointerProvider.EventType.Cancel,
                    changedButton: 1
                );
            }
            pointers.Clear();
            RawUiPointerProvider.End();
        }

        private static void DispatchButtonChanges(
            int pointerId,
            Vector2 position,
            int previous,
            int current
        )
        {
            int running = previous;
            for (int bit = 0; bit < 3; bit++)
            {
                int mask = 1 << bit;
                if ((previous & mask) == (current & mask))
                    continue;
                bool pressed = (current & mask) != 0;
                running = pressed ? running | mask : running & ~mask;
                RawUiPointerProvider.Dispatch(
                    pointerId,
                    position,
                    Vector2.zero,
                    running,
                    pressed
                        ? RawUiPointerProvider.EventType.Press
                        : RawUiPointerProvider.EventType.Release,
                    (uint)mask
                );
            }
        }

        private static void ReleaseButtons(int pointerId, Vector2 position, int buttons)
        {
            int running = buttons;
            for (int bit = 0; bit < 3; bit++)
            {
                int mask = 1 << bit;
                if ((running & mask) == 0)
                    continue;
                running &= ~mask;
                RawUiPointerProvider.Dispatch(
                    pointerId,
                    position,
                    Vector2.zero,
                    running,
                    RawUiPointerProvider.EventType.Release,
                    (uint)mask
                );
            }
        }

        private PanelHit Pick(Vector2 screen)
        {
            foreach (
                UIDocument document in documents()
                    .Where(value => value != null && value.isActiveAndEnabled)
                    .OrderByDescending(value => value.sortingOrder)
            )
            {
                if (document.rootVisualElement.panel is not IPanel panel)
                    continue;
                Vector2 position = PanelPosition(panel, screen);
                if (panel.Pick(position) is VisualElement element)
                    return new PanelHit(element);
            }
            return new PanelHit(null);
        }

        private IPanel? CapturedPanel(int pointerId) =>
            documents()
                .Where(value => value != null && value.isActiveAndEnabled)
                .Select(value => value.rootVisualElement.panel)
                .FirstOrDefault(panel => panel?.GetCapturingElement(pointerId) is not null);

        private ObjectId? ObjectIdentity(VisualElement? target)
        {
            for (VisualElement? current = target; current is not null; current = current.parent)
            {
                ObjectId? value = identity(current);
                if (value is not null)
                    return value;
            }
            return null;
        }

        private static Vector2 PanelPosition(IPanel panel, Vector2 screen) =>
            RuntimePanelUtils.ScreenToPanel(panel, new Vector2(screen.x, Screen.height - screen.y));

        private static int UiPointerId(int pointerId) =>
            pointerId == 0
                ? PointerId.mousePointerId
                : PointerId.touchPointerIdBase + pointerId - 1;

        private sealed class PointerState
        {
            internal PointerState(Vector2 screenPosition) => ScreenPosition = screenPosition;

            internal Vector2 ScreenPosition;
            internal int Buttons;
        }

        private readonly struct PanelHit
        {
            internal PanelHit(VisualElement? element) => Element = element;

            internal VisualElement? Element { get; }
        }

        /// <summary>
        /// Unity exposes this provider only to its UI module. Reflection keeps the controlled
        /// source below production picking and default-control processing without fabricating
        /// UI Toolkit events or synthetic Input System devices.
        /// </summary>
        private static class RawUiPointerProvider
        {
            internal enum EventType
            {
                Move = 1,
                Press = 3,
                Release = 4,
                Cancel = 6,
            }

            private const BindingFlags InstanceField =
                BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic;
            private static readonly Type PointerEvent = RequireType(
                "UnityEngine.InputForUI.PointerEvent"
            );
            private static readonly Type PointerType =
                PointerEvent.GetNestedType("Type") ?? throw Missing("PointerEvent.Type");
            private static readonly Type PointerButton =
                PointerEvent.GetNestedType("Button") ?? throw Missing("PointerEvent.Button");
            private static readonly Type ButtonsState =
                PointerEvent.GetNestedType("ButtonsState")
                ?? throw Missing("PointerEvent.ButtonsState");
            private static readonly Type EventSource = RequireType(
                "UnityEngine.InputForUI.EventSource"
            );
            private static readonly Type DefaultEventSystem = RequireUiType(
                "UnityEngine.UIElements.DefaultEventSystem"
            );
            private static object? eventSystem;
            private static object? processor;
            private static MethodInfo? processPointerEvent;

            internal static void Dispatch(
                int pointerId,
                Vector2 bottomLeftPosition,
                Vector2 bottomLeftDelta,
                int buttons,
                EventType type,
                uint changedButton
            )
            {
                Begin();
                object value = Activator.CreateInstance(PointerEvent)!;
                Set(value, "type", Enum.ToObject(PointerType, (int)type));
                Set(value, "pointerIndex", pointerId == 0 ? 0 : pointerId - 1);
                Set(
                    value,
                    "position",
                    new Vector2(bottomLeftPosition.x, Screen.height - bottomLeftPosition.y)
                );
                Set(value, "deltaPosition", new Vector2(bottomLeftDelta.x, -bottomLeftDelta.y));
                Set(value, "displayIndex", 0);
                Set(value, "pressure", buttons == 0 ? 0f : 0.5f);
                Set(value, "button", Enum.ToObject(PointerButton, changedButton));
                object buttonState = Activator.CreateInstance(ButtonsState)!;
                Field(buttonState.GetType(), "_state").SetValue(buttonState, (uint)buttons);
                Set(value, "buttonsState", buttonState);
                Set(value, "clickCount", 1);
                Field(PointerEvent, "<eventSource>k__BackingField")
                    .SetValue(value, Enum.ToObject(EventSource, pointerId == 0 ? 3 : 5));
                processPointerEvent!.Invoke(processor, new[] { value });
            }

            internal static void End()
            {
                if (eventSystem is null)
                    return;
                processor!.GetType().GetMethod("Reset", InstanceField)!.Invoke(processor, null);
                eventSystem = null;
                processor = null;
                processPointerEvent = null;
            }

            private static void Begin()
            {
                if (eventSystem is not null)
                    return;
                eventSystem =
                    Activator.CreateInstance(DefaultEventSystem)
                    ?? throw Missing("DefaultEventSystem instance");
                MethodInfo getProcessor =
                    DefaultEventSystem.GetMethod("get_inputForUIProcessor", InstanceField)
                    ?? throw Missing("DefaultEventSystem.inputForUIProcessor");
                processor =
                    getProcessor.Invoke(eventSystem, null)
                    ?? throw Missing("DefaultEventSystem input processor");
                processPointerEvent =
                    processor
                        .GetType()
                        .GetMethod(
                            "ProcessPointerEvent",
                            InstanceField,
                            null,
                            new[] { PointerEvent },
                            null
                        )
                    ?? throw Missing("InputForUIProcessor.ProcessPointerEvent");
            }

            private static void Set(object target, string name, object value) =>
                Field(target.GetType(), name).SetValue(target, value);

            private static FieldInfo Field(Type type, string name) =>
                type.GetField(name, InstanceField) ?? throw Missing($"{type.Name}.{name}");

            private static Type RequireType(string name) =>
                Type.GetType($"{name}, UnityEngine.InputForUIModule", throwOnError: false)
                ?? throw Missing(name);

            private static Type RequireUiType(string name) =>
                typeof(VisualElement).Assembly.GetType(name, throwOnError: false)
                ?? throw Missing(name);

            private static InvalidOperationException Missing(string member) =>
                new($"Unity's production raw UI pointer seam is unavailable: {member}.");
        }
    }
}
