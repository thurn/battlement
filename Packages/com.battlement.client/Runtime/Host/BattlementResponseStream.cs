#nullable enable

using System;
using System.Collections.Generic;
using System.Diagnostics;
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

        public long LastDecodeTicks { get; private set; }

        public long LastApplyTicks { get; private set; }

        public void Enqueue(
            Func<ReadOnlyMemory<byte>, IDisposable?, IBattlementResponseView> decode,
            ReadOnlyMemory<byte> payload,
            bool isInitial,
            SessionId? previousSession,
            IDisposable? payloadOwner = null
        )
        {
            Reservation reservation = Reserve(decode, isInitial, previousSession);
            try
            {
                reservation.Commit(payload, payloadOwner);
            }
            catch
            {
                reservation.Release();
                throw;
            }
        }

        public Reservation Reserve(
            Func<ReadOnlyMemory<byte>, IDisposable?, IBattlementResponseView> decode,
            bool isInitial = false,
            SessionId? previousSession = null,
            Action<IBattlementResponseView>? decoded = null,
            Action<Exception>? decodeFailed = null
        )
        {
            if (queuedResponses >= MaximumQueuedResponses)
            {
                throw new InvalidDataException(
                    $"Battlement cannot queue more than {MaximumQueuedResponses} responses."
                );
            }
            var value = new PendingResponse(
                decode,
                isInitial,
                previousSession,
                decoded,
                decodeFailed
            );
            LinkedListNode<PendingResponse> node = pending.AddLast(value);
            queuedResponses++;
            return new Reservation(this, node, nextSequence++);
        }

        public Reservation Reserve(
            Func<ReadOnlyMemory<byte>, Response<ICommand>> decode,
            bool isInitial = false,
            SessionId? previousSession = null,
            Action<Response<ICommand>>? decoded = null,
            Action<Exception>? decodeFailed = null
        ) =>
            Reserve(
                (payload, owner) =>
                {
                    try
                    {
                        Response<ICommand> response = decode(payload);
                        decoded?.Invoke(response);
                        return new BattlementOwnedResponseView(response, owner);
                    }
                    catch
                    {
                        owner?.Dispose();
                        throw;
                    }
                },
                isInitial,
                previousSession,
                decoded: null,
                decodeFailed
            );

        public void Drain(
            Func<IBattlementResponseView, bool, SessionId?, bool> validate,
            Action<SessionId, IBattlementResponseView, int> apply,
            Func<bool> isPaused,
            Func<bool> isStopped,
            bool observeTiming
        )
        {
            LastDecodeTicks = 0;
            LastApplyTicks = 0;
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
                            long decodeStarted = observeTiming ? Stopwatch.GetTimestamp() : 0;
                            try
                            {
                                using (BattlementProfiler.ResponseParsing.Auto())
                                {
                                    IDisposable? owner = active.PayloadOwner;
                                    active.PayloadOwner = null;
                                    active.Response = active.Decode(active.Payload, owner);
                                }
                            }
                            catch (Exception exception)
                            {
                                active.DecodeFailed?.Invoke(exception);
                                RetireActive();
                                throw;
                            }
                            if (observeTiming)
                            {
                                LastDecodeTicks = checked(
                                    LastDecodeTicks + Stopwatch.GetTimestamp() - decodeStarted
                                );
                            }
                            active.IsDecoded = true;
                        }
                        if (!active.DecodedNotified)
                        {
                            active.DecodedNotified = true;
                            active.Decoded?.Invoke(active.Response!);
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
                            && active.NextMessageIndex < active.Response!.MessageCount
                        )
                        {
                            int messageIndex = active.NextMessageIndex++;
                            long applyStarted = observeTiming ? Stopwatch.GetTimestamp() : 0;
                            apply(active.Response.SessionId, active.Response, messageIndex);
                            if (observeTiming)
                            {
                                LastApplyTicks = checked(
                                    LastApplyTicks + Stopwatch.GetTimestamp() - applyStarted
                                );
                            }
                        }

                        if (
                            active is not null
                            && active.NextMessageIndex >= active.Response!.MessageCount
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
            active?.Response?.Dispose();
            active?.PayloadOwner?.Dispose();
            foreach (PendingResponse response in pending)
                response.PayloadOwner?.Dispose();
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

        private void Commit(
            LinkedListNode<PendingResponse> node,
            ReadOnlyMemory<byte> payload,
            IDisposable? payloadOwner
        )
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
            node.Value.PayloadOwner = payloadOwner;
            node.Value.IsCommitted = true;
            queuedBytes += payload.Length;
        }

        private void Commit(LinkedListNode<PendingResponse> node, IBattlementResponseView response)
        {
            if (node.List != pending || node.Value.IsCommitted)
            {
                throw new InvalidOperationException("Response reservation is not pending.");
            }
            node.Value.Response = response;
            node.Value.IsDecoded = true;
            node.Value.IsCommitted = true;
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
            active.Response?.Dispose();
            active.PayloadOwner?.Dispose();
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

            public void Commit(ReadOnlyMemory<byte> payload, IDisposable? payloadOwner = null)
            {
                BattlementResponseStream current =
                    owner ?? throw new InvalidOperationException("Response reservation is closed.");
                try
                {
                    current.Commit(node, payload, payloadOwner);
                    owner = null;
                }
                catch
                {
                    payloadOwner?.Dispose();
                    throw;
                }
            }

            public void Commit(IBattlementResponseView response)
            {
                BattlementResponseStream current =
                    owner ?? throw new InvalidOperationException("Response reservation is closed.");
                try
                {
                    current.Commit(node, response);
                    owner = null;
                }
                catch
                {
                    response.Dispose();
                    throw;
                }
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
                Func<ReadOnlyMemory<byte>, IDisposable?, IBattlementResponseView> decode,
                bool isInitial,
                SessionId? previousSession,
                Action<IBattlementResponseView>? decoded,
                Action<Exception>? decodeFailed
            )
            {
                Decode = decode;
                IsInitial = isInitial;
                PreviousSession = previousSession;
                Decoded = decoded;
                DecodeFailed = decodeFailed;
            }

            public Func<ReadOnlyMemory<byte>, IDisposable?, IBattlementResponseView> Decode { get; }

            public Action<IBattlementResponseView>? Decoded { get; }

            public Action<Exception>? DecodeFailed { get; }

            public ReadOnlyMemory<byte> Payload { get; set; }

            public IDisposable? PayloadOwner { get; set; }

            public IBattlementResponseView? Response { get; set; }

            public bool IsInitial { get; }

            public SessionId? PreviousSession { get; }

            public int NextMessageIndex { get; set; }

            public bool IsValidated { get; set; }

            public bool IsCommitted { get; set; }

            public bool IsDecoded { get; set; }

            public bool DecodedNotified { get; set; }
        }
    }
}
