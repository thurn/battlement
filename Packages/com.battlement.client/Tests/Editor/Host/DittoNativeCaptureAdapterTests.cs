#nullable enable

using System;
using System.Collections.Generic;
using NUnit.Framework;
using UnityEngine;

namespace Battlement.Tests
{
    public sealed class DittoNativeCaptureAdapterTests
    {
        [TestCase((int)DittoCaptureRowOrder.BottomUp, (int)DittoCaptureChannelOrder.Rgba)]
        [TestCase((int)DittoCaptureRowOrder.BottomUp, (int)DittoCaptureChannelOrder.Bgra)]
        [TestCase((int)DittoCaptureRowOrder.TopDown, (int)DittoCaptureChannelOrder.Rgba)]
        [TestCase((int)DittoCaptureRowOrder.TopDown, (int)DittoCaptureChannelOrder.Bgra)]
        public void ProbeAndPngRoundTripEveryNativePixelLayout(int rowsValue, int channelsValue)
        {
            var rows = (DittoCaptureRowOrder)rowsValue;
            var channels = (DittoCaptureChannelOrder)channelsValue;
            var layout = new DittoCapturePixelLayout(rows, channels);
            byte[] native = DittoCapturePixels.Bytes(2, 2, DittoCapturePixels.ProbeColors, layout);

            Assert.That(
                DittoCapturePixels.TryProbe(native, out DittoCapturePixelLayout? detected),
                Is.True
            );
            Assert.That(detected, Is.EqualTo(layout));
            Assert.That(
                DittoCapturePixels.TryEncode(native, 2, 2, detected!, out byte[] png),
                Is.True
            );

            var decoded = new Texture2D(1, 1, TextureFormat.RGBA32, false, true);
            try
            {
                Assert.That(ImageConversion.LoadImage(decoded, png, false), Is.True);
                Assert.That(decoded.width, Is.EqualTo(2));
                Assert.That(decoded.height, Is.EqualTo(2));
                Assert.That(decoded.GetPixels32(), Is.EqualTo(DittoCapturePixels.ProbeColors));
                TestContext.Progress.WriteLine(Convert.ToBase64String(png));
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(decoded);
            }
        }

        [Test]
        public void MalformedReadbackAndProcessLossHaveClosedFailureResults()
        {
            Assert.That(
                DittoCapturePixels.TryProbe(new byte[15], out DittoCapturePixelLayout? layout),
                Is.False
            );
            Assert.That(layout, Is.Null);
            Assert.That(
                DittoCapturePixels.TryEncode(
                    new byte[15],
                    2,
                    2,
                    new DittoCapturePixelLayout(
                        DittoCaptureRowOrder.BottomUp,
                        DittoCaptureChannelOrder.Rgba
                    ),
                    out byte[] png
                ),
                Is.False
            );
            Assert.That(png, Is.Empty);

            var lost = (DittoNativeCaptureResult.Unavailable)
                DittoNativeCaptureAdapter.ProcessLost();
            Assert.That(lost.Failure.Code, Is.EqualTo(DittoErrorCode.ImageCaptureFailed));
            Assert.That(lost.Failure.Reason, Does.Contain("exited"));
        }

        [Test]
        public void FlipsFramebufferRowsWithoutMirroringColumns()
        {
            byte[] pixels = { 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16 };

            Assert.That(
                DittoCapturePixels.FlipRows(pixels, 2, 2),
                Is.EqualTo(new byte[] { 9, 10, 11, 12, 13, 14, 15, 16, 1, 2, 3, 4, 5, 6, 7, 8 })
            );
        }

        [Test]
        public void DelayedReadbackBindsPixelsFromTheCommittedRenderGeneration()
        {
            var layout = new DittoCapturePixelLayout(
                DittoCaptureRowOrder.TopDown,
                DittoCaptureChannelOrder.Rgba
            );
            Color32[] committed =
            {
                new(255, 0, 0, 255),
                new(0, 255, 0, 255),
                new(0, 0, 255, 255),
                new(255, 255, 255, 255),
            };
            var source = new DelayedFrameSource(
                DittoCapturePixels.FlipRows(DittoCapturePixels.Bytes(2, 2, committed, layout), 2, 2)
            );
            var owner = new GameObject("Ditto delayed capture test");
            DittoNativeCaptureResult? result = null;
            DittoNativeCaptureAdapter adapter = DittoNativeCaptureAdapter.AttachForTesting(
                owner,
                2,
                2,
                layout,
                source
            );
            DittoRenderCommit commit = adapter.CommitPresentedFrame(7);
            adapter.CaptureCommittedFrame(commit, value => result = value);
            source.ReplaceLive(new byte[16]);
            source.Complete(0);
            adapter.CompletePendingCallbacksForTesting();
            var captured = (DittoNativeCaptureResult.Captured)result!;

            var decoded = new Texture2D(1, 1, TextureFormat.RGBA32, false, true);
            try
            {
                Assert.That(ImageConversion.LoadImage(decoded, captured.Png, false), Is.True);
                Assert.That(decoded.GetPixels32(), Is.EqualTo(committed));
                Assert.That(captured.Commit, Is.EqualTo(commit));
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(decoded);
                UnityEngine.Object.DestroyImmediate(owner);
            }
        }

        [Test]
        public void TimedOutReadbackCannotFailANewerCapture()
        {
            var owner = new GameObject("Ditto stale readback test");
            var source = new DelayedFrameSource(new byte[] { 1, 2, 3, 4 });
            try
            {
                DittoNativeCaptureAdapter adapter = DittoNativeCaptureAdapter.AttachForTesting(
                    owner,
                    1,
                    1,
                    new DittoCapturePixelLayout(
                        DittoCaptureRowOrder.TopDown,
                        DittoCaptureChannelOrder.Rgba
                    ),
                    source
                );
                DittoRenderCommit commit = adapter.CommitPresentedFrame(1);
                DittoNativeCaptureResult? first = null;
                DittoNativeCaptureResult? second = null;
                adapter.CaptureCommittedFrame(commit, value => first = value);
                adapter.ExpirePendingCaptureForTesting();
                adapter.CaptureCommittedFrame(commit, value => second = value);

                source.Complete(0);
                adapter.CompletePendingCallbacksForTesting();
                Assert.That(first, Is.TypeOf<DittoNativeCaptureResult.Unavailable>());
                Assert.That(second, Is.Null);

                source.Complete(1);
                adapter.CompletePendingCallbacksForTesting();
                Assert.That(second, Is.TypeOf<DittoNativeCaptureResult.Captured>());
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(owner);
            }
        }

        [Test]
        public void MismatchedAndStaleRenderGenerationsAreRejected()
        {
            var layout = new DittoCapturePixelLayout(
                DittoCaptureRowOrder.TopDown,
                DittoCaptureChannelOrder.Rgba
            );
            byte[] pixels = DittoCapturePixels.Bytes(2, 2, DittoCapturePixels.ProbeColors, layout);
            var retained = new DittoRenderCommit(8, 12, 1234);

            Assert.That(
                DittoNativeCaptureAdapter.BindCapturedPixels(
                    retained,
                    new DittoRenderCommit(8, 11, 1234),
                    pixels,
                    2,
                    2,
                    layout
                ),
                Is.TypeOf<DittoNativeCaptureResult.Unavailable>()
            );
            Assert.That(
                DittoNativeCaptureAdapter.BindCapturedPixels(
                    retained,
                    new DittoRenderCommit(9, 12, 1234),
                    pixels,
                    2,
                    2,
                    layout
                ),
                Is.TypeOf<DittoNativeCaptureResult.Unavailable>()
            );
        }

        [Test]
        public void AdapterRejectsUnsupportedTargetsAndCaptureBeforeProbe()
        {
            var owner = new GameObject("Ditto native capture test");
            try
            {
                Assert.That(
                    () => DittoNativeCaptureAdapter.Attach(owner, DittoPlatform.Webgl, 1, 1, null),
                    Throws.TypeOf<ArgumentOutOfRangeException>()
                );
                DittoNativeCaptureAdapter adapter = DittoNativeCaptureAdapter.Attach(
                    owner,
                    DittoPlatform.Macos,
                    1,
                    1,
                    null
                );
                DittoNativeCaptureResult? result = null;

                adapter.CaptureCommittedFrame(
                    new DittoRenderCommit(1, 1, 0),
                    value => result = value
                );

                Assert.That(result, Is.TypeOf<DittoNativeCaptureResult.Unavailable>());
                var unavailable = (DittoNativeCaptureResult.Unavailable)result!;
                Assert.That(
                    unavailable.Failure.Code,
                    Is.EqualTo(DittoErrorCode.ImageCaptureFailed)
                );
                Assert.That(unavailable.Failure.Reason, Does.Contain("startup probe"));
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(owner);
            }
        }

        [Test]
        public void ScenarioWaitsForAsynchronousCaptureWithoutAdvancingFrames()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            TimeSpan current = TimeSpan.Zero;
            System.Action<DittoScreenshotStepOutcome>? finishCapture = null;
            DittoRenderCommit? capturedCommit = null;
            var screenshot = new DittoResolvedStep(
                0,
                "async",
                100,
                new DittoStepAction.Screenshot(
                    new DittoScreenshot("async", new DittoComparison("0", false, "0"))
                )
            );
            var scenario = new DittoResolvedScenario(
                Guid.NewGuid().ToString("D"),
                0,
                "async capture",
                null,
                DittoMotion.Controlled,
                1_000,
                new[] { screenshot }
            );
            DittoScreenshotCapture capture = (_, frame, completion) =>
            {
                capturedCommit = frame;
                finishCapture = completion;
            };
            using var executor = new DittoScenarioExecutor(
                harness.Runner,
                scenario,
                DittoPlatform.Macos,
                100,
                100,
                new Dictionary<string, ObjectId>(),
                1_000,
                () => current,
                capture,
                (_, _) => "P0001"
            );

            Assert.That(executor.Advance(), Is.False);
            executor.CompletePresentedFrame();
            Assert.That(executor.Advance(), Is.False);
            executor.CompletePresentedFrame();
            Assert.That(executor.Advance(), Is.False);
            executor.CompletePresentedFrame();
            Assert.That(executor.Advance(), Is.False);
            Assert.That(finishCapture, Is.Not.Null);
            Assert.That(capturedCommit!.Frame, Is.GreaterThanOrEqualTo(2));
            Assert.That(executor.Advance(), Is.False);
            finishCapture!(
                new DittoScreenshotStepOutcome(Guid.NewGuid().ToString("D"), null, false)
            );

            Assert.That(executor.Advance(), Is.True);
            Assert.That(executor.Result!.Status, Is.EqualTo(DittoExecutionStatus.Passed));
            Assert.That(executor.Result.Steps[0].DurationMs, Is.Zero);
        }

        private sealed class DelayedFrameSource : IDittoNativeCommittedFrameSource
        {
            private readonly List<Action<byte[], bool>> completions = new();
            private byte[] live;
            private byte[] committed = Array.Empty<byte>();

            public DelayedFrameSource(byte[] live) => this.live = live;

            public ulong CommitPresentedFrame(uint width, uint height)
            {
                committed = (byte[])live.Clone();
                return 1234;
            }

            public void ReadCommittedFrame(Action<byte[], bool> completion) =>
                completions.Add(completion);

            public byte[] ReadCommittedFrame() => (byte[])committed.Clone();

            public void ReplaceLive(byte[] pixels) => live = pixels;

            public void Complete(int index) => completions[index]((byte[])committed.Clone(), false);

            public void Dispose() { }
        }
    }
}
