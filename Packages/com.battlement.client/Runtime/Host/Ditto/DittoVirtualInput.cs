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
        DittoInputFrameKind Kind,
        Vector2? Position = null,
        Key? Key = null
    );

    internal sealed class DittoVirtualInput : IDisposable
    {
        internal const string VirtualMouseName = "Ditto Virtual Mouse";

        private const float DragSegmentLength = 0.05f;
        private const int TouchId = 1;

        private readonly Queue<DittoInputFrame> frames = new();
        private readonly HashSet<Key> heldKeys = new();
        private readonly DittoPlatform platform;
        private readonly uint width;
        private readonly uint height;
        private Mouse? mouse;
        private Keyboard? keyboard;
        private Touchscreen? touchscreen;
        private bool pointerHeld;
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
            if (platform != DittoPlatform.IosSimulator)
            {
                mouse = InputSystem.AddDevice<Mouse>(VirtualMouseName);
                mouse.MakeCurrent();
                keyboard = InputSystem.AddDevice<Keyboard>("Ditto Virtual Keyboard");
                keyboard.MakeCurrent();
            }
        }

        public int PendingFrameCount => frames.Count;

        public bool SupportsHover => platform != DittoPlatform.IosSimulator;

        public bool HasHeldInput => pointerHeld || heldKeys.Count > 0;

        public void Click(Vector2 position)
        {
            RequireIdle();
            if (platform == DittoPlatform.IosSimulator)
            {
                frames.Enqueue(new DittoInputFrame(DittoInputFrameKind.TouchBegin, position));
                frames.Enqueue(new DittoInputFrame(DittoInputFrameKind.TouchEnd, position));
                return;
            }

            frames.Enqueue(new DittoInputFrame(DittoInputFrameKind.Move, position));
            frames.Enqueue(new DittoInputFrame(DittoInputFrameKind.Press, position));
            frames.Enqueue(new DittoInputFrame(DittoInputFrameKind.Release, position));
        }

        public bool Hover(Vector2 position)
        {
            RequireIdle();
            if (!SupportsHover)
            {
                return false;
            }

            frames.Enqueue(new DittoInputFrame(DittoInputFrameKind.Move, position));
            return true;
        }

        public int Drag(Vector2 from, Vector2 to)
        {
            RequireIdle();
            int segmentCount = DragSegmentCount(from, to);
            if (platform == DittoPlatform.IosSimulator)
            {
                frames.Enqueue(new DittoInputFrame(DittoInputFrameKind.TouchBegin, from));
                EnqueueSegments(from, to, segmentCount, DittoInputFrameKind.TouchMove);
                frames.Enqueue(new DittoInputFrame(DittoInputFrameKind.TouchEnd, to));
                return segmentCount;
            }

            frames.Enqueue(new DittoInputFrame(DittoInputFrameKind.Move, from));
            frames.Enqueue(new DittoInputFrame(DittoInputFrameKind.Press, from));
            EnqueueSegments(from, to, segmentCount, DittoInputFrameKind.Move);
            frames.Enqueue(new DittoInputFrame(DittoInputFrameKind.Release, to));
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
                    frames.Enqueue(new DittoInputFrame(DittoInputFrameKind.KeyDown, Key: key));
                    break;
                case DittoKeyAction.Up:
                    RequireKeyState(key, true);
                    frames.Enqueue(new DittoInputFrame(DittoInputFrameKind.KeyUp, Key: key));
                    break;
                case DittoKeyAction.Tap:
                    RequireKeyState(key, false);
                    frames.Enqueue(new DittoInputFrame(DittoInputFrameKind.KeyDown, Key: key));
                    frames.Enqueue(new DittoInputFrame(DittoInputFrameKind.KeyUp, Key: key));
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
            Remove(mouse);
            Remove(keyboard);
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
                frames.Enqueue(
                    new DittoInputFrame(kind, Vector2.Lerp(from, to, (float)index / count))
                );
            }
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
            touchscreen ??= InputSystem.AddDevice<Touchscreen>("Ditto Virtual Touchscreen");
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
            keyboard ??= InputSystem.AddDevice<Keyboard>("Ditto Virtual Keyboard");
            InputSystem.QueueStateEvent(keyboard, new KeyboardState(heldKeys.ToArray()));
        }

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
