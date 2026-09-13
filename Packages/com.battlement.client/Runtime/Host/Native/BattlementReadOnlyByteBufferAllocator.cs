#nullable enable

using System;
using Google.FlatBuffers;

namespace Battlement
{
    /// <summary>Read-only FlatBuffers storage over managed or externally owned memory.</summary>
    internal sealed class BattlementReadOnlyByteBufferAllocator : ByteBufferAllocator
    {
        private readonly ReadOnlyMemory<byte> memory;

        internal BattlementReadOnlyByteBufferAllocator(ReadOnlyMemory<byte> memory)
        {
            this.memory = memory;
            Length = memory.Length;
        }

        public override Span<byte> Span =>
            throw new InvalidOperationException("Published FlatBuffer storage is immutable.");

        public override ReadOnlySpan<byte> ReadOnlySpan => memory.Span;

        public override Memory<byte> Memory =>
            throw new InvalidOperationException("Published FlatBuffer storage is immutable.");

        public override ReadOnlyMemory<byte> ReadOnlyMemory => memory;

        public override void GrowFront(int newSize) =>
            throw new InvalidOperationException("Published FlatBuffer storage cannot grow.");
    }
}
