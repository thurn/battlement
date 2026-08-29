#nullable enable

using System;
using UnityEngine;

namespace Battlement
{
    /// <summary>Connects a scene-authored runner with production dependencies.</summary>
    [DisallowMultipleComponent]
    [RequireComponent(typeof(BattlementRunner))]
    public sealed class BattlementBootstrap : MonoBehaviour
    {
        [SerializeField]
        private bool autoConnect = true;

        /// <summary>Whether this component configures and connects its runner on startup.</summary>
        public bool AutoConnect => autoConnect;

        private void Start()
        {
            if (!autoConnect)
            {
                return;
            }

            BattlementRunner runner = GetComponent<BattlementRunner>();
            runner.Configure(
                new BattlementRunnerOptions(
                    Transport(runner),
                    new BattlementAddressablesAssetStorage(),
                    BattlementJson.Instance
                )
            );
#if BATTLEMENT_DITTO_DIAGNOSTICS
            if (BattlementDittoPlayerBootstrap.IsActive)
            {
                return;
            }
#endif
            runner.Connect();
        }

        private static IBattlementTransport Transport(BattlementRunner runner) =>
            runner.TransportKind switch
            {
                BattlementTransportKind.Native => new BattlementNativeTransport(),
                BattlementTransportKind.Http => new BattlementHttpTransport(
                    runner.HttpTransport.BaseUrl
                ),
                _ => throw new ArgumentOutOfRangeException(nameof(runner)),
            };
    }
}
