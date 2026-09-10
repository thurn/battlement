#nullable enable

using System;
using NUnit.Framework;
using UnityEngine;

namespace Battlement.Tests
{
    public sealed class DittoWebCaptureAdapterTests
    {
        [Test]
        public void ProbeAndCapturePreserveExactCanvasAndUploadFacts()
        {
            var owner = new GameObject("Ditto WebGL adapter test");
            var bridge = new FakeBridge();
            try
            {
                DittoWebCaptureAdapter adapter = DittoWebCaptureAdapter.Attach(
                    owner,
                    320,
                    180,
                    bridge
                );
                Assert.That(bridge.InstalledOwner, Is.EqualTo(owner.name));
                DittoWebProbeResult? probe = null;

                adapter.Probe(value => probe = value);
                Assert.That(bridge.ProbeCall, Is.EqualTo((owner.name, 320u, 180u)));
                adapter.CompleteWebProbe(
                    "{\"ok\":true,\"artifactId\":\"\",\"sha256\":\"\","
                        + "\"width\":320,\"height\":180,\"frame\":0,"
                        + "\"renderGeneration\":0,\"reason\":\"\"}"
                );

                Assert.That(
                    probe,
                    Is.EqualTo(new DittoWebProbeResult.Passed("webgl-canvas-png", 320, 180))
                );
                string artifactId = Guid.NewGuid().ToString("D");
                DittoWebCaptureResult? capture = null;
                DittoRenderCommit commit = adapter.CommitPresentedFrame(42);
                adapter.UploadCommittedFrame(
                    $"http://127.0.0.1:8123/ditto/route/jobs/job/artifacts/{artifactId}",
                    artifactId,
                    commit,
                    value => capture = value
                );
                Assert.That(bridge.CaptureCall!.Value.ArtifactId, Is.EqualTo(artifactId));
                Assert.That(bridge.CaptureCall.Value.Frame, Is.EqualTo(42));
                Assert.That(bridge.CaptureCall.Value.RenderGeneration, Is.EqualTo(1));
                adapter.CompleteWebCapture(
                    "{\"ok\":true,\"artifactId\":\""
                        + artifactId
                        + "\",\"sha256\":\""
                        + new string('a', 64)
                        + "\",\"width\":320,\"height\":180,\"frame\":42,"
                        + "\"renderGeneration\":1,\"reason\":\"\"}"
                );

                Assert.That(
                    capture,
                    Is.EqualTo(
                        new DittoWebCaptureResult.Uploaded(
                            artifactId,
                            new string('a', 64),
                            320,
                            180,
                            commit
                        )
                    )
                );
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(owner);
            }
        }

        [Test]
        public void InvalidProbeAndUploadRemainClosedFailures()
        {
            var owner = new GameObject("Ditto WebGL failure test");
            try
            {
                var bridge = new FakeBridge();
                DittoWebCaptureAdapter adapter = DittoWebCaptureAdapter.Attach(
                    owner,
                    100,
                    50,
                    bridge
                );
                DittoWebProbeResult? probe = null;
                adapter.Probe(value => probe = value);
                adapter.CompleteWebProbe(
                    "{\"ok\":false,\"artifactId\":\"\",\"sha256\":\"\","
                        + "\"width\":0,\"height\":0,\"frame\":0,"
                        + "\"renderGeneration\":0,\"reason\":\"tainted canvas\"}"
                );
                Assert.That(probe, Is.TypeOf<DittoWebProbeResult.Failed>());

                DittoWebCaptureResult? capture = null;
                adapter.UploadCommittedFrame(
                    "http://127.0.0.1:8123/unused",
                    Guid.NewGuid().ToString("D"),
                    new DittoRenderCommit(1, 1, 0),
                    value => capture = value
                );
                Assert.That(capture, Is.TypeOf<DittoWebCaptureResult.Unavailable>());
                Assert.That(bridge.CaptureCall, Is.Null);
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(owner);
            }
        }

        [Test]
        public void CaptureRequiresAnExactPresentationCommit()
        {
            var owner = new GameObject("Ditto WebGL exact commit test");
            try
            {
                var bridge = new FakeBridge { CommitSucceeds = false };
                DittoWebCaptureAdapter adapter = DittoWebCaptureAdapter.Attach(
                    owner,
                    100,
                    50,
                    bridge
                );
                adapter.Probe(_ => { });
                adapter.CompleteWebProbe(
                    "{\"ok\":true,\"artifactId\":\"\",\"sha256\":\"\","
                        + "\"width\":100,\"height\":50,\"frame\":0,"
                        + "\"renderGeneration\":0,\"reason\":\"\"}"
                );

                Assert.That(
                    () => adapter.CommitPresentedFrame(1),
                    Throws.InvalidOperationException.With.Message.Contains("freeze pixels")
                );

                bridge.CommitSucceeds = true;
                DittoRenderCommit commit = adapter.CommitPresentedFrame(2);
                DittoWebCaptureResult? result = null;
                string artifactId = Guid.NewGuid().ToString("D");
                adapter.UploadCommittedFrame(
                    "http://127.0.0.1:8123/unused",
                    artifactId,
                    commit,
                    value => result = value
                );
                adapter.CompleteWebCapture(
                    "{\"ok\":true,\"artifactId\":\""
                        + artifactId
                        + "\",\"sha256\":\""
                        + new string('a', 64)
                        + "\",\"width\":100,\"height\":50,\"frame\":2,"
                        + "\"renderGeneration\":999,\"reason\":\"\"}"
                );
                Assert.That(result, Is.TypeOf<DittoWebCaptureResult.Unavailable>());
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(owner);
            }
        }

        [TestCase("http://127.0.0.1:8192/ditto/abc/launcher", "http://127.0.0.1:8192/ditto/abc")]
        [TestCase("http://127.0.0.1:8192/launcher", "http://127.0.0.1:8192")]
        public void LauncherUrlResolvesItsSameOriginRoute(string launcher, string expected)
        {
            Assert.That(DittoWebSessionRoute.TryResolve(launcher, out string route), Is.True);
            Assert.That(route, Is.EqualTo(expected));
            Assert.That(
                DittoWebSessionRoute.TryResolve("https://127.0.0.1/ditto/x/launcher", out _),
                Is.False
            );
        }

        private sealed class FakeBridge : IDittoWebBrowserBridge
        {
            public bool CommitSucceeds { get; set; } = true;
            public string? InstalledOwner { get; private set; }

            public (string Owner, uint Width, uint Height)? ProbeCall { get; private set; }

            public (
                string Owner,
                string ArtifactId,
                ulong Frame,
                ulong RenderGeneration
            )? CaptureCall { get; private set; }

            public void Install(string owner) => InstalledOwner = owner;

            public void Probe(string owner, uint width, uint height) =>
                ProbeCall = (owner, width, height);

            public bool Commit(
                string owner,
                uint width,
                uint height,
                ulong frame,
                ulong renderGeneration
            ) => CommitSucceeds;

            public void Capture(
                string owner,
                string url,
                string artifactId,
                uint width,
                uint height,
                ulong frame,
                ulong renderGeneration
            ) => CaptureCall = (owner, artifactId, frame, renderGeneration);
        }
    }
}
