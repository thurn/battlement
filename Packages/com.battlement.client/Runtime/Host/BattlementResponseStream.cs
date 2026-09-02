#nullable enable

using System;
using System.Collections.Generic;
using System.IO;

namespace Battlement
{
    /// <summary>Queues serialized responses and advances them in admission order.</summary>
    internal sealed class BattlementResponseStream
    {
        private const int MaximumResponseBytes = 16 * 1024 * 1024;
        private const int MaximumQueuedResponses = 256;
        private const long MaximumQueuedBytes = 64L * 1024 * 1024;

        private readonly LinkedList<PendingResponse> pending = new();
        private PendingResponse? active;
        private bool isProcessing;
        private int queuedResponses;
        private long queuedBytes;
        private ulong nextSequence;

        public bool HasPending => isProcessing || active is not null || pending.Count > 0;

        public void Enqueue(
            Func<ReadOnlyMemory<byte>, Response<ICommand>> decode,
            ReadOnlyMemory<byte> payload,
            bool isInitial,
            SessionId? previousSession
        )
        {
            Reservation reservation = Reserve(decode, isInitial, previousSession);
            try
            {
                reservation.Commit(payload);
            }
            catch
            {
                reservation.Release();
                throw;
            }
        }

        public Reservation Reserve(
            Func<ReadOnlyMemory<byte>, Response<ICommand>> decode,
            bool isInitial = false,
            SessionId? previousSession = null
        )
        {
            if (queuedResponses >= MaximumQueuedResponses)
            {
                throw new InvalidDataException(
                    $"Battlement cannot queue more than {MaximumQueuedResponses} responses."
                );
            }
            var value = new PendingResponse(decode, isInitial, previousSession);
            LinkedListNode<PendingResponse> node = pending.AddLast(value);
            queuedResponses++;
            return new Reservation(this, node, nextSequence++);
        }

        public void Drain(
            Func<Response<ICommand>, bool, SessionId?, bool> validate,
            Action<SessionId, ResponseMessage<ICommand>> apply,
            Func<bool> isPaused,
            Func<bool> isStopped
        )
        {
            if (isProcessing || isPaused() || isStopped())
            {
                return;
            }

            isProcessing = true;
            try
            {
                using (BattlementProfiler.ResponseApplication.Auto())
                {
                    while (!isStopped() && !isPaused())
                    {
                        active ??= TakeNext();
                        if (active is null)
                        {
                            return;
                        }

                        if (!active.IsDecoded)
                        {
                            using (BattlementProfiler.ResponseParsing.Auto())
                            {
                                active.Response = active.Decode(active.Payload);
                            }
                            active.IsDecoded = true;
                        }

                        if (!active.IsValidated)
                        {
                            active.IsValidated = true;
                            if (
                                !validate(
                                    active.Response!,
                                    active.IsInitial,
                                    active.PreviousSession
                                )
                            )
                            {
                                if (active is not null)
                                {
                                    RetireActive();
                                }
                                continue;
                            }
                        }

                        while (
                            !isStopped()
                            && !isPaused()
                            && active is not null
                            && active.NextMessageIndex < active.Response!.Messages.Count
                        )
                        {
                            ResponseMessage<ICommand> message = active.Response.Messages[
                                active.NextMessageIndex++
                            ];
                            apply(active.Response.SessionId, message);
                        }

                        if (
                            active is not null
                            && active.NextMessageIndex >= active.Response!.Messages.Count
                        )
                        {
                            RetireActive();
                        }
                    }
                }
            }
            finally
            {
                isProcessing = false;
                if (isStopped())
                {
                    Clear();
                }
            }
        }

        public void Clear()
        {
            active = null;
            pending.Clear();
            queuedResponses = 0;
            queuedBytes = 0;
            nextSequence = 0;
        }

        private PendingResponse? TakeNext()
        {
            if (
                pending.First is not LinkedListNode<PendingResponse> next
                || !next.Value.IsCommitted
            )
            {
                return null;
            }

            pending.RemoveFirst();
            return next.Value;
        }

        private void Commit(LinkedListNode<PendingResponse> node, ReadOnlyMemory<byte> payload)
        {
            if (node.List != pending || node.Value.IsCommitted)
            {
                throw new InvalidOperationException("Response reservation is not pending.");
            }
            if (payload.Length > MaximumResponseBytes)
            {
                throw new InvalidDataException(
                    $"A Battlement response cannot exceed {MaximumResponseBytes} bytes."
                );
            }
            if (queuedBytes + payload.Length > MaximumQueuedBytes)
            {
                throw new InvalidDataException(
                    $"Battlement cannot queue more than {MaximumQueuedBytes} response bytes."
                );
            }
            node.Value.Payload = payload;
            node.Value.IsCommitted = true;
            queuedBytes += payload.Length;
        }

        private void Release(LinkedListNode<PendingResponse> node)
        {
            if (node.List != pending || node.Value.IsCommitted)
            {
                throw new InvalidOperationException("Response reservation cannot be released.");
            }
            pending.Remove(node);
            queuedResponses--;
        }

        private void RetireActive()
        {
            if (active is null)
            {
                throw new InvalidOperationException("No active response can be retired.");
            }
            queuedResponses--;
            queuedBytes -= active.Payload.Length;
            active = null;
        }

        internal sealed class Reservation
        {
            private BattlementResponseStream? owner;
            private readonly LinkedListNode<PendingResponse> node;

            internal Reservation(
                BattlementResponseStream owner,
                LinkedListNode<PendingResponse> node,
                ulong sequence
            ) => (this.owner, this.node, Sequence) = (owner, node, sequence);

            public ulong Sequence { get; }

            public void Commit(ReadOnlyMemory<byte> payload)
            {
                BattlementResponseStream current =
                    owner ?? throw new InvalidOperationException("Response reservation is closed.");
                current.Commit(node, payload);
                owner = null;
            }

            public void Release()
            {
                BattlementResponseStream current =
                    owner ?? throw new InvalidOperationException("Response reservation is closed.");
                current.Release(node);
                owner = null;
            }
        }

        internal sealed class PendingResponse
        {
            public PendingResponse(
                Func<ReadOnlyMemory<byte>, Response<ICommand>> decode,
                bool isInitial,
                SessionId? previousSession
            )
            {
                Decode = decode;
                IsInitial = isInitial;
                PreviousSession = previousSession;
            }

            public Func<ReadOnlyMemory<byte>, Response<ICommand>> Decode { get; }

            public ReadOnlyMemory<byte> Payload { get; set; }

            public Response<ICommand>? Response { get; set; }

            public bool IsInitial { get; }

            public SessionId? PreviousSession { get; }

            public int NextMessageIndex { get; set; }

            public bool IsValidated { get; set; }

            public bool IsCommitted { get; set; }

            public bool IsDecoded { get; set; }
        }
    }
}
