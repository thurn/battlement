#nullable enable

using System;
using System.Collections.Generic;
using System.IO;

namespace Masonry
{
    /// <summary>Queues decoded responses and advances them in protocol order.</summary>
    internal sealed class MasonryResponseStream
    {
        private const int MaximumResponseBytes = 16 * 1024 * 1024;
        private const int MaximumQueuedResponses = 256;

        private readonly LinkedList<PendingResponse> pending = new();
        private PendingResponse? active;
        private bool isProcessing;

        public void Enqueue(
            IMasonryProtocolCodec codec,
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
                        $"A Masonry response cannot exceed {MaximumResponseBytes} bytes."
                    );
                }

                Response response;
                using (MasonryProfiler.ResponseParsing.Auto())
                {
                    response = codec.DeserializeResponse(payload);
                }

                if (pending.Count >= MaximumQueuedResponses)
                {
                    throw new InvalidDataException(
                        $"Masonry cannot queue more than {MaximumQueuedResponses} responses."
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
            Func<Response, bool, SessionId?, bool> validate,
            Action<SessionId, ResponseMessage<Command>> apply,
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
                using (MasonryProfiler.ResponseApplication.Auto())
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
                            ResponseMessage<Command> message = active.Response.Messages[
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
            public PendingResponse(Response response, bool isInitial, SessionId? previousSession)
            {
                Response = response;
                IsInitial = isInitial;
                PreviousSession = previousSession;
            }

            public Response Response { get; }

            public bool IsInitial { get; }

            public SessionId? PreviousSession { get; }

            public int NextMessageIndex { get; set; }

            public bool IsValidated { get; set; }
        }
    }
}
