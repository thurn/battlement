#nullable enable

using System;
using UnityEngine;

namespace Masonry
{
    /// <summary>Connects a scene-authored runner with Masonry's production dependencies.</summary>
    [DisallowMultipleComponent]
    [RequireComponent(typeof(MasonryRunner))]
    public sealed class MasonryBootstrap : MonoBehaviour
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

            MasonryRunner runner = GetComponent<MasonryRunner>();
            runner.Configure(
                new MasonryRunnerOptions(
                    Transport(runner),
                    new MasonryAddressablesAssetStorage(),
                    MasonryMessagePack.Instance
                )
            );
            runner.Connect();
        }

        private static IMasonryTransport Transport(MasonryRunner runner) =>
            runner.TransportKind switch
            {
                MasonryTransportKind.Native => new MasonryNativeTransport(),
                MasonryTransportKind.Http => new MasonryHttpTransport(runner.HttpTransport.BaseUrl),
                _ => throw new ArgumentOutOfRangeException(nameof(runner)),
            };
    }
}
