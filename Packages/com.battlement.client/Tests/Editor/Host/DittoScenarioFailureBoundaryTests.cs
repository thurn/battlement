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
    public sealed class DittoScenarioFailureBoundaryTests
    {
        private const string FixturePath =
            "Packages/com.battlement.client/Tests/Fixtures/Ditto/lifecycle-contract.json";
        private const string PlayerSessionId = "0197b35f-6d12-71ac-b370-0bb2cbced1b2";

        [SetUp]
        public void SetUp() => BattlementLogStore.Clear();

        [TestCase("unity.log", (int)DittoErrorCode.RuntimeUnityError)]
        [TestCase("unity.assert", (int)DittoErrorCode.RuntimeUnityAssert)]
        [TestCase("unity.exception", (int)DittoErrorCode.RuntimeUnityException)]
        public void FunctionalGateClassifiesUnityErrorRecords(string eventName, int expectedValue)
        {
            using var gate = new DittoFunctionalErrorGate(BattlementLogStore.Observe());
            gate.Open();
            BattlementLogStore.Add(
                "unity",
                new BattlementLogRecord(BattlementLogSeverity.Error, eventName, "fixture failure")
            );

            DittoDetectedFailure failure = gate.Poll()!;

            Assert.That(failure.Code, Is.EqualTo((DittoErrorCode)expectedValue));
            Assert.That(failure.Source, Is.EqualTo(DittoErrorSource.Unity));
            Assert.That(failure.BattlementErrorId, Is.Null);
        }

        [Test]
        public void StructuredPanicCorrelatesOnceAndCaughtEnvelopeStaysDiagnostic()
        {
            using var gate = new DittoFunctionalErrorGate(BattlementLogStore.Observe());
            gate.Open();
            BattlementLogStore.Add(
                "battlement",
                new BattlementLogRecord(
                    BattlementLogSeverity.Error,
                    "battlement.session.failed",
                    "native panic",
                    new Dictionary<string, string>
                    {
                        ["error_id"] = "native-fixture-error",
                        ["source"] = "Native",
                        ["status"] = "Panic",
                        ["type"] = "RestartRequired",
                    }
                )
            );
            BattlementLogStore.Add(
                "unity",
                new BattlementLogRecord(
                    BattlementLogSeverity.Error,
                    "unity.exception",
                    "BattlementCaughtFailureException: native panic"
                )
            );

            DittoDetectedFailure failure = gate.Poll()!;

            Assert.That(failure.Code, Is.EqualTo(DittoErrorCode.RuntimePanic));
            Assert.That(failure.Source, Is.EqualTo(DittoErrorSource.Rust));
            Assert.That(failure.BattlementErrorId, Is.EqualTo("native-fixture-error"));

            using var envelopeOnly = new DittoFunctionalErrorGate(BattlementLogStore.Observe());
            envelopeOnly.Open();
            BattlementLogStore.Add(
                "unity",
                new BattlementLogRecord(
                    BattlementLogSeverity.Error,
                    "unity.exception",
                    "BattlementCaughtFailureException: already correlated"
                )
            );
            BattlementLogStore.Add(
                "rust",
                new BattlementLogRecord(
                    BattlementLogSeverity.Error,
                    "game.tracing.error",
                    "ordinary tracing diagnostic"
                )
            );
            Assert.That(envelopeOnly.Poll(), Is.Null);
        }

        [Test]
        public void RuntimeFailureFreezesStepsAndClosesACompleteCorrelatedSpan()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            DittoResolvedScenario scenario = Scenario();
            DittoJob job = Job(scenario);
            var transport = new RecordingTransport();
            using var delivery = new DittoLogDelivery(
                BattlementLogStore.Observe(),
                transport,
                () => { }
            );
            delivery.BindFirstJob(job, PlayerSessionId, Array.Empty<string>());
            delivery.Flush(AssertSuccess);
            var references = new ReferenceAllocator();
            using var context = new DittoScenarioContext(
                job,
                scenario,
                delivery,
                BattlementLogStore.Observe(),
                references.Allocate
            );
            context.Begin();
            context.EngineStarted(Guid.NewGuid().ToString("D"));
            using var executor = new DittoScenarioExecutor(
                harness.Runner,
                scenario,
                DittoPlatform.Macos,
                100,
                100,
                new Dictionary<string, ObjectId>(),
                5_000,
                () => harness.Clock.Elapsed,
                _ => new DittoScreenshotStepOutcome(null, null, false),
                context.ReportFunctionalError,
                observeFailure: context.PollFailure,
                stepStarted: context.StepStarted,
                stepEnded: context.StepEnded
            );

            for (var advance = 0; advance < 8 && executor.CurrentStepIndex is null; advance++)
            {
                Assert.That(executor.Advance(), Is.False);
            }
            Assert.That(executor.CurrentStepIndex, Is.EqualTo(0));
            BattlementLogStore.Add(
                "unity",
                new BattlementLogRecord(
                    BattlementLogSeverity.Error,
                    "unity.assert",
                    "fixture assertion"
                )
            );
            Drain(executor);
            DittoScenarioExecution execution = executor.Result!;
            Assert.That(
                execution.Steps.Select(step => step.Status),
                Is.EqualTo(new[] { DittoStepStatus.Failed, DittoStepStatus.NotRun })
            );
            Assert.That(execution.PrimaryErrorRef, Is.EqualTo("P0001"));
            bool? captured = null;
            context.CaptureFailureFrame(
                executor.LastCommittedFrame,
                (frame, completion) =>
                    completion(new DittoNativeCaptureResult.Captured(Png(), 1, 1, frame)),
                value => captured = value
            );
            Assert.That(captured, Is.True);
            context.CloseForBoundary();
            context.EngineEnded(DittoExecutionStatus.Failed);
            DittoScenarioComplete? complete = null;
            context.Complete(
                execution,
                new DittoPlayerResetFailure(DittoBoundaryStage.Destroy, "fixture destroy failure"),
                7,
                Array.Empty<DittoNativeVideoInput>(),
                value => complete = value
            );

            Assert.That(complete, Is.Not.Null);
            Assert.That(complete!.FailureFrame, Is.TypeOf<DittoPlayerFailureFrame.Captured>());
            var boundary = (DittoScenarioBoundary.Failed)complete.Boundary;
            Assert.That(boundary.Stage, Is.EqualTo(DittoBoundaryStage.Destroy));
            Assert.That(boundary.ErrorRef, Is.EqualTo("P0002"));
            string transcript = string.Concat(
                transport
                    .Requests.Where(RequestIsLog)
                    .Select(request => Encoding.UTF8.GetString(request.Body))
            );
            AssertOrder(
                transcript,
                "\"context\":\"scenario-started\"",
                "\"context\":\"step-started\"",
                "\"event_name\":\"unity.assert\"",
                "\"context\":\"error-observed\"",
                "\"context\":\"step-ended\"",
                "\"context\":\"artifact-accepted\"",
                "\"context\":\"engine-ended\"",
                "runtime.destroy-failed",
                "\"context\":\"scenario-ended\""
            );
            TestContext.Progress.WriteLine(transcript);
        }

        private static DittoResolvedScenario Scenario() =>
            new(
                Guid.NewGuid().ToString("D"),
                0,
                "failure boundary",
                null,
                DittoMotion.Controlled,
                5_000,
                new[]
                {
                    new DittoResolvedStep(
                        0,
                        "active",
                        1_000,
                        new DittoStepAction.Wait(new DittoWait.Frames(300))
                    ),
                    new DittoResolvedStep(
                        1,
                        "unreached",
                        1_000,
                        new DittoStepAction.Wait(new DittoWait.Frames(1))
                    ),
                }
            );

        private static DittoJob Job(DittoResolvedScenario scenario)
        {
            JObject fixture = JObject.Parse(File.ReadAllText(FixturePath));
            return DittoJobCodec.Decode(
                Encoding.UTF8.GetBytes(fixture["job"]!.ToString(Formatting.None))
            ) with
            {
                Scenarios = new[] { scenario },
            };
        }

        private static byte[] Png() =>
            Convert.FromBase64String(
                "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAFgwJ/"
                    + "lJOCiwAAAABJRU5ErkJggg=="
            );

        private static void Drain(DittoScenarioExecutor executor)
        {
            for (var index = 0; index < 20 && !executor.Advance(); index++) { }
            Assert.That(executor.Result, Is.Not.Null);
        }

        private static bool RequestIsLog(DittoDeliveryRequest request) =>
            request.ContentType == "application/x-ndjson";

        private static void AssertSuccess(bool value) => Assert.That(value, Is.True);

        private static void AssertOrder(string value, params string[] markers)
        {
            var previous = -1;
            foreach (string marker in markers)
            {
                int current = value.IndexOf(marker, previous + 1, StringComparison.Ordinal);
                Assert.That(current, Is.GreaterThan(previous), marker);
                previous = current;
            }
        }

        private sealed class ReferenceAllocator
        {
            private int next;

            public string Allocate(DittoErrorCode _, string __) => $"P{++next:0000}";
        }

        private sealed class RecordingTransport : IDittoDeliveryTransport
        {
            public List<DittoDeliveryRequest> Requests { get; } = new();

            public void Send(DittoDeliveryRequest request, Action<DittoDeliveryResponse> completion)
            {
                Requests.Add(request);
                completion(
                    new DittoDeliveryResponse.Accepted(
                        request.ContentType == "image/png" ? ArtifactAck(request) : LogAck(request)
                    )
                );
            }

            public void SendAfter(
                TimeSpan _,
                DittoDeliveryRequest request,
                Action<DittoDeliveryResponse> completion
            ) => Send(request, completion);

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
