#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;
using UnityEngine.EventSystems;
using UnityEngine.InputSystem.UI;
using Object = UnityEngine.Object;
using ProtocolVector3 = Masonry.Vector3;

namespace Masonry
{
    /// <summary>Raycasts Input System pointer devices and emits ordered core actions.</summary>
    internal sealed class MasonryPointerInput : IDisposable
    {
        private readonly Func<ActionBody, bool> emit;
        private readonly Dictionary<int, PointerState> pointers = new();
        private readonly List<RaycastResult> raycastResults = new();
        private readonly EventSystem eventSystem;
        private readonly GameObject? ownedEventSystemObject;
        private readonly InputSystemUIInputModule? ownedInputModule;
        private PhysicsRaycaster? raycaster;
        private bool ownsRaycaster;

        public MasonryPointerInput(Transform owner, Func<ActionBody, bool> emitAction)
        {
            emit = emitAction;
            eventSystem = Object.FindAnyObjectByType<EventSystem>(FindObjectsInactive.Include);
            if (eventSystem == null)
            {
                ownedEventSystemObject = new GameObject("Masonry EventSystem");
                ownedEventSystemObject.transform.SetParent(owner, false);
                eventSystem = ownedEventSystemObject.AddComponent<EventSystem>();
            }

            if (!eventSystem.TryGetComponent(out InputSystemUIInputModule _))
            {
                ownedInputModule = eventSystem.gameObject.AddComponent<InputSystemUIInputModule>();
                ownedInputModule.AssignDefaultActions();
            }
        }

        public void SetCamera(Camera? camera)
        {
            RemoveOwnedRaycaster();
            if (camera == null)
            {
                raycaster = null;
                return;
            }

            raycaster = camera.GetComponent<PhysicsRaycaster>();
            if (raycaster == null)
            {
                raycaster = camera.gameObject.AddComponent<PhysicsRaycaster>();
                ownsRaycaster = true;
            }
        }

        public void Update(bool isInputAvailable)
        {
            SortedDictionary<int, MasonryPointerSample> samples = MasonryPointerDevices.Read(
                pointers.Keys
            );
            if (!isInputAvailable || raycaster == null)
            {
                SynchronizeWithoutEmitting(samples);
                return;
            }

            foreach (int pointerId in samples.Keys.Union(pointers.Keys).OrderBy(id => id).ToArray())
            {
                bool isPresent = samples.TryGetValue(pointerId, out MasonryPointerSample sample);
                PointerState state = GetState(pointerId, sample.Position);
                if (!isPresent)
                {
                    sample = MasonryPointerSample.Absent(state.Position);
                }

                Process(pointerId, state, sample);
                if (!isPresent)
                {
                    pointers.Remove(pointerId);
                }
            }
        }

        public void CancelPresses()
        {
            foreach (PointerState pointer in pointers.Values)
            {
                pointer.CancelPresses();
            }
        }

        public void Suspend()
        {
            foreach (PointerState pointer in pointers.Values)
            {
                pointer.Target = null;
                pointer.Hit = default;
                pointer.CancelPresses();
            }
        }

        public void Reset()
        {
            pointers.Clear();
            raycastResults.Clear();
        }

        public void Dispose()
        {
            RemoveOwnedRaycaster();
            if (ownedInputModule != null)
            {
                Destroy(ownedInputModule);
            }

            if (ownedEventSystemObject != null)
            {
                Destroy(ownedEventSystemObject);
            }
        }

        private void Process(int pointerId, PointerState state, MasonryPointerSample sample)
        {
            PointerHit hit = sample.IsPresent ? Raycast(pointerId, sample.Position) : default;
            MasonryIdentity? target = hit.Identity;
            state.CancelUnavailablePresses();
            if (!ReferenceEquals(state.Target, target))
            {
                if (
                    !EmitHover(
                        state.Target,
                        PointerEvent.Exit,
                        pointerId,
                        sample.Position,
                        state.Hit
                    )
                )
                {
                    Reset();
                    return;
                }

                state.Target = target;
                state.Hit = hit.World;
                if (!EmitHover(target, PointerEvent.Enter, pointerId, sample.Position, hit.World))
                {
                    Reset();
                    return;
                }
            }
            else if (target != null)
            {
                state.Hit = hit.World;
            }

            state.Position = sample.Position;
            if (sample.IsCancelled)
            {
                state.CancelPresses();
                state.SetButtons(sample.Buttons);
                return;
            }

            foreach (PointerButton button in Enum.GetValues(typeof(PointerButton)))
            {
                bool wasPressed = state.IsPressed(button);
                bool isPressed = sample.Buttons.Contains(button);
                if (isPressed && !wasPressed)
                {
                    state.Press(button, target);
                    if (
                        !EmitButton(
                            target,
                            PointerEvent.Down,
                            pointerId,
                            sample.Position,
                            hit.World,
                            button
                        )
                    )
                    {
                        Reset();
                        return;
                    }
                }
                else if (!isPressed && wasPressed)
                {
                    bool wasCancelled = state.IsCancelled(button);
                    MasonryIdentity? pressedTarget = state.Release(button);
                    if (wasCancelled)
                    {
                        continue;
                    }

                    if (
                        !EmitButton(
                            target,
                            PointerEvent.Up,
                            pointerId,
                            sample.Position,
                            hit.World,
                            button
                        )
                    )
                    {
                        Reset();
                        return;
                    }

                    if (
                        ReferenceEquals(pressedTarget, target)
                        && !EmitButton(
                            target,
                            PointerEvent.Click,
                            pointerId,
                            sample.Position,
                            hit.World,
                            button
                        )
                    )
                    {
                        Reset();
                        return;
                    }
                }
            }
        }

        private bool EmitHover(
            MasonryIdentity? identity,
            PointerEvent pointerEvent,
            int pointerId,
            UnityEngine.Vector2 screen,
            UnityEngine.Vector3 world
        )
        {
            if (!CanEmit(identity, pointerEvent))
            {
                return true;
            }

            var position = new ScreenPosition(screen.x, screen.y);
            var hit = new ProtocolVector3(world.x, world.y, world.z);
            return emit(
                pointerEvent == PointerEvent.Enter
                    ? new ActionBody.PointerEnter(
                        new ObjectId(identity!.Id),
                        position,
                        hit,
                        pointerId
                    )
                    : new ActionBody.PointerExit(
                        new ObjectId(identity!.Id),
                        position,
                        hit,
                        pointerId
                    )
            );
        }

        private bool EmitButton(
            MasonryIdentity? identity,
            PointerEvent pointerEvent,
            int pointerId,
            UnityEngine.Vector2 screen,
            UnityEngine.Vector3 world,
            PointerButton button
        )
        {
            if (!CanEmit(identity, pointerEvent))
            {
                return true;
            }

            var objectId = new ObjectId(identity!.Id);
            var position = new ScreenPosition(screen.x, screen.y);
            var hit = new ProtocolVector3(world.x, world.y, world.z);
            return emit(
                pointerEvent switch
                {
                    PointerEvent.Down => new ActionBody.PointerDown(
                        objectId,
                        position,
                        hit,
                        pointerId,
                        button
                    ),
                    PointerEvent.Up => new ActionBody.PointerUp(
                        objectId,
                        position,
                        hit,
                        pointerId,
                        button
                    ),
                    PointerEvent.Click => new ActionBody.PointerClick(
                        objectId,
                        position,
                        hit,
                        pointerId,
                        button
                    ),
                    _ => throw new InvalidOperationException("Unknown pointer button event."),
                }
            );
        }

        private PointerHit Raycast(int pointerId, UnityEngine.Vector2 position)
        {
            raycastResults.Clear();
            var eventData = new PointerEventData(eventSystem)
            {
                pointerId = pointerId,
                position = position,
            };
            raycaster!.Raycast(eventData, raycastResults);
            if (raycastResults.Count == 0)
            {
                return default;
            }

            RaycastResult closest = raycastResults.OrderBy(result => result.distance).First();
            return new PointerHit(
                MasonryIdentity.FindNearest(closest.gameObject),
                closest.worldPosition
            );
        }

        private void SynchronizeWithoutEmitting(SortedDictionary<int, MasonryPointerSample> samples)
        {
            Suspend();

            foreach ((int id, MasonryPointerSample sample) in samples)
            {
                PointerState state = GetState(id, sample.Position);
                state.Position = sample.Position;
                state.SetButtons(sample.Buttons);
            }

            foreach (int id in pointers.Keys.Except(samples.Keys).ToArray())
            {
                pointers.Remove(id);
            }
        }

        private PointerState GetState(int id, UnityEngine.Vector2 position)
        {
            if (!pointers.TryGetValue(id, out PointerState state))
            {
                state = new PointerState(position);
                pointers.Add(id, state);
            }

            return state;
        }

        private void RemoveOwnedRaycaster()
        {
            if (ownsRaycaster && raycaster != null)
            {
                Destroy(raycaster);
            }

            raycaster = null;
            ownsRaycaster = false;
        }

        private static bool CanEmit(MasonryIdentity? identity, PointerEvent pointerEvent)
        {
            if (identity == null || !identity.IsAvailableForPointerInput)
            {
                return false;
            }

            return identity.IsPointerEventEnabled(pointerEvent);
        }

        private static void Destroy(Object value)
        {
            if (Application.isPlaying)
            {
                Object.Destroy(value);
            }
            else
            {
                Object.DestroyImmediate(value);
            }
        }

        private readonly struct PointerHit
        {
            public PointerHit(MasonryIdentity? identity, UnityEngine.Vector3 world) =>
                (Identity, World) = (identity, world);

            public MasonryIdentity? Identity { get; }

            public UnityEngine.Vector3 World { get; }
        }

        private sealed class PointerState
        {
            private readonly Dictionary<PointerButton, MasonryIdentity?> presses = new();
            private readonly HashSet<PointerButton> buttons = new();
            private readonly HashSet<PointerButton> cancelledButtons = new();

            public PointerState(UnityEngine.Vector2 position) => Position = position;

            public UnityEngine.Vector2 Position { get; set; }

            public MasonryIdentity? Target { get; set; }

            public UnityEngine.Vector3 Hit { get; set; }

            public bool IsPressed(PointerButton button) => buttons.Contains(button);

            public bool IsCancelled(PointerButton button) => cancelledButtons.Contains(button);

            public void Press(PointerButton button, MasonryIdentity? target)
            {
                buttons.Add(button);
                cancelledButtons.Remove(button);
                presses[button] = target;
            }

            public MasonryIdentity? Release(PointerButton button)
            {
                buttons.Remove(button);
                cancelledButtons.Remove(button);
                presses.Remove(button, out MasonryIdentity? target);
                return target;
            }

            public void SetButtons(IEnumerable<PointerButton> values)
            {
                buttons.Clear();
                buttons.UnionWith(values);
                cancelledButtons.IntersectWith(buttons);
            }

            public void CancelPresses()
            {
                cancelledButtons.UnionWith(buttons);
                presses.Clear();
            }

            public void CancelUnavailablePresses()
            {
                foreach (PointerButton button in presses.Keys.ToArray())
                {
                    MasonryIdentity? target = presses[button];
                    if (target == null || !target.IsAvailableForPointerInput)
                    {
                        cancelledButtons.Add(button);
                        presses.Remove(button);
                    }
                }
            }
        }
    }
}
