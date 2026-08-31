#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.InputSystem;

namespace Battlement.Tests
{
    public sealed class DittoScenarioExecutorTests : InputTestFixture
    {
        [Test]
        public void ExecutesEveryNonVideoStepOnMacos()
        {
            ExecuteEveryNonVideoStepAndRetainNamesAndProductionInput(DittoPlatform.Macos);
        }

        [Test]
        public void ExecutesEveryNonVideoStepOnWebgl()
        {
            ExecuteEveryNonVideoStepAndRetainNamesAndProductionInput(DittoPlatform.Webgl);
        }

        [Test]
        public void ControlledWaitFramesCompleteInOneHostAdvance()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            DittoResolvedScenario scenario = Scenario(
                10_000,
                Step(0, new DittoStepAction.Wait(new DittoWait.Frames(200)))
            );
            using DittoScenarioExecutor executor = Executor(harness, scenario, () => TimeSpan.Zero);

            Drain(executor);
            Assert.That(executor.LastCommittedFrame, Is.GreaterThanOrEqualTo(202));
            Assert.That(executor.Result!.Status, Is.EqualTo(DittoExecutionStatus.Passed));
        }

        [Test]
        public void ControlledInputYieldsAfterOneVirtualFrame()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            DittoResolvedScenario scenario = Scenario(
                10_000,
                Step(0, new DittoStepAction.Click(Coordinates(0.1, 0.2)))
            );
            using DittoScenarioExecutor executor = Executor(harness, scenario, () => TimeSpan.Zero);

            Assert.That(executor.Advance(), Is.False);
            Assert.That(executor.LastCommittedFrame, Is.EqualTo(1));

            Drain(executor);
            Assert.That(executor.Result!.Status, Is.EqualTo(DittoExecutionStatus.Passed));
        }

        [Test]
        public void SettledInputObservesTwoQuietFramesAfterVirtualInputDrains()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            DittoResolvedScenario scenario = Scenario(
                10_000,
                Step(0, new DittoStepAction.Click(Coordinates(0.1, 0.2)))
            );
            using DittoScenarioExecutor executor = Executor(harness, scenario, () => TimeSpan.Zero);

            while (executor.CurrentStepIndex is null)
            {
                Assert.That(executor.Advance(), Is.False);
            }
            ulong inputStartedAt = executor.LastCommittedFrame;

            Drain(executor);

            Assert.That(executor.LastCommittedFrame - inputStartedAt, Is.GreaterThanOrEqualTo(3));
            Assert.That(executor.Result!.Status, Is.EqualTo(DittoExecutionStatus.Passed));
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
                        new DittoWait.Object(
                            new DittoObjectCondition(
                                Guid.NewGuid().ToString("D"),
                                DittoObjectState.Absent
                            )
                        )
                    )
                )
            );
            using DittoScenarioExecutor executor = Executor(harness, scenario, () => TimeSpan.Zero);

            Drain(executor);

            Assert.That(executor.LastCommittedFrame, Is.GreaterThanOrEqualTo(2));
            Assert.That(executor.Result!.Status, Is.EqualTo(DittoExecutionStatus.Passed));
        }

        private void ExecuteEveryNonVideoStepAndRetainNamesAndProductionInput(
            DittoPlatform platform
        )
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            string missing = Guid.NewGuid().ToString("D");
            string artifact = Guid.NewGuid().ToString("D");
            DittoResolvedScenario scenario = Scenario(
                5_000,
                Step(0, new DittoStepAction.Click(Coordinates(0.1, 0.2)), "click"),
                Step(1, new DittoStepAction.Wait(new DittoWait.Frames(2)), "frames"),
                Step(2, new DittoStepAction.Hover(Coordinates(0.3, 0.4)), "hover"),
                Step(3, new DittoStepAction.Drag(Coordinates(0.3, 0.4), Coordinates(0.8, 0.7))),
                Step(4, new DittoStepAction.Key("Enter", DittoKeyAction.Tap), "key"),
                Step(
                    5,
                    new DittoStepAction.Wait(
                        new DittoWait.Object(
                            new DittoObjectCondition(missing, DittoObjectState.Absent)
                        )
                    )
                ),
                Step(
                    6,
                    new DittoStepAction.Assert(
                        new DittoObjectCondition(missing, DittoObjectState.Absent)
                    ),
                    "assert"
                ),
                Step(
                    7,
                    new DittoStepAction.Screenshot(
                        new DittoScreenshot("ready", new DittoComparison("0", false, "0"))
                    ),
                    "capture"
                )
            );
            using DittoScenarioExecutor executor = Executor(
                harness,
                scenario,
                () => TimeSpan.Zero,
                _ => new DittoScreenshotStepOutcome(artifact, null, false),
                platform: platform
            );

            Drain(executor);

            DittoScenarioExecution result = executor.Result!;
            Assert.That(result.Status, Is.EqualTo(DittoExecutionStatus.Passed));
            Assert.That(
                result.Steps.Select(step => step.Status),
                Is.All.EqualTo(DittoStepStatus.Passed)
            );
            Assert.That(
                result.Steps.Select(step => step.Name),
                Is.EqualTo(
                    new[] { "click", "frames", "hover", null, "key", null, "assert", "capture" }
                )
            );
            Assert.That(result.Steps[6].Assertion!.Observed, Is.True);
            Assert.That(result.Steps[7].ScreenshotArtifactId, Is.EqualTo(artifact));
            Mouse mouse = InputSystem.devices.OfType<Mouse>().Single();
            Assert.That(
                mouse.position.ReadValue().x,
                Is.EqualTo((Screen.width - 1) * 0.8f).Within(0.01f)
            );
            Assert.That(
                mouse.position.ReadValue().y,
                Is.EqualTo((Screen.height - 1) * 0.3f).Within(0.01f)
            );
            Keyboard keyboard = InputSystem.devices.OfType<Keyboard>().Single();
            Assert.That(keyboard.enterKey.isPressed, Is.False);
            foreach (DittoPlayerStepResult step in result.Steps)
            {
                DittoCompletionValidation.ValidateStepResult(scenario.Steps[(int)step.Index], step);
            }
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
                        new DittoWait.Object(
                            new DittoObjectCondition(
                                Guid.NewGuid().ToString("D"),
                                DittoObjectState.Exists
                            )
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
            }
            Assert.That(executor.CurrentStepIndex, Is.EqualTo(0));
            current = TimeSpan.FromMilliseconds(
                expected == DittoDeadlineKind.Run ? runTimeout : scenarioTimeout
            );
            if (expected == DittoDeadlineKind.Step)
            {
                current = TimeSpan.FromMilliseconds(setupDuration + stepTimeout);
            }
            Assert.That(executor.Advance(), Is.True);

            DittoScenarioExecution result = executor.Result!;
            Assert.That(result.Steps[0].Status, Is.EqualTo(DittoStepStatus.Failed));
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
            DittoPlatform platform = DittoPlatform.Macos
        )
        {
            var errors = 0;
            return new DittoScenarioExecutor(
                harness.Runner,
                scenario,
                platform,
                checked((uint)Screen.width),
                checked((uint)Screen.height),
                new Dictionary<string, ObjectId>(),
                runTimeout,
                now,
                capture ?? (_ => throw new AssertionException("Unexpected screenshot.")),
                (_, _) => $"P{++errors:0000}",
                setup
            );
        }

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
            for (var frame = 0; frame < 256; frame++)
            {
                if (executor.Advance())
                {
                    return;
                }
            }
            Assert.Fail("Scenario did not finish within 256 committed frames.");
        }
    }
}
