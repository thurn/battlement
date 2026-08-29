#nullable enable

using System;
using System.Collections.Generic;
using System.IO;

namespace Battlement
{
    /// <summary>Queues decoded responses and advances them in protocol order.</summary>
    internal sealed class BattlementResponseStream
    {
        private const int MaximumResponseBytes = 16 * 1024 * 1024;
        private const int MaximumQueuedResponses = 256;

        private readonly LinkedList<PendingResponse> pending = new();
        private PendingResponse? active;
        private bool isProcessing;

        public bool HasPending => isProcessing || active is not null || pending.Count > 0;

        public void Enqueue(
            Func<ReadOnlyMemory<byte>, Response<ICommand>> decode,
            ReadOnlyMemory<byte> payload,
            bool isInitial,
            SessionId? previousSession
        )
        {
            bool ownsProcessing = !isProcessing;
            LinkedListNode<PendingResponse>? preceding = ownsProcessing ? pending.Last : null;
            if (ownsProcessing)
            {
                isProcessing = true;
            }

            try
            {
                if (payload.Length > MaximumResponseBytes)
                {
                    throw new InvalidDataException(
                        $"A Battlement response cannot exceed {MaximumResponseBytes} bytes."
                    );
                }

                Response<ICommand> response;
                using (BattlementProfiler.ResponseParsing.Auto())
                {
                    response = decode(payload);
                }

                if (pending.Count >= MaximumQueuedResponses)
                {
                    throw new InvalidDataException(
                        $"Battlement cannot queue more than {MaximumQueuedResponses} responses."
                    );
                }

                var queued = new PendingResponse(response, isInitial, previousSession);
                if (!ownsProcessing)
                {
                    pending.AddLast(queued);
                }
                else if (preceding is not null)
                {
                    pending.AddAfter(preceding, queued);
                }
                else
                {
                    pending.AddFirst(queued);
                }
            }
            finally
            {
                if (ownsProcessing)
                {
                    isProcessing = false;
                }
            }
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

                        if (!active.IsValidated)
                        {
                            active.IsValidated = true;
                            if (
                                !validate(active.Response, active.IsInitial, active.PreviousSession)
                            )
                            {
                                active = null;
                                continue;
                            }
                        }

                        while (
                            !isStopped()
                            && !isPaused()
                            && active is not null
                            && active.NextMessageIndex < active.Response.Messages.Count
                        )
                        {
                            ResponseMessage<ICommand> message = active.Response.Messages[
                                active.NextMessageIndex++
                            ];
                            apply(active.Response.SessionId, message);
                        }

                        if (
                            active is not null
                            && active.NextMessageIndex >= active.Response.Messages.Count
                        )
                        {
                            active = null;
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
        }

        private PendingResponse? TakeNext()
        {
            if (pending.First is not LinkedListNode<PendingResponse> next)
            {
                return null;
            }

            pending.RemoveFirst();
            return next.Value;
        }

        private sealed class PendingResponse
        {
            public PendingResponse(
                Response<ICommand> response,
                bool isInitial,
                SessionId? previousSession
            )
            {
                Response = response;
                IsInitial = isInitial;
                PreviousSession = previousSession;
            }

            public Response<ICommand> Response { get; }

            public bool IsInitial { get; }

            public SessionId? PreviousSession { get; }

            public int NextMessageIndex { get; set; }

            public bool IsValidated { get; set; }
        }
    }
}
