#nullable enable

using System;
using System.Collections.Generic;
using MessagePack;

namespace Battlement
{
    internal static partial class ProtocolFormat
    {
        private static readonly string[] PointerEventVariants =
        {
            "Enter",
            "Exit",
            "Down",
            "Up",
            "Click",
        };

        private static readonly string[] PointerButtonVariants = { "Left", "Middle", "Right" };
        private static readonly string[] DragModeVariants = { "SnapToPointer", "PreserveOffset" };
        private static readonly string[] BatchStartVariants = { "Now", "AfterEarlierBlockingWork" };
        private static readonly string[] ConflictPolicyVariants = { "Cancel", "Wait" };
        private static readonly string[] ImageFitVariants = { "Stretch", "Contain", "Cover" };
        private static readonly string[] CameraProjectionVariants =
        {
            "Perspective",
            "Orthographic",
        };
        private static readonly string[] CameraClearModeVariants =
        {
            "Skybox",
            "SolidColor",
            "Depth",
            "Nothing",
        };
        private static readonly string[] LightTypeVariants = { "Directional", "Point", "Spot" };
        private static readonly string[] ShadowModeVariants = { "None", "Hard", "Soft" };
        private static readonly string[] HorizontalAlignmentVariants =
        {
            "Left",
            "Center",
            "Right",
            "Justified",
        };
        private static readonly string[] VerticalAlignmentVariants = { "Top", "Middle", "Bottom" };
        private static readonly string[] RepeatModeVariants = { "Restart", "PingPong" };
        private static readonly string[] EasingVariants =
        {
            "Linear",
            "InSine",
            "OutSine",
            "InOutSine",
            "InQuad",
            "OutQuad",
            "InOutQuad",
            "InCubic",
            "OutCubic",
            "InOutCubic",
            "InQuart",
            "OutQuart",
            "InOutQuart",
            "InQuint",
            "OutQuint",
            "InOutQuint",
            "InExpo",
            "OutExpo",
            "InOutExpo",
            "InCirc",
            "OutCirc",
            "InOutCirc",
            "InBack",
            "OutBack",
            "InOutBack",
            "InElastic",
            "OutElastic",
            "InOutElastic",
            "InBounce",
            "OutBounce",
            "InOutBounce",
        };
        private static readonly string[] KeyCodeVariants =
        {
            "Escape",
            "F1",
            "F2",
            "F3",
            "F4",
            "F5",
            "F6",
            "F7",
            "F8",
            "F9",
            "F10",
            "F11",
            "F12",
            "Backquote",
            "Digit0",
            "Digit1",
            "Digit2",
            "Digit3",
            "Digit4",
            "Digit5",
            "Digit6",
            "Digit7",
            "Digit8",
            "Digit9",
            "Minus",
            "Equal",
            "Backspace",
            "Tab",
            "KeyA",
            "KeyB",
            "KeyC",
            "KeyD",
            "KeyE",
            "KeyF",
            "KeyG",
            "KeyH",
            "KeyI",
            "KeyJ",
            "KeyK",
            "KeyL",
            "KeyM",
            "KeyN",
            "KeyO",
            "KeyP",
            "KeyQ",
            "KeyR",
            "KeyS",
            "KeyT",
            "KeyU",
            "KeyV",
            "KeyW",
            "KeyX",
            "KeyY",
            "KeyZ",
            "BracketLeft",
            "BracketRight",
            "Backslash",
            "CapsLock",
            "Semicolon",
            "Quote",
            "Enter",
            "ShiftLeft",
            "ShiftRight",
            "ControlLeft",
            "ControlRight",
            "AltLeft",
            "AltRight",
            "MetaLeft",
            "MetaRight",
            "Comma",
            "Period",
            "Slash",
            "Space",
            "ContextMenu",
            "Insert",
            "Delete",
            "Home",
            "End",
            "PageUp",
            "PageDown",
            "ArrowLeft",
            "ArrowRight",
            "ArrowUp",
            "ArrowDown",
            "PrintScreen",
            "ScrollLock",
            "Pause",
            "NumLock",
            "Numpad0",
            "Numpad1",
            "Numpad2",
            "Numpad3",
            "Numpad4",
            "Numpad5",
            "Numpad6",
            "Numpad7",
            "Numpad8",
            "Numpad9",
            "NumpadDecimal",
            "NumpadAdd",
            "NumpadSubtract",
            "NumpadMultiply",
            "NumpadDivide",
            "NumpadEnter",
        };

        private static void WriteVector3(ref MessagePackWriter writer, Vector3 value)
        {
            WriteArrayHeader(ref writer, 3);
            writer.Write(value.X);
            writer.Write(value.Y);
            writer.Write(value.Z);
        }

        private static Vector3 ReadVector3(ref MessagePackReader reader)
        {
            ReadArrayHeader(ref reader, 3);
            return new Vector3(reader.ReadDouble(), reader.ReadDouble(), reader.ReadDouble());
        }

        private static void WriteScreenPosition(ref MessagePackWriter writer, ScreenPosition value)
        {
            WriteArrayHeader(ref writer, 2);
            writer.Write(value.X);
            writer.Write(value.Y);
        }

        private static ScreenPosition ReadScreenPosition(ref MessagePackReader reader)
        {
            ReadArrayHeader(ref reader, 2);
            return new ScreenPosition(reader.ReadDouble(), reader.ReadDouble());
        }

        private static void WriteScreenSize(ref MessagePackWriter writer, ScreenSize value)
        {
            WriteArrayHeader(ref writer, 2);
            writer.Write(value.Width);
            writer.Write(value.Height);
        }

        private static ScreenSize ReadScreenSize(ref MessagePackReader reader)
        {
            ReadArrayHeader(ref reader, 2);
            return new ScreenSize(reader.ReadUInt32(), reader.ReadUInt32());
        }

        private static void WriteQuaternion(ref MessagePackWriter writer, Quaternion value)
        {
            WriteArrayHeader(ref writer, 4);
            writer.Write(value.X);
            writer.Write(value.Y);
            writer.Write(value.Z);
            writer.Write(value.W);
        }

        private static Quaternion ReadQuaternion(ref MessagePackReader reader)
        {
            ReadArrayHeader(ref reader, 4);
            return new Quaternion(
                reader.ReadDouble(),
                reader.ReadDouble(),
                reader.ReadDouble(),
                reader.ReadDouble()
            );
        }

        private static void WriteRgbColor(ref MessagePackWriter writer, RgbColor value)
        {
            WriteArrayHeader(ref writer, 3);
            writer.Write(value.Red);
            writer.Write(value.Green);
            writer.Write(value.Blue);
        }

        private static RgbColor ReadRgbColor(ref MessagePackReader reader)
        {
            ReadArrayHeader(ref reader, 3);
            return new RgbColor(reader.ReadDouble(), reader.ReadDouble(), reader.ReadDouble());
        }

        private static void WriteColor(ref MessagePackWriter writer, Color value)
        {
            WriteArrayHeader(ref writer, 4);
            writer.Write(value.Red);
            writer.Write(value.Green);
            writer.Write(value.Blue);
            writer.Write(value.Alpha);
        }

        private static Color ReadColor(ref MessagePackReader reader)
        {
            ReadArrayHeader(ref reader, 4);
            return new Color(
                reader.ReadDouble(),
                reader.ReadDouble(),
                reader.ReadDouble(),
                reader.ReadDouble()
            );
        }

        private static void WriteLocalTransform(ref MessagePackWriter writer, LocalTransform value)
        {
            WriteArrayHeader(ref writer, 3);
            WriteVector3(ref writer, value.Position);
            WriteQuaternion(ref writer, value.Rotation);
            WriteVector3(ref writer, value.Scale);
        }

        private static LocalTransform ReadLocalTransform(ref MessagePackReader reader)
        {
            ReadArrayHeader(ref reader, 3);
            return new LocalTransform(
                ReadVector3(ref reader),
                ReadQuaternion(ref reader),
                ReadVector3(ref reader)
            );
        }

        private static void WritePointerEvent(ref MessagePackWriter writer, PointerEvent value) =>
            WriteEnum(ref writer, (int)value, PointerEventVariants, nameof(PointerEvent));

        private static PointerEvent ReadPointerEvent(ref MessagePackReader reader) =>
            (PointerEvent)ReadEnum(ref reader, PointerEventVariants, nameof(PointerEvent));

        private static void WriteDragMode(ref MessagePackWriter writer, DragMode value) =>
            WriteEnum(ref writer, (int)value, DragModeVariants, nameof(DragMode));

        private static DragMode ReadDragMode(ref MessagePackReader reader) =>
            (DragMode)ReadEnum(ref reader, DragModeVariants, nameof(DragMode));

        private static void WritePointerButton(ref MessagePackWriter writer, PointerButton value) =>
            WriteEnum(ref writer, (int)value, PointerButtonVariants, nameof(PointerButton));

        private static PointerButton ReadPointerButton(ref MessagePackReader reader) =>
            (PointerButton)ReadEnum(ref reader, PointerButtonVariants, nameof(PointerButton));

        private static void WriteBatchStart(ref MessagePackWriter writer, BatchStart value) =>
            WriteEnum(ref writer, (int)value, BatchStartVariants, nameof(BatchStart));

        private static BatchStart ReadBatchStart(ref MessagePackReader reader) =>
            (BatchStart)ReadEnum(ref reader, BatchStartVariants, nameof(BatchStart));

        private static void WriteConflictPolicy(
            ref MessagePackWriter writer,
            ConflictPolicy value
        ) => WriteEnum(ref writer, (int)value, ConflictPolicyVariants, nameof(ConflictPolicy));

        private static ConflictPolicy ReadConflictPolicy(ref MessagePackReader reader) =>
            (ConflictPolicy)ReadEnum(ref reader, ConflictPolicyVariants, nameof(ConflictPolicy));

        private static void WriteImageFit(ref MessagePackWriter writer, ImageFit value) =>
            WriteEnum(ref writer, (int)value, ImageFitVariants, nameof(ImageFit));

        private static ImageFit ReadImageFit(ref MessagePackReader reader) =>
            (ImageFit)ReadEnum(ref reader, ImageFitVariants, nameof(ImageFit));

        private static void WriteCameraProjection(
            ref MessagePackWriter writer,
            CameraProjection value
        ) => WriteEnum(ref writer, (int)value, CameraProjectionVariants, nameof(CameraProjection));

        private static CameraProjection ReadCameraProjection(ref MessagePackReader reader) =>
            (CameraProjection)ReadEnum(
                ref reader,
                CameraProjectionVariants,
                nameof(CameraProjection)
            );

        private static void WriteCameraClearMode(
            ref MessagePackWriter writer,
            CameraClearMode value
        ) => WriteEnum(ref writer, (int)value, CameraClearModeVariants, nameof(CameraClearMode));

        private static CameraClearMode ReadCameraClearMode(ref MessagePackReader reader) =>
            (CameraClearMode)ReadEnum(ref reader, CameraClearModeVariants, nameof(CameraClearMode));

        private static void WriteLightType(ref MessagePackWriter writer, LightType value) =>
            WriteEnum(ref writer, (int)value, LightTypeVariants, nameof(LightType));

        private static LightType ReadLightType(ref MessagePackReader reader) =>
            (LightType)ReadEnum(ref reader, LightTypeVariants, nameof(LightType));

        private static void WriteShadowMode(ref MessagePackWriter writer, ShadowMode value) =>
            WriteEnum(ref writer, (int)value, ShadowModeVariants, nameof(ShadowMode));

        private static ShadowMode ReadShadowMode(ref MessagePackReader reader) =>
            (ShadowMode)ReadEnum(ref reader, ShadowModeVariants, nameof(ShadowMode));

        private static void WriteHorizontalAlignment(
            ref MessagePackWriter writer,
            HorizontalAlignment value
        ) =>
            WriteEnum(
                ref writer,
                (int)value,
                HorizontalAlignmentVariants,
                nameof(HorizontalAlignment)
            );

        private static HorizontalAlignment ReadHorizontalAlignment(ref MessagePackReader reader) =>
            (HorizontalAlignment)ReadEnum(
                ref reader,
                HorizontalAlignmentVariants,
                nameof(HorizontalAlignment)
            );

        private static void WriteVerticalAlignment(
            ref MessagePackWriter writer,
            VerticalAlignment value
        ) =>
            WriteEnum(ref writer, (int)value, VerticalAlignmentVariants, nameof(VerticalAlignment));

        private static VerticalAlignment ReadVerticalAlignment(ref MessagePackReader reader) =>
            (VerticalAlignment)ReadEnum(
                ref reader,
                VerticalAlignmentVariants,
                nameof(VerticalAlignment)
            );

        private static void WriteRepeatMode(ref MessagePackWriter writer, RepeatMode value) =>
            WriteEnum(ref writer, (int)value, RepeatModeVariants, nameof(RepeatMode));

        private static RepeatMode ReadRepeatMode(ref MessagePackReader reader) =>
            (RepeatMode)ReadEnum(ref reader, RepeatModeVariants, nameof(RepeatMode));

        private static void WriteEasing(ref MessagePackWriter writer, Easing value) =>
            WriteEnum(ref writer, (int)value, EasingVariants, nameof(Easing));

        private static Easing ReadEasing(ref MessagePackReader reader) =>
            (Easing)ReadEnum(ref reader, EasingVariants, nameof(Easing));

        private static void WriteTween(ref MessagePackWriter writer, Tween value)
        {
            WriteArrayHeader(ref writer, 4);
            writer.Write(Milliseconds(value.Delay));
            writer.Write(Milliseconds(value.Duration));
            WriteEasing(ref writer, value.Easing);
            WriteTweenRepeat(ref writer, value.Repeat);
        }

        private static Tween ReadTween(ref MessagePackReader reader)
        {
            ReadArrayHeader(ref reader, 4);
            TimeSpan delay = ReadDuration(ref reader);
            TimeSpan duration = ReadDuration(ref reader);
            Easing easing = ReadEasing(ref reader);
            TweenRepeat repeat = ReadTweenRepeat(ref reader);
            return new Tween(duration, delay, easing, repeat);
        }

        private static void WriteTweenRepeat(ref MessagePackWriter writer, TweenRepeat value)
        {
            switch (value)
            {
                case TweenRepeat.Once:
                    WriteVariant(ref writer, "Once");
                    break;
                case TweenRepeat.Count count:
                    WriteVariantHeader(ref writer, "Count");
                    WriteArrayHeader(ref writer, 2);
                    writer.Write(count.AdditionalTraversals);
                    WriteRepeatMode(ref writer, count.Mode);
                    break;
                case TweenRepeat.Forever forever:
                    WriteVariantHeader(ref writer, "Forever");
                    WriteRepeatMode(ref writer, forever.Mode);
                    break;
                default:
                    throw new MessagePackSerializationException("Unknown tween repeat value.");
            }
        }

        private static TweenRepeat ReadTweenRepeat(ref MessagePackReader reader)
        {
            if (reader.NextMessagePackType == MessagePackType.String)
            {
                string unit = ReadVariant(ref reader);
                return unit == "Once"
                    ? new TweenRepeat.Once()
                    : throw UnknownVariant(nameof(TweenRepeat), unit);
            }

            string variant = ReadVariantHeader(ref reader);
            switch (variant)
            {
                case "Count":
                    ReadArrayHeader(ref reader, 2);
                    return new TweenRepeat.Count(reader.ReadUInt32(), ReadRepeatMode(ref reader));
                case "Forever":
                    return new TweenRepeat.Forever(ReadRepeatMode(ref reader));
                default:
                    throw UnknownVariant(nameof(TweenRepeat), variant);
            }
        }

        private static void WriteKeyCode(ref MessagePackWriter writer, KeyCode value) =>
            WriteEnum(ref writer, (int)value, KeyCodeVariants, nameof(KeyCode));

        private static KeyCode ReadKeyCode(ref MessagePackReader reader) =>
            (KeyCode)ReadEnum(ref reader, KeyCodeVariants, nameof(KeyCode));

        private static void WriteKeyCodes(
            ref MessagePackWriter writer,
            IReadOnlyList<KeyCode> values
        )
        {
            writer.WriteArrayHeader(values.Count);
            foreach (KeyCode value in values)
            {
                WriteKeyCode(ref writer, value);
            }
        }

        private static IReadOnlyList<KeyCode> ReadKeyCodes(ref MessagePackReader reader)
        {
            int count = reader.ReadArrayHeader();
            var values = new KeyCode[count];
            for (int index = 0; index < count; index++)
            {
                values[index] = ReadKeyCode(ref reader);
            }

            return values;
        }
    }
}
