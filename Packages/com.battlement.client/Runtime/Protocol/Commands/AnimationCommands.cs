#nullable enable

using System;

namespace Battlement
{
    public abstract partial record CommandBody
    {
        public static class Animator
        {
            /// <summary>Play an Animator state directly.</summary>
            /// <param name="ObjectId">Target prefab object with a supported Animator.</param>
            /// <param name="State">Animator state name.</param>
            /// <param name="Layer">Nonnegative Animator layer index.</param>
            /// <param name="NormalizedStartTime">Normalized starting time in [0, 1].</param>
            /// <param name="Wait">Explicit operation duration for group scheduling.</param>
            public sealed record Play(
                ObjectId ObjectId,
                string State,
                uint Layer = 0,
                double NormalizedStartTime = 0,
                TimeSpan Wait = default
            ) : CommandBody;

            /// <summary>Cross-fade to an Animator state.</summary>
            /// <param name="ObjectId">Target prefab object with a supported Animator.</param>
            /// <param name="State">Animator state name.</param>
            /// <param name="CrossFadeDuration">Positive cross-fade duration.</param>
            /// <param name="Layer">Nonnegative Animator layer index.</param>
            /// <param name="NormalizedStartTime">Normalized starting time in [0, 1].</param>
            /// <param name="Wait">Explicit operation duration for group scheduling.</param>
            public sealed record CrossFade(
                ObjectId ObjectId,
                string State,
                TimeSpan CrossFadeDuration,
                uint Layer = 0,
                double NormalizedStartTime = 0,
                TimeSpan Wait = default
            ) : CommandBody;

            /// <summary>Set a persistent boolean Animator parameter.</summary>
            /// <param name="ObjectId">Target prefab object with a supported Animator.</param>
            /// <param name="Parameter">Parameter name.</param>
            /// <param name="Value">New boolean value.</param>
            public sealed record SetBool(ObjectId ObjectId, string Parameter, bool Value)
                : CommandBody;

            /// <summary>Set a persistent integer Animator parameter.</summary>
            /// <param name="ObjectId">Target prefab object with a supported Animator.</param>
            /// <param name="Parameter">Parameter name.</param>
            /// <param name="Value">New signed 32-bit value.</param>
            public sealed record SetInt(ObjectId ObjectId, string Parameter, int Value)
                : CommandBody;

            /// <summary>Set a persistent floating-point Animator parameter.</summary>
            /// <param name="ObjectId">Target prefab object with a supported Animator.</param>
            /// <param name="Parameter">Parameter name.</param>
            /// <param name="Value">New finite floating-point value.</param>
            public sealed record SetFloat(ObjectId ObjectId, string Parameter, double Value)
                : CommandBody;

            /// <summary>Fire an Animator trigger.</summary>
            /// <param name="ObjectId">Target prefab object with a supported Animator.</param>
            /// <param name="Parameter">Parameter name.</param>
            public sealed record SetTrigger(ObjectId ObjectId, string Parameter) : CommandBody;

            /// <summary>Set nonnegative Animator playback speed.</summary>
            /// <param name="ObjectId">Target prefab object with a supported Animator.</param>
            /// <param name="Speed">Nonnegative playback speed.</param>
            public sealed record SetSpeed(ObjectId ObjectId, double Speed) : CommandBody;
        }
    }
}
