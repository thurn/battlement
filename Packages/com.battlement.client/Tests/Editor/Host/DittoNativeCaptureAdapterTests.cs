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

                adapter.CaptureCommittedFrame(1, value => result = value);

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
            ulong capturedFrame = 0;
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
                DittoMotion.Controlled,
                1_000,
                new[] { screenshot }
            );
            DittoScreenshotCapture capture = (_, frame, completion) =>
            {
                capturedFrame = frame;
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
            Assert.That(executor.Advance(), Is.False);
            Assert.That(finishCapture, Is.Not.Null);
            Assert.That(capturedFrame, Is.EqualTo(2));
            Assert.That(executor.Advance(), Is.False);
            finishCapture!(
                new DittoScreenshotStepOutcome(Guid.NewGuid().ToString("D"), null, false)
            );

            Assert.That(executor.Advance(), Is.True);
            Assert.That(executor.Result!.Status, Is.EqualTo(DittoExecutionStatus.Passed));
            Assert.That(executor.Result.Steps[0].DurationMs, Is.Zero);
        }
    }
}
