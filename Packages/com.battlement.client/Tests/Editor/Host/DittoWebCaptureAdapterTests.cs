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
                        + "\"width\":320,\"height\":180,\"frame\":0,\"reason\":\"\"}"
                );

                Assert.That(
                    probe,
                    Is.EqualTo(new DittoWebProbeResult.Passed("webgl-canvas-png", 320, 180))
                );
                string artifactId = Guid.NewGuid().ToString("D");
                DittoWebCaptureResult? capture = null;
                adapter.UploadCommittedFrame(
                    $"http://127.0.0.1:8123/ditto/route/jobs/job/artifacts/{artifactId}",
                    artifactId,
                    42,
                    value => capture = value
                );
                Assert.That(bridge.CaptureCall!.Value.ArtifactId, Is.EqualTo(artifactId));
                Assert.That(bridge.CaptureCall.Value.Frame, Is.EqualTo(42));
                adapter.CompleteWebCapture(
                    "{\"ok\":true,\"artifactId\":\""
                        + artifactId
                        + "\",\"sha256\":\""
                        + new string('a', 64)
                        + "\",\"width\":320,\"height\":180,\"frame\":42,\"reason\":\"\"}"
                );

                Assert.That(
                    capture,
                    Is.EqualTo(
                        new DittoWebCaptureResult.Uploaded(
                            artifactId,
                            new string('a', 64),
                            320,
                            180,
                            42
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
                        + "\"width\":0,\"height\":0,\"frame\":0,\"reason\":\"tainted canvas\"}"
                );
                Assert.That(probe, Is.TypeOf<DittoWebProbeResult.Failed>());

                DittoWebCaptureResult? capture = null;
                adapter.UploadCommittedFrame(
                    "http://127.0.0.1:8123/unused",
                    Guid.NewGuid().ToString("D"),
                    1,
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
            public string? InstalledOwner { get; private set; }

            public (string Owner, uint Width, uint Height)? ProbeCall { get; private set; }

            public (string Owner, string ArtifactId, ulong Frame)? CaptureCall { get; private set; }

            public void Install(string owner) => InstalledOwner = owner;

            public void Probe(string owner, uint width, uint height) =>
                ProbeCall = (owner, width, height);

            public void Capture(
                string owner,
                string url,
                string artifactId,
                uint width,
                uint height,
                ulong frame
            ) => CaptureCall = (owner, artifactId, frame);
        }
    }
}
