using System;

namespace Battlement
{
    /// <summary>Shared routing category for a playing sound.</summary>
    public enum AudioBus
    {
        Music,
        Effects,
    }

    /// <summary>Host-owned gains shared by active and future audio sources.</summary>
    public readonly struct AudioMix
    {
        public AudioMix(double master, double music, double effects, bool muted = false) =>
            (Master, Music, Effects, Muted) = (master, music, effects, muted);

        public double Master { get; }
        public double Music { get; }
        public double Effects { get; }
        public bool Muted { get; }

        public static AudioMix FullVolume => new(1, 1, 1);

        internal float Gain(AudioBus bus) =>
            Muted
                ? 0f
                : (float)(
                    Master
                    * (
                        bus switch
                        {
                            AudioBus.Music => Music,
                            AudioBus.Effects => Effects,
                            _ => throw new ArgumentOutOfRangeException(nameof(bus)),
                        }
                    )
                );
    }
}
