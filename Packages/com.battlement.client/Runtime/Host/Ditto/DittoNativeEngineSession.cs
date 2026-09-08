#nullable enable

using System;

namespace Battlement
{
    internal sealed class DittoNativeEngineSession
    {
        internal const string SemanticFixtureEnvironment = "BATTLEMENT_DITTO_SEMANTIC_FIXTURE";
        internal const string DeterministicRuntimeEnvironment = "BATTLEMENT_DITTO_ACTIVE";

        private readonly BattlementNativeTransport transport;
        private BattlementTransportResult? destroyResult;
        private bool connected;

        private DittoNativeEngineSession(BattlementNativeTransport transport, string id) =>
            (this.transport, Id) = (transport, id);

        public string Id { get; }

        public bool IsDestroyed => destroyResult is not null;

        public static DittoNativeEngineSession? Create(
            BattlementNativeTransport transport,
            out BattlementTransportResult result,
            string? semanticFixture = null
        )
        {
            if (transport is null)
            {
                throw new ArgumentNullException(nameof(transport));
            }

            string id = Guid.NewGuid().ToString("D");
            result = WithSemanticFixture(semanticFixture, transport.CreateDittoEngine);
            return result.Status == BattlementTransportStatus.Success
                ? new DittoNativeEngineSession(transport, id)
                : null;
        }

        internal static T WithSemanticFixture<T>(string? value, Func<T> action)
        {
            string? previous = Environment.GetEnvironmentVariable(SemanticFixtureEnvironment);
            string? previousRuntime = Environment.GetEnvironmentVariable(
                DeterministicRuntimeEnvironment
            );
            try
            {
                Environment.SetEnvironmentVariable(SemanticFixtureEnvironment, value);
                Environment.SetEnvironmentVariable(DeterministicRuntimeEnvironment, "1");
                return action();
            }
            finally
            {
                Environment.SetEnvironmentVariable(SemanticFixtureEnvironment, previous);
                Environment.SetEnvironmentVariable(
                    DeterministicRuntimeEnvironment,
                    previousRuntime
                );
            }
        }

        public BattlementTransportResult Connect(ReadOnlyMemory<byte> json)
        {
            if (IsDestroyed)
            {
                throw new InvalidOperationException("A destroyed engine session cannot connect.");
            }
            if (connected)
            {
                throw new InvalidOperationException("An engine session connects exactly once.");
            }

            connected = true;
            return transport.ConnectDittoEngine(json);
        }

        public BattlementTransportResult Destroy()
        {
            destroyResult ??= transport.DestroyDittoEngine();
            return destroyResult;
        }
    }
}
