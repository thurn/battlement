#nullable enable

using System;
using System.IO;
using Google.FlatBuffers;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement
{
    internal static class BattlementInputCaptureWire
    {
        internal static InputCaptureCommand Read(Wire.InputCapturePayload value)
        {
            var id = new ObjectId(
                BattlementFlatBufferCore.ReadUuid(value.CaptureId, "input capture")
            );
            return value.Operation switch
            {
                Wire.InputCaptureOperation.BeginKeyboard => new InputCaptureCommand.Begin(
                    new(id, InputCaptureDevice.Keyboard)
                ),
                Wire.InputCaptureOperation.BeginController => new InputCaptureCommand.Begin(
                    new(id, InputCaptureDevice.Controller)
                ),
                Wire.InputCaptureOperation.End => new InputCaptureCommand.End(id),
                _ => throw new InvalidDataException("Unknown input capture operation."),
            };
        }

        internal static Offset<Wire.InputCaptureAction> Write(
            FlatBufferBuilder builder,
            InputCaptureEvent value
        )
        {
            int device = 0;
            Wire.InputCaptureResultKind kind;
            PhysicalKey key = default;
            ControllerButton button = default;
            ControllerDirection direction = default;
            InputCaptureCancellation cancellation = default;
            switch (value.Result)
            {
                case InputCaptureResult.Key captured:
                    kind = Wire.InputCaptureResultKind.Key;
                    device = captured.DeviceId;
                    key = captured.PhysicalKey;
                    if (!Enum.IsDefined(typeof(PhysicalKey), key))
                        throw new InvalidDataException("Unknown captured key.");
                    break;
                case InputCaptureResult.Button captured:
                    kind = Wire.InputCaptureResultKind.Button;
                    device = captured.DeviceId;
                    button = captured.Value;
                    if (!Enum.IsDefined(typeof(ControllerButton), button))
                        throw new InvalidDataException("Unknown captured button.");
                    break;
                case InputCaptureResult.Direction captured:
                    kind = Wire.InputCaptureResultKind.Direction;
                    device = captured.DeviceId;
                    direction = captured.Value;
                    if (captured.Source != ControllerNavigationSource.Dpad || captured.Repeat)
                        throw new InvalidDataException("Capture requires an initial D-pad press.");
                    if (!Enum.IsDefined(typeof(ControllerDirection), direction))
                        throw new InvalidDataException("Unknown captured direction.");
                    break;
                case InputCaptureResult.Cancelled cancelled:
                    kind = Wire.InputCaptureResultKind.Cancelled;
                    cancellation = cancelled.Reason;
                    if (!Enum.IsDefined(typeof(InputCaptureCancellation), cancellation))
                        throw new InvalidDataException("Unknown capture cancellation.");
                    break;
                default:
                    throw new InvalidDataException("Unknown capture result.");
            }
            if (device < 0)
                throw new InvalidDataException("Capture device identities must be nonnegative.");
            Wire.InputCaptureAction.StartInputCaptureAction(builder);
            Wire.InputCaptureAction.AddResult(builder, kind);
            Wire.InputCaptureAction.AddDeviceId(builder, device);
            Wire.InputCaptureAction.AddKey(builder, (Wire.PhysicalKey)key);
            Wire.InputCaptureAction.AddButton(builder, (Wire.ControllerButton)button);
            Wire.InputCaptureAction.AddDirection(builder, (Wire.ControllerDirection)direction);
            Wire.InputCaptureAction.AddCancellation(
                builder,
                (Wire.InputCaptureCancellation)cancellation
            );
            Wire.InputCaptureAction.AddCaptureId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, value.Id.Value)
            );
            return Wire.InputCaptureAction.EndInputCaptureAction(builder);
        }
    }
}
