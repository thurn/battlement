#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using NUnit.Framework;
using UnityEngine;

namespace Battlement.Tests
{
    public sealed class DittoScenarioExecutorTests
    {
        [Test]
        public void ControlledAdvanceConsumesPresentedFrames()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            DittoResolvedScenario scenario = Scenario(
                10_000,
                Step(0, new DittoStepAction.Advance(200))
            );
            using DittoScenarioExecutor executor = Executor(harness, scenario, () => TimeSpan.Zero);

            Drain(executor);
            Assert.That(executor.LastCommittedFrame, Is.GreaterThanOrEqualTo(202));
            Assert.That(executor.Result!.Status, Is.EqualTo(DittoExecutionStatus.Passed));
        }

        [Test]
        public void CoordinatePointerActionFailsAtTheClickStep()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            DittoResolvedScenario scenario = Scenario(
                10_000,
                Step(0, new DittoStepAction.Click(Coordinates(0.1, 0.2))),
                Step(1, new DittoStepAction.Advance(1))
            );
            using DittoScenarioExecutor executor = Executor(harness, scenario, () => TimeSpan.Zero);

            Drain(executor);

            Assert.That(
                executor.Result!.Steps.Select(step => step.Status),
                Is.EqualTo(new[] { DittoStepStatus.Failed, DittoStepStatus.NotRun })
            );
        }

        [Test]
        public void SemanticClickWithoutDeliveryReceiptFailsAtThatStep()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            ObjectId camera = new(Guid.NewGuid());
            ObjectId target = new(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    session,
                    objects: new[] { CameraObject(camera), Cube(target) },
                    inputCameraId: camera
                )
            );
            harness.Runner.Connect();
            harness.Transport.DefaultSubmitResult = FakeBattlementTransport.ResponseResult(
                new Response(session, Array.Empty<ResponseMessage<Command>>())
            );
            Physics.SyncTransforms();
            string? diagnostic = null;
            DittoResolvedScenario scenario = Scenario(
                10_000,
                Step(0, new DittoStepAction.Click(new DittoInputTarget.Object("target"))),
                Step(1, new DittoStepAction.Advance(1))
            );
            using DittoScenarioExecutor executor = Executor(
                harness,
                scenario,
                () => TimeSpan.Zero,
                aliases: new Dictionary<string, ObjectId> { ["target"] = target },
                error: value => diagnostic = value
            );

            Drain(executor);

            Assert.That(
                executor.Result!.Steps.Select(step => step.Status),
                Is.EqualTo(new[] { DittoStepStatus.InfrastructureError, DittoStepStatus.NotRun })
            );
            Assert.That(diagnostic, Does.Contain("no semantic delivery receipt"));
        }

        [Test]
        public void FramebufferFailureIsInfrastructureAtTheScreenshotStep()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            DittoResolvedScenario scenario = Scenario(
                1_000,
                Step(
                    0,
                    new DittoStepAction.Screenshot(
                        new DittoScreenshot("frame", new DittoComparison("0", false, "0"))
                    )
                ),
                Step(1, new DittoStepAction.Advance(1))
            );
            using DittoScenarioExecutor executor = Executor(
                harness,
                scenario,
                () => TimeSpan.Zero,
                _ => new DittoScreenshotStepOutcome(
                    null,
                    "P0001",
                    false,
                    DittoStepStatus.InfrastructureError
                )
            );

            Drain(executor);

            Assert.That(
                executor.Result!.Steps.Select(step => step.Status),
                Is.EqualTo(new[] { DittoStepStatus.InfrastructureError, DittoStepStatus.NotRun })
            );
        }

        [Test]
        public void ObjectWaitObservesAnAlreadyMatchingConditionBeforeAdvancing()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            DittoResolvedScenario scenario = Scenario(
                10_000,
                Step(
                    0,
                    new DittoStepAction.Wait(
                        new DittoObjectCondition(
                            Guid.NewGuid().ToString("D"),
                            DittoObjectState.Absent
                        )
                    )
                )
            );
            using DittoScenarioExecutor executor = Executor(harness, scenario, () => TimeSpan.Zero);

            Drain(executor);

            Assert.That(executor.LastCommittedFrame, Is.GreaterThanOrEqualTo(2));
            Assert.That(executor.Result!.Status, Is.EqualTo(DittoExecutionStatus.Passed));
        }

        [Test]
        public void AssertionFailureStopsLaterStepsWhileScreenshotFailureCanContinue()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            string missing = Guid.NewGuid().ToString("D");
            DittoResolvedScenario assertion = Scenario(
                1_000,
                Step(
                    0,
                    new DittoStepAction.Assert(
                        new DittoObjectCondition(missing, DittoObjectState.Exists)
                    )
                ),
                Step(1, new DittoStepAction.Click(Coordinates(0.5, 0.5)))
            );
            using DittoScenarioExecutor assertionExecutor = Executor(
                harness,
                assertion,
                () => TimeSpan.Zero
            );

            Drain(assertionExecutor);

            Assert.That(
                assertionExecutor.Result!.Steps.Select(step => step.Status),
                Is.EqualTo(new[] { DittoStepStatus.Failed, DittoStepStatus.NotRun })
            );
            Assert.That(assertionExecutor.Result.Steps[0].Assertion!.Observed, Is.False);

            assertionExecutor.Dispose();

            string checkpoint = Guid.NewGuid().ToString("D");
            DittoResolvedScenario screenshot = Scenario(
                1_000,
                Step(
                    0,
                    new DittoStepAction.Screenshot(
                        new DittoScreenshot("mismatch", new DittoComparison("0", false, "0"))
                    )
                ),
                Step(
                    1,
                    new DittoStepAction.Assert(
                        new DittoObjectCondition(missing, DittoObjectState.Absent)
                    )
                )
            );
            using DittoScenarioExecutor screenshotExecutor = Executor(
                harness,
                screenshot,
                () => TimeSpan.Zero,
                _ => new DittoScreenshotStepOutcome(checkpoint, "P0001", true)
            );

            Drain(screenshotExecutor);

            Assert.That(
                screenshotExecutor.Result!.Steps.Select(step => step.Status),
                Is.EqualTo(new[] { DittoStepStatus.Failed, DittoStepStatus.Passed })
            );
            Assert.That(screenshotExecutor.Result.PrimaryErrorRef, Is.EqualTo("P0001"));
        }

        [TestCase(100, 1_000, 1_000, 0, (int)DittoDeadlineKind.Step)]
        [TestCase(100, 100, 1_000, 50, (int)DittoDeadlineKind.Scenario)]
        [TestCase(100, 100, 80, 50, (int)DittoDeadlineKind.Run)]
        public void EarliestDeadlineStopsTheReachedStepAndNamesItsOwner(
            int stepTimeoutValue,
            int scenarioTimeoutValue,
            int runTimeoutValue,
            int setupDurationValue,
            int expectedValue
        )
        {
            var expected = (DittoDeadlineKind)expectedValue;
            ulong stepTimeout = checked((ulong)stepTimeoutValue);
            ulong scenarioTimeout = checked((ulong)scenarioTimeoutValue);
            ulong runTimeout = checked((ulong)runTimeoutValue);
            ulong setupDuration = checked((ulong)setupDurationValue);
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            TimeSpan current = TimeSpan.Zero;
            DittoResolvedScenario scenario = Scenario(
                scenarioTimeout,
                Step(
                    0,
                    new DittoStepAction.Wait(
                        new DittoObjectCondition(
                            Guid.NewGuid().ToString("D"),
                            DittoObjectState.Exists
                        )
                    ),
                    timeout: stepTimeout
                ),
                Step(1, new DittoStepAction.Click(Coordinates(0.5, 0.5)))
            );
            using DittoScenarioExecutor executor = Executor(
                harness,
                scenario,
                () => current,
                setup: () => current = TimeSpan.FromMilliseconds(setupDuration),
                runTimeout: runTimeout
            );

            for (var advance = 0; advance < 8 && executor.CurrentStepIndex is null; advance++)
            {
                Assert.That(executor.Advance(), Is.False);
                CompletePresentation(executor);
            }
            Assert.That(executor.CurrentStepIndex, Is.EqualTo(0));
            current = TimeSpan.FromMilliseconds(
                expected == DittoDeadlineKind.Run ? runTimeout : scenarioTimeout
            );
            if (expected == DittoDeadlineKind.Step)
            {
                current = TimeSpan.FromMilliseconds(setupDuration + stepTimeout);
            }
            Assert.That(executor.Advance(), Is.False);
            CompletePresentation(executor);
            Assert.That(executor.Advance(), Is.True);

            DittoScenarioExecution result = executor.Result!;
            Assert.That(result.Steps[0].Status, Is.EqualTo(DittoStepStatus.InfrastructureError));
            Assert.That(result.Steps[0].ExpiredDeadline, Is.EqualTo(expected));
            Assert.That(result.Steps[1].Status, Is.EqualTo(DittoStepStatus.NotRun));
            Assert.That(
                result.ExpiredDeadline,
                Is.EqualTo(expected == DittoDeadlineKind.Step ? null : expected)
            );
            Assert.That(result.StartupDurationMs, Is.EqualTo(setupDuration));
            Assert.That(
                result.StartupDurationMs + result.ExecutionDurationMs,
                Is.LessThanOrEqualTo(scenarioTimeout)
            );
            Assert.That(
                result.StartupDurationMs + result.ExecutionDurationMs,
                Is.LessThanOrEqualTo(runTimeout)
            );
        }

        private static DittoScenarioExecutor Executor(
            BattlementTestHarness harness,
            DittoResolvedScenario scenario,
            Func<TimeSpan> now,
            Func<DittoResolvedStep, DittoScreenshotStepOutcome>? capture = null,
            System.Action? setup = null,
            ulong runTimeout = 10_000,
            DittoPlatform platform = DittoPlatform.Macos,
            IReadOnlyDictionary<string, ObjectId>? aliases = null,
            System.Action<string>? error = null
        )
        {
            var errors = 0;
            return new DittoScenarioExecutor(
                harness.Runner,
                scenario,
                platform,
                checked((uint)Screen.width),
                checked((uint)Screen.height),
                aliases ?? new Dictionary<string, ObjectId>(),
                runTimeout,
                now,
                capture ?? (_ => throw new AssertionException("Unexpected screenshot.")),
                (_, message) =>
                {
                    error?.Invoke(message);
                    return $"P{++errors:0000}";
                },
                setup
            );
        }

        private static BattlementGameObject CameraObject(ObjectId id) =>
            new(
                id,
                new GameObjectKind.Camera(
                    new CameraState
                    {
                        Projection = CameraProjection.Orthographic,
                        OrthographicSize = 3,
                    }
                ),
                new ParentScene.Persistent(),
                null,
                true,
                new LocalTransform(
                    new Battlement.Vector3(0, 0, -10),
                    Quaternion.Identity,
                    Battlement.Vector3.One
                ),
                Array.Empty<PointerEvent>()
            );

        private static BattlementGameObject Cube(ObjectId id) =>
            new(
                id,
                new GameObjectKind.Cube(),
                new ParentScene.Persistent(),
                null,
                true,
                LocalTransform.Identity,
                new[] { PointerEvent.Click }
            );

        private static DittoResolvedScenario Scenario(
            ulong timeout,
            params DittoResolvedStep[] steps
        ) =>
            new(
                Guid.NewGuid().ToString("D"),
                0,
                "executor",
                null,
                DittoMotion.Controlled,
                timeout,
                steps
            );

        private static DittoResolvedStep Step(
            uint index,
            DittoStepAction action,
            string? name = null,
            ulong timeout = 1_000
        ) => new(index, name, timeout, action);

        private static DittoInputTarget Coordinates(double x, double y) =>
            new DittoInputTarget.Coordinates(x, y);

        private static void Drain(DittoScenarioExecutor executor)
        {
            for (var frame = 0; frame < 512; frame++)
            {
                if (executor.Advance())
                {
                    return;
                }
                CompletePresentation(executor);
            }
            Assert.Fail("Scenario did not finish within 512 presented frames.");
        }

        private static void CompletePresentation(DittoScenarioExecutor executor)
        {
            if (executor.AwaitingPresentation)
            {
                executor.CompletePresentedFrame();
            }
        }
    }
}
