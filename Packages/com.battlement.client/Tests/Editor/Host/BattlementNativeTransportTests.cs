#nullable enable

using System;
using System.IO;
using System.Runtime.InteropServices;
using NUnit.Framework;
using UnityEngine;

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
                Assert.That(result.Diagnostic, Does.Contain("invalid connect MessagePack"));
            }

            using (BattlementNativeTransport failed = Transport("engine-error"))
            {
                BattlementTransportResult result = failed.Connect(ConnectBytes("engine-error"));
                Assert.That(result.Status, Is.EqualTo(BattlementTransportStatus.EngineError));
                Assert.That(result.Diagnostic, Is.EqualTo("fixture engine error"));
            }

            using (BattlementNativeTransport panicked = Transport("panic-connect"))
            {
                BattlementTransportResult result = panicked.Connect(ConnectBytes("panic-connect"));
                Assert.That(result.Status, Is.EqualTo(BattlementTransportStatus.Panic));
                Assert.That(result.Diagnostic, Is.EqualTo("Rust panic in battlement_connect"));
            }

            Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
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

        private static BattlementNativeTransport Transport(string platform) => new();

        private static byte[] ConnectBytes(string platform) =>
            BattlementMessagePack.SerializeConnect(
                new Connect(
                    platform,
                    Application.unityVersion,
                    new ScreenSize((uint)Screen.width, (uint)Screen.height)
                )
            );

        private static byte[] ClientMessageBytes()
        {
            string projectRoot = Path.GetFullPath(Path.Combine(Application.dataPath, ".."));
            return File.ReadAllBytes(
                Path.Combine(
                    projectRoot,
                    "Packages/com.battlement.client/Tests/Fixtures/",
                    "csharp-client-pointer-enter.msgpack"
                )
            );
        }

        private static BattlementTransportStatus Success => BattlementTransportStatus.Success;

        private static BattlementTransportStatus NoMessage => BattlementTransportStatus.NoMessage;

        private static class NativeFixture
        {
            [DllImport("battlement_rules", CallingConvention = CallingConvention.Cdecl)]
            internal static extern UIntPtr fixture_outstanding_buffers();

            [DllImport("battlement_rules", CallingConvention = CallingConvention.Cdecl)]
            internal static extern UIntPtr fixture_connect_calls();
        }
    }
}
