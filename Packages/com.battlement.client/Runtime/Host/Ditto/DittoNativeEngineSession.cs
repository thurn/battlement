#nullable enable

using System;

namespace Battlement
{
    internal sealed class DittoNativeEngineSession
    {
        private readonly BattlementNativeTransport transport;
        private BattlementTransportResult? destroyResult;
        private bool connected;

        private DittoNativeEngineSession(BattlementNativeTransport transport, string id) =>
            (this.transport, Id) = (transport, id);

        public string Id { get; }

        public bool IsDestroyed => destroyResult is not null;

        public static DittoNativeEngineSession? Create(
            BattlementNativeTransport transport,
            out BattlementTransportResult result
        )
        {
            if (transport is null)
            {
                throw new ArgumentNullException(nameof(transport));
            }

            string id = Guid.NewGuid().ToString("D");
            result = transport.CreateDittoEngine();
            return result.Status == BattlementTransportStatus.Success
                ? new DittoNativeEngineSession(transport, id)
                : null;
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
