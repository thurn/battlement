#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using System.Security.Cryptography;

namespace Battlement
{
    internal sealed record DittoDeliveryRequest(
        string Method,
        string Path,
        string ContentType,
        IReadOnlyDictionary<string, string> Headers,
        byte[] Body
    );

    internal abstract record DittoDeliveryResponse
    {
        internal sealed record Accepted(byte[] Body) : DittoDeliveryResponse;

        internal sealed record Rejected(int Status, byte[] Body) : DittoDeliveryResponse;

        internal sealed record Uncertain(string Reason) : DittoDeliveryResponse;
    }

    internal interface IDittoDeliveryTransport
    {
        void Send(DittoDeliveryRequest request, Action<DittoDeliveryResponse> completion);

        void SendAfter(
            TimeSpan delay,
            DittoDeliveryRequest request,
            Action<DittoDeliveryResponse> completion
        );
    }

    internal sealed record DittoPngArtifact(
        string ScenarioId,
        uint? StepIndex,
        string ArtifactId,
        DittoArtifactKind Kind,
        uint Width,
        uint Height,
        byte[] Png
    );

    internal sealed class DittoLogDelivery : IDisposable
    {
        private const int MaximumBatchBytes = 1024 * 1024;
        private const int MaximumPngBytes = 64 * 1024 * 1024;
        private const int MaximumRecords = 2_048;

        private readonly Queue<SerializedEntry> pending = new();
        private readonly BattlementLogObserver observer;
        private readonly IDittoDeliveryTransport transport;
        private readonly System.Action drainNative;
        private ActiveBatch? activeBatch;
        private DittoJob? job;
        private string playerSessionId = string.Empty;
        private IReadOnlyList<string> redactions = Array.Empty<string>();
        private ulong? lastCapturedSequence;
        private bool windowOpen = true;

        public DittoLogDelivery(
            BattlementLogObserver observer,
            IDittoDeliveryTransport transport,
            System.Action? drainNative = null
        )
        {
            this.observer = observer ?? throw new ArgumentNullException(nameof(observer));
            this.transport = transport ?? throw new ArgumentNullException(nameof(transport));
            this.drainNative = drainNative ?? BattlementNativeLogging.Drain;
        }

        public DittoPlayerInfrastructureFailure? Failure { get; private set; }

        public ulong? FirstLogSequence { get; private set; }

        public ulong? LastLogSequence => lastCapturedSequence;

        public void BindFirstJob(
            DittoJob value,
            string sessionId,
            IReadOnlyList<string> routeRedactions
        )
        {
            RequireUnbound();
            Bind(value, sessionId, routeRedactions);
            drainNative();
            Collect();
            QueueContext(new DittoContext.JobStarted(value.RunId), "job started");
        }

        public void BindWarmJob(
            DittoJob value,
            string sessionId,
            IReadOnlyList<string> routeRedactions
        )
        {
            RequireUnbound();
            drainNative();
            observer.Drain();
            windowOpen = true;
            Bind(value, sessionId, routeRedactions);
            QueueContext(new DittoContext.JobStarted(value.RunId), "job started");
        }

        public void EmitContext(DittoContext body, string message, Action<bool>? completion = null)
        {
            QueueContext(body, message);
            if (RequiresFlush(body))
            {
                Flush(
                    completion
                        ?? throw new ArgumentNullException(
                            nameof(completion),
                            "Completion boundaries require an acknowledgement callback."
                        )
                );
                return;
            }
            completion?.Invoke(true);
        }

        public void Flush(Action<bool> completion)
        {
            RequireBound();
            Collect();
            if (activeBatch is not null)
            {
                throw new InvalidOperationException("A Ditto log flush is already active.");
            }
            SendNext(completion ?? throw new ArgumentNullException(nameof(completion)));
        }

        public void UploadArtifact(DittoPngArtifact artifact, Action<bool> completion)
        {
            RequireBound();
            if (artifact.Png.Length == 0 || artifact.Png.Length > MaximumPngBytes)
            {
                Fail(DittoErrorCode.TransportRequestFailed, "PNG artifact size is invalid.");
                completion(false);
                return;
            }
            string sha256 = Hash(artifact.Png);
            var request = new DittoDeliveryRequest(
                "PUT",
                $"jobs/{job!.JobId}/artifacts/{artifact.ArtifactId}",
                "image/png",
                new Dictionary<string, string>
                {
                    ["X-Ditto-SHA256"] = sha256,
                    ["X-Ditto-Width"] = artifact.Width.ToString(),
                    ["X-Ditto-Height"] = artifact.Height.ToString(),
                },
                artifact.Png
            );
            SendWithRetry(
                request,
                response =>
                {
                    if (!Accepted(response, out byte[] body))
                    {
                        completion(false);
                        return;
                    }
                    try
                    {
                        DittoArtifactAck ack = DittoLifecycleCodec.Decode<DittoArtifactAck>(body);
                        DittoLifecycleValidation.ValidateArtifactAck(
                            ack,
                            artifact.ArtifactId,
                            sha256
                        );
                    }
                    catch (Exception exception)
                    {
                        Fail(DittoErrorCode.TransportArtifactConflict, exception.Message);
                        completion(false);
                        return;
                    }
                    EmitContext(
                        new DittoContext.ArtifactAccepted(
                            artifact.ScenarioId,
                            artifact.StepIndex,
                            artifact.ArtifactId,
                            artifact.Kind
                        ),
                        "artifact accepted",
                        completion
                    );
                }
            );
        }

        public void ConfirmUploadedArtifact(
            string scenarioId,
            uint? stepIndex,
            string artifactId,
            DittoArtifactKind kind,
            Action<bool> completion
        ) =>
            EmitContext(
                new DittoContext.ArtifactAccepted(scenarioId, stepIndex, artifactId, kind),
                "artifact accepted",
                completion
            );

        public void CloseAfterTerminalAcknowledgement()
        {
            Collect();
            if (activeBatch is not null || pending.Count != 0)
            {
                throw new InvalidOperationException("Logs must be acknowledged before closing.");
            }
            observer.Drain();
            job = null;
            redactions = Array.Empty<string>();
            playerSessionId = string.Empty;
            FirstLogSequence = null;
            lastCapturedSequence = null;
            windowOpen = false;
#if BATTLEMENT_DITTO_DIAGNOSTICS
            BattlementDittoPlayerBootstrap.WaitForNextJob();
#endif
        }

        public void Dispose() => observer.Dispose();

        private void Bind(DittoJob value, string sessionId, IReadOnlyList<string> routeRedactions)
        {
            DittoJobValidation.Validate(value);
            DittoLifecycleValidation.Identifier("player_session_id", sessionId);
            job = value;
            playerSessionId = sessionId;
            redactions = value
                .LogRedactions.Concat(routeRedactions)
                .Where(secret => secret.Length > 0)
                .Distinct()
                .ToArray();
        }

        private void QueueContext(DittoContext body, string message)
        {
            RequireBound();
            drainNative();
            Collect();
            BattlementLogStore.AddContext(
                "ditto-player",
                new BattlementLogRecord(
                    body is DittoContext.ErrorObserved
                        ? BattlementLogSeverity.Error
                        : BattlementLogSeverity.Information,
                    "ditto.context",
                    message
                ),
                body
            );
            Collect();
        }

        private void Collect()
        {
            BattlementLogEntry[] entries = observer.Drain();
            if (!windowOpen || job is null)
            {
                return;
            }
            foreach (BattlementLogEntry entry in entries)
            {
                if (lastCapturedSequence.HasValue && entry.Sequence != lastCapturedSequence + 1)
                {
                    Fail(DittoErrorCode.TransportLogGap, "The captured log sequence has a gap.");
                    return;
                }
                if (pending.Count == MaximumRecords)
                {
                    Fail(
                        DittoErrorCode.TransportLogBufferOverflow,
                        "The Ditto delivery queue exceeded 2,048 records."
                    );
                    return;
                }
                byte[] bytes = DittoEventSerialization.Encode(
                    entry,
                    job.JobId,
                    playerSessionId,
                    redactions
                );
                if (bytes.Length > MaximumBatchBytes)
                {
                    Fail(
                        DittoErrorCode.TransportLogRecordOversize,
                        "One serialized log record exceeded 1 MiB."
                    );
                    return;
                }
                pending.Enqueue(new SerializedEntry(entry.Sequence, bytes));
                FirstLogSequence ??= entry.Sequence;
                lastCapturedSequence = entry.Sequence;
                if (
                    entry.Record.EventName
                    is "battlement.logging.records_dropped"
                        or "battlement.logging.failed"
                )
                {
                    Fail(
                        entry.Record.EventName == "battlement.logging.records_dropped"
                            ? DittoErrorCode.TransportLogBufferOverflow
                            : DittoErrorCode.TransportRequestFailed,
                        "The native logging bridge reported incomplete history."
                    );
                }
            }
            if (observer.Overflowed)
            {
                Fail(
                    DittoErrorCode.TransportLogBufferOverflow,
                    "The Ditto observer delivery queue exceeded 2,048 records."
                );
            }
        }

        private void SendNext(Action<bool> completion)
        {
            if (pending.Count == 0)
            {
                completion(true);
                return;
            }
            var entries = new List<SerializedEntry>();
            int length = 0;
            foreach (SerializedEntry entry in pending)
            {
                if (length + entry.Bytes.Length > MaximumBatchBytes)
                {
                    break;
                }
                entries.Add(entry);
                length += entry.Bytes.Length;
            }
            byte[] body = new byte[length];
            int offset = 0;
            foreach (SerializedEntry entry in entries)
            {
                Buffer.BlockCopy(entry.Bytes, 0, body, offset, entry.Bytes.Length);
                offset += entry.Bytes.Length;
            }
            activeBatch = new ActiveBatch(entries, body);
            string hash = Hash(body);
            var request = new DittoDeliveryRequest(
                "PUT",
                $"jobs/{job!.JobId}/logs/{playerSessionId}?first_sequence={entries[0].Sequence}",
                "application/x-ndjson",
                new Dictionary<string, string> { ["X-Ditto-SHA256"] = hash },
                body
            );
            SendWithRetry(request, response => CompleteBatch(response, completion));
        }

        private void CompleteBatch(DittoDeliveryResponse response, Action<bool> completion)
        {
            if (!Accepted(response, out byte[] body))
            {
                activeBatch = null;
                completion(false);
                return;
            }
            ActiveBatch batch = activeBatch!;
            try
            {
                ulong next = checked(batch.Entries[^1].Sequence + 1);
                DittoLogBatchAck ack = DittoLifecycleCodec.Decode<DittoLogBatchAck>(body);
                DittoLifecycleValidation.ValidateLogAck(ack, playerSessionId, next);
            }
            catch (Exception exception)
            {
                activeBatch = null;
                Fail(DittoErrorCode.TransportLogConflict, exception.Message);
                completion(false);
                return;
            }
            foreach (SerializedEntry entry in batch.Entries)
            {
                if (pending.Dequeue().Sequence != entry.Sequence)
                {
                    throw new InvalidOperationException("The Ditto retry buffer changed order.");
                }
            }
            activeBatch = null;
            SendNext(completion);
        }

        private void SendWithRetry(
            DittoDeliveryRequest request,
            Action<DittoDeliveryResponse> completion,
            bool retried = false
        ) =>
            transport.Send(
                request,
                response =>
                {
                    if (response is DittoDeliveryResponse.Uncertain && !retried)
                    {
                        transport.SendAfter(TimeSpan.FromMilliseconds(100), request, completion);
                        return;
                    }
                    completion(response);
                }
            );

        private bool Accepted(DittoDeliveryResponse response, out byte[] body)
        {
            if (response is DittoDeliveryResponse.Accepted accepted)
            {
                body = accepted.Body;
                return true;
            }
            body = Array.Empty<byte>();
            if (response is DittoDeliveryResponse.Rejected rejected)
            {
                try
                {
                    DittoHttpError error = DittoLifecycleCodec.Decode<DittoHttpError>(
                        rejected.Body
                    );
                    DittoLifecycleValidation.ValidateHttpError(error);
                    Fail(error.Code, error.Message);
                    return false;
                }
                catch (Exception exception)
                {
                    Fail(DittoErrorCode.TransportRequestFailed, exception.Message);
                    return false;
                }
            }
            Fail(
                DittoErrorCode.TransportRequestFailed,
                ((DittoDeliveryResponse.Uncertain)response).Reason
            );
            return false;
        }

        private void Fail(DittoErrorCode code, string reason) =>
            Failure ??= new DittoPlayerInfrastructureFailure(code, reason);

        private static string Hash(byte[] bytes)
        {
            using SHA256 sha256 = SHA256.Create();
            return BitConverter
                .ToString(sha256.ComputeHash(bytes))
                .Replace("-", "")
                .ToLowerInvariant();
        }

        private static bool RequiresFlush(DittoContext body) =>
            body
                is DittoContext.StepEnded
                    or DittoContext.ArtifactAccepted
                    or DittoContext.ScenarioEnded
                    or DittoContext.JobEnded;

        private void RequireBound()
        {
            if (job is null || !windowOpen)
            {
                throw new InvalidOperationException("No Ditto job capture window is open.");
            }
        }

        private void RequireUnbound()
        {
            if (job is not null)
            {
                throw new InvalidOperationException("A Ditto job is already bound.");
            }
        }

        private sealed record SerializedEntry(ulong Sequence, byte[] Bytes);

        private sealed record ActiveBatch(IReadOnlyList<SerializedEntry> Entries, byte[] Bytes);
    }
}
