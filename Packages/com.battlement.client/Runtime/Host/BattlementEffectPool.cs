#nullable enable

using System;
using UnityEngine;

namespace Battlement
{
    /// <summary>Receives deterministic callbacks when a pooled effect changes ownership.</summary>
    public interface IBattlementPoolReset
    {
        /// <summary>Resets state immediately before a pooled effect becomes active.</summary>
        void OnBattlementAcquire();

        /// <summary>Releases state immediately before a pooled effect becomes inactive.</summary>
        void OnBattlementRelease();
    }

    /// <summary>Opts a particle-effect prefab into Battlement-managed instance pooling.</summary>
    [DisallowMultipleComponent]
    public sealed class BattlementEffectPool : MonoBehaviour
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
