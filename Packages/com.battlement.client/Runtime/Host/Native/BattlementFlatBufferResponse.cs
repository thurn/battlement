#nullable enable

using System;
using System.Buffers.Binary;
using System.IO;
using Google.FlatBuffers;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement
{
    /// <summary>
    /// Owns one verified response and exposes generated readers while its storage lease is live.
    /// </summary>
    internal sealed partial class BattlementFlatBufferResponse
        : IBattlementResponseView,
            IBattlementFlatBufferViewOwner
    {
        private const int MaximumResponseBytes = 16 * 1024 * 1024;
        private const int MaximumMessages = 256;
        private const int MaximumVerifierDepth = 64;
        private const int MaximumVerifierTables = 1_000_000;
        private const ulong MaximumApparentBytes = 64UL * 1024 * 1024;

        private readonly IDisposable? storageOwner;
        private readonly ByteBuffer bytes;
        private Wire.Response response;
        private int references = 1;
        private bool rootDisposed;
        private bool storageDisposed;

        internal static bool HasIdentifier(ReadOnlyMemory<byte> payload)
        {
            ReadOnlySpan<byte> span = payload.Span;
            return span.Length >= 12
                && span[8] == (byte)'B'
                && span[9] == (byte)'T'
                && span[10] == (byte)'R'
                && span[11] == (byte)'S';
        }

        internal BattlementFlatBufferResponse(
            ReadOnlyMemory<byte> payload,
            IDisposable? storageOwner
        )
        {
            this.storageOwner = storageOwner;
            try
            {
                if (payload.Length is < 12 or > MaximumResponseBytes)
                    throw new InvalidDataException(
                        $"A Battlement response must contain between 12 and "
                            + $"{MaximumResponseBytes} bytes."
                    );
                uint declared = BinaryPrimitives.ReadUInt32LittleEndian(payload.Span.Slice(0, 4));
                if (declared != payload.Length - 4)
                    throw new InvalidDataException(
                        "The response size prefix does not match its finished range."
                    );

                ByteBufferAllocator allocator = storageOwner is BattlementNativeBufferMemory native
                    ? new BattlementNativeByteBufferAllocator(native)
                    : new BattlementReadOnlyByteBufferAllocator(payload);
                bytes = new ByteBuffer(allocator, 0);
                var options = new Options(
                    MaximumVerifierDepth,
                    MaximumVerifierTables,
                    MaximumApparentBytes,
                    stringEndCheck: true,
                    alignmentCheck: true
                );
                var verifier = new Verifier(bytes, options);
                if (!verifier.VerifyBuffer("BTRS", sizePrefixed: true, Wire.ResponseVerify.Verify))
                    throw new InvalidDataException("The response FlatBuffer failed verification.");

                bytes.Position = FlatBufferConstants.SizePrefixLength;
                response = Wire.Response.GetRootAsResponse(bytes);
                ValidateSemantics();
            }
            catch
            {
                storageOwner?.Dispose();
                throw;
            }
        }

        public SessionId SessionId
        {
            get
            {
                RequireLive();
                return new SessionId(ReadUuid(response.SessionId, "response session"));
            }
        }

        public int MessageCount
        {
            get
            {
                RequireLive();
                return response.MessagesLength;
            }
        }

        internal Wire.ResponseMessageEntry Message(int index)
        {
            RequireLive();
            if ((uint)index >= (uint)response.MessagesLength)
                throw new ArgumentOutOfRangeException(nameof(index));
            return response.Messages(index)
                ?? throw new InvalidDataException("A verified response message is absent.");
        }

        public bool IsSnapshot(int index) =>
            Message(index).MessageType == Wire.ResponseMessage.Snapshot;

        public IBattlementSnapshotView ReadSnapshot(int index)
        {
            Wire.ResponseMessageEntry entry = Message(index);
            if (entry.MessageType != Wire.ResponseMessage.Snapshot)
                throw new InvalidDataException("The response message is not a snapshot.");
            return new BattlementFlatBufferSnapshotView(this, entry.MessageAsSnapshot());
        }

        public IBattlementBatchView ReadBatch(int index)
        {
            Wire.ResponseMessageEntry entry = Message(index);
            if (entry.MessageType != Wire.ResponseMessage.Batch)
                throw new InvalidDataException("The response message is not a batch.");
            return new BattlementFlatBufferBatchView(this, entry.MessageAsBatch());
        }

        internal IDisposable Retain()
        {
            RequireLive();
            references = checked(references + 1);
            return new Lease(this);
        }

        public void Dispose()
        {
            if (rootDisposed)
                return;
            rootDisposed = true;
            Release();
        }

        internal static Guid ReadUuid(Wire.Uuid? value, string field)
        {
            Wire.Uuid uuid =
                value ?? throw new InvalidDataException($"The {field} UUID is absent.");
            Span<byte> canonical = stackalloc byte[16];
            for (int index = 0; index < canonical.Length; index++)
                canonical[index] = uuid.Bytes(index);
            ReadOnlySpan<byte> order = stackalloc byte[16]
            {
                3,
                2,
                1,
                0,
                5,
                4,
                7,
                6,
                8,
                9,
                10,
                11,
                12,
                13,
                14,
                15,
            };
            Span<byte> mixed = stackalloc byte[16];
            for (int index = 0; index < mixed.Length; index++)
                mixed[order[index]] = canonical[index];
            var result = new Guid(mixed);
            if (result == Guid.Empty)
                throw new InvalidDataException($"The {field} UUID is zero.");
            return result;
        }

        private void ValidateSemantics()
        {
            _ = SessionId;
            if (response.MessagesLength > MaximumMessages)
                throw new InvalidDataException(
                    $"A response cannot contain more than {MaximumMessages} messages."
                );
            for (int index = 0; index < response.MessagesLength; index++)
            {
                Wire.ResponseMessageEntry message =
                    response.Messages(index)
                    ?? throw new InvalidDataException("A response message is absent.");
                switch (message.MessageType)
                {
                    case Wire.ResponseMessage.Snapshot:
                        Wire.Snapshot snapshot = message.MessageAsSnapshot();
                        ValidateSnapshot(snapshot, SessionId.Value);
                        ValidateDirectSnapshot(snapshot);
                        break;
                    case Wire.ResponseMessage.Batch:
                        ValidateBatch(message.MessageAsBatch());
                        break;
                    case Wire.ResponseMessage.NONE:
                        break;
                    default:
                        throw new InvalidDataException("A response message tag is unknown.");
                }
            }
        }

        internal void RequireLiveView() => RequireStorage();

        IDisposable IBattlementFlatBufferViewOwner.RetainView() => Retain();

        void IBattlementFlatBufferViewOwner.RequireLiveView() => RequireStorage();

        private void RequireLive()
        {
            if (rootDisposed)
                throw new ObjectDisposedException(nameof(BattlementFlatBufferResponse));
            RequireStorage();
        }

        private void RequireStorage()
        {
            if (storageDisposed || references <= 0)
                throw new ObjectDisposedException(nameof(BattlementFlatBufferResponse));
        }

        private void Release()
        {
            if (storageDisposed)
                return;
            references--;
            if (references > 0)
                return;
            storageDisposed = true;
            storageOwner?.Dispose();
        }

        private sealed class Lease : IDisposable
        {
            private BattlementFlatBufferResponse? owner;

            internal Lease(BattlementFlatBufferResponse owner) => this.owner = owner;

            public void Dispose()
            {
                BattlementFlatBufferResponse? current = owner;
                owner = null;
                current?.Release();
            }
        }
    }
}
