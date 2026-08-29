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
    public sealed class DittoLifecycleContractTests
    {
        private const string FixturePath =
            "Packages/com.battlement.client/Tests/Fixtures/Ditto/lifecycle-contract.json";
        private const string PlayerSessionId = "0197b35f-6d12-71ac-b370-0bb2cbced1b2";
        private const string JobId = "0197b35f-6c59-7b98-b1f0-a39f5ee54db8";
        private const string ArtifactId = "0197b35f-6ef0-78df-8b96-b31bc9959181";
        private const string Hash =
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

        [Test]
        public void SharedLifecycleFixtureAcceptsExactMixedExchange()
        {
            JObject fixture = Fixture();
            DittoJob job = Job();
            DittoStarted started = Decode<DittoStarted>(fixture["started"]!);
            DittoLifecycleValidation.ValidateStarted(started, job, PlayerSessionId, null);

            DittoScenarioComplete complete = Decode<DittoScenarioComplete>(
                fixture["scenario_complete"]!
            );
            DittoCompletionValidation.ValidateScenarioComplete(complete, job, new[] { "P0001" });
            Assert.That(
                JToken.DeepEquals(
                    JToken.Parse(Encoding.UTF8.GetString(DittoLifecycleCodec.Encode(complete))),
                    fixture["scenario_complete"]
                ),
                Is.True
            );

            byte[] expected = Ndjson((JArray)fixture["events"]!);
            IReadOnlyList<DittoEventRecord> records = DittoLifecycleCodec.DecodeNdjson(
                expected,
                job,
                PlayerSessionId,
                81
            );
            Assert.That(records.Count, Is.EqualTo(2));
            Assert.That(records[0], Is.TypeOf<DittoOrdinaryLogRecord>());
            Assert.That(records[1], Is.TypeOf<DittoContextRecord>());
            Assert.That(
                Encoding.UTF8.GetString(DittoLifecycleCodec.EncodeNdjson(records)),
                Is.EqualTo(Encoding.UTF8.GetString(expected))
            );
        }

        [Test]
        public void SharedLifecycleFixtureRejectsEveryInvalidMutation()
        {
            JObject fixture = Fixture();
            foreach (JObject invalid in fixture["invalid"]!.Children<JObject>())
            {
                string target = (string)invalid["target"]!;
                JToken changed = fixture[target]!.DeepClone();
                Apply(changed, invalid);
                Assert.Throws<JsonSerializationException>(
                    () => Validate(target, changed),
                    (string?)invalid["name"]
                );
            }
        }

        [Test]
        public void EveryContextAndConditionalLifecycleResponseRoundTrips()
        {
            foreach (JToken token in Fixture()["contexts"]!)
            {
                DittoContext context = Decode<DittoContext>(token);
                Assert.That(
                    Encoding.UTF8.GetString(DittoLifecycleCodec.Encode(context)),
                    Is.EqualTo(token.ToString(Formatting.None))
                );
            }

            DittoLifecycleValidation.ValidateLogAck(
                new DittoLogBatchAck(PlayerSessionId, 83),
                PlayerSessionId,
                83
            );
            DittoLifecycleValidation.ValidateArtifactAck(
                new DittoArtifactAck(ArtifactId, Hash),
                ArtifactId,
                Hash
            );
            DittoLifecycleValidation.ValidateScenarioDecision(
                new DittoScenarioDecision(DittoNextAction.Continue, 1, null, null, null)
            );
            DittoLifecycleValidation.ValidateCompleteAck(new DittoJobCompleteAck(JobId), JobId);
            DittoLifecycleValidation.ValidateFailedAck(
                new DittoJobFailedAck(JobId, "E0001"),
                JobId
            );
            DittoLifecycleValidation.ValidateHttpError(
                new DittoHttpError("E0002", DittoErrorCode.TransportLogGap, "gap", 83, null)
            );
        }

        [Test]
        public void ScenarioDecisionDecodesNullableErrorCode()
        {
            DittoScenarioDecision decision = Decode<DittoScenarioDecision>(
                JObject.Parse(
                    "{\"action\":\"stop\",\"completed_failures\":0,"
                        + "\"error_id\":\"E0001\",\"error_code\":\"image.capture-failed\","
                        + "\"message\":\"capture failed\"}"
                )
            );

            Assert.That(decision.ErrorCode, Is.EqualTo(DittoErrorCode.ImageCaptureFailed));
            DittoLifecycleValidation.ValidateScenarioDecision(decision);
        }

        [Test]
        public void TerminalAccountingAndFailureResponsesAreClosed()
        {
            DittoJob job = Job();
            string scenarioId = job.Scenarios[0].Id;
            DittoLifecycleValidation.ValidateJobComplete(
                new DittoJobComplete(
                    job.JobId,
                    90,
                    new[] { scenarioId },
                    Array.Empty<DittoUnstartedScenario>(),
                    DittoTerminalReason.Completed,
                    12
                ),
                job
            );
            DittoLifecycleValidation.ValidateJobFailed(
                new DittoJobFailed(
                    job.JobId,
                    new DittoPlayerInfrastructureFailure(
                        DittoErrorCode.RuntimeProcessExit,
                        "player exited"
                    ),
                    null,
                    Array.Empty<string>(),
                    new[] { new DittoUnstartedScenario(scenarioId, "run-infrastructure-error") }
                ),
                job
            );
            Assert.Throws<JsonSerializationException>(() =>
                DittoLifecycleValidation.ValidateScenarioDecision(
                    new DittoScenarioDecision(
                        DittoNextAction.Continue,
                        0,
                        "E0001",
                        DittoErrorCode.StartupMismatch,
                        "mismatch"
                    )
                )
            );
        }

        private static void Validate(string target, JToken changed)
        {
            if (target == "started")
            {
                DittoLifecycleValidation.ValidateStarted(
                    Decode<DittoStarted>(changed),
                    Job(),
                    PlayerSessionId,
                    null
                );
                return;
            }
            if (target == "scenario_complete")
            {
                DittoCompletionValidation.ValidateScenarioComplete(
                    Decode<DittoScenarioComplete>(changed),
                    Job(),
                    new[] { "P0001" }
                );
                return;
            }
            DittoLifecycleCodec.DecodeNdjson(Ndjson((JArray)changed), Job(), PlayerSessionId, 81);
        }

        private static T Decode<T>(JToken value) =>
            DittoLifecycleCodec.Decode<T>(Encoding.UTF8.GetBytes(value.ToString(Formatting.None)));

        private static DittoJob Job()
        {
            JObject fixture = Fixture();
            return DittoJobCodec.Decode(
                Encoding.UTF8.GetBytes(fixture["job"]!.ToString(Formatting.None))
            );
        }

        private static JObject Fixture() => JObject.Parse(File.ReadAllText(FixturePath));

        private static byte[] Ndjson(JArray records) =>
            Encoding.UTF8.GetBytes(
                string.Join("\n", records.Select(record => record.ToString(Formatting.None))) + "\n"
            );

        private static void Apply(JToken root, JObject mutation)
        {
            string[] parts = ((string)mutation["pointer"]!).Split('/').Skip(1).ToArray();
            JToken parent = root;
            foreach (string part in parts.Take(parts.Length - 1))
            {
                parent = parent is JArray array ? array[int.Parse(part)] : parent[part]!;
            }
            string leaf = parts[^1];
            JToken replacement = mutation["value"]!.DeepClone();
            if (parent is JArray values)
            {
                values[int.Parse(leaf)] = replacement;
            }
            else
            {
                ((JObject)parent)[leaf] = replacement;
            }
        }
    }
}
