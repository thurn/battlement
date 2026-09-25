#nullable enable

namespace Battlement
{
    /// <summary>Physical device family accepted by an exclusive capture.</summary>
    public enum InputCaptureDevice
    {
        Keyboard,
        Controller,
    }

    public enum InputCaptureCancellation
    {
        FocusLost,
        DeviceDisconnected,
        Superseded,
    }

    public sealed record InputCaptureRequest(ObjectId Id, InputCaptureDevice Device);

    public abstract record InputCaptureCommand
    {
        public sealed record Begin(InputCaptureRequest Request) : InputCaptureCommand;

        public sealed record End(ObjectId Id) : InputCaptureCommand;
    }

    /// <summary>One terminal physical input or explicit cancellation.</summary>
    public abstract record InputCaptureResult
    {
        public sealed record Key(int DeviceId, PhysicalKey PhysicalKey) : InputCaptureResult;

        public sealed record Button(int DeviceId, ControllerButton Value) : InputCaptureResult;

        public sealed record Direction(
            int DeviceId,
            ControllerDirection Value,
            ControllerNavigationSource Source,
            bool Repeat
        ) : InputCaptureResult;

        public sealed record Cancelled(InputCaptureCancellation Reason) : InputCaptureResult;
    }

    public sealed record InputCaptureEvent(ObjectId Id, InputCaptureResult Result);
}
