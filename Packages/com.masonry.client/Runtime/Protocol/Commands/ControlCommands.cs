#nullable enable

using System;
using System.Collections.Generic;

namespace Masonry
{
    public abstract partial record CommandBody
    {
        public static class Time
        {
            /// <summary>Wait for a positive duration. This command must be blocking.</summary>
            /// <param name="Duration">Positive wait duration.</param>
            public sealed record Wait(TimeSpan Duration) : CommandBody;
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
            /// <param name="IsEnabled">Whether Masonry accepts input actions.</param>
            public sealed record SetEnabled(bool IsEnabled) : CommandBody;

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
            public sealed record SetGlobalKeys(IReadOnlyList<KeyCode> Keys) : CommandBody;
        }
    }
}
