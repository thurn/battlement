#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using Newtonsoft.Json;
using UnityEngine.InputSystem;

namespace Battlement
{
    internal static class DittoJobValidation
    {
        public static void Validate(DittoJob job)
        {
            Identifier("job_id", job.JobId);
            Identifier("run_id", job.RunId);
            Require(
                job.RemainingRunTimeoutMs is > 0 and <= 3_600_000,
                "remaining_run_timeout_ms must be from 1 through 3600000"
            );
            Redactions(job.LogRedactions);
            Profile(job.Profile);
            Require(job.Scenarios.Count <= 128, "job may contain at most 128 scenarios");
            var ids = new HashSet<string>(StringComparer.Ordinal);
            var names = new HashSet<string>(StringComparer.Ordinal);
            uint? previousIndex = null;
            foreach (DittoResolvedScenario scenario in job.Scenarios)
            {
                Identifier("scenario.id", scenario.Id);
                Require(ids.Add(scenario.Id), "scenario IDs must be unique");
                Name("scenario.name", scenario.Name);
                Require(names.Add(scenario.Name), "scenario names must be unique");
                if (previousIndex.HasValue)
                {
                    Require(
                        scenario.RunIndex > previousIndex.Value,
                        "scenario run_index values must increase"
                    );
                }
                previousIndex = scenario.RunIndex;
                Scenario(job, scenario);
            }
        }

        private static void Redactions(IReadOnlyList<string> redactions)
        {
            Require(redactions.Count <= 128, "log_redactions may contain at most 128 values");
            var unique = new HashSet<string>(StringComparer.Ordinal);
            foreach (string value in redactions)
            {
                Require(value.Length > 0, "log redactions must not be empty");
                Require(
                    Bytes(value) <= 4_096,
                    "log redactions may contain at most 4096 UTF-8 bytes"
                );
                Require(unique.Add(value), "log redactions must be unique");
            }
        }

        private static void Profile(DittoResolvedProfile profile)
        {
            Name("profile.name", profile.Name);
            Sha256("profile.build_fingerprint", profile.BuildFingerprint);
            Sha256("profile.source_fingerprint", profile.SourceFingerprint);
            Display(profile.Platform, profile.Display);
            var unique = new HashSet<DittoCapability>(profile.Capabilities);
            Require(
                unique.Count == profile.Capabilities.Count,
                "profile capabilities must be unique"
            );
            DittoCapability? unsupported = profile.Platform switch
            {
                DittoPlatform.Webgl => DittoCapability.Video,
                DittoPlatform.IosSimulator => DittoCapability.Hover,
                _ => null,
            };
            Require(
                !unsupported.HasValue || !unique.Contains(unsupported.Value),
                "profile contains a capability unsupported by its platform"
            );
        }

        private static void Display(DittoPlatform platform, DittoDisplay display)
        {
            Require(display.Width > 0 && display.Height > 0, "display dimensions must be positive");
            Require(
                double.IsFinite(display.Scale) && display.Scale > 0,
                "display scale must be finite and positive"
            );
            Require(display.SafeArea.Count == 4, "display safe area requires four values");
            uint x = display.SafeArea[0];
            uint y = display.SafeArea[1];
            uint width = display.SafeArea[2];
            uint height = display.SafeArea[3];
            Require(width > 0 && height > 0, "display safe area must be nonempty");
            Require(
                (ulong)x + width <= display.Width && (ulong)y + height <= display.Height,
                "display safe area must fit inside the framebuffer"
            );
            if (platform == DittoPlatform.IosSimulator)
            {
                Require(
                    display.Orientation.HasValue,
                    "iOS Simulator display requires an orientation"
                );
                return;
            }
            Require(!display.Orientation.HasValue, "desktop display must not have an orientation");
            Require(
                IsDesktopSafeArea(display, x, y, width, height),
                "desktop safe area must equal the framebuffer"
            );
        }

        private static bool IsDesktopSafeArea(
            DittoDisplay display,
            uint x,
            uint y,
            uint width,
            uint height
        )
        {
            bool origin = x == 0 && y == 0;
            bool dimensions = width == display.Width && height == display.Height;
            return origin && dimensions;
        }

        private static void Scenario(DittoJob job, DittoResolvedScenario scenario)
        {
            Require(scenario.TimeoutMs > 0, "scenario timeout must be positive");
            Require(
                scenario.TimeoutMs <= job.RemainingRunTimeoutMs,
                "scenario timeout may not exceed the remaining run timeout"
            );
            Require(
                scenario.Steps.Count is > 0 and <= 128,
                "scenario must contain 1 through 128 steps"
            );
            var state = new ScenarioState();
            for (var index = 0; index < scenario.Steps.Count; index++)
            {
                DittoResolvedStep step = scenario.Steps[index];
                Require(step.Index == index, "step indices must match authored order");
                Step(job, scenario, step, state);
            }
            Require(state.HeldKeys.Count == 0, "keys must be released before the scenario ends");
            Require(state.ActiveVideo is null, "video start must have a matching stop");
        }

        private static void Step(
            DittoJob job,
            DittoResolvedScenario scenario,
            DittoResolvedStep step,
            ScenarioState state
        )
        {
            Require(step.TimeoutMs > 0, "step timeout must be positive");
            Require(
                step.TimeoutMs <= scenario.TimeoutMs,
                "step timeout may not exceed the scenario timeout"
            );
            if (step.Name is not null)
            {
                Name("step.name", step.Name);
                Require(state.Names.Add(step.Name), "step names must be unique within a scenario");
            }
            switch (step.Action)
            {
                case DittoStepAction.Click click:
                    Capability(job, DittoCapability.Click);
                    Target(click.Target);
                    break;
                case DittoStepAction.Hover hover:
                    Capability(job, DittoCapability.Hover);
                    Target(hover.Target);
                    break;
                case DittoStepAction.Drag drag:
                    Capability(job, DittoCapability.Drag);
                    Target(drag.From);
                    Target(drag.To);
                    break;
                case DittoStepAction.Key key:
                    Capability(job, DittoCapability.Key);
                    Key(key, state);
                    break;
                case DittoStepAction.Wait wait:
                    Wait(scenario.Motion, wait.Value);
                    break;
                case DittoStepAction.Assert assertion:
                    Identifier("object condition", assertion.Condition.Object);
                    break;
                case DittoStepAction.Screenshot screenshot:
                    Capability(job, DittoCapability.Png);
                    Screenshot(screenshot.Value, state);
                    break;
                case DittoStepAction.Video video:
                    Capability(job, DittoCapability.Video);
                    Video(video.Value, state);
                    break;
                default:
                    throw new JsonSerializationException("Unknown Ditto step action.");
            }
        }

        private static void Capability(DittoJob job, DittoCapability required) =>
            Require(
                job.Profile.Capabilities.Contains(required),
                $"step requires unsupported capability {required}"
            );

        private static void Target(DittoInputTarget target)
        {
            switch (target)
            {
                case DittoInputTarget.Object value:
                    Identifier("input target", value.Id);
                    break;
                case DittoInputTarget.Coordinates value:
                    Require(
                        Coordinate(value.X) && Coordinate(value.Y),
                        "input coordinates must be finite and from 0.0 through 1.0"
                    );
                    break;
                default:
                    throw new JsonSerializationException("Unknown input target.");
            }
        }

        private static bool Coordinate(double value) =>
            double.IsFinite(value) && value is >= 0 and <= 1;

        private static void Wait(DittoMotion motion, DittoWait wait)
        {
            switch (wait)
            {
                case DittoWait.Frames frames:
                    Require(frames.Count > 0, "frame wait must be positive");
                    Require(
                        motion == DittoMotion.Controlled,
                        "frame wait requires controlled motion"
                    );
                    break;
                case DittoWait.Object value:
                    Identifier("object condition", value.Condition.Object);
                    break;
                default:
                    throw new JsonSerializationException("Unknown wait variant.");
            }
        }

        private static void Key(DittoStepAction.Key key, ScenarioState state)
        {
            Require(
                key.Value.Length is > 0 and <= 128 && key.Value.All(IsAsciiAlphaNumeric),
                "key must be a Unity Input System Key enum name"
            );
            Require(
                Enum.TryParse(key.Value, false, out Key parsed)
                    && Enum.GetName(typeof(Key), parsed) == key.Value
                    && parsed != UnityEngine.InputSystem.Key.None,
                "key must be a Unity Input System Key enum name"
            );
            switch (key.Action)
            {
                case DittoKeyAction.Down:
                    Require(state.HeldKeys.Add(key.Value), "key is already held");
                    break;
                case DittoKeyAction.Up:
                    Require(state.HeldKeys.Remove(key.Value), "key is not held");
                    break;
                case DittoKeyAction.Tap:
                    Require(!state.HeldKeys.Contains(key.Value), "key is already held");
                    break;
                default:
                    throw new JsonSerializationException("Unknown key action.");
            }
        }

        private static bool IsAsciiAlphaNumeric(char value) =>
            value is >= '0' and <= '9' || value is >= 'A' and <= 'Z' || value is >= 'a' and <= 'z';

        private static void Screenshot(DittoScreenshot screenshot, ScenarioState state)
        {
            Name("screenshot.name", screenshot.Name);
            Require(
                state.Checkpoints.Add(screenshot.Name),
                "screenshot names must be unique within a scenario"
            );
            Decimal("comparison.threshold", screenshot.Comparison.Threshold, "1");
            Decimal(
                "comparison.max_changed_percent",
                screenshot.Comparison.MaxChangedPercent,
                "100"
            );
        }

        private static void Video(DittoVideo video, ScenarioState state)
        {
            if (video is DittoVideo.Stop)
            {
                Require(state.ActiveVideo is not null, "video stop has no matching start");
                state.ActiveVideo = null;
                return;
            }
            var start = (DittoVideo.Start)video;
            Name("video.name", start.Name);
            Require(state.ActiveVideo is null, "videos may not overlap");
            Require(state.Videos.Add(start.Name), "video names must be unique within a scenario");
            Require(start.Motion != DittoMotion.Instant, "video motion must not be instant");
            Require(
                start.MaxDurationMs is > 0 and <= 30_000,
                "video duration must be from 1 through 30000 milliseconds"
            );
            state.ActiveVideo = start.Name;
        }

        private static void Name(string field, string value)
        {
            Require(value.Length > 0, $"{field} must not be empty");
            Require(Bytes(value) <= 128, $"{field} may contain at most 128 UTF-8 bytes");
        }

        private static void Identifier(string field, string value)
        {
            bool parsed = Guid.TryParseExact(value, "D", out Guid id);
            Require(parsed, $"{field} must be a UUID");
            Require(id != Guid.Empty, $"{field} must not be nil");
            Require(id.ToString("D") == value, $"{field} must use canonical lowercase UUID text");
        }

        private static void Sha256(string field, string value) =>
            Require(
                value.Length == 64 && value.All(IsLowerHex),
                $"{field} must contain exactly 64 lowercase hexadecimal digits"
            );

        private static bool IsLowerHex(char value) =>
            value is >= '0' and <= '9' || value is >= 'a' and <= 'f';

        private static void Decimal(string field, string value, string maximum)
        {
            Require(value.Length > 0, $"{field} must be an unsigned base-10 decimal");
            Require(
                !value.StartsWith("+", StringComparison.Ordinal)
                    && !value.StartsWith("-", StringComparison.Ordinal),
                $"{field} must be an unsigned base-10 decimal without an exponent"
            );
            Require(
                !value.Contains("e") && !value.Contains("E"),
                $"{field} must be an unsigned base-10 decimal without an exponent"
            );
            string[] parts = value.Split('.');
            bool integerValid =
                parts.Length > 0 && parts[0].Length > 0 && parts[0].All(IsAsciiDigit);
            bool fractionValid =
                parts.Length == 1 || parts[1].Length > 0 && parts[1].All(IsAsciiDigit);
            Require(
                parts.Length <= 2 && integerValid && fractionValid,
                $"{field} must contain digits with at most one decimal point"
            );
            Require(
                parts[0].Length == 1 || parts[0][0] != '0',
                $"{field} must not contain a redundant leading zero"
            );
            bool atMaximum =
                parts[0] == maximum && (parts.Length == 1 || parts[1].All(value => value == '0'));
            bool shorter = parts[0].Length < maximum.Length;
            bool lower =
                parts[0].Length == maximum.Length && string.CompareOrdinal(parts[0], maximum) < 0;
            Require(shorter || lower || atMaximum, $"{field} must be from 0 through {maximum}");
        }

        private static bool IsAsciiDigit(char value) => value is >= '0' and <= '9';

        private static int Bytes(string value) => Encoding.UTF8.GetByteCount(value);

        private static void Require(bool condition, string message)
        {
            if (!condition)
            {
                throw new JsonSerializationException(message);
            }
        }

        private sealed class ScenarioState
        {
            public HashSet<string> Names { get; } = new(StringComparer.Ordinal);

            public HashSet<string> Checkpoints { get; } = new(StringComparer.Ordinal);

            public HashSet<string> Videos { get; } = new(StringComparer.Ordinal);

            public HashSet<string> HeldKeys { get; } = new(StringComparer.Ordinal);

            public string? ActiveVideo { get; set; }
        }
    }
}
