#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using NUnit.Framework;

namespace Battlement.Tests
{
    public sealed class DittoNativeVideoRecorderTests : UnityEngine.InputSystem.InputTestFixture
    {
        [Test]
        public void NormalizesTopDownRgbaAndAutomaticallyTruncatesAtThirtyFramesPerSecond()
        {
            string directory = TemporaryDirectory();
            try
            {
                using var recorder = new DittoNativeVideoRecorder(directory, 1, 2);
                string inputId = recorder.Begin(4, 34, TimeSpan.Zero);
                var layout = new DittoCapturePixelLayout(
                    DittoCaptureRowOrder.BottomUp,
                    DittoCaptureChannelOrder.Bgra
                );
                byte[] bottomUpBgra = { 30, 20, 10, 40, 70, 60, 50, 80 };

                Assert.That(
                    recorder.AppendFrame(
                        bottomUpBgra,
                        layout,
                        TimeSpan.FromTicks(TimeSpan.TicksPerSecond / 30)
                    ),
                    Is.False
                );
                Assert.That(
                    recorder.AppendFrame(
                        bottomUpBgra,
                        layout,
                        TimeSpan.FromTicks(2 * TimeSpan.TicksPerSecond / 30)
                    ),
                    Is.True
                );
                recorder.Stop();

                DittoNativeVideoInput input = recorder.Inputs.Single();
                Assert.That(input.InputId, Is.EqualTo(inputId));
                Assert.That(input.StartStepIndex, Is.EqualTo(4));
                Assert.That(input.FrameCount, Is.EqualTo(2));
                Assert.That(input.Truncated, Is.True);
                Assert.That(
                    File.ReadAllBytes(input.Path),
                    Is.EqualTo(
                        new byte[]
                        {
                            50,
                            60,
                            70,
                            80,
                            10,
                            20,
                            30,
                            40,
                            50,
                            60,
                            70,
                            80,
                            10,
                            20,
                            30,
                            40,
                        }
                    )
                );
                Assert.That(input.Sha256, Has.Length.EqualTo(64));
            }
            finally
            {
                Directory.Delete(directory, true);
            }
        }

        [Test]
        public void PairedExecutionCapturesInterveningActionsAndScreenshotThenRestoresMotion()
        {
            string directory = TemporaryDirectory();
            try
            {
                using BattlementTestHarness harness = BattlementTestHarness.Create();
                using var recorder = new DittoNativeVideoRecorder(directory, 2, 2);
                string screenshotId = Guid.NewGuid().ToString("D");
                var scenario = new DittoResolvedScenario(
                    Guid.NewGuid().ToString("D"),
                    0,
                    "video",
                    DittoMotion.Instant,
                    5_000,
                    new[]
                    {
                        Step(
                            0,
                            new DittoStepAction.Video(
                                new DittoVideo.Start("clip", DittoMotion.Controlled, 10_000)
                            )
                        ),
                        Step(
                            1,
                            new DittoStepAction.Click(new DittoInputTarget.Coordinates(0.5, 0.5))
                        ),
                        Step(
                            2,
                            new DittoStepAction.Screenshot(
                                new DittoScreenshot("inside", new DittoComparison("0", false, "0"))
                            )
                        ),
                        Step(3, new DittoStepAction.Video(new DittoVideo.Stop())),
                        Step(
                            4,
                            new DittoStepAction.Assert(
                                new DittoObjectCondition(
                                    Guid.NewGuid().ToString("D"),
                                    DittoObjectState.Absent
                                )
                            )
                        ),
                    }
                );
                using var executor = new DittoScenarioExecutor(
                    harness.Runner,
                    scenario,
                    DittoPlatform.Macos,
                    2,
                    2,
                    new Dictionary<string, ObjectId>(),
                    5_000,
                    () => TimeSpan.Zero,
                    _ => new DittoScreenshotStepOutcome(screenshotId, null, false),
                    (_, _) => "P0001",
                    video: recorder,
                    videoFrame: _ => Enumerable.Repeat((byte)127, 16).ToArray(),
                    nativeVideoLayout: new DittoCapturePixelLayout(
                        DittoCaptureRowOrder.TopDown,
                        DittoCaptureChannelOrder.Rgba
                    )
                );

                Drain(executor);

                Assert.That(executor.Result!.Status, Is.EqualTo(DittoExecutionStatus.Passed));
                Assert.That(executor.Result.Steps[0].VideoInputId, Is.Not.Null);
                Assert.That(
                    executor.Result.Steps[2].ScreenshotArtifactId,
                    Is.EqualTo(screenshotId)
                );
                Assert.That(executor.Result.Steps[4].Status, Is.EqualTo(DittoStepStatus.Passed));
                Assert.That(recorder.Inputs.Single().FrameCount, Is.GreaterThan(0));
                Assert.That(recorder.Inputs.Single().Truncated, Is.False);
                Assert.That(
                    harness.Runner.DittoElapsed,
                    Is.EqualTo(
                        TimeSpan.FromTicks(
                            checked((long)recorder.Inputs.Single().FrameCount)
                                * TimeSpan.TicksPerSecond
                                / 30
                        )
                    )
                );
            }
            finally
            {
                Directory.Delete(directory, true);
            }
        }

        [Test]
        public void RuntimeFailureTruncatesAndAllowsTheLaterStopToConsumeItsPair()
        {
            string directory = TemporaryDirectory();
            try
            {
                using var recorder = new DittoNativeVideoRecorder(directory, 1, 1);
                recorder.Begin(0, 1_000, TimeSpan.Zero);
                recorder.AppendFrame(
                    new byte[] { 1, 2, 3, 4 },
                    new DittoCapturePixelLayout(
                        DittoCaptureRowOrder.TopDown,
                        DittoCaptureChannelOrder.Rgba
                    ),
                    TimeSpan.FromTicks(TimeSpan.TicksPerSecond / 30)
                );

                recorder.TruncateForRuntimeFailure();
                recorder.Stop();

                Assert.That(recorder.Inputs.Single().Truncated, Is.True);
                Assert.That(recorder.IsActive, Is.False);
            }
            finally
            {
                Directory.Delete(directory, true);
            }
        }

        [Test]
        public void RuntimeFailureBeforeTheFirstFrameDiscardsTheEmptyInput()
        {
            string directory = TemporaryDirectory();
            try
            {
                using var recorder = new DittoNativeVideoRecorder(directory, 1, 1);
                recorder.Begin(0, 1_000, TimeSpan.Zero);

                Assert.That(recorder.TruncateForRuntimeFailure(), Is.False);
                recorder.Stop();

                Assert.That(recorder.Inputs, Is.Empty);
                Assert.That(recorder.IsActive, Is.False);
                Assert.That(Directory.EnumerateFiles(directory), Is.Empty);
            }
            finally
            {
                Directory.Delete(directory, true);
            }
        }

        [Test]
        public void ImmediateStepFailureRemovesTheDiscardedVideoReference()
        {
            string directory = TemporaryDirectory();
            try
            {
                using BattlementTestHarness harness = BattlementTestHarness.Create();
                using var recorder = new DittoNativeVideoRecorder(directory, 1, 1);
                var scenario = new DittoResolvedScenario(
                    Guid.NewGuid().ToString("D"),
                    0,
                    "video failure",
                    DittoMotion.Instant,
                    5_000,
                    new[]
                    {
                        Step(
                            0,
                            new DittoStepAction.Video(
                                new DittoVideo.Start("clip", DittoMotion.Controlled, 1_000)
                            )
                        ),
                        Step(
                            1,
                            new DittoStepAction.Assert(
                                new DittoObjectCondition(
                                    Guid.NewGuid().ToString("D"),
                                    DittoObjectState.Exists
                                )
                            )
                        ),
                    }
                );
                using var executor = new DittoScenarioExecutor(
                    harness.Runner,
                    scenario,
                    DittoPlatform.Macos,
                    1,
                    1,
                    new Dictionary<string, ObjectId>(),
                    5_000,
                    () => TimeSpan.Zero,
                    _ => new DittoScreenshotStepOutcome(null, null, false),
                    (_, _) => "P0001",
                    video: recorder,
                    videoFrame: _ => new byte[] { 1, 2, 3, 4 },
                    nativeVideoLayout: new DittoCapturePixelLayout(
                        DittoCaptureRowOrder.TopDown,
                        DittoCaptureChannelOrder.Rgba
                    )
                );

                Drain(executor);

                Assert.That(executor.Result!.Status, Is.EqualTo(DittoExecutionStatus.Failed));
                Assert.That(executor.Result.Steps[0].VideoInputId, Is.Null);
                Assert.That(recorder.Inputs, Is.Empty);
            }
            finally
            {
                Directory.Delete(directory, true);
            }
        }

        private static DittoResolvedStep Step(uint index, DittoStepAction action) =>
            new(index, null, 1_000, action);

        private static void Drain(DittoScenarioExecutor executor)
        {
            for (var frame = 0; frame < 256; frame++)
            {
                if (executor.Advance())
                {
                    return;
                }
            }
            Assert.Fail("Video scenario did not complete.");
        }

        private static string TemporaryDirectory()
        {
            string path = Path.Combine(Path.GetTempPath(), "ditto-video-" + Guid.NewGuid());
            Directory.CreateDirectory(path);
            return path;
        }
    }
}
