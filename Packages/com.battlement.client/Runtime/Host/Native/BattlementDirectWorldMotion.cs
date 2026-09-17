#nullable enable

namespace Battlement
{
    internal readonly struct BattlementDirectWorldMotion
    {
        public BattlementDirectWorldMotion(ObjectId objectId, MotionDescriptor? motion) =>
            (ObjectId, Motion) = (objectId, motion);

        public ObjectId ObjectId { get; }
        public MotionDescriptor? Motion { get; }
    }
}
