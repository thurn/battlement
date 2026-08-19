#nullable enable

using System;
using System.IO;
using System.Runtime.InteropServices;
using NUnit.Framework;
using UnityEngine;
using Object = UnityEngine.Object;

namespace Masonry.Tests
{
    public sealed class MasonryNativeTransportTests
    {
        [Test]
        public void RunnerReusesOneEngineAcrossConnectsAndDestroysItAtShutdown()
        {
            ulong callsBefore = NativeFixture.fixture_connect_calls().ToUInt64();
            var transport = Transport("normal");
            var assetStorage = new FakeMasonryAssetStorage();
            var host = new GameObject("Native transport host");
            MasonryRunner runner = host.AddComponent<MasonryRunner>();
            runner.Configure(new MasonryRunnerOptions(transport, assetStorage));

            try
            {
                runner.Connect();
                Assert.That(transport.LastConnectResult!.Status, Is.EqualTo(Success));
                runner.Reconnect();
                Assert.That(transport.LastConnectResult!.Status, Is.EqualTo(Success));
                Assert.That(
                    NativeFixture.fixture_connect_calls().ToUInt64(),
                    Is.EqualTo(callsBefore + 2)
                );
                Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
            }
            finally
            {
                runner.Dispose();
                Object.DestroyImmediate(host);
            }

            Assert.That(assetStorage.IsDisposed, Is.True);
            Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
        }

        [Test]
        public void SubmitAndPollExposeOwnedResponsesAndNoMessageDistinctly()
        {
            using (MasonryNativeTransport transport = Transport("normal"))
            {
                Assert.That(transport.Connect().Status, Is.EqualTo(Success));

                MasonryTransportResult submit = transport.Submit(ClientMessageBytes());
                Assert.That(submit.Status, Is.EqualTo(Success));
                Assert.That(submit.Payload.IsEmpty, Is.False);
                Assert.That(transport.Poll().Status, Is.EqualTo(NoMessage));
                Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
            }

            using MasonryNativeTransport polling = Transport("poll-response");
            Assert.That(polling.Connect().Status, Is.EqualTo(Success));
            MasonryTransportResult poll = polling.Poll();
            Assert.That(poll.Status, Is.EqualTo(Success));
            Assert.That(poll.Payload.IsEmpty, Is.False);
            Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
        }

        [Test]
        public void FixedErrorStatusesPreserveDiagnosticsAndReleaseNativeBuffers()
        {
            using (MasonryNativeTransport malformed = new(new byte[] { 0xc1 }))
            {
                MasonryTransportResult result = malformed.Connect();
                Assert.That(result.Status, Is.EqualTo(MasonryTransportStatus.InvalidArgument));
                Assert.That(result.Diagnostic, Does.Contain("invalid connect MessagePack"));
            }

            using (MasonryNativeTransport failed = Transport("engine-error"))
            {
                MasonryTransportResult result = failed.Connect();
                Assert.That(result.Status, Is.EqualTo(MasonryTransportStatus.EngineError));
                Assert.That(result.Diagnostic, Is.EqualTo("fixture engine error"));
            }

            using (MasonryNativeTransport panicked = Transport("panic-connect"))
            {
                MasonryTransportResult result = panicked.Connect();
                Assert.That(result.Status, Is.EqualTo(MasonryTransportStatus.Panic));
                Assert.That(result.Diagnostic, Is.EqualTo("Rust panic in masonry_connect"));
            }

            Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
        }

        [Test]
        public void PayloadLimitAcceptsSixteenMiBAndFreesRejectedOutput()
        {
            using (MasonryNativeTransport boundary = Transport("maximum-response"))
            {
                MasonryTransportResult result = boundary.Connect();
                Assert.That(result.Status, Is.EqualTo(Success));
                Assert.That(result.Payload.Length, Is.EqualTo(16 * 1024 * 1024));
                Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
            }

            using (MasonryNativeTransport oversized = Transport("oversized-response"))
            {
                MasonryTransportResult result = oversized.Connect();
                Assert.That(result.Status, Is.EqualTo(MasonryTransportStatus.AbiError));
                Assert.That(result.Diagnostic, Does.Contain("exceeded"));
                Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
            }
        }

        [Test]
        public void WrongThreadCallsDoNotEnterTheNativeEngine()
        {
            using MasonryNativeTransport transport = Transport("normal");
            MasonryTransportResult? result = null;
            var thread = new System.Threading.Thread(() => result = transport.Connect());
            thread.Start();
            thread.Join();
            Assert.That(result!.Status, Is.EqualTo(MasonryTransportStatus.AbiError));
            Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
        }

        [Test]
        public void PlatformMappingNamesTheRequiredPluginArtifact()
        {
#if UNITY_EDITOR_OSX
            Assert.That(
                MasonryNativeTransport.RequiredPluginName,
                Is.EqualTo("libmasonry_rules.dylib")
            );
#elif UNITY_EDITOR_WIN
            Assert.That(MasonryNativeTransport.RequiredPluginName, Is.EqualTo("masonry_rules.dll"));
#endif
        }

        private static MasonryNativeTransport Transport(string platform) =>
            new(
                MasonryMessagePack.SerializeConnect(
                    new Connect(
                        platform,
                        Application.unityVersion,
                        new ScreenSize((uint)Screen.width, (uint)Screen.height)
                    )
                )
            );

        private static byte[] ClientMessageBytes()
        {
            string projectRoot = Path.GetFullPath(Path.Combine(Application.dataPath, ".."));
            return File.ReadAllBytes(
                Path.Combine(
                    projectRoot,
                    "Packages/com.masonry.client/Tests/Fixtures/",
                    "csharp-client-pointer-enter.msgpack"
                )
            );
        }

        private static MasonryTransportStatus Success => MasonryTransportStatus.Success;

        private static MasonryTransportStatus NoMessage => MasonryTransportStatus.NoMessage;

        private static class NativeFixture
        {
            [DllImport("masonry_rules", CallingConvention = CallingConvention.Cdecl)]
            internal static extern UIntPtr fixture_outstanding_buffers();

            [DllImport("masonry_rules", CallingConvention = CallingConvention.Cdecl)]
            internal static extern UIntPtr fixture_connect_calls();
        }
    }
}
