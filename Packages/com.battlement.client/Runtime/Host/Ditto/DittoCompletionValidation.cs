#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using Newtonsoft.Json;

namespace Battlement
{
    internal static class DittoCompletionValidation
    {
        public static void ValidateScenarioComplete(
            DittoScenarioComplete complete,
            DittoJob job,
            IReadOnlyList<string> observedErrorRefs
        )
        {
            DittoResolvedScenario scenario = DittoLifecycleValidation.Scenario(
                job,
                complete.ScenarioId
            );
            Require(
                complete.Steps.Count == scenario.Steps.Count,
                "completion must retain every authored step"
            );
            Require(complete.Artifacts.Count <= 128, "scenario may reach at most 128 artifacts");
            Require(
                complete.VideoInputs.Count <= 64,
                "scenario may contain at most 64 video inputs"
            );
            ulong elapsed;
            try
            {
                elapsed = checked(complete.StartupDurationMs + complete.ExecutionDurationMs);
            }
            catch (OverflowException exception)
            {
                throw new JsonSerializationException("scenario duration overflow", exception);
            }
            Require(elapsed <= scenario.TimeoutMs, "scenario completion exceeds its deadline");
            var observed = new HashSet<string>(observedErrorRefs, StringComparer.Ordinal);
            Require(
                observed.Count == observedErrorRefs.Count,
                "observed error references must be unique"
            );
            foreach (string errorRef in observedErrorRefs)
            {
                DittoLifecycleValidation.PlayerErrorRef(errorRef);
            }
            for (var index = 0; index < scenario.Steps.Count; index++)
            {
                DittoPlayerStepResult result = complete.Steps[index];
                ValidateStepResult(scenario.Steps[index], result);
                foreach (string errorRef in result.ErrorRefs)
                {
                    Require(
                        observed.Contains(errorRef),
                        "step contains an unobserved error reference"
                    );
                }
            }
            ValidateStatus(complete, observed);
            HashSet<string> artifactIds = ValidateArtifacts(complete, scenario);
            ValidateFailureFrame(complete, artifactIds, observed);
            ValidateVideos(complete, scenario);
            ValidateBoundary(complete.Boundary, observed);
        }

        internal static void ValidateStepResult(
            DittoResolvedStep expected,
            DittoPlayerStepResult result
        )
        {
            Require(result.Index == expected.Index, "step result index does not match the job");
            Require(result.Name == expected.Name, "step result name does not match the job");
            Require(
                result.Kind == DittoLifecycleValidation.StepName(expected.Action),
                "step result kind does not match the job"
            );
            Require(result.DurationMs <= expected.TimeoutMs, "step result exceeds its deadline");
            Require(result.ErrorRefs.Count <= 16, "step may contain at most 16 error references");
            var unique = new HashSet<string>(StringComparer.Ordinal);
            foreach (string errorRef in result.ErrorRefs)
            {
                DittoLifecycleValidation.PlayerErrorRef(errorRef);
                Require(unique.Add(errorRef), "step error references must be unique");
            }
            switch (result.Status)
            {
                case DittoStepStatus.Passed:
                    Require(
                        !result.ExpiredDeadline.HasValue && result.ErrorRefs.Count == 0,
                        "passed step must not contain an expiry or errors"
                    );
                    break;
                case DittoStepStatus.Failed:
                case DittoStepStatus.InfrastructureError:
                    Require(result.ErrorRefs.Count > 0, "failed step requires an error reference");
                    break;
                case DittoStepStatus.NotRun:
                    Require(
                        NotRunHasNoPayload(result),
                        "not-run step must have zero duration and no reached payload"
                    );
                    break;
                case DittoStepStatus.Interrupted:
                    break;
                default:
                    throw new JsonSerializationException("Unknown step status.");
            }
            ValidateAssertion(expected, result);
            ValidateScreenshot(expected, result);
            ValidateVideo(expected, result);
        }

        private static bool NotRunHasNoPayload(DittoPlayerStepResult result)
        {
            bool timing = result.DurationMs == 0 && !result.ExpiredDeadline.HasValue;
            bool errors = result.ErrorRefs.Count == 0 && result.Assertion is null;
            bool media = result.ScreenshotArtifactId is null && result.VideoInputId is null;
            return timing && errors && media;
        }

        private static void ValidateAssertion(
            DittoResolvedStep expected,
            DittoPlayerStepResult result
        )
        {
            if (result.Assertion is null)
            {
                bool completed = result.Status is DittoStepStatus.Passed or DittoStepStatus.Failed;
                Require(
                    expected.Action is not DittoStepAction.Assert || !completed,
                    "completed assertion step requires an assertion payload"
                );
                return;
            }
            Require(
                expected.Action is DittoStepAction.Assert,
                "assertion payload belongs only to an assertion step"
            );
            var condition = ((DittoStepAction.Assert)expected.Action).Condition;
            DittoAssertionResult assertion = result.Assertion;
            DittoLifecycleValidation.Identifier("assertion object", assertion.Object);
            Require(
                assertion.Object == condition.Object,
                "assertion object does not match the job"
            );
            Require(assertion.State == condition.State, "assertion state does not match the job");
            Require(assertion.Expected, "assertion expected value must be true");
            Require(assertion.Passed == assertion.Observed, "assertion passed must equal observed");
            Require(
                assertion.Passed == (result.Status == DittoStepStatus.Passed),
                "assertion result must agree with step status"
            );
        }

        private static void ValidateScreenshot(
            DittoResolvedStep expected,
            DittoPlayerStepResult result
        )
        {
            if (result.ScreenshotArtifactId is null)
            {
                Require(
                    expected.Action is not DittoStepAction.Screenshot
                        || result.Status != DittoStepStatus.Passed,
                    "passed screenshot step requires an artifact ID"
                );
                return;
            }
            Require(
                expected.Action is DittoStepAction.Screenshot,
                "screenshot artifact belongs only to a screenshot step"
            );
            DittoLifecycleValidation.Identifier(
                "screenshot artifact_id",
                result.ScreenshotArtifactId
            );
        }

        private static void ValidateVideo(DittoResolvedStep expected, DittoPlayerStepResult result)
        {
            if (result.VideoInputId is null)
            {
                return;
            }
            Require(
                expected.Action is DittoStepAction.Video { Value: DittoVideo.Start },
                "video input belongs only to a video start step"
            );
            DittoLifecycleValidation.Identifier("video input_id", result.VideoInputId);
        }

        private static void ValidateStatus(DittoScenarioComplete complete, HashSet<string> observed)
        {
            if (complete.PrimaryErrorRef is not null)
            {
                DittoLifecycleValidation.PlayerErrorRef(complete.PrimaryErrorRef);
                Require(
                    observed.Contains(complete.PrimaryErrorRef),
                    "primary_error_ref must resolve to an observed error"
                );
            }
            switch (complete.ExecutionStatus)
            {
                case DittoExecutionStatus.Passed:
                    Require(
                        complete.PrimaryErrorRef is null,
                        "passed scenario has no primary error"
                    );
                    Require(
                        complete.Steps.All(step => step.Status == DittoStepStatus.Passed),
                        "passed scenario requires every step to pass"
                    );
                    break;
                case DittoExecutionStatus.Failed:
                    Require(
                        complete.PrimaryErrorRef is not null,
                        "failed scenario requires a primary error reference"
                    );
                    break;
                case DittoExecutionStatus.Interrupted:
                    break;
                default:
                    throw new JsonSerializationException("Unknown execution status.");
            }
        }

        private static HashSet<string> ValidateArtifacts(
            DittoScenarioComplete complete,
            DittoResolvedScenario scenario
        )
        {
            var ids = new HashSet<string>(StringComparer.Ordinal);
            foreach (DittoReachedArtifact artifact in complete.Artifacts)
            {
                DittoLifecycleValidation.Identifier("artifact_id", artifact.ArtifactId);
                Require(ids.Add(artifact.ArtifactId), "reached artifact IDs must be unique");
                DittoLifecycleValidation.ArtifactKind(artifact.Kind);
                if (artifact.StepIndex.HasValue)
                {
                    Require(
                        artifact.StepIndex.Value < scenario.Steps.Count,
                        "artifact step_index is outside the scenario"
                    );
                }
                if (artifact.Kind is DittoArtifactKind.Screenshot screenshot)
                {
                    ValidateScreenshotArtifact(complete, scenario, artifact, screenshot);
                }
                else
                {
                    ValidateFailureArtifact(complete, artifact);
                }
            }
            foreach (DittoPlayerStepResult step in complete.Steps)
            {
                if (step.ScreenshotArtifactId is not null)
                {
                    Require(
                        ids.Contains(step.ScreenshotArtifactId),
                        "step references an unknown artifact"
                    );
                }
            }
            return ids;
        }

        private static void ValidateScreenshotArtifact(
            DittoScenarioComplete complete,
            DittoResolvedScenario scenario,
            DittoReachedArtifact artifact,
            DittoArtifactKind.Screenshot screenshot
        )
        {
            Require(artifact.StepIndex.HasValue, "screenshot artifact requires a step_index");
            uint index = artifact.StepIndex!.Value;
            Require(
                scenario.Steps[(int)index].Action is DittoStepAction.Screenshot,
                "screenshot artifact must reference a screenshot step"
            );
            var expected = (DittoStepAction.Screenshot)scenario.Steps[(int)index].Action;
            Require(
                screenshot.Checkpoint == expected.Value.Name,
                "artifact checkpoint does not match the job"
            );
            Require(
                complete.Steps[(int)index].ScreenshotArtifactId == artifact.ArtifactId,
                "artifact ID does not match its screenshot step result"
            );
        }

        private static void ValidateFailureArtifact(
            DittoScenarioComplete complete,
            DittoReachedArtifact artifact
        )
        {
            Require(
                complete.FailureFrame is DittoPlayerFailureFrame.Captured,
                "failure-frame artifact requires a captured failure frame"
            );
            var captured = (DittoPlayerFailureFrame.Captured)complete.FailureFrame!;
            Require(
                captured.ArtifactId == artifact.ArtifactId,
                "failure-frame artifact does not match the captured frame"
            );
        }

        private static void ValidateFailureFrame(
            DittoScenarioComplete complete,
            HashSet<string> artifactIds,
            HashSet<string> observed
        )
        {
            if (complete.FailureFrame is null)
            {
                return;
            }
            DittoLifecycleValidation.FailureFrame(complete.FailureFrame);
            if (complete.FailureFrame is DittoPlayerFailureFrame.Captured captured)
            {
                Require(
                    artifactIds.Contains(captured.ArtifactId),
                    "failure frame references an unknown artifact"
                );
                Require(
                    complete.Artifacts.Any(artifact =>
                        artifact.ArtifactId == captured.ArtifactId
                        && artifact.Kind is DittoArtifactKind.FailureFrame
                    ),
                    "captured failure frame must reference a failure-frame artifact"
                );
                return;
            }
            var unavailable = (DittoPlayerFailureFrame.Unavailable)complete.FailureFrame;
            Require(
                unavailable.ErrorRef is null || observed.Contains(unavailable.ErrorRef),
                "failure frame contains an unobserved error reference"
            );
        }

        private static void ValidateVideos(
            DittoScenarioComplete complete,
            DittoResolvedScenario scenario
        )
        {
            var ids = new HashSet<string>(StringComparer.Ordinal);
            foreach (DittoNativeVideoInput input in complete.VideoInputs)
            {
                DittoLifecycleValidation.NativeVideo(input, scenario);
                Require(ids.Add(input.InputId), "native video input IDs must be unique");
                Require(
                    complete.Steps[(int)input.StartStepIndex].VideoInputId == input.InputId,
                    "native video input does not match its start-step result"
                );
            }
            foreach (DittoPlayerStepResult step in complete.Steps)
            {
                if (step.VideoInputId is not null)
                {
                    Require(
                        ids.Contains(step.VideoInputId),
                        "step references an unknown native video input"
                    );
                }
            }
        }

        private static void ValidateBoundary(
            DittoScenarioBoundary boundary,
            HashSet<string> observed
        )
        {
            if (boundary is DittoScenarioBoundary.Failed failed)
            {
                DittoLifecycleValidation.PlayerErrorRef(failed.ErrorRef);
                Require(
                    observed.Contains(failed.ErrorRef),
                    "boundary contains an unobserved error reference"
                );
            }
        }

        private static void Require(bool condition, string message) =>
            DittoLifecycleValidation.Require(condition, message);
    }
}
