#nullable enable

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Threading;
using System.Threading.Tasks;

namespace Battlement
{
    /// <summary>Queues serialized responses and advances them in admission order.</summary>
    internal sealed class BattlementResponseStream
    {
        private const int MaximumResponseBytes = 16 * 1024 * 1024;
        private const int MaximumQueuedResponses = 256;
        private const long MaximumQueuedBytes = 64L * 1024 * 1024;
        private const int BackgroundDecodeThresholdBytes = 32 * 1024;

        private readonly LinkedList<PendingResponse> pending = new();
        private PendingResponse? active;
        private bool isProcessing;
        private int queuedResponses;
        private long queuedBytes;
        private ulong nextSequence;
        private Task<BackgroundDecodeResult>? backgroundDecodeTail;

        public bool HasPending => isProcessing || active is not null || pending.Count > 0;

        public long LastDecodeTicks { get; private set; }

        public long LastApplyTicks { get; private set; }

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
            SessionId? previousSession = null,
            Action<Response<ICommand>>? decoded = null,
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

        public void Drain(
            Func<Response<ICommand>, bool, SessionId?, bool> validate,
            Action<SessionId, ResponseMessage<ICommand>> apply,
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
                            if (active.BackgroundDecode is Task<BackgroundDecodeResult> task)
                            {
                                if (!task.IsCompleted)
                                {
                                    return;
                                }
                                BackgroundDecodeResult result = task.GetAwaiter().GetResult();
                                if (observeTiming)
                                {
                                    LastDecodeTicks = checked(
                                        LastDecodeTicks + result.ElapsedTicks
                                    );
                                }
                                if (result.Exception is Exception exception)
                                {
                                    active.DecodeFailed?.Invoke(exception);
                                    throw exception;
                                }
                                active.Response =
                                    result.Response
                                    ?? throw new InvalidDataException(
                                        "Background response decoding returned no result."
                                    );
                            }
                            else
                            {
                                long decodeStarted = observeTiming ? Stopwatch.GetTimestamp() : 0;
                                try
                                {
                                    using (BattlementProfiler.ResponseParsing.Auto())
                                    {
                                        active.Response = active.Decode(active.Payload);
                                    }
                                }
                                catch (Exception exception)
                                {
                                    active.DecodeFailed?.Invoke(exception);
                                    throw;
                                }
                                if (observeTiming)
                                {
                                    LastDecodeTicks = checked(
                                        LastDecodeTicks + Stopwatch.GetTimestamp() - decodeStarted
                                    );
                                }
                            }
                            active.IsDecoded = true;
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
                            && active.NextMessageIndex < active.Response!.Messages.Count
                        )
                        {
                            ResponseMessage<ICommand> message = active.Response.Messages[
                                active.NextMessageIndex++
                            ];
                            long applyStarted = observeTiming ? Stopwatch.GetTimestamp() : 0;
                            apply(active.Response.SessionId, message);
                            if (observeTiming)
                            {
                                LastApplyTicks = checked(
                                    LastApplyTicks + Stopwatch.GetTimestamp() - applyStarted
                                );
                            }
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
            backgroundDecodeTail = null;
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
            bool decodeInBackground
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
            node.Value.IsCommitted = true;
            queuedBytes += payload.Length;
            if (decodeInBackground && payload.Length >= BackgroundDecodeThresholdBytes)
            {
                PendingResponse response = node.Value;
                Task<BackgroundDecodeResult>? predecessor = backgroundDecodeTail;
                backgroundDecodeTail = predecessor is null
                    ? Task.Run(() => DecodeInBackground(response))
                    : predecessor.ContinueWith(
                        _ => DecodeInBackground(response),
                        CancellationToken.None,
                        TaskContinuationOptions.ExecuteSynchronously,
                        TaskScheduler.Default
                    );
                response.BackgroundDecode = backgroundDecodeTail;
            }
        }

        private static BackgroundDecodeResult DecodeInBackground(PendingResponse pending)
        {
            long started = Stopwatch.GetTimestamp();
            try
            {
                Response<ICommand> response;
                using (BattlementProfiler.ResponseParsing.Auto())
                {
                    response = pending.Decode(pending.Payload);
                }
                return new BackgroundDecodeResult(
                    response,
                    null,
                    Stopwatch.GetTimestamp() - started
                );
            }
            catch (Exception exception)
            {
                return new BackgroundDecodeResult(
                    null,
                    exception,
                    Stopwatch.GetTimestamp() - started
                );
            }
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
            if (ReferenceEquals(active.BackgroundDecode, backgroundDecodeTail))
            {
                backgroundDecodeTail = null;
            }
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

            public void Commit(ReadOnlyMemory<byte> payload, bool decodeInBackground = false)
            {
                BattlementResponseStream current =
                    owner ?? throw new InvalidOperationException("Response reservation is closed.");
                current.Commit(node, payload, decodeInBackground);
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
                SessionId? previousSession,
                Action<Response<ICommand>>? decoded,
                Action<Exception>? decodeFailed
            )
            {
                Decode = decode;
                IsInitial = isInitial;
                PreviousSession = previousSession;
                Decoded = decoded;
                DecodeFailed = decodeFailed;
            }

            public Func<ReadOnlyMemory<byte>, Response<ICommand>> Decode { get; }

            public Action<Response<ICommand>>? Decoded { get; }

            public Action<Exception>? DecodeFailed { get; }

            public Task<BackgroundDecodeResult>? BackgroundDecode { get; set; }

            public ReadOnlyMemory<byte> Payload { get; set; }

            public Response<ICommand>? Response { get; set; }

            public bool IsInitial { get; }

            public SessionId? PreviousSession { get; }

            public int NextMessageIndex { get; set; }

            public bool IsValidated { get; set; }

            public bool IsCommitted { get; set; }

            public bool IsDecoded { get; set; }
        }

        internal sealed record BackgroundDecodeResult(
            Response<ICommand>? Response,
            Exception? Exception,
            long ElapsedTicks
        );
    }
}
