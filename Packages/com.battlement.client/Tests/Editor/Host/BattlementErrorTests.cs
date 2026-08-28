#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Battlement.Errors;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;
using UnityEngine.UIElements;

namespace Battlement.Tests
{
    public sealed class BattlementErrorTests
    {
        [Test]
        public void NativePanicCanBeDismissedToCreateANewEngineSession()
        {
            var sink = new FakeBattlementErrorSink();
            var presenter = new FakeBattlementFailurePresenter();
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                errorSink: sink,
                failurePresenter: presenter
            );
            harness.Transport.EnqueueConnect(
                new BattlementTransportResult(
                    BattlementTransportStatus.Panic,
                    diagnostic: "\u001b[31mRust panic in battlement_connect\u001b[0m"
                        + "\nsecret engine detail"
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
            Assert.That(presenter.Last.ErrorId, Does.Match("^[A-Za-z0-9_-]{22}$"));
            Assert.That(sink.Errors, Has.Count.EqualTo(1));
            Assert.That(sink.Errors[0].Source, Is.EqualTo(BattlementErrorSource.Native));
            Assert.That(sink.Errors[0].StackTrace, Does.Contain("secret engine detail"));
            Assert.That(sink.Errors[0].StackTrace, Does.Not.Contain("\u001b"));
            Assert.That(sink.Errors[0].Fields.ContainsKey("diagnostic"), Is.False);
            harness.Runner.ContinueAfterFailure();

            Assert.That(harness.Runner.CurrentFailure, Is.Null);
            Assert.That(harness.Runner.IsRestartRequired, Is.False);
            Assert.That(
                harness.Transport.Calls,
                Is.EqualTo(new[] { "connect", "stop", "connect" })
            );
        }

        [Test]
        public void NativePanicReportIncludesRecentRustTracing()
        {
            BattlementLogStore.Clear();
            BattlementLogStore.Add(
                "rust",
                new BattlementLogRecord(
                    BattlementLogSeverity.Information,
                    "rules.before_panic",
                    "moving card"
                )
            );
            var sink = new FakeBattlementErrorSink();
            var reporter = new BattlementErrorReporter(new BattlementUnityLogger(), sink);
            LogAssert.Expect(
                LogType.Error,
                new System.Text.RegularExpressions.Regex(
                    @"^\[Battlement/Managed\]\[battlement\.session\.failed\] caught panic"
                )
            );

            reporter.Report(
                BattlementErrorType.SessionFailed,
                BattlementErrorSource.Native,
                "battlement.session.failed",
                "caught panic",
                stackTrace: "rust backtrace"
            );

            Assert.That(
                sink.Errors.Single().RecentRecords.Select(record => record.EventName),
                Does.Contain("rules.before_panic")
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
        public void DevelopmentDiagnosticsTakePriorityOverPlayerFallback()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                suppressDevelopmentErrorDialogs: false
            );
            harness.Transport.EnqueueConnect(
                new BattlementTransportResult(
                    BattlementTransportStatus.Panic,
                    diagnostic: "\u001b[1;38;5;9mdeveloper <diagnostic>\u001b[0m"
                )
            );

            harness.Runner.Connect();

            VisualElement[] roots = harness
                .Runner.GetComponentsInChildren<UIDocument>()
                .Select(document => document.rootVisualElement)
                .ToArray();
            VisualElement player = roots.Single(root =>
                root.ClassListContains("battlement-error-overlay--player")
            );
            VisualElement development = roots.Single(root =>
                root.ClassListContains("battlement-error-overlay--development")
            );
            Assert.That(player.style.display.value, Is.EqualTo(DisplayStyle.None));
            Assert.That(development.style.display.value, Is.EqualTo(DisplayStyle.Flex));
            Label details = development.Q<Label>(className: "battlement-error-details");
            Assert.That(details.text, Does.Contain("<color=#FBBF24><b>"));
            Assert.That(details.text, Does.Contain("developer <noparse><</noparse>diagnostic>"));
            Assert.That(details.text, Does.Not.Contain("\u001b"));
        }

        [Test]
        public void UnityExceptionIsLoggedWithoutInterruptingThePlayer()
        {
            var sink = new FakeBattlementErrorSink();
            using BattlementTestHarness harness = BattlementTestHarness.Create(errorSink: sink);
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
            Assert.That(sink.Errors, Has.Count.EqualTo(1));
            Assert.That(sink.Errors[0].Type, Is.EqualTo(BattlementErrorType.Logged));
            Assert.That(
                sink.Errors[0].EventName,
                Is.EqualTo("battlement.unhandled_unity_exception")
            );
            Assert.That(sink.Errors[0].Message, Does.Contain("player update exploded"));
            Assert.That(
                sink.Errors[0].StackTrace,
                Does.Contain(nameof(UnityExceptionIsLoggedWithoutInterruptingThePlayer))
            );
        }

        [Test]
        public void UnityExceptionUsesReadableDiagnosticStyle()
        {
            var sink = new FakeBattlementErrorSink();
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                errorSink: sink,
                suppressDevelopmentErrorDialogs: false
            );
            harness.Runner.Connect();

            try
            {
                ThrowOuterDiagnosticException();
            }
            catch (Exception exception)
            {
                harness.Runner.ReportUnhandledException(exception);
            }

            VisualElement development = harness
                .Runner.GetComponentsInChildren<UIDocument>()
                .Select(document => document.rootVisualElement)
                .Single(root => root.ClassListContains("battlement-error-overlay--development"));
            Label details = development.Q<Label>(className: "battlement-error-details");
            Assert.That(
                details.text,
                Does.Contain("<color=#FF6B6B><b>InvalidOperationException</b></color>")
            );
            Assert.That(details.text, Does.Contain("outer <noparse><</noparse>failure>"));
            Assert.That(details.text, Does.Contain("Caused by:"));
            Assert.That(details.text, Does.Contain("ArgumentException"));
            Assert.That(details.text, Does.Contain("<color=#FBBF24><b>"));
            Assert.That(details.text, Does.Contain(nameof(ThrowOuterDiagnosticException)));
            Assert.That(details.text, Does.Not.Contain("Battlement.BattlementRunner"));
            Assert.That(sink.Errors[0].Message, Is.EqualTo("outer <failure>"));
            Assert.That(sink.Errors[0].Exception!.ToString(), Does.Contain("inner failure"));
        }

        [Test]
        public void CSharpDiagnosticCapsFramesAndReportsHiddenWork()
        {
            string stackTrace =
                "System.Threading.Tasks.Task.Execute()"
                + Environment.NewLine
                + string.Join(
                    Environment.NewLine,
                    Enumerable
                        .Range(0, 34)
                        .Select(index =>
                            $"Game.Actions.Frame{index} () (at Assets/Game.cs:{index + 1})"
                        )
                );

            BattlementFormattedText formatted = BattlementCSharpExceptionText.Format(
                null,
                stackTrace,
                "InvalidOperationException: failure"
            );

            Assert.That(formatted.PlainText, Does.Contain(": 1 framework frame hidden :"));
            Assert.That(formatted.PlainText, Does.Contain("31: Game.Actions.Frame31()"));
            Assert.That(formatted.PlainText, Does.Not.Contain("Game.Actions.Frame32()"));
            Assert.That(formatted.PlainText, Does.Contain(": 2 additional frames omitted :"));
        }

        [Test]
        public void TransportFailureDoesNotSurfaceToPlayerAndManualReconnectWorks()
        {
            var sink = new FakeBattlementErrorSink();
            var presenter = new FakeBattlementFailurePresenter();
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                errorSink: sink,
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
                $"battlement-errors-{Guid.NewGuid():N}"
            );
            try
            {
                var sink = new BattlementFileErrorSink(directory);
                for (int index = 0; index < 21; index++)
                {
                    sink.Report(
                        new BattlementError(
                            $"TEST-{index:D4}",
                            DateTimeOffset.UtcNow,
                            BattlementErrorType.SessionFailed,
                            BattlementErrorSource.Unity,
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

        private static void ThrowOuterDiagnosticException()
        {
            try
            {
                throw new ArgumentException("inner failure");
            }
            catch (ArgumentException exception)
            {
                throw new InvalidOperationException("outer <failure>", exception);
            }
        }
    }

    internal sealed class FakeBattlementErrorSink : IBattlementErrorSink
    {
        public List<BattlementError> Errors { get; } = new();

        public void Report(BattlementError error) => Errors.Add(error);
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
