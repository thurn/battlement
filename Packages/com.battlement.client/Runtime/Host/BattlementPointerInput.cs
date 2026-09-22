#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using Battlement.UI;
using UnityEngine;
using UnityEngine.EventSystems;
using UnityEngine.InputSystem.UI;
using ControlledPointerSample = Battlement.BattlementControlledPointerSample;
using Object = UnityEngine.Object;
using ProtocolVector3 = Battlement.Vector3;

namespace Battlement
{
    /// <summary>Raycasts Input System pointer devices and emits ordered core actions.</summary>
    internal sealed class BattlementPointerInput : IDisposable
    {
        private readonly Func<ActionBody, bool> emit;
        private readonly Dictionary<int, PointerState> pointers = new();
        private readonly List<RaycastResult> raycastResults = new();
        private readonly EventSystem eventSystem;
        private readonly InputSystemUIInputModule inputModule;
        private readonly GameObject? ownedEventSystemObject;
        private readonly InputSystemUIInputModule? ownedInputModule;
        private readonly BattlementControlledPointerSource controlled = new();
        private readonly SortedDictionary<int, BattlementControlledPointerSample> controlledState =
            new();
        private PhysicsRaycaster? raycaster;
        private bool ownsRaycaster;
        private bool? inputModuleEnabledBeforeDitto;
        private BattlementLogicalPointerInput? logical;
        private Func<bool> modalBlocked = () => false;
        private Func<int, UnityEngine.Vector2, bool> blocksWorld = (_, _) => false;
        private Func<
            int,
            UnityEngine.Vector2,
            int,
            bool,
            bool,
            BattlementUiControlledPointerResult
        >? controlledUi;
        private System.Action? resetControlledUi;

        internal bool IsWorldCaptured(int id) =>
            logical?.IsCaptured(BattlementPointerDevices.WorldPointerId(id)) == true;

        internal BattlementControlledPointerSpace ControlledSpace =>
            new(
                Screen.width,
                Screen.height,
                raycaster != null && raycaster.eventCamera != null
                    ? raycaster.eventCamera.GetEntityId().ToString()
                    : null
            );

        internal void ConfigureControlledUi(
            Func<
                int,
                UnityEngine.Vector2,
                int,
                bool,
                bool,
                BattlementUiControlledPointerResult
            > process,
            System.Action reset
        ) => (controlledUi, resetControlledUi) = (process, reset);

        internal void ConfigureLogical(
            Func<UiEvent, UiEventDisposition?> emitEvent,
            Func<int, UnityEngine.Vector2, bool> blocks,
            Func<bool> modal
        )
        {
            logical = new BattlementLogicalPointerInput(emitEvent);
            blocksWorld = blocks;
            modalBlocked = modal;
        }

        public BattlementPointerInput(Transform owner, Func<ActionBody, bool> emitAction)
        {
            emit = emitAction;
            eventSystem = Object.FindAnyObjectByType<EventSystem>(FindObjectsInactive.Include);
            if (eventSystem == null)
            {
                ownedEventSystemObject = new GameObject("Battlement EventSystem");
                ownedEventSystemObject.transform.SetParent(owner, false);
                eventSystem = ownedEventSystemObject.AddComponent<EventSystem>();
            }

            if (!eventSystem.TryGetComponent(out inputModule))
            {
                ownedInputModule = eventSystem.gameObject.AddComponent<InputSystemUIInputModule>();
                ownedInputModule.AssignDefaultActions();
                inputModule = ownedInputModule;
            }
        }

        public (TimeSpan Delay, TimeSpan Interval) NavigationTiming =>
            (
                TimeSpan.FromSeconds(inputModule.moveRepeatDelay),
                TimeSpan.FromSeconds(inputModule.moveRepeatRate)
            );

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
                raycaster = camera.gameObject.AddComponent<BattlementPhysicsRaycaster>();
                ownsRaycaster = true;
            }
        }

        public void Update(bool isInputAvailable, bool readPhysical = true)
        {
            if (controlled.IsActive)
            {
                if (!isInputAvailable)
                {
                    if (controlled.HasPendingSamples)
                        FailControlled("Controlled pointer input became unavailable.");
                    return;
                }
                ProcessControlled();
                return;
            }
            if (!readPhysical)
                return;
            SortedDictionary<int, BattlementPointerSample> samples = BattlementPointerDevices.Read(
                pointers.Keys
            );
            if (!isInputAvailable || raycaster == null)
            {
                SynchronizeWithoutEmitting(samples);
                return;
            }

            foreach (int pointerId in samples.Keys.Union(pointers.Keys).OrderBy(id => id).ToArray())
            {
                bool isPresent = samples.TryGetValue(pointerId, out BattlementPointerSample sample);
                PointerState state = GetState(pointerId, sample.Position);
                if (!isPresent)
                {
                    sample = BattlementPointerSample.Absent(state.Position);
                }

                Process(pointerId, state, sample, null);
                if (!isPresent)
                {
                    pointers.Remove(pointerId);
                }
            }
        }

        public void CancelPresses()
        {
            resetControlledUi?.Invoke();
            logical?.Reset();
            controlledState.Clear();
            foreach (PointerState pointer in pointers.Values)
            {
                pointer.CancelGestures();
            }
        }

        public BattlementControlledPointerLease BeginDittoControl(string session)
        {
            if (inputModuleEnabledBeforeDitto is not null)
                throw new InvalidOperationException("Ditto already controls native input.");
            inputModuleEnabledBeforeDitto = inputModule.enabled;
            inputModule.enabled = false;
            Suspend();
            return controlled.Begin(session);
        }

        public void EndDittoControl(BattlementControlledPointerLease lease)
        {
            if (inputModuleEnabledBeforeDitto is not bool wasEnabled)
                throw new InvalidOperationException("Ditto does not control native input.");
            try
            {
                controlled.End(lease);
                Reset();
            }
            finally
            {
                inputModule.enabled = wasEnabled;
                inputModuleEnabledBeforeDitto = null;
            }
        }

        public void EnqueueControlled(BattlementControlledPointerSample sample) =>
            controlled.Enqueue(sample);

        public IReadOnlyList<BattlementControlledPointerReceipt> TakeControlledReceipts(
            BattlementControlledPointerLease lease
        ) => controlled.TakeReceipts(lease);

        public string? ControlledFailure => controlled.Failure;

        public void FailControlled(string reason)
        {
            controlled.Fail(reason);
            Reset();
        }

        public void Suspend()
        {
            resetControlledUi?.Invoke();
            logical?.Reset();
            controlledState.Clear();
            foreach (PointerState pointer in pointers.Values)
            {
                pointer.Target = null;
                pointer.Hit = default;
                pointer.CancelGestures();
            }
        }

        public void Reset()
        {
            resetControlledUi?.Invoke();
            logical?.Reset();
            controlledState.Clear();
            foreach (PointerState pointer in pointers.Values)
            {
                pointer.CancelGestures();
            }

            pointers.Clear();
            raycastResults.Clear();
        }

        public void Dispose()
        {
            logical?.Reset();
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

        private BattlementPointerProcessingResult Process(
            int pointerId,
            PointerState state,
            BattlementPointerSample sample,
            BattlementUiControlledPointerResult? ui
        )
        {
            bool blocked = modalBlocked();
            if (blocked)
                state.CancelGestures();
            else
                state.UpdateDrag(sample.Position);
            PointerHit hit = sample.IsPresent ? Raycast(pointerId, sample.Position) : default;
            BattlementIdentity? target = hit.Identity;
            // A native drag owns its gesture through release, even when it crosses
            // a target with logical pointer handlers (for example, a captured piece).
            if (
                state.DragIdentity == null
                && logical?.Process(pointerId, sample, target, blocked, state.Buttons) == true
            )
            {
                state.CancelGestures();
                state.Target = null;
                state.Position = sample.Position;
                state.SetButtons(sample.Buttons);
                return Result(
                    ui,
                    target,
                    logical.CaptureOwner(pointerId),
                    target == null ? "none" : "world-logical"
                );
            }
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
                    return Result(ui, target, null, "rejected");
                }

                state.Target = target;
                state.Hit = hit.World;
                if (!EmitHover(target, PointerEvent.Enter, pointerId, sample.Position, hit.World))
                {
                    Reset();
                    return Result(ui, target, null, "rejected");
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
                return Result(ui, target, DragIdentity(state), Route(ui, target, state));
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
                        return Result(ui, target, null, "rejected");
                    }

                    if (!BeginDrag(state, target, pointerId, sample.Position, button))
                    {
                        Reset();
                        return Result(ui, target, null, "rejected");
                    }
                }
                else if (!isPressed && wasPressed)
                {
                    bool wasCancelled = state.IsCancelled(button);
                    bool wasDragging = state.IsDragging(button);
                    BattlementIdentity? pressedTarget = state.Release(button);
                    if (wasCancelled)
                    {
                        continue;
                    }

                    if (!EndDrag(state, pointerId, sample.Position, button))
                    {
                        Reset();
                        return Result(ui, target, null, "rejected");
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
                        return Result(ui, target, null, "rejected");
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
                        return Result(ui, target, null, "rejected");
                    }
                }
            }
            return Result(ui, target, DragIdentity(state), Route(ui, target, state));
        }

        private void ProcessControlled()
        {
            var processed = new HashSet<int>();
            foreach (BattlementControlledPointerSample input in controlled.Drain())
            {
                processed.Add(input.PointerId);
                if (!ProcessControlled(input, recordReceipt: true))
                    return;
                if (input.IsPresent && !input.IsCancelled)
                    controlledState[input.PointerId] = input;
                else
                    controlledState.Remove(input.PointerId);
            }

            foreach (
                BattlementControlledPointerSample retained in controlledState
                    .Values.Where(value => !processed.Contains(value.PointerId))
                    .ToArray()
            )
                if (!ProcessControlled(retained, recordReceipt: false))
                    return;
        }

        private bool ProcessControlled(ControlledPointerSample input, bool recordReceipt)
        {
            if (input.Space != ControlledSpace)
            {
                FailControlled(
                    $"Controlled pointer sample {input.Sequence} does not match "
                        + "the active viewport and camera."
                );
                return false;
            }
            BattlementPointerSample sample = new(
                input.Position,
                input.Buttons.ToHashSet(),
                input.IsPresent,
                input.IsCancelled
            );
            BattlementUiControlledPointerResult? ui = controlledUi?.Invoke(
                input.PointerId,
                sample.Position,
                UiButtons(sample.Buttons),
                sample.IsPresent,
                sample.IsCancelled
            );
            PointerState state = GetState(input.PointerId, sample.Position);
            BattlementPointerProcessingResult result = Process(input.PointerId, state, sample, ui);
            if (!sample.IsPresent)
                pointers.Remove(input.PointerId);
            if (result.Route == "rejected")
            {
                FailControlled(
                    $"Controlled pointer sample {input.Sequence} was rejected by the host."
                );
                return false;
            }
            if (recordReceipt)
                controlled.Record(
                    new BattlementControlledPointerReceipt(
                        input.Lease.Session,
                        input.Lease.Generation,
                        input.Sequence,
                        input.PointerId,
                        input.Position,
                        input.ExpectedTarget,
                        result.Hit,
                        result.Capture,
                        result.Route,
                        input.PresentationBoundary
                    )
                );
            return true;
        }

        private static BattlementPointerProcessingResult Result(
            BattlementUiControlledPointerResult? ui,
            BattlementIdentity? world,
            ObjectId? capture,
            string route
        ) =>
            new(
                ui?.Hit ?? (world == null ? null : new ObjectId(world.Id)),
                ui?.Capture ?? capture,
                ui?.Handled == true && world == null ? "ui-toolkit" : route
            );

        private static ObjectId? DragIdentity(PointerState state) =>
            state.DragIdentity is BattlementIdentity value ? new ObjectId(value.Id) : null;

        private static int UiButtons(IReadOnlyCollection<PointerButton> buttons) =>
            (buttons.Contains(PointerButton.Left) ? 1 : 0)
            | (buttons.Contains(PointerButton.Right) ? 2 : 0)
            | (buttons.Contains(PointerButton.Middle) ? 4 : 0);

        private static string Route(
            BattlementUiControlledPointerResult? ui,
            BattlementIdentity? world,
            PointerState state
        ) =>
            ui?.Handled == true && world == null ? "ui-toolkit"
            : state.DragIdentity != null ? "world-drag"
            : world != null ? "world-pointer"
            : "none";

        private bool BeginDrag(
            PointerState state,
            BattlementIdentity? identity,
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

        private bool IsDragged(BattlementIdentity identity) =>
            pointers.Values.Any(state => ReferenceEquals(state.DragIdentity, identity));

        private static ActionBody DragAction(
            BattlementIdentity identity,
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
            BattlementIdentity? identity,
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
            BattlementIdentity? identity,
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

        internal BattlementIdentity? PickWorld(int pointerId, UnityEngine.Vector2 position) =>
            raycaster == null ? null : Raycast(pointerId, position).Identity;

        private PointerHit Raycast(int pointerId, UnityEngine.Vector2 position)
        {
            raycastResults.Clear();
            var eventData = new PointerEventData(eventSystem)
            {
                pointerId = pointerId,
                position = position,
            };
            if (blocksWorld(pointerId, position))
                return default;
            foreach (BaseRaycaster module in RaycasterManager.GetRaycasters())
            {
                if (module == null || !module.IsActive() || module is PhysicsRaycaster)
                    continue;
                module.Raycast(eventData, raycastResults);
                if (raycastResults.Count != 0)
                    return default;
            }
            raycaster!.Raycast(eventData, raycastResults);
            if (raycastResults.Count == 0)
            {
                return default;
            }

            foreach (
                RaycastResult result in raycastResults
                    .Where(value => value.module == raycaster)
                    .OrderByDescending(value =>
                        PointerSettings(value.gameObject)?.InteractionLayer ?? 0
                    )
                    .ThenBy(value => value.distance)
                    .ThenByDescending(value => PointerSettings(value.gameObject)?.Order ?? 0)
            )
            {
                BattlementIdentity? identity = BattlementIdentity.FindNearest(result.gameObject);
                if (identity != null && !identity.IsAvailableForPointerInput)
                    continue;
                if (
                    identity != null
                    && identity.WorldPointer is not null
                    && !identity.HasPointerEvents
                )
                    continue;
                if (!BattlementWorldDocumentCollider.IsGenerated(result.gameObject))
                    return new PointerHit(identity, result.worldPosition);
            }
            return default;
        }

        private static WorldPointerSettings? PointerSettings(GameObject target)
        {
            BattlementIdentity? identity = BattlementIdentity.FindNearest(target);
            return identity == null ? null : identity.WorldPointer;
        }

        private void SynchronizeWithoutEmitting(
            SortedDictionary<int, BattlementPointerSample> samples
        )
        {
            Suspend();

            foreach ((int id, BattlementPointerSample sample) in samples)
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

        private static bool CanEmit(BattlementIdentity? identity, PointerEvent pointerEvent)
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
            public PointerHit(BattlementIdentity? identity, UnityEngine.Vector3 world) =>
                (Identity, World) = (identity, world);

            public BattlementIdentity? Identity { get; }

            public UnityEngine.Vector3 World { get; }
        }

        private readonly struct BattlementPointerProcessingResult
        {
            internal BattlementPointerProcessingResult(
                ObjectId? hit,
                ObjectId? capture,
                string route
            ) => (Hit, Capture, Route) = (hit, capture, route);

            internal ObjectId? Hit { get; }
            internal ObjectId? Capture { get; }
            internal string Route { get; }
        }

        private sealed class PointerState
        {
            private readonly Dictionary<PointerButton, BattlementIdentity?> presses = new();
            private readonly HashSet<PointerButton> buttons = new();
            private readonly HashSet<PointerButton> cancelledButtons = new();
            private DragState? drag;
            private PointerButton dragButton;

            public PointerState(UnityEngine.Vector2 position) => Position = position;

            public UnityEngine.Vector2 Position { get; set; }

            public BattlementIdentity? Target { get; set; }

            public UnityEngine.Vector3 Hit { get; set; }

            public BattlementIdentity? DragIdentity => drag?.Identity;

            public IEnumerable<PointerButton> Buttons => buttons;

            public bool IsPressed(PointerButton button) => buttons.Contains(button);

            public bool IsCancelled(PointerButton button) => cancelledButtons.Contains(button);

            public bool IsDragging(PointerButton button) =>
                drag is not null && dragButton == button;

            public void Press(PointerButton button, BattlementIdentity? target)
            {
                buttons.Add(button);
                cancelledButtons.Remove(button);
                presses[button] = target;
            }

            public BattlementIdentity? Release(PointerButton button)
            {
                buttons.Remove(button);
                cancelledButtons.Remove(button);
                presses.Remove(button, out BattlementIdentity? target);
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
                    BattlementIdentity? target = presses[button];
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
                BattlementIdentity identity,
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

            public BattlementIdentity Identity { get; }

            public bool IsAvailable =>
                Identity != null
                && Identity.gameObject != null
                && Identity.IsAvailableForPointerInput;

            public UnityEngine.Vector3 StartPosition { get; }

            public static DragState Create(
                BattlementIdentity identity,
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
