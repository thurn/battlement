#nullable enable

using System.IO;
using System.Linq;
using System.Reflection;
using System.Text;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using NUnit.Framework;
using UnityEngine;

namespace Battlement.Tests
{
    public sealed class DittoJobContractTests
    {
        private const string FixturePath =
            "Packages/com.battlement.client/Tests/Fixtures/Ditto/job-contract.json";

        [Test]
        public void SharedFixtureAcceptsValidJobAndRejectsEveryInvalidMutation()
        {
            JObject fixture = JObject.Parse(File.ReadAllText(FixturePath));
            JObject valid = (JObject)fixture["valid"]!;
            DittoJob job = Decode(valid);

            Assert.That(job.Scenarios[0].Steps.Count, Is.EqualTo(13));
            Assert.That(job.Profile.Capabilities, Does.Contain(DittoCapability.Video));
            foreach (JObject invalid in fixture["invalid"]!.Children<JObject>())
            {
                JObject changed = (JObject)valid.DeepClone();
                Apply(changed, invalid);
                Assert.Throws<JsonSerializationException>(
                    () => Decode(changed),
                    (string?)invalid["name"]
                );
            }
        }

        [Test]
        public void DiagnosticsDisabledAssemblyHasNoExecutorOrLogViewerReference()
        {
#if !BATTLEMENT_DITTO_DIAGNOSTICS
            Assembly runtime = typeof(BattlementRunner).Assembly;
            Assert.That(runtime.GetType("Battlement.BattlementDittoPlayerBootstrap"), Is.Null);
#if !UNITY_EDITOR
            FieldInfo[] fields = typeof(BattlementLoggingHost).GetFields(
                BindingFlags.Instance | BindingFlags.NonPublic
            );
            Assert.That(
                fields.Any(field => field.FieldType == typeof(BattlementLogViewer)),
                Is.False
            );
#endif
#endif
        }

        [Test]
        public void RunnerDiagnosticsAreEnabledByDefault()
        {
            var host = new GameObject("runner");
            try
            {
                Assert.That(host.AddComponent<BattlementRunner>().RunnerDiagnostics, Is.True);
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(host);
            }
        }

        private static DittoJob Decode(JObject value) =>
            DittoJobCodec.Decode(Encoding.UTF8.GetBytes(value.ToString(Formatting.None)));

        private static void Apply(JObject job, JObject mutation)
        {
            string pointer = (string)mutation["pointer"]!;
            string[] parts = pointer
                .Split('/')
                .Skip(1)
                .Select(part => part.Replace("~1", "/").Replace("~0", "~"))
                .ToArray();
            JToken parent = job;
            foreach (string part in parts.Take(parts.Length - 1))
            {
                parent = parent is JArray array ? array[int.Parse(part)] : parent[part]!;
            }
            string leaf = parts[^1];
            if ((bool?)mutation["remove"] == true)
            {
                ((JObject)parent).Property(leaf)!.Remove();
                return;
            }
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
