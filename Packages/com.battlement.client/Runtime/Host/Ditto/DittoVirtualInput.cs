#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.InputSystem.LowLevel;
using InputTouchPhase = UnityEngine.InputSystem.TouchPhase;

namespace Battlement
{
    internal enum DittoInputFrameKind
    {
        Move,
        Press,
        Release,
        TouchBegin,
        TouchMove,
        TouchEnd,
        KeyDown,
        KeyUp,
    }

    internal sealed record DittoInputFrame(
        ulong Id,
        DittoInputFrameKind Kind,
        Vector2? Position = null,
        Key? Key = null,
        string? TransactionId = null
    );

    internal sealed class DittoVirtualInput : IDisposable
    {
        internal const string VirtualMouseName = "Ditto Virtual Mouse";
        internal const string VirtualTouchscreenName = "Ditto Virtual Touchscreen";

        private const float DragSegmentLength = 0.05f;
        private const int TouchId = 1;
        private static readonly Key[] CommandModifiers =
        {
            UnityEngine.InputSystem.Key.LeftAlt,
            UnityEngine.InputSystem.Key.RightAlt,
            UnityEngine.InputSystem.Key.LeftCtrl,
            UnityEngine.InputSystem.Key.RightCtrl,
            UnityEngine.InputSystem.Key.LeftMeta,
            UnityEngine.InputSystem.Key.RightMeta,
        };

        private readonly Queue<DittoInputFrame> frames = new();
        private readonly HashSet<Key> heldKeys = new();
        private readonly DittoPlatform platform;
        private readonly uint width;
        private readonly uint height;
        private Mouse? mouse;
        private Touchscreen? touchscreen;
        private InputDevice? awaitingDevice;
        private ulong awaitingFrameId;
        private ulong lastAppliedFrameId;
        private bool pointerHeld;
        private ulong nextFrameId;
        private bool disposed;

        public DittoVirtualInput(DittoPlatform platform, uint width, uint height)
        {
            if (width == 0 || height == 0)
            {
                throw new ArgumentOutOfRangeException(
                    nameof(width),
                    "The render surface is empty."
                );
            }

            this.platform = platform;
            this.width = width;
            this.height = height;
            InputSystem.onAfterUpdate += ObserveInputUpdate;
            if (platform != DittoPlatform.IosSimulator)
            {
                mouse = InputSystem.AddDevice<Mouse>(VirtualMouseName);
                mouse.MakeCurrent();
            }
        }

        public int PendingFrameCount => frames.Count + (awaitingDevice is null ? 0 : 1);

        public bool CanQueueNextFrame => awaitingDevice is null && frames.Count != 0;

        public bool SupportsHover => platform != DittoPlatform.IosSimulator;

        public bool HasHeldInput => pointerHeld || heldKeys.Count > 0;

        public ulong LastAppliedFrameId => lastAppliedFrameId;

        public ulong Click(Vector2 position, string transactionId)
        {
            RequireIdle();
            if (platform == DittoPlatform.IosSimulator)
            {
                Enqueue(DittoInputFrameKind.TouchBegin, position, transactionId: transactionId);
                return Enqueue(
                    DittoInputFrameKind.TouchEnd,
                    position,
                    transactionId: transactionId
                );
            }

            Enqueue(DittoInputFrameKind.Move, position, transactionId: transactionId);
            Enqueue(DittoInputFrameKind.Press, position, transactionId: transactionId);
            return Enqueue(DittoInputFrameKind.Release, position, transactionId: transactionId);
        }

        public bool Hover(Vector2 position)
        {
            RequireIdle();
            if (!SupportsHover)
            {
                return false;
            }

            Enqueue(DittoInputFrameKind.Move, position);
            return true;
        }

        public int Drag(Vector2 from, Vector2 to)
        {
            RequireIdle();
            int segmentCount = DragSegmentCount(from, to);
            if (platform == DittoPlatform.IosSimulator)
            {
                Enqueue(DittoInputFrameKind.TouchBegin, from);
                EnqueueSegments(from, to, segmentCount, DittoInputFrameKind.TouchMove);
                Enqueue(DittoInputFrameKind.TouchEnd, to);
                return segmentCount;
            }

            Enqueue(DittoInputFrameKind.Move, from);
            Enqueue(DittoInputFrameKind.Press, from);
            EnqueueSegments(from, to, segmentCount, DittoInputFrameKind.Move);
            Enqueue(DittoInputFrameKind.Release, to);
            return segmentCount;
        }

        public void Key(string value, DittoKeyAction action)
        {
            RequireIdle();
            if (
                !Enum.TryParse(value, false, out Key key)
                || Enum.GetName(typeof(Key), key) != value
            )
            {
                throw new ArgumentException(
                    $"Unknown Unity Input System key '{value}'.",
                    nameof(value)
                );
            }
            if (key == UnityEngine.InputSystem.Key.None)
            {
                throw new ArgumentException("Key.None cannot be authored.", nameof(value));
            }

            switch (action)
            {
                case DittoKeyAction.Down:
                    RequireKeyState(key, false);
                    Enqueue(DittoInputFrameKind.KeyDown, key: key);
                    break;
                case DittoKeyAction.Up:
                    RequireKeyState(key, true);
                    Enqueue(DittoInputFrameKind.KeyUp, key: key);
                    break;
                case DittoKeyAction.Tap:
                    RequireKeyState(key, false);
                    Enqueue(DittoInputFrameKind.KeyDown, key: key);
                    Enqueue(DittoInputFrameKind.KeyUp, key: key);
                    break;
                default:
                    throw new ArgumentOutOfRangeException(nameof(action));
            }
        }

        public DittoInputFrame QueueNextFrame()
        {
            ThrowIfDisposed();
            if (!frames.TryDequeue(out DittoInputFrame frame))
            {
                throw new InvalidOperationException("No virtual input frame is pending.");
            }

            if (frame.Key is Key key)
            {
                QueueKey(key, frame.Kind == DittoInputFrameKind.KeyDown);
            }
            else if (platform == DittoPlatform.IosSimulator)
            {
                QueueTouch(frame);
            }
            else
            {
                QueueMouse(frame);
            }
            Debug.Log(
                $"[Battlement/Ditto-trace] input-dequeued id={frame.Id} kind={frame.Kind} "
                    + $"transaction={frame.TransactionId ?? "none"} "
                    + $"position={frame.Position?.ToString() ?? "none"} "
                    + $"key={frame.Key?.ToString() ?? "none"}"
            );
            return frame;
        }

        public string? HeldInputDiagnostic()
        {
            if (pointerHeld)
            {
                return "Ditto virtual pointer button remains held.";
            }
            return heldKeys.Count == 0
                ? null
                : "Ditto virtual keys remain held: "
                    + $"{string.Join(", ", heldKeys.OrderBy(key => key))}.";
        }

        public void Dispose()
        {
            if (disposed)
            {
                return;
            }

            disposed = true;
            InputSystem.onAfterUpdate -= ObserveInputUpdate;
            Remove(mouse);
            Remove(touchscreen);
            frames.Clear();
        }

        private int DragSegmentCount(Vector2 from, Vector2 to)
        {
            float horizontal = (to.x - from.x) / Math.Max(1, width - 1);
            float vertical = (to.y - from.y) / Math.Max(1, height - 1);
            return Math.Max(
                1,
                Mathf.CeilToInt(new Vector2(horizontal, vertical).magnitude / DragSegmentLength)
            );
        }

        private void EnqueueSegments(Vector2 from, Vector2 to, int count, DittoInputFrameKind kind)
        {
            for (int index = 1; index <= count; index++)
            {
                Enqueue(kind, Vector2.Lerp(from, to, (float)index / count));
            }
        }

        private ulong Enqueue(
            DittoInputFrameKind kind,
            Vector2? position = null,
            Key? key = null,
            string? transactionId = null
        )
        {
            var frame = new DittoInputFrame(++nextFrameId, kind, position, key, transactionId);
            frames.Enqueue(frame);
            Debug.Log(
                $"[Battlement/Ditto-trace] input-enqueued id={frame.Id} kind={frame.Kind} "
                    + $"transaction={frame.TransactionId ?? "none"} "
                    + $"position={frame.Position?.ToString() ?? "none"} "
                    + $"key={frame.Key?.ToString() ?? "none"}"
            );
            return frame.Id;
        }

        private void QueueMouse(DittoInputFrame frame)
        {
            pointerHeld = frame.Kind switch
            {
                DittoInputFrameKind.Press => true,
                DittoInputFrameKind.Release => false,
                _ => pointerHeld,
            };
            mouse ??= InputSystem.AddDevice<Mouse>(VirtualMouseName);
            awaitingDevice = mouse;
            awaitingFrameId = frame.Id;
            InputSystem.QueueStateEvent(
                mouse,
                new MouseState { position = ToInputPosition(frame.Position!.Value) }.WithButton(
                    MouseButton.Left,
                    pointerHeld
                )
            );
        }

        private void QueueTouch(DittoInputFrame frame)
        {
            InputTouchPhase phase = frame.Kind switch
            {
                DittoInputFrameKind.TouchBegin => InputTouchPhase.Began,
                DittoInputFrameKind.TouchMove => InputTouchPhase.Moved,
                DittoInputFrameKind.TouchEnd => InputTouchPhase.Ended,
                _ => throw new InvalidOperationException("Invalid iOS virtual input frame."),
            };
            pointerHeld = phase != InputTouchPhase.Ended;
            touchscreen ??= InputSystem.AddDevice<Touchscreen>(VirtualTouchscreenName);
            awaitingDevice = touchscreen;
            awaitingFrameId = frame.Id;
            InputSystem.QueueStateEvent(
                touchscreen,
                new TouchState
                {
                    touchId = TouchId,
                    phase = phase,
                    position = ToInputPosition(frame.Position!.Value),
                    pressure = pointerHeld ? 1 : 0,
                }
            );
        }

        private void QueueKey(Key key, bool pressed)
        {
            if (pressed)
            {
                heldKeys.Add(key);
            }
            else
            {
                heldKeys.Remove(key);
            }
        }

        private void ObserveInputUpdate()
        {
            if (
                awaitingDevice is not null
                && InputState.currentUpdateType == InputUpdateType.Dynamic
            )
            {
                Debug.Log(
                    $"[Battlement/Ditto-trace] input-system-consumed id={awaitingFrameId} "
                        + $"device={awaitingDevice.name} "
                        + $"update={InputState.currentUpdateType}"
                );
                lastAppliedFrameId = awaitingFrameId;
                awaitingFrameId = 0;
                awaitingDevice = null;
            }
        }

        public char? TextCharacter(Key key)
        {
            string name = key.ToString();
            if (name.Length == 1 && name[0] is >= 'A' and <= 'Z')
            {
                return
                    heldKeys.Contains(UnityEngine.InputSystem.Key.LeftShift)
                    || heldKeys.Contains(UnityEngine.InputSystem.Key.RightShift)
                    ? name[0]
                    : char.ToLowerInvariant(name[0]);
            }
            if (name.StartsWith("Digit", StringComparison.Ordinal) && name.Length == 6)
            {
                return name[5];
            }
            return key == UnityEngine.InputSystem.Key.Space ? ' ' : null;
        }

        private bool HasCommandModifier() => heldKeys.Overlaps(CommandModifiers);

        public bool HasCommandModifiers => HasCommandModifier();

        public bool IsHeld(Key key) => heldKeys.Contains(key);

        private Vector2 ToInputPosition(Vector2 position) =>
            new(position.x, height - 1 - position.y);

        private void RequireIdle()
        {
            ThrowIfDisposed();
            if (frames.Count > 0)
            {
                throw new InvalidOperationException("A virtual input sequence is already pending.");
            }
        }

        private void RequireKeyState(Key key, bool expectedHeld)
        {
            if (heldKeys.Contains(key) != expectedHeld)
            {
                throw new InvalidOperationException(
                    expectedHeld ? $"Key {key} is not held." : $"Key {key} is already held."
                );
            }
        }

        private void ThrowIfDisposed()
        {
            if (disposed)
            {
                throw new ObjectDisposedException(nameof(DittoVirtualInput));
            }
        }

        private static void Remove(InputDevice? device)
        {
            if (device?.added == true)
            {
                InputSystem.RemoveDevice(device);
            }
        }
    }
}
