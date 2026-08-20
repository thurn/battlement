#nullable enable

using System;
using UnityEngine;

namespace Masonry
{
    /// <summary>Receives deterministic callbacks when a pooled effect changes ownership.</summary>
    public interface IMasonryPoolReset
    {
        /// <summary>Resets state immediately before a pooled effect becomes active.</summary>
        void OnMasonryAcquire();

        /// <summary>Releases state immediately before a pooled effect becomes inactive.</summary>
        void OnMasonryRelease();
    }

    /// <summary>Opts a particle-effect prefab into Masonry-managed instance pooling.</summary>
    [DisallowMultipleComponent]
    public sealed class MasonryEffectPool : MonoBehaviour
    {
        [SerializeField]
        [Range(1, 128)]
        private int maxInactiveCount = 16;

        /// <summary>Maximum number of inactive instances retained for this effect.</summary>
        public int MaxInactiveCount
        {
            get => maxInactiveCount;
            set
            {
                if (value is < 1 or > 128)
                {
                    throw new ArgumentOutOfRangeException(
                        nameof(value),
                        "The inactive effect limit must be between 1 and 128."
                    );
                }

                maxInactiveCount = value;
            }
        }
    }
}
