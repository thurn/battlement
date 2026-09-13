#nullable enable

using System;
using System.Buffers.Binary;
using System.IO;
using Google.FlatBuffers;
using CoreWire = Battlement.FlatBuffers.Generated;

namespace Battlement
{
    /// <summary>
    /// Retains a native response while a generated composed-schema reader is in use.
    /// </summary>
    internal sealed class BattlementCustomFlatBufferResponse
        : IBattlementResponseView,
            IBattlementFlatBufferViewOwner
    {
        private const int MaximumResponseBytes = 16 * 1024 * 1024;

        private readonly IBattlementFlatBufferResponseViewSchema schema;
        private readonly IDisposable? storageOwner;
        private readonly ByteBuffer bytes;
        private int references = 1;
        private bool rootDisposed;
        private bool storageDisposed;

        internal BattlementCustomFlatBufferResponse(
            ReadOnlyMemory<byte> payload,
            IDisposable? storageOwner,
            IBattlementFlatBufferResponseViewSchema schema
        )
        {
            this.schema = schema;
            this.storageOwner = storageOwner;
            try
            {
                if (payload.Length is < 12 or > MaximumResponseBytes)
                    throw new InvalidDataException("A composed response has an invalid size.");
                if (
                    BinaryPrimitives.ReadUInt32LittleEndian(payload.Span[..4])
                    != payload.Length - 4
                )
                    throw new InvalidDataException(
                        "A composed response has an invalid size prefix."
                    );
                ByteBufferAllocator allocator = storageOwner is BattlementNativeBufferMemory native
                    ? new BattlementNativeByteBufferAllocator(native)
                    : new BattlementReadOnlyByteBufferAllocator(payload);
                bytes = new ByteBuffer(allocator, 0);
                schema.ValidateResponse(bytes);
                bytes.Position = FlatBufferConstants.SizePrefixLength;
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
                RequireRoot();
                return schema.ReadSessionId(bytes);
            }
        }

        public int MessageCount
        {
            get
            {
                RequireRoot();
                return schema.ReadMessageCount(bytes);
            }
        }

        public bool IsSnapshot(int index)
        {
            RequireRoot();
            return schema.IsSnapshot(bytes, index);
        }

        public IBattlementSnapshotView ReadSnapshot(int index)
        {
            RequireRoot();
            return new BattlementFlatBufferSnapshotView(
                this,
                schema.ReadSnapshotTable(bytes, index)
            );
        }

        public IBattlementBatchView ReadBatch(int index)
        {
            RequireRoot();
            if (schema.IsSnapshot(bytes, index))
                throw new InvalidDataException("The response message is not a batch.");
            return new BatchView(this, index);
        }

        public void Dispose()
        {
            if (rootDisposed)
                return;
            rootDisposed = true;
            Release();
        }

        private IDisposable Retain()
        {
            RequireRoot();
            references = checked(references + 1);
            return new Lease(this);
        }

        private void RequireRoot()
        {
            if (rootDisposed)
                throw new ObjectDisposedException(nameof(BattlementCustomFlatBufferResponse));
            RequireStorage();
        }

        private void RequireStorage()
        {
            if (storageDisposed || references <= 0)
                throw new ObjectDisposedException(nameof(BattlementCustomFlatBufferResponse));
        }

        IDisposable IBattlementFlatBufferViewOwner.RetainView() => Retain();

        void IBattlementFlatBufferViewOwner.RequireLiveView() => RequireStorage();

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
            private BattlementCustomFlatBufferResponse? owner;

            internal Lease(BattlementCustomFlatBufferResponse owner) => this.owner = owner;

            public void Dispose()
            {
                BattlementCustomFlatBufferResponse? current = owner;
                owner = null;
                current?.Release();
            }
        }

        private sealed class BatchView : IBattlementBatchView
        {
            private readonly BattlementCustomFlatBufferResponse response;
            private readonly int messageIndex;
            private IDisposable? lease;

            internal BatchView(BattlementCustomFlatBufferResponse response, int messageIndex) =>
                (this.response, this.messageIndex, lease) = (
                    response,
                    messageIndex,
                    response.Retain()
                );

            public BatchId Id => Read(schema => schema.ReadBatchId(response.bytes, messageIndex));
            public SessionId SessionId =>
                Read(schema => schema.ReadBatchSessionId(response.bytes, messageIndex));
            public ActionId? CausedByActionId =>
                Read(schema => schema.ReadCausedByActionId(response.bytes, messageIndex));
            public BatchStart Start =>
                Read(schema => schema.ReadBatchStart(response.bytes, messageIndex));
            public int GroupCount =>
                Read(schema => schema.ReadGroupCount(response.bytes, messageIndex));

            public int CommandCount(int groupIndex) =>
                Read(schema => schema.ReadCommandCount(response.bytes, messageIndex, groupIndex));

            public CommandId CommandId(int groupIndex, int commandIndex) =>
                Read(schema =>
                    schema.ReadCommandId(response.bytes, messageIndex, groupIndex, commandIndex)
                );

            public BattlementCommandExecution ReadCommand(int groupIndex, int commandIndex)
            {
                CommandId id = CommandId(groupIndex, commandIndex);
                bool core = Read(schema =>
                    schema.IsCoreCommand(response.bytes, messageIndex, groupIndex, commandIndex)
                );
                if (core)
                {
                    CoreWire.CoreCommand table = Read(schema =>
                        schema.ReadCoreCommandTable(
                            response.bytes,
                            messageIndex,
                            groupIndex,
                            commandIndex
                        )
                    );
                    if (BattlementDirectCommandReader.TryRead(table, id, response, out var direct))
                        return direct;
                    throw new InvalidDataException(
                        $"Core FlatBuffer command {table.Kind} has no direct host reader."
                    );
                }
                bool blocking = Read(schema =>
                    schema.ReadCommandIsBlocking(
                        response.bytes,
                        messageIndex,
                        groupIndex,
                        commandIndex
                    )
                );
                return new BattlementCommandExecution(
                    id,
                    blocking,
                    new DirectCustomCommand(
                        response,
                        messageIndex,
                        groupIndex,
                        commandIndex,
                        id,
                        blocking
                    )
                );
            }

            public bool IsAssetPreparation(int groupIndex, int commandIndex) =>
                Read(schema =>
                    schema.IsAssetPreparation(
                        response.bytes,
                        messageIndex,
                        groupIndex,
                        commandIndex
                    )
                );

            public void Dispose()
            {
                lease?.Dispose();
                lease = null;
            }

            private T Read<T>(Func<IBattlementFlatBufferResponseViewSchema, T> read)
            {
                response.RequireStorage();
                if (lease is null)
                    throw new ObjectDisposedException(nameof(BatchView));
                return read(response.schema);
            }
        }

        private sealed class DirectCustomCommand : IBattlementDirectCustomCommand
        {
            private readonly BattlementCustomFlatBufferResponse response;
            private readonly int messageIndex;
            private readonly int groupIndex;
            private readonly int commandIndex;
            private readonly CommandId commandId;
            private readonly bool isBlocking;

            internal DirectCustomCommand(
                BattlementCustomFlatBufferResponse response,
                int messageIndex,
                int groupIndex,
                int commandIndex,
                CommandId commandId,
                bool isBlocking
            ) =>
                (
                    this.response,
                    this.messageIndex,
                    this.groupIndex,
                    this.commandIndex,
                    this.commandId,
                    this.isBlocking
                ) = (response, messageIndex, groupIndex, commandIndex, commandId, isBlocking);

            public IBattlementCommandOperation? Launch(
                IBattlementFlatBufferCustomCommandDispatcher dispatcher,
                TimeSpan now
            )
            {
                response.RequireStorage();
                return response.schema.LaunchCustomCommand(
                    response.bytes,
                    messageIndex,
                    groupIndex,
                    commandIndex,
                    commandId,
                    isBlocking,
                    dispatcher,
                    now
                );
            }
        }
    }
}
