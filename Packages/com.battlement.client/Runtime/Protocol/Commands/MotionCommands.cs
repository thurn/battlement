#nullable enable

namespace Battlement
{
    public abstract partial record CommandBody
    {
        public static class Motion
        {
            /// <summary>Mutates one stable Reactant motion value.</summary>
            public sealed record ValueCommand(MotionValueOperation Payload) : CommandBody;

            /// <summary>Mutates one motion-value playback generation.</summary>
            public sealed record ValuePlayback(MotionValuePlaybackOperation Payload) : CommandBody;

            /// <summary>Mutates one descriptor-slot playback generation.</summary>
            public sealed record Playback(MotionPlaybackOperation Payload) : CommandBody;

            /// <summary>Sets or advances one controlled motion clock.</summary>
            public sealed record ControlledClock(MotionControlledClockOperation Payload)
                : CommandBody;

            /// <summary>Broadcast through one animation-controls identity.</summary>
            public sealed record Control(MotionControlOperation Payload) : CommandBody;

            /// <summary>Execute one closed animation-scope operation.</summary>
            public sealed record Scope(MotionScopeOperation Payload) : CommandBody;
        }
    }
}
