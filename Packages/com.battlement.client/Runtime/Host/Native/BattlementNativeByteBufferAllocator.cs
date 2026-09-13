#nullable enable

using System;
using Google.FlatBuffers;

namespace Battlement
{
    /// <summary>Read-only FlatBuffers storage over one guarded Rust buffer lease.</summary>
    internal sealed class BattlementNativeByteBufferAllocator : ByteBufferAllocator
    {
        private readonly BattlementNativeBufferMemory owner;

        internal BattlementNativeByteBufferAllocator(BattlementNativeBufferMemory owner)
        {
            this.owner = owner;
            Length = owner.ReadOnlyMemory.Length;
        }

        public override Span<byte> Span =>
            throw new InvalidOperationException("Published FlatBuffer storage is immutable.");

        public override ReadOnlySpan<byte> ReadOnlySpan => owner.ReadOnlySpan;

        public override Memory<byte> Memory =>
            throw new InvalidOperationException("Published FlatBuffer storage is immutable.");

        public override ReadOnlyMemory<byte> ReadOnlyMemory =>
            throw new InvalidOperationException(
                "Native-backed ReadOnlyMemory cannot escape a guarded read scope."
            );

        public override void GrowFront(int newSize) =>
            throw new InvalidOperationException("Published FlatBuffer storage cannot grow.");
    }
}
