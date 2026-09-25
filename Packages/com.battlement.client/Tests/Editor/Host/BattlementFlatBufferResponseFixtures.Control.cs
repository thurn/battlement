#nullable enable

using Google.FlatBuffers;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement.Tests
{
    internal static partial class BattlementFlatBufferResponseFixtures
    {
        private static Payload Wait(FlatBufferBuilder builder, CommandBody.Time.Wait value) =>
            new(
                Wire.CoreCommandKind.TimeWait,
                Wire.CoreCommandPayload.WaitPayload,
                Wire.WaitPayload.CreateWaitPayload(builder, Milliseconds(value.Duration)).Value
            );

        private static Payload Cancel(FlatBufferBuilder builder, CommandBody.Operation.Cancel value)
        {
            Wire.CancelOperationPayload.StartCancelOperationPayload(builder);
            Wire.CancelOperationPayload.AddCommandId(builder, Uuid(builder, value.CommandId.Value));
            return new(
                Wire.CoreCommandKind.OperationCancel,
                Wire.CoreCommandPayload.CancelOperationPayload,
                Wire.CancelOperationPayload.EndCancelOperationPayload(builder).Value
            );
        }

        private static Payload SetInputEnabled(
            FlatBufferBuilder builder,
            CommandBody.Input.SetEnabled value
        ) =>
            new(
                Wire.CoreCommandKind.InputSetEnabled,
                Wire.CoreCommandPayload.SetInputEnabledPayload,
                Wire.SetInputEnabledPayload.CreateSetInputEnabledPayload(
                    builder,
                    value.IsEnabled
                ).Value
            );

        private static Payload InputCamera(
            FlatBufferBuilder builder,
            CommandBody.Input.SetCamera value
        ) => ObjectId(builder, value.ObjectId, Wire.CoreCommandKind.InputSetCamera);

        private static Payload PointerEvents(
            FlatBufferBuilder builder,
            CommandBody.Input.SetPointerEvents value
        )
        {
            VectorOffset events = PointerEventVector(builder, value.Events);
            Wire.PointerEventsPayload.StartPointerEventsPayload(builder);
            Wire.PointerEventsPayload.AddEvents(builder, events);
            Wire.PointerEventsPayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            return new(
                Wire.CoreCommandKind.InputSetPointerEvents,
                Wire.CoreCommandPayload.PointerEventsPayload,
                Wire.PointerEventsPayload.EndPointerEventsPayload(builder).Value
            );
        }

        private static Payload GlobalKeys(
            FlatBufferBuilder builder,
            CommandBody.Input.SetGlobalKeys value
        )
        {
            var keys = new short[value.Keys.Count];
            for (int index = 0; index < keys.Length; index++)
                keys[index] = (short)value.Keys[index];
            return new(
                Wire.CoreCommandKind.InputSetGlobalKeys,
                Wire.CoreCommandPayload.GlobalKeysPayload,
                Wire.GlobalKeysPayload.CreateGlobalKeysPayload(
                    builder,
                    EnumVector(builder, keys)
                ).Value
            );
        }

        private static Payload InputCapture(FlatBufferBuilder builder, InputCaptureCommand value)
        {
            (ObjectId id, Wire.InputCaptureOperation operation) = value switch
            {
                InputCaptureCommand.Begin begin => (
                    begin.Request.Id,
                    begin.Request.Device == InputCaptureDevice.Keyboard
                        ? Wire.InputCaptureOperation.BeginKeyboard
                        : Wire.InputCaptureOperation.BeginController
                ),
                InputCaptureCommand.End end => (end.Id, Wire.InputCaptureOperation.End),
                _ => throw new System.InvalidOperationException("Unknown capture operation."),
            };
            Wire.InputCapturePayload.StartInputCapturePayload(builder);
            Wire.InputCapturePayload.AddOperation(builder, operation);
            Wire.InputCapturePayload.AddCaptureId(builder, Uuid(builder, id.Value));
            return new(
                Wire.CoreCommandKind.InputCapture,
                Wire.CoreCommandPayload.InputCapturePayload,
                Wire.InputCapturePayload.EndInputCapturePayload(builder).Value
            );
        }

        private static Payload ControllerInput(
            FlatBufferBuilder builder,
            CommandBody.Input.SetController value
        ) =>
            new(
                Wire.CoreCommandKind.InputSetController,
                Wire.CoreCommandPayload.ControllerInputSettings,
                WriteControllerInput(builder, value.Settings).Value
            );

        private static Payload ControllerVibration(
            FlatBufferBuilder builder,
            CommandBody.Controller.Vibrate value
        ) =>
            new(
                Wire.CoreCommandKind.ControllerVibrate,
                Wire.CoreCommandPayload.ControllerVibrationPayload,
                Wire.ControllerVibrationPayload.CreateControllerVibrationPayload(
                    builder,
                    value.LowFrequency,
                    value.HighFrequency,
                    Milliseconds(value.Duration)
                ).Value
            );
    }
}
