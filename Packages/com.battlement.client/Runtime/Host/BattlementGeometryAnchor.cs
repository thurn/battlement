#nullable enable

using System;
using UnityEngine;

namespace Battlement
{
    /// <summary>Names an authored transform for world-space geometry observation.</summary>
    [DisallowMultipleComponent]
    public sealed class BattlementGeometryAnchor : MonoBehaviour
    {
        [SerializeField]
        private string anchorName = string.Empty;

        /// <summary>Gets or sets the nonempty name used by world-anchor observations.</summary>
        public string Name
        {
            get => anchorName;
            set =>
                anchorName = !string.IsNullOrEmpty(value)
                    ? value
                    : throw new ArgumentException(
                        "A geometry anchor name must be nonempty.",
                        nameof(value)
                    );
        }
    }
}
