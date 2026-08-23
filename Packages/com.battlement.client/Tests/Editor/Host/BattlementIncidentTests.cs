#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using NUnit.Framework;

namespace Battlement.Tests
{
    public sealed class BattlementIncidentTests
    {
        [Test]
        public void NativePanicCanBeDismissedToCreateANewEngineSession()
        {
            var sink = new FakeBattlementIncidentSink();
            var presenter = new FakeBattlementFailurePresenter();
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                incidentSink: sink,
                failurePresenter: presenter
            );
            harness.Transport.EnqueueConnect(
                new BattlementTransportResult(
                    BattlementTransportStatus.Panic,
                    diagnostic: "Rust panic in battlement_connect: secret engine detail"
                )
            );

            harness.Runner.Connect();

            Assert.That(harness.Runner.IsRestartRequired, Is.False);
            Assert.That(harness.Runner.IsInputAvailable, Is.False);
            Assert.That(harness.Transport.Calls, Is.EqualTo(new[] { "connect", "stop" }));
            Assert.That(presenter.Last, Is.EqualTo(harness.Runner.CurrentFailure));
            Assert.That(
                presenter.Last!.Kind,
                Is.EqualTo(BattlementPlayerFailureKind.ContinueAllowed)
            );
            Assert.That(presenter.Last.IncidentId, Does.Match("^[A-Za-z0-9_-]{22}$"));
            Assert.That(sink.Incidents, Has.Count.EqualTo(1));
            Assert.That(sink.Incidents[0].Source, Is.EqualTo(BattlementIncidentSource.Native));
            Assert.That(
                sink.Incidents[0].Fields["diagnostic"],
                Does.Contain("secret engine detail")
            );
            harness.Runner.ContinueAfterFailure();

            Assert.That(harness.Runner.CurrentFailure, Is.Null);
            Assert.That(harness.Runner.IsRestartRequired, Is.False);
            Assert.That(
                harness.Transport.Calls,
                Is.EqualTo(new[] { "connect", "stop", "connect" })
            );
        }

        [Test]
        public void RepeatedNativePanicRequiresRestart()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            BattlementTransportResult panic = new(
                BattlementTransportStatus.Panic,
                diagnostic: "deterministic panic"
            );
            harness.Transport.EnqueueConnect(panic);
            harness.Transport.EnqueueConnect(panic);

            harness.Runner.Connect();
            harness.Runner.ContinueAfterFailure();

            Assert.That(harness.Runner.IsRestartRequired, Is.True);
            Assert.That(
                harness.Runner.CurrentFailure!.Kind,
                Is.EqualTo(BattlementPlayerFailureKind.RestartRequired)
            );
            Assert.Throws<InvalidOperationException>(() => harness.Runner.Reconnect());
        }

        [Test]
        public void UnityExceptionIsLoggedWithoutInterruptingThePlayer()
        {
            var sink = new FakeBattlementIncidentSink();
            using BattlementTestHarness harness = BattlementTestHarness.Create(incidentSink: sink);
            harness.Runner.Connect();

            try
            {
                throw new InvalidOperationException("player update exploded");
            }
            catch (Exception exception)
            {
                harness.Runner.ReportUnhandledException(exception);
            }

            Assert.That(harness.Runner.IsRestartRequired, Is.False);
            Assert.That(harness.Runner.CurrentFailure, Is.Null);
            Assert.That(harness.Runner.IsInputAvailable, Is.True);
            Assert.That(harness.Transport.Calls, Does.Not.Contain("stop"));
            Assert.That(sink.Incidents, Has.Count.EqualTo(1));
            Assert.That(
                sink.Incidents[0].Disposition,
                Is.EqualTo(BattlementFailureDisposition.Logged)
            );
            Assert.That(
                sink.Incidents[0].EventName,
                Is.EqualTo("battlement.unhandled_unity_exception")
            );
            Assert.That(sink.Incidents[0].Message, Does.Contain("player update exploded"));
            Assert.That(
                sink.Incidents[0].StackTrace,
                Does.Contain(nameof(UnityExceptionIsLoggedWithoutInterruptingThePlayer))
            );
        }

        [Test]
        public void TransportFailureDoesNotSurfaceToPlayerAndManualReconnectWorks()
        {
            var sink = new FakeBattlementIncidentSink();
            var presenter = new FakeBattlementFailurePresenter();
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                incidentSink: sink,
                failurePresenter: presenter
            );
            harness.Transport.EnqueueConnect(
                new BattlementTransportResult(
                    BattlementTransportStatus.TransportError,
                    diagnostic: "offline"
                )
            );

            harness.Runner.Connect();

            Assert.That(harness.Runner.IsRestartRequired, Is.False);
            Assert.That(presenter.Last, Is.Null);
            harness.Runner.Reconnect();
            Assert.That(harness.Runner.CurrentFailure, Is.Null);
            Assert.That(presenter.HideCalls, Is.EqualTo(2));
        }

        [Test]
        public void FileSinkRetainsOnlyRecentDeveloperDiagnostics()
        {
            string directory = Path.Combine(
                Path.GetTempPath(),
                $"battlement-incidents-{Guid.NewGuid():N}"
            );
            try
            {
                var sink = new BattlementFileIncidentSink(directory);
                for (int index = 0; index < 21; index++)
                {
                    sink.Report(
                        new BattlementIncident(
                            $"TEST-{index:D4}",
                            DateTimeOffset.UtcNow,
                            BattlementFailureDisposition.SessionFailed,
                            BattlementIncidentSource.Unity,
                            "test.failure",
                            "developer-only detail",
                            new InvalidOperationException("original exception"),
                            "original stack",
                            new Dictionary<string, string> { ["session_id"] = "private" },
                            Array.Empty<BattlementLogRecord>()
                        )
                    );
                }

                string[] reports = Directory.GetFiles(directory, "*.json");
                Assert.That(reports, Has.Length.EqualTo(20));
                Assert.That(File.Exists(Path.Combine(directory, "TEST-0000.json")), Is.False);
                Assert.That(File.Exists(Path.Combine(directory, "TEST-0020.json")), Is.True);
                string report = File.ReadAllText(reports[0]);
                Assert.That(report, Does.Contain("developer-only detail"));
                Assert.That(report, Does.Contain("original exception"));
                Assert.That(report, Does.Contain("original stack"));
            }
            finally
            {
                if (Directory.Exists(directory))
                {
                    Directory.Delete(directory, true);
                }
            }
        }
    }

    internal sealed class FakeBattlementIncidentSink : IBattlementIncidentSink
    {
        public List<BattlementIncident> Incidents { get; } = new();

        public void Report(BattlementIncident incident) => Incidents.Add(incident);
    }

    internal sealed class FakeBattlementFailurePresenter : IBattlementFailurePresenter
    {
        public BattlementPlayerFailure? Last { get; private set; }

        public int HideCalls { get; private set; }

        public void Show(BattlementPlayerFailure failure) => Last = failure;

        public void Hide()
        {
            Last = null;
            HideCalls++;
        }
    }
}
