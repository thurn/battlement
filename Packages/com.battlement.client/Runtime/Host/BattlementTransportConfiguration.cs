#nullable enable

using System;
using UnityEngine;

namespace Battlement
{
    /// <summary>Scene-authored transport implementation selected by a runner.</summary>
    public enum BattlementTransportKind
    {
        Native,
        Http,
    }

    /// <summary>Serialized native transport configuration.</summary>
    [Serializable]
    public sealed class BattlementNativeTransportConfiguration
    {
        /// <summary>Gets the fixed native rules-engine library base name.</summary>
        public string LibraryName => "battlement_rules";
    }

    /// <summary>Serialized localhost HTTP transport configuration.</summary>
    [Serializable]
    public sealed class BattlementHttpTransportConfiguration
    {
        [SerializeField]
        private string baseUrl = "http://127.0.0.1:8080";

        /// <summary>Gets the configured localhost service base URL.</summary>
        public string BaseUrl => baseUrl;
    }
}
