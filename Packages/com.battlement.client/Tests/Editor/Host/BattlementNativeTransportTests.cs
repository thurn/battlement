#nullable enable

using System;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;

namespace Battlement.Tests
{
    public sealed class BattlementNativeTransportTests
    {
        [Test]
        public void RunnerReusesOneEngineAcrossConnectsAndDestroysItAtShutdown()
        {
            ulong callsBefore = NativeFixture.fixture_connect_calls().ToUInt64();
            var transport = Transport("normal");
            using (transport)
            {
                Assert.That(transport.Connect(ConnectBytes("normal")).Status, Is.EqualTo(Success));
                Assert.That(transport.LastConnectResult!.Status, Is.EqualTo(Success));
                transport.Stop();
                Assert.That(transport.Connect(ConnectBytes("normal")).Status, Is.EqualTo(Success));
                Assert.That(transport.LastConnectResult!.Status, Is.EqualTo(Success));
                Assert.That(
                    NativeFixture.fixture_connect_calls().ToUInt64(),
                    Is.EqualTo(callsBefore + 2)
                );
                Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
            }

            Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
        }

        [Test]
        public void DittoScenarioSessionsCreateDistinctEnginesOnlyWhenReached()
        {
            BattlementNativeLogging.Drain();
            BattlementLogStore.Clear();
            using var transport = new BattlementNativeTransport();
            Assert.That(transport.HasEngine, Is.False);
            Assert.That(EngineJournal(), Is.Empty);

            DittoNativeEngineSession first = CreateSession(transport);
            Assert.That(Guid.TryParse(first.Id, out _), Is.True);
            Assert.That(transport.HasEngine, Is.True);
            Assert.That(first.Connect(ConnectBytes("ditto-one")).Status, Is.EqualTo(Success));
            Assert.Throws<InvalidOperationException>(() => first.Connect(ConnectBytes("normal")));
            Assert.That(first.Destroy().Status, Is.EqualTo(Success));
            Assert.That(first.Destroy().Status, Is.EqualTo(Success));
            Assert.That(transport.HasEngine, Is.False);

            DittoNativeEngineSession second = CreateSession(transport);
            Assert.That(second.Id, Is.Not.EqualTo(first.Id));
            Assert.That(second.Connect(ConnectBytes("ditto-two")).Status, Is.EqualTo(Success));
            Assert.That(second.Destroy().Status, Is.EqualTo(Success));
            Assert.That(transport.HasEngine, Is.False);

            BattlementLogEntry[] journal = EngineJournal();
            Assert.That(
                journal.Select(entry => entry.Record.EventName),
                Is.EqualTo(
                    new[]
                    {
                        "fixture.engine.created",
                        "fixture.engine.connected",
                        "fixture.engine.destroyed",
                        "fixture.engine.created",
                        "fixture.engine.connected",
                        "fixture.engine.destroyed",
                    }
                )
            );
            string firstEngine = journal[0].Record.Fields!["engine_id"];
            string secondEngine = journal[3].Record.Fields!["engine_id"];
            Assert.That(secondEngine, Is.Not.EqualTo(firstEngine));
            Assert.That(journal[1].Record.Fields!["platform"], Is.EqualTo("ditto-one"));
            Assert.That(journal[4].Record.Fields!["platform"], Is.EqualTo("ditto-two"));
        }

        [Test]
        public void DittoScenarioCreationRejectsAnUnfinishedEngineWithoutDestroyingIt()
        {
            using var transport = new BattlementNativeTransport();
            DittoNativeEngineSession first = CreateSession(transport);

            DittoNativeEngineSession? duplicate = DittoNativeEngineSession.Create(
                transport,
                out BattlementTransportResult result
            );

            Assert.That(duplicate, Is.Null);
            Assert.That(result.Status, Is.EqualTo(BattlementTransportStatus.AbiError));
            Assert.That(result.Diagnostic, Does.Contain("already active"));
            Assert.That(transport.HasEngine, Is.True);
            Assert.That(first.Destroy().Status, Is.EqualTo(Success));
        }

        [Test]
        public void DittoScenarioCreationScopesItsSemanticFixtureToEngineConstruction()
        {
            const string original = "outer-fixture";
            string? previous = Environment.GetEnvironmentVariable(
                DittoNativeEngineSession.SemanticFixtureEnvironment
            );
            Environment.SetEnvironmentVariable(
                DittoNativeEngineSession.SemanticFixtureEnvironment,
                original
            );
            try
            {
                string? observed = DittoNativeEngineSession.WithSemanticFixture(
                    "castling",
                    () =>
                        Environment.GetEnvironmentVariable(
                            DittoNativeEngineSession.SemanticFixtureEnvironment
                        )
                );

                Assert.That(observed, Is.EqualTo("castling"));
                Assert.That(
                    Environment.GetEnvironmentVariable(
                        DittoNativeEngineSession.SemanticFixtureEnvironment
                    ),
                    Is.EqualTo(original)
                );
            }
            finally
            {
                Environment.SetEnvironmentVariable(
                    DittoNativeEngineSession.SemanticFixtureEnvironment,
                    previous
                );
            }
        }

        [Test]
        public void SubmitAndPollExposeOwnedResponsesAndNoMessageDistinctly()
        {
            using (BattlementNativeTransport transport = Transport("normal"))
            {
                Assert.That(transport.Connect(ConnectBytes("normal")).Status, Is.EqualTo(Success));

                BattlementTransportResult submit = transport.Submit(ClientMessageBytes());
                Assert.That(submit.Status, Is.EqualTo(Success));
                Assert.That(submit.Payload.IsEmpty, Is.False);
                Assert.That(transport.Poll().Status, Is.EqualTo(NoMessage));
                Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
            }

            using BattlementNativeTransport polling = Transport("poll-response");
            Assert.That(polling.Connect(ConnectBytes("poll-response")).Status, Is.EqualTo(Success));
            BattlementTransportResult poll = polling.Poll();
            Assert.That(poll.Status, Is.EqualTo(Success));
            Assert.That(poll.Payload.IsEmpty, Is.False);
            Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
        }

        [Test]
        public void FixedErrorStatusesPreserveDiagnosticsAndReleaseNativeBuffers()
        {
            using (BattlementNativeTransport malformed = new())
            {
                BattlementTransportResult result = malformed.Connect(new byte[] { 0xc1 });
                Assert.That(result.Status, Is.EqualTo(BattlementTransportStatus.InvalidArgument));
                Assert.That(result.Diagnostic, Does.Contain("invalid connect JSON"));
            }

            using (BattlementNativeTransport failed = Transport("engine-error"))
            {
                BattlementTransportResult result = failed.Connect(ConnectBytes("engine-error"));
                Assert.That(result.Status, Is.EqualTo(BattlementTransportStatus.EngineError));
                Assert.That(result.Diagnostic, Is.EqualTo("fixture engine error"));
            }

            Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
        }

        [Test]
        public void CaughtPanicPreservesLeadingRustTracingAndReleasesNativeBuffers()
        {
            BattlementNativeLogging.Drain();
            BattlementLogStore.Clear();
            LogAssert.Expect(
                LogType.Log,
                new System.Text.RegularExpressions.Regex(
                    @"^\[Battlement/Rust\]\[[^\]]+\] Preparing fixture connect panic"
                )
            );
            LogAssert.Expect(
                LogType.Log,
                new System.Text.RegularExpressions.Regex(
                    @"^\[Battlement/Rust\]\[[^\]]+\] Triggering fixture connect panic"
                )
            );
            using BattlementNativeTransport panicked = Transport("panic-connect");

            BattlementTransportResult result = panicked.Connect(ConnectBytes("panic-connect"));

            Assert.That(result.Status, Is.EqualTo(BattlementTransportStatus.Panic));
            Assert.That(result.Diagnostic, Does.Contain("fixture connect panic"));
            string[] leadingMessages = BattlementLogStore
                .Snapshot(out _)
                .Where(entry => entry.Source == "rust")
                .Select(entry => entry.Record.Message)
                .Where(message => message.Contains("fixture connect panic"))
                .ToArray();
            Assert.That(
                leadingMessages,
                Is.EqualTo(
                    new[] { "Preparing fixture connect panic", "Triggering fixture connect panic" }
                )
            );
            Assert.That(
                panicked.Connect(ConnectBytes("normal")).Status,
                Is.EqualTo(BattlementTransportStatus.Success),
                "A panic must destroy the poisoned engine before a new game starts."
            );
            Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
        }

        [Test]
        public void RustTracingIsForwardedToUnityAndTheLogViewerStore()
        {
            BattlementNativeLogging.Drain();
            BattlementLogStore.Clear();
            LogAssert.Expect(
                LogType.Log,
                new System.Text.RegularExpressions.Regex(
                    @"^\[Battlement/Rust\]\[fixture\.rust_event\] native trace"
                )
            );

            NativeFixture.fixture_trace();
            BattlementNativeLogging.Drain();

            BattlementLogEntry record = BattlementLogStore
                .Snapshot(out _)
                .Single(entry => entry.Record.EventName == "fixture.rust_event");
            Assert.That(record.Source, Is.EqualTo("rust"));
            Assert.That(record.Record.Message, Is.EqualTo("native trace"));
            Assert.That(record.Record.Fields!["mode"], Is.EqualTo("test"));
        }

        [Test]
        public void CaughtDestroyPanicIsForwardedToUnityAndTheLogViewerStore()
        {
            BattlementLogStore.Clear();
            using var transport = new BattlementNativeTransport();
            DittoNativeEngineSession session = CreateSession(transport);
            Assert.That(session.Connect(ConnectBytes("panic-destroy")).Status, Is.EqualTo(Success));
            LogAssert.Expect(
                LogType.Error,
                new System.Text.RegularExpressions.Regex(
                    @"^\[Battlement/Rust\]\[battlement\.rust\.destroy_panic\] "
                        + @"Rust engine panicked during destruction\."
                )
            );

            BattlementTransportResult result = session.Destroy();

            Assert.That(result.Status, Is.EqualTo(BattlementTransportStatus.Panic));
            Assert.That(result.Diagnostic, Does.Contain("fixture destroy panic"));
            Assert.That(transport.HasEngine, Is.False);
            BattlementLogEntry record = BattlementLogStore
                .Snapshot(out _)
                .Single(entry => entry.Record.EventName == "battlement.rust.destroy_panic");
            Assert.That(record.Record.StackTrace, Does.Contain("fixture destroy panic"));
        }

        [Test]
        public void DestroyStatusesRequireBoundedDiagnosticsAndPreserveClassification()
        {
            byte[] diagnostic = Encoding.UTF8.GetBytes("fixture destroy error");
            IntPtr data = Marshal.AllocHGlobal(diagnostic.Length);
            try
            {
                Marshal.Copy(diagnostic, 0, data, diagnostic.Length);
                BattlementTransportResult result = BattlementNativeTransport.TranslateDestroy(
                    3,
                    new BattlementNativeBuffer(data, (ulong)diagnostic.Length)
                );
                Assert.That(result.Status, Is.EqualTo(BattlementTransportStatus.EngineError));
                Assert.That(result.Diagnostic, Is.EqualTo("fixture destroy error"));

                result = BattlementNativeTransport.TranslateDestroy(3, default);
                Assert.That(result.Status, Is.EqualTo(BattlementTransportStatus.AbiError));
                Assert.That(result.Diagnostic, Does.Contain("without a diagnostic"));

                result = BattlementNativeTransport.TranslateDestroy(
                    3,
                    new BattlementNativeBuffer(
                        data,
                        BattlementNativeTransport.MaximumPayloadBytes + 1u
                    )
                );
                Assert.That(result.Status, Is.EqualTo(BattlementTransportStatus.AbiError));
                Assert.That(result.Diagnostic, Does.Contain("exceeded"));
            }
            finally
            {
                Marshal.FreeHGlobal(data);
            }
        }

        [Test]
        public void PayloadLimitAcceptsSixteenMiBAndFreesRejectedOutput()
        {
            using (BattlementNativeTransport boundary = Transport("maximum-response"))
            {
                BattlementTransportResult result = boundary.Connect(
                    ConnectBytes("maximum-response")
                );
                Assert.That(result.Status, Is.EqualTo(Success));
                Assert.That(result.Payload.Length, Is.EqualTo(16 * 1024 * 1024));
                Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
            }

            using (BattlementNativeTransport oversized = Transport("oversized-response"))
            {
                BattlementTransportResult result = oversized.Connect(
                    ConnectBytes("oversized-response")
                );
                Assert.That(result.Status, Is.EqualTo(BattlementTransportStatus.AbiError));
                Assert.That(result.Diagnostic, Does.Contain("exceeded"));
                Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
            }
        }

        [Test]
        public void WrongThreadCallsDoNotEnterTheNativeEngine()
        {
            using BattlementNativeTransport transport = Transport("normal");
            BattlementTransportResult? result = null;
            var thread = new System.Threading.Thread(() =>
                result = transport.Connect(ConnectBytes("normal"))
            );
            thread.Start();
            thread.Join();
            Assert.That(result!.Status, Is.EqualTo(BattlementTransportStatus.AbiError));
            Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
        }

        [Test]
        public void PlatformMappingNamesTheRequiredPluginArtifact()
        {
#if UNITY_EDITOR_OSX
            Assert.That(
                BattlementNativeTransport.RequiredPluginName,
                Is.EqualTo("libbattlement_rules.dylib")
            );
#elif UNITY_EDITOR_WIN
            Assert.That(
                BattlementNativeTransport.RequiredPluginName,
                Is.EqualTo("battlement_rules.dll")
            );
#endif
        }

        [Test]
        public void NativeBufferShapeValidationRejectsMalformedAndOversizedOutputs()
        {
            Assert.That(
                new BattlementNativeBuffer(IntPtr.Zero, 1).ValidateShape(16),
                Does.Contain("{NULL,0}")
            );
            Assert.That(
                new BattlementNativeBuffer(new IntPtr(1), 0).ValidateShape(16),
                Does.Contain("{NULL,0}")
            );
            Assert.That(
                new BattlementNativeBuffer(new IntPtr(1), 17).ValidateShape(16),
                Does.Contain("16-byte limit")
            );
            Assert.That(new BattlementNativeBuffer(new IntPtr(1), 16).ValidateShape(16), Is.Null);
        }

        private static BattlementNativeTransport Transport(string platform) => new();

        private static DittoNativeEngineSession CreateSession(BattlementNativeTransport transport)
        {
            DittoNativeEngineSession? session = DittoNativeEngineSession.Create(
                transport,
                out BattlementTransportResult result
            );
            Assert.That(result.Status, Is.EqualTo(Success));
            return session!;
        }

        private static BattlementLogEntry[] EngineJournal() =>
            BattlementLogStore
                .Snapshot(out _)
                .Where(entry => entry.Record.EventName.StartsWith("fixture.engine."))
                .ToArray();

        private static byte[] ConnectBytes(string platform) =>
            BattlementJson.SerializeConnect(
                new Connect(
                    platform,
                    Application.unityVersion,
                    new ScreenSize((uint)Screen.width, (uint)Screen.height)
                )
            );

        private static byte[] ClientMessageBytes() =>
            BattlementJson.SerializeClientMessage(
                new ClientMessage<CoreErrorCode, byte>.ActionMessage(
                    new Action(
                        new ActionId(new Guid("00000002-1234-5678-90ab-000000000002")),
                        new SessionId(new Guid("00112233-4455-6677-8899-aabbccddeeff")),
                        new ActionBody.KeyDown(PhysicalKey.KeyA)
                    )
                )
            );

        private static BattlementTransportStatus Success => BattlementTransportStatus.Success;

        private static BattlementTransportStatus NoMessage => BattlementTransportStatus.NoMessage;

        private static class NativeFixture
        {
            [DllImport("battlement_rules", CallingConvention = CallingConvention.Cdecl)]
            internal static extern UIntPtr fixture_outstanding_buffers();

            [DllImport("battlement_rules", CallingConvention = CallingConvention.Cdecl)]
            internal static extern UIntPtr fixture_connect_calls();

            [DllImport("battlement_rules", CallingConvention = CallingConvention.Cdecl)]
            internal static extern void fixture_trace();
        }
    }
}
