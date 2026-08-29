#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using NUnit.Framework;

namespace Battlement.Tests
{
    public sealed class DittoLogDeliveryTests
    {
        private const string FixturePath =
            "Packages/com.battlement.client/Tests/Fixtures/Ditto/lifecycle-contract.json";
        private const string PlayerSessionId = "0197b35f-6d12-71ac-b370-0bb2cbced1b2";

        [SetUp]
        public void SetUp() => BattlementLogStore.Clear();

        [Test]
        public void LostAcknowledgementReplaysExactRedactedMixedNdjson()
        {
            var transport = new RecordingTransport { LoseNextAcknowledgement = true };
            using var delivery = Delivery(transport);
            BattlementLogStore.Add(
                "rust",
                new BattlementLogRecord(
                    BattlementLogSeverity.Information,
                    "chess.move",
                    "token-123 moved",
                    new Dictionary<string, string>
                    {
                        ["detail"] = "/secret/path",
                        ["source"] = "token-123",
                    }
                ),
                DateTimeOffset.FromUnixTimeMilliseconds(1_787_953_800_000)
            );

            delivery.BindFirstJob(
                Job() with
                {
                    LogRedactions = new[] { "/secret/path" },
                },
                PlayerSessionId,
                new[] { "token-123" }
            );
            delivery.EmitContext(
                new DittoContext.ScenarioStarted(ScenarioId()),
                "scenario token-123 started"
            );
            bool? completed = null;
            delivery.Flush(value => completed = value);

            Assert.That(completed, Is.True);
            Assert.That(transport.Requests, Has.Count.EqualTo(2));
            Assert.That(transport.Requests[0], Is.SameAs(transport.Requests[1]));
            Assert.That(
                transport.RetryDelays,
                Is.EqualTo(new[] { TimeSpan.FromMilliseconds(100) })
            );
            string body = Encoding.UTF8.GetString(transport.Requests[0].Body);
            Assert.That(body, Does.EndWith("\n"));
            Assert.That(body, Does.Not.Contain("token-123"));
            Assert.That(body, Does.Not.Contain("/secret/path"));
            Assert.That(body, Does.Contain("<redacted> moved"));
            Assert.That(
                body.Split('\n', StringSplitOptions.RemoveEmptyEntries),
                Has.Length.EqualTo(3)
            );
            TestContext.Progress.WriteLine(body);
        }

        [Test]
        public void ArtifactIsAcknowledgedBeforeItsContextMarkerAndFlush()
        {
            var transport = new RecordingTransport();
            using var delivery = Delivery(transport);
            delivery.BindFirstJob(Job(), PlayerSessionId, Array.Empty<string>());
            delivery.Flush(AssertSuccess);
            transport.Requests.Clear();
            string artifactId = Guid.NewGuid().ToString("D");
            byte[] png = Convert.FromBase64String(
                "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAFgwJ/"
                    + "lJOCiwAAAABJRU5ErkJggg=="
            );

            delivery.UploadArtifact(
                new DittoPngArtifact(
                    ScenarioId(),
                    3,
                    artifactId,
                    new DittoArtifactKind.Screenshot("board"),
                    1,
                    1,
                    png
                ),
                AssertSuccess
            );

            Assert.That(transport.Requests, Has.Count.EqualTo(2));
            Assert.That(transport.Requests[0].ContentType, Is.EqualTo("image/png"));
            Assert.That(transport.Requests[0].Body, Is.EqualTo(png));
            Assert.That(transport.Requests[1].ContentType, Is.EqualTo("application/x-ndjson"));
            string marker = Encoding.UTF8.GetString(transport.Requests[1].Body);
            Assert.That(marker, Does.Contain("\"context\":\"artifact-accepted\""));
            Assert.That(marker, Does.Contain(artifactId));
            TestContext.Progress.WriteLine(Convert.ToBase64String(png));
            TestContext.Progress.WriteLine(marker);
        }

        [Test]
        public void WarmIdleRecordsNeverEnterTheNextJobWindow()
        {
            var transport = new RecordingTransport();
            using var delivery = Delivery(transport);
            delivery.BindFirstJob(Job(), PlayerSessionId, Array.Empty<string>());
            delivery.Flush(AssertSuccess);
            delivery.CloseAfterTerminalAcknowledgement();
            BattlementLogStore.Add(
                "unity",
                new BattlementLogRecord(BattlementLogSeverity.Information, "unity.log", "warm idle")
            );
            DittoJob next = Job() with
            {
                JobId = Guid.NewGuid().ToString("D"),
                RunId = Guid.NewGuid().ToString("D"),
            };

            delivery.BindWarmJob(next, PlayerSessionId, Array.Empty<string>());
            BattlementLogStore.Add(
                "unity",
                new BattlementLogRecord(
                    BattlementLogSeverity.Information,
                    "unity.log",
                    "inside next job"
                )
            );
            transport.Requests.Clear();
            delivery.Flush(AssertSuccess);

            string body = Encoding.UTF8.GetString(transport.Requests.Single().Body);
            Assert.That(body, Does.Not.Contain("warm idle"));
            Assert.That(body, Does.Contain("inside next job"));
        }

        [Test]
        public void GapConflictOversizeNativeDropAndOverflowFailClosed()
        {
            AssertRejectedFailure(
                DittoErrorCode.TransportLogGap,
                new DittoHttpError("E0001", DittoErrorCode.TransportLogGap, "gap", 1, null)
            );
            AssertRejectedFailure(
                DittoErrorCode.TransportLogConflict,
                new DittoHttpError(
                    "E0001",
                    DittoErrorCode.TransportLogConflict,
                    "conflict",
                    null,
                    null
                )
            );

            using (DittoLogDelivery oversize = Delivery(new RecordingTransport()))
            {
                BattlementLogStore.Add(
                    "unity",
                    new BattlementLogRecord(
                        BattlementLogSeverity.Error,
                        "unity.log",
                        new string('x', 1024 * 1024)
                    )
                );
                oversize.BindFirstJob(Job(), PlayerSessionId, Array.Empty<string>());
                Assert.That(
                    oversize.Failure!.Code,
                    Is.EqualTo(DittoErrorCode.TransportLogRecordOversize)
                );
            }

            BattlementLogStore.Clear();
            using (DittoLogDelivery dropped = Delivery(new RecordingTransport()))
            {
                BattlementLogStore.Add(
                    "rust",
                    new BattlementLogRecord(
                        BattlementLogSeverity.Warning,
                        "battlement.logging.records_dropped",
                        "native records dropped"
                    )
                );
                dropped.BindFirstJob(Job(), PlayerSessionId, Array.Empty<string>());
                Assert.That(
                    dropped.Failure!.Code,
                    Is.EqualTo(DittoErrorCode.TransportLogBufferOverflow)
                );
            }

            BattlementLogStore.Clear();
            using (DittoLogDelivery overflow = Delivery(new RecordingTransport()))
            {
                for (var index = 0; index <= 2_048; index++)
                {
                    BattlementLogStore.Add(
                        "unity",
                        new BattlementLogRecord(
                            BattlementLogSeverity.Information,
                            "unity.log",
                            index.ToString()
                        )
                    );
                }
                overflow.BindFirstJob(Job(), PlayerSessionId, Array.Empty<string>());
                Assert.That(
                    overflow.Failure!.Code,
                    Is.EqualTo(DittoErrorCode.TransportLogBufferOverflow)
                );
            }
        }

        private static DittoLogDelivery Delivery(RecordingTransport transport) =>
            new(BattlementLogStore.Observe(), transport, () => { });

        private static DittoJob Job()
        {
            JObject fixture = JObject.Parse(File.ReadAllText(FixturePath));
            return DittoJobCodec.Decode(
                Encoding.UTF8.GetBytes(fixture["job"]!.ToString(Formatting.None))
            );
        }

        private static string ScenarioId() => Job().Scenarios[0].Id;

        private static void AssertSuccess(bool value) => Assert.That(value, Is.True);

        private static void AssertRejectedFailure(DittoErrorCode expected, DittoHttpError error)
        {
            BattlementLogStore.Clear();
            var transport = new RecordingTransport { Rejection = error };
            using DittoLogDelivery delivery = Delivery(transport);
            delivery.BindFirstJob(Job(), PlayerSessionId, Array.Empty<string>());
            delivery.Flush(value => Assert.That(value, Is.False));
            Assert.That(delivery.Failure!.Code, Is.EqualTo(expected));
        }

        private sealed class RecordingTransport : IDittoDeliveryTransport
        {
            public readonly List<DittoDeliveryRequest> Requests = new();

            public bool LoseNextAcknowledgement { get; set; }

            public DittoHttpError? Rejection { get; set; }

            public List<TimeSpan> RetryDelays { get; } = new();

            public void Send(DittoDeliveryRequest request, Action<DittoDeliveryResponse> completion)
            {
                Requests.Add(request);
                if (LoseNextAcknowledgement)
                {
                    LoseNextAcknowledgement = false;
                    completion(new DittoDeliveryResponse.Uncertain("acknowledgement lost"));
                    return;
                }
                if (Rejection is not null)
                {
                    completion(
                        new DittoDeliveryResponse.Rejected(
                            409,
                            DittoLifecycleCodec.Encode(Rejection)
                        )
                    );
                    return;
                }
                completion(
                    new DittoDeliveryResponse.Accepted(
                        request.ContentType == "image/png" ? ArtifactAck(request) : LogAck(request)
                    )
                );
            }

            public void SendAfter(
                TimeSpan delay,
                DittoDeliveryRequest request,
                Action<DittoDeliveryResponse> completion
            )
            {
                RetryDelays.Add(delay);
                Send(request, completion);
            }

            private static byte[] ArtifactAck(DittoDeliveryRequest request) =>
                DittoLifecycleCodec.Encode(
                    new DittoArtifactAck(
                        request.Path.Split('/')[^1],
                        request.Headers["X-Ditto-SHA256"]
                    )
                );

            private static byte[] LogAck(DittoDeliveryRequest request)
            {
                string[] lines = Encoding
                    .UTF8.GetString(request.Body)
                    .Split('\n', StringSplitOptions.RemoveEmptyEntries);
                ulong next = JObject.Parse(lines[^1]).Value<ulong>("sequence") + 1;
                return DittoLifecycleCodec.Encode(new DittoLogBatchAck(PlayerSessionId, next));
            }
        }
    }
}
