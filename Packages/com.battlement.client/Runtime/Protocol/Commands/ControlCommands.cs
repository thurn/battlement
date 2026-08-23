#nullable enable

using System;
using System.Collections.Generic;
using Newtonsoft.Json;

namespace Battlement
{
    public abstract partial record CommandBody
    {
        public static class Time
        {
            /// <summary>Wait for a positive duration. This command must be blocking.</summary>
            /// <param name="Duration">Positive wait duration.</param>
            public sealed record Wait([property: JsonProperty("duration_ms")] TimeSpan Duration)
                : CommandBody;
        }

        public static class Operation
        {
            /// <summary>Cancel a running operation, or no-op for an executed command.</summary>
            /// <param name="CommandId">Identity of the command and operation to cancel.</param>
            public sealed record Cancel(CommandId CommandId) : CommandBody;
        }

        public static class Input
        {
            /// <summary>Gate all pointer and key input.</summary>
            /// <param name="IsEnabled">Whether Battlement accepts input actions.</param>
            public sealed record SetEnabled([property: JsonProperty("enabled")] bool IsEnabled)
                : CommandBody;

            /// <summary>Select the enabled camera used for input raycasting.</summary>
            /// <param name="ObjectId">Target camera object.</param>
            public sealed record SetCamera(ObjectId ObjectId) : CommandBody;

            /// <summary>Replace the unique pointer-event set for an object.</summary>
            /// <param name="ObjectId">Target game object.</param>
            /// <param name="Events">Unique enabled pointer-event kinds.</param>
            public sealed record SetPointerEvents(
                ObjectId ObjectId,
                IReadOnlyList<PointerEvent> Events
            ) : CommandBody;

            /// <summary>Replace the unique set of enabled global physical keys.</summary>
            /// <param name="Keys">Unique enabled W3C physical key codes.</param>
            public sealed record SetGlobalKeys(IReadOnlyList<PhysicalKey> Keys) : CommandBody;

            /// <summary>Replace controller-button and navigation settings.</summary>
            public sealed record SetController(ControllerInputSettings Settings) : CommandBody;
        }

        public static class Controller
        {
            /// <summary>Run both controller vibration motors for a bounded duration.</summary>
            public sealed record Vibrate(
                double LowFrequency,
                double HighFrequency,
                [property: JsonProperty("duration_ms")] TimeSpan Duration
            ) : CommandBody;
        }
    }

    /// <summary>Controller buttons and discrete left-stick/D-pad navigation settings.</summary>
    public sealed record ControllerInputSettings(
        IReadOnlyList<ControllerButton> Buttons,
        bool NavigationEnabled = true,
        double? StickDeadZone = null,
        [property: JsonProperty("repeat_delay_ms")] TimeSpan? RepeatDelay = null,
        [property: JsonProperty("repeat_interval_ms")] TimeSpan? RepeatInterval = null
    );
}
