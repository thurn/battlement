#nullable enable

using System;
using System.Buffers;
using System.Threading;

namespace Battlement
{
    /// <summary>Guards a Rust-owned immutable finished range until deterministic release.</summary>
    internal sealed unsafe class BattlementNativeBufferMemory : MemoryManager<byte>
    {
        private readonly IntPtr data;
        private readonly int length;
        private readonly int owningThreadId;
        private readonly int generation;
        private readonly Func<int> currentGeneration;
        private readonly Action<ulong> release;
        private readonly Action<ulong> enqueueFinalizerRelease;
        private int released;

        internal BattlementNativeBufferMemory(
            BattlementNativeBuffer buffer,
            int owningThreadId,
            int generation,
            Func<int> currentGeneration,
            Action<ulong> release,
            Action<ulong> enqueueFinalizerRelease
        )
        {
            Handle = buffer.Handle;
            AllocationBytes = checked((long)buffer.AllocationBytes);
            data = buffer.Data;
            length = checked((int)buffer.Length);
            this.owningThreadId = owningThreadId;
            this.generation = generation;
            this.currentGeneration = currentGeneration;
            this.release = release;
            this.enqueueFinalizerRelease = enqueueFinalizerRelease;
        }

        ~BattlementNativeBufferMemory()
        {
            if (Interlocked.Exchange(ref released, 1) == 0)
                enqueueFinalizerRelease(Handle);
        }

        internal ulong Handle { get; }

        internal long AllocationBytes { get; }

        internal ReadOnlyMemory<byte> ReadOnlyMemory => Memory;

        internal ReadOnlySpan<byte> ReadOnlySpan
        {
            get
            {
                ValidateAccess();
                return new ReadOnlySpan<byte>(data.ToPointer(), length);
            }
        }

        public override Span<byte> GetSpan()
        {
            ValidateAccess();
            return new Span<byte>(data.ToPointer(), length);
        }

        public override MemoryHandle Pin(int elementIndex = 0)
        {
            ValidateAccess();
            if ((uint)elementIndex > (uint)length)
                throw new ArgumentOutOfRangeException(nameof(elementIndex));
            return new MemoryHandle((byte*)data.ToPointer() + elementIndex);
        }

        public override void Unpin() { }

        protected override void Dispose(bool disposing)
        {
            if (!disposing)
            {
                if (Interlocked.Exchange(ref released, 1) == 0)
                    enqueueFinalizerRelease(Handle);
                return;
            }
            RequireOwningThread();
            if (Interlocked.Exchange(ref released, 1) != 0)
                return;
            GC.SuppressFinalize(this);
            release(Handle);
        }

        internal void ForceRelease()
        {
            if (Interlocked.Exchange(ref released, 1) != 0)
                return;
            GC.SuppressFinalize(this);
            release(Handle);
        }

        private void ValidateAccess()
        {
            if (Volatile.Read(ref released) != 0 || generation != currentGeneration())
                throw new ObjectDisposedException(nameof(BattlementNativeBufferMemory));
            RequireOwningThread();
        }

        private void RequireOwningThread()
        {
            if (Thread.CurrentThread.ManagedThreadId != owningThreadId)
                throw new InvalidOperationException(
                    "Native response memory is accessible only on its owning Unity thread."
                );
        }
    }
}
