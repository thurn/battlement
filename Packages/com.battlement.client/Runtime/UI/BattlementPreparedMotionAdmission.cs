#nullable enable

using System;

namespace Battlement.UI
{
    internal sealed class BattlementPreparedMotionAdmission : IDisposable
    {
        private readonly BattlementMotionWorld world;
        private readonly Guid hostId;
        private readonly DescriptorState? prepared;
        private bool committed;
        private bool disposed;

        public BattlementPreparedMotionAdmission(
            BattlementMotionWorld world,
            Guid hostId,
            DescriptorState? prepared
        ) => (this.world, this.hostId, this.prepared) = (world, hostId, prepared);

        public void Commit()
        {
            if (committed || disposed)
                throw new InvalidOperationException("Motion admission was already committed.");
            world.Commit(hostId, prepared);
            committed = true;
        }

        public void Dispose()
        {
            if (committed || disposed)
                return;
            disposed = true;
            if (prepared is null)
                return;
            prepared.Abort();
        }
    }
}
