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
                pointer.CancelGestures();
            }
        }

        public void Suspend()
        {
            foreach (PointerState pointer in pointers.Values)
            {
                pointer.Target = null;
                pointer.Hit = default;
                pointer.CancelGestures();
            }
        }

        public void Reset()
        {
            foreach (PointerState pointer in pointers.Values)
            {
                pointer.CancelGestures();
            }

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
            state.UpdateDrag(sample.Position);
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
                state.CancelGestures();
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

                    if (!BeginDrag(state, target, pointerId, sample.Position, button))
                    {
                        Reset();
                        return;
                    }
                }
                else if (!isPressed && wasPressed)
                {
                    bool wasCancelled = state.IsCancelled(button);
                    bool wasDragging = state.IsDragging(button);
                    MasonryIdentity? pressedTarget = state.Release(button);
                    if (wasCancelled)
                    {
                        continue;
                    }

                    if (!EndDrag(state, pointerId, sample.Position, button))
                    {
                        Reset();
                        return;
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
                        !wasDragging
                        && ReferenceEquals(pressedTarget, target)
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

        private bool BeginDrag(
            PointerState state,
            MasonryIdentity? identity,
            int pointerId,
            UnityEngine.Vector2 screen,
            PointerButton button
        )
        {
            if (button != PointerButton.Left || identity == null)
            {
                return true;
            }

            if (identity.DragMode is not DragMode mode || IsDragged(identity))
            {
                return true;
            }

            Camera camera = raycaster!.eventCamera;
            DragState drag = DragState.Create(identity, mode, camera, screen);
            if (!emit(DragAction(identity, pointerId, screen, drag.StartPosition, true)))
            {
                return false;
            }

            if (!drag.IsAvailable)
            {
                return true;
            }

            state.StartDrag(button, drag);
            state.UpdateDrag(screen);
            return true;
        }

        private bool EndDrag(
            PointerState state,
            int pointerId,
            UnityEngine.Vector2 screen,
            PointerButton button
        )
        {
            DragState? drag = state.EndDrag(button);
            if (drag is null || !drag.IsAvailable)
            {
                return true;
            }

            return emit(
                DragAction(
                    drag.Identity,
                    pointerId,
                    screen,
                    drag.Identity.transform.position,
                    false
                )
            );
        }

        private bool IsDragged(MasonryIdentity identity) =>
            pointers.Values.Any(state => ReferenceEquals(state.DragIdentity, identity));

        private static ActionBody DragAction(
            MasonryIdentity identity,
            int pointerId,
            UnityEngine.Vector2 screen,
            UnityEngine.Vector3 world,
            bool isStart
        )
        {
            var objectId = new ObjectId(identity.Id);
            var position = new ScreenPosition(screen.x, screen.y);
            var worldPosition = new ProtocolVector3(world.x, world.y, world.z);
            return isStart
                ? new ActionBody.DragStart(objectId, position, worldPosition, pointerId)
                : new ActionBody.DragEnd(objectId, position, worldPosition, pointerId);
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
            private DragState? drag;
            private PointerButton dragButton;

            public PointerState(UnityEngine.Vector2 position) => Position = position;

            public UnityEngine.Vector2 Position { get; set; }

            public MasonryIdentity? Target { get; set; }

            public UnityEngine.Vector3 Hit { get; set; }

            public MasonryIdentity? DragIdentity => drag?.Identity;

            public bool IsPressed(PointerButton button) => buttons.Contains(button);

            public bool IsCancelled(PointerButton button) => cancelledButtons.Contains(button);

            public bool IsDragging(PointerButton button) =>
                drag is not null && dragButton == button;

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

            public void StartDrag(PointerButton button, DragState value)
            {
                dragButton = button;
                drag = value;
            }

            public DragState? EndDrag(PointerButton button)
            {
                if (!IsDragging(button))
                {
                    return null;
                }

                DragState value = drag!;
                drag = null;
                return value;
            }

            public void UpdateDrag(UnityEngine.Vector2 position)
            {
                if (drag is not null && drag.IsAvailable)
                {
                    drag.Update(position);
                }
            }

            public void SetButtons(IEnumerable<PointerButton> values)
            {
                buttons.Clear();
                buttons.UnionWith(values);
                cancelledButtons.IntersectWith(buttons);
            }

            public void CancelGestures()
            {
                cancelledButtons.UnionWith(buttons);
                presses.Clear();
                drag?.Restore();
                drag = null;
            }

            public void CancelUnavailablePresses()
            {
                if (drag is not null && !drag.IsAvailable)
                {
                    cancelledButtons.Add(dragButton);
                    drag.Restore();
                    drag = null;
                }

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

        private sealed class DragState
        {
            private readonly Camera camera;
            private readonly Plane plane;
            private readonly UnityEngine.Vector3 offset;

            private DragState(
                MasonryIdentity identity,
                Camera inputCamera,
                Plane movementPlane,
                UnityEngine.Vector3 pickupOffset
            )
            {
                Identity = identity;
                camera = inputCamera;
                plane = movementPlane;
                offset = pickupOffset;
                StartPosition = identity.transform.position;
            }

            public MasonryIdentity Identity { get; }

            public bool IsAvailable =>
                Identity != null
                && Identity.gameObject != null
                && Identity.IsAvailableForPointerInput;

            public UnityEngine.Vector3 StartPosition { get; }

            public static DragState Create(
                MasonryIdentity identity,
                DragMode mode,
                Camera camera,
                UnityEngine.Vector2 screen
            )
            {
                UnityEngine.Vector3 start = identity.transform.position;
                var plane = new Plane(DragPlaneNormal(camera), start);
                UnityEngine.Vector3 pointer = PointOnPlane(camera, plane, screen);
                UnityEngine.Vector3 offset =
                    mode == DragMode.PreserveOffset ? start - pointer : UnityEngine.Vector3.zero;
                return new DragState(identity, camera, plane, offset);
            }

            public void Update(UnityEngine.Vector2 screen) =>
                Identity.transform.position = PointOnPlane(camera, plane, screen) + offset;

            public void Restore()
            {
                if (Identity != null && Identity.gameObject != null)
                {
                    Identity.transform.position = StartPosition;
                }
            }

            private static UnityEngine.Vector3 PointOnPlane(
                Camera camera,
                Plane plane,
                UnityEngine.Vector2 screen
            )
            {
                Ray ray = camera.ScreenPointToRay(screen);
                if (!plane.Raycast(ray, out float distance))
                {
                    throw new InvalidOperationException(
                        "Pointer ray did not intersect drag plane."
                    );
                }

                return ray.GetPoint(distance);
            }

            private static UnityEngine.Vector3 DragPlaneNormal(Camera camera)
            {
                // Drag on the axis-aligned plane that most directly faces the camera. The plane
                // passes through the object's pickup position, so angled board cameras typically
                // select a horizontal XZ plane while front- and side-facing cameras select XY
                // or YZ.
                UnityEngine.Vector3 facing = camera.transform.forward;
                UnityEngine.Vector3 magnitude = new(
                    Mathf.Abs(facing.x),
                    Mathf.Abs(facing.y),
                    Mathf.Abs(facing.z)
                );
                if (magnitude.x >= magnitude.y && magnitude.x >= magnitude.z)
                {
                    return UnityEngine.Vector3.right;
                }

                return magnitude.y >= magnitude.z
                    ? UnityEngine.Vector3.up
                    : UnityEngine.Vector3.forward;
            }
        }
    }
}
