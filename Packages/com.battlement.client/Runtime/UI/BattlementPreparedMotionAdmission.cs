#nullable enable

using System;

namespace Battlement.UI
{
    internal sealed class BattlementPreparedMotionAdmission
    {
        private readonly BattlementMotionWorld world;
        private readonly Guid hostId;
        private readonly DescriptorState? prepared;
        private bool committed;
        private readonly MotionDescriptor? paintUpdate;

        public BattlementPreparedMotionAdmission(
            BattlementMotionWorld world,
            Guid hostId,
            DescriptorState? prepared,
            MotionDescriptor? paintUpdate = null
        ) =>
            (this.world, this.hostId, this.prepared, this.paintUpdate) = (
                world,
                hostId,
                prepared,
                paintUpdate
            );

        public void Commit()
        {
            if (committed)
                throw new InvalidOperationException("Motion admission was already committed.");
            if (paintUpdate is not null)
                prepared!.UpdateStaticPaint(paintUpdate);
            else
                world.Commit(hostId, prepared);
            committed = true;
        }
    }
}
