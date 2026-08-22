#nullable enable

using System;

namespace Battlement
{
    public abstract partial record CommandBody
    {
        public static class Particle
        {
            /// <summary>
            /// Recursively play particle systems on an object and its descendants.
            /// </summary>
            /// <param name="ObjectId">
            /// Target object whose hierarchy contains particle systems.
            /// </param>
            /// <param name="Restart">Whether to restart systems already playing.</param>
            public sealed record Play(ObjectId ObjectId, bool Restart = false) : CommandBody;

            /// <summary>
            /// Recursively stop particle systems on an object and its descendants.
            /// </summary>
            /// <param name="ObjectId">
            /// Target object whose hierarchy contains particle systems.
            /// </param>
            /// <param name="Clear">Whether to clear live particles after stopping.</param>
            public sealed record Stop(ObjectId ObjectId, bool Clear = false) : CommandBody;

            /// <summary>Spawn a prepared temporary particle-effect prefab.</summary>
            /// <param name="Address">Prepared particle-effect-prefab address.</param>
            /// <param name="Location">Source of the initial world position.</param>
            /// <param name="Lifetime">Positive effect lifetime.</param>
            public sealed record Spawn(
                ParticleEffectAddress Address,
                ParticleSpawnLocation Location,
                TimeSpan Lifetime
            ) : CommandBody;
        }

        public static class Audio
        {
            /// <summary>Play a prepared audio clip.</summary>
            /// <param name="Address">Prepared audio-clip address.</param>
            /// <param name="Volume">Initial volume in [0, 1].</param>
            /// <param name="Pitch">Playback pitch in (0, 3].</param>
            /// <param name="Loop">Whether playback loops until explicitly stopped.</param>
            /// <param name="FadeIn">Fade-in duration.</param>
            public sealed record Play(
                AudioClipAddress Address,
                double Volume = 1,
                double Pitch = 1,
                bool Loop = false,
                TimeSpan FadeIn = default
            ) : CommandBody;

            /// <summary>Stop audio started by a previous audio-play command.</summary>
            /// <param name="AudioCommandId">Identity of the audio playback command.</param>
            /// <param name="FadeOut">Fade-out duration.</param>
            public sealed record Stop(CommandId AudioCommandId, TimeSpan FadeOut = default)
                : CommandBody;

            /// <summary>Set a playing audio operation's volume immediately.</summary>
            /// <param name="AudioCommandId">Identity of the audio playback command.</param>
            /// <param name="Volume">Requested volume in [0, 1].</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record SetVolume(
                CommandId AudioCommandId,
                double Volume,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;

            /// <summary>Tween a playing audio operation's volume.</summary>
            /// <param name="AudioCommandId">Identity of the audio playback command.</param>
            /// <param name="Volume">Requested final volume in [0, 1].</param>
            /// <param name="Tween">Tween timing and repetition.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record TweenVolume(
                CommandId AudioCommandId,
                double Volume,
                Tween Tween,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;
        }
    }

    /// <summary>Source of a temporary particle effect's initial world position.</summary>
    public abstract record ParticleSpawnLocation
    {
        private ParticleSpawnLocation() { }

        /// <summary>Use a game object's current world position.</summary>
        /// <param name="ObjectId">Source game object.</param>
        public sealed record AtGameObject(ObjectId ObjectId) : ParticleSpawnLocation;

        /// <summary>Use an explicit world-space position.</summary>
        /// <param name="Position">Initial world-space position.</param>
        public sealed record AtWorldPosition(Vector3 Position) : ParticleSpawnLocation;
    }
}
