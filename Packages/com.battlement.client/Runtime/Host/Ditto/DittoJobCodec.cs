#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Battlement
{
    internal static class DittoJobCodec
    {
        private static readonly UTF8Encoding StrictUtf8 = new(false, true);

        public static DittoJob Decode(ReadOnlyMemory<byte> bytes)
        {
            try
            {
                JToken token = Parse(StrictUtf8.GetString(bytes.Span));
                DittoJob job = Job(Object(token, "job"));
                DittoJobValidation.Validate(job);
                return job;
            }
            catch (JsonSerializationException)
            {
                throw;
            }
            catch (Exception exception)
            {
                throw new JsonSerializationException(
                    "The Ditto job is invalid: " + exception.Message,
                    exception
                );
            }
        }

        private static JToken Parse(string text)
        {
            using var input = new StringReader(text);
            using var reader = new JsonTextReader(input)
            {
                DateParseHandling = DateParseHandling.None,
                MaxDepth = 128,
            };
            JToken token = JToken.ReadFrom(
                reader,
                new JsonLoadSettings
                {
                    DuplicatePropertyNameHandling = DuplicatePropertyNameHandling.Error,
                }
            );
            if (reader.Read())
            {
                throw new JsonSerializationException("A job must contain one JSON value.");
            }
            return token;
        }

        private static DittoJob Job(JObject value)
        {
            Exact(
                value,
                "job_id",
                "run_id",
                "remaining_run_timeout_ms",
                "log_redactions",
                "command",
                "profile",
                "scenarios"
            );
            return new DittoJob(
                String(Field(value, "job_id")),
                String(Field(value, "run_id")),
                UInt64(Field(value, "remaining_run_timeout_ms")),
                Array(value, "log_redactions", String),
                Command(Field(value, "command")),
                Profile(Object(Field(value, "profile"), "profile")),
                Array(value, "scenarios", token => Scenario(Object(token, "scenario")))
            );
        }

        private static DittoResolvedProfile Profile(JObject value)
        {
            Exact(
                value,
                "name",
                "platform",
                "display",
                "build_fingerprint",
                "source_fingerprint",
                "capabilities"
            );
            return new DittoResolvedProfile(
                String(Field(value, "name")),
                Platform(Field(value, "platform")),
                Display(Object(Field(value, "display"), "display")),
                String(Field(value, "build_fingerprint")),
                String(Field(value, "source_fingerprint")),
                Array(value, "capabilities", Capability)
            );
        }

        private static DittoDisplay Display(JObject value)
        {
            Exact(value, "width", "height", "scale", "orientation", "safe_area");
            JToken orientation = Field(value, "orientation");
            return new DittoDisplay(
                UInt32(Field(value, "width")),
                UInt32(Field(value, "height")),
                Number(Field(value, "scale")),
                orientation.Type == JTokenType.Null ? null : Orientation(orientation),
                Array(value, "safe_area", UInt32)
            );
        }

        private static DittoResolvedScenario Scenario(JObject value)
        {
            ExactOptional(
                value,
                "fixture",
                "id",
                "run_index",
                "name",
                "motion",
                "timeout_ms",
                "steps"
            );
            JToken? fixture = value["fixture"];
            return new DittoResolvedScenario(
                String(Field(value, "id")),
                UInt32(Field(value, "run_index")),
                String(Field(value, "name")),
                fixture is null || fixture.Type == JTokenType.Null ? null : String(fixture),
                Motion(Field(value, "motion")),
                UInt64(Field(value, "timeout_ms")),
                Array(value, "steps", token => Step(Object(token, "step")))
            );
        }

        private static DittoResolvedStep Step(JObject value)
        {
            Exact(value, "index", "name", "timeout_ms", "action");
            JToken name = Field(value, "name");
            return new DittoResolvedStep(
                UInt32(Field(value, "index")),
                name.Type == JTokenType.Null ? null : String(name),
                UInt64(Field(value, "timeout_ms")),
                Action(Object(Field(value, "action"), "action"))
            );
        }

        private static DittoStepAction Action(JObject value)
        {
            if (value.Properties().Count() != 1)
            {
                throw new JsonSerializationException("An action must have exactly one variant.");
            }
            JProperty variant = value.Properties().Single();
            JObject body = Object(variant.Value, variant.Name);
            return variant.Name switch
            {
                "click" => Click(body),
                "hover" => new DittoStepAction.Hover(TargetBody(body)),
                "drag" => Drag(body),
                "key" => Key(body),
                "wait" => new DittoStepAction.Wait(Wait(body)),
                "assert" => new DittoStepAction.Assert(Condition(body)),
                "accessibility-assert" => new DittoStepAction.AccessibilityAssert(
                    AccessibilityAssertion(body)
                ),
                "accessibility-action" => AccessibilityActionStep(body),
                "screenshot" => new DittoStepAction.Screenshot(Screenshot(body)),
                "video" => new DittoStepAction.Video(Video(body)),
                _ => throw new JsonSerializationException($"Unknown action {variant.Name}."),
            };
        }

        private static DittoAccessibilityAssertion AccessibilityAssertion(JObject value)
        {
            Exact(
                value,
                "target",
                "role",
                "name",
                "selected",
                "disabled",
                "current_page",
                "parent"
            );
            return new DittoAccessibilityAssertion(
                AccessibilityTarget(Object(Field(value, "target"), "target")),
                AccessibilityRole(Field(value, "role")),
                String(Field(value, "name")),
                NullableBoolean(Field(value, "selected")),
                NullableBoolean(Field(value, "disabled")),
                NullableBoolean(Field(value, "current_page")),
                value["parent"]!.Type == JTokenType.Null
                    ? null
                    : AccessibilityTarget(Object(Field(value, "parent"), "parent"))
            );
        }

        private static bool? NullableBoolean(JToken value) =>
            value.Type == JTokenType.Null ? null : Boolean(value);

        private static DittoStepAction.AccessibilityAction AccessibilityActionStep(JObject value)
        {
            Exact(value, "target", "action");
            return new DittoStepAction.AccessibilityAction(
                AccessibilityTarget(Object(Field(value, "target"), "target")),
                AccessibilityActionValue(Field(value, "action"))
            );
        }

        private static DittoAccessibilityTarget AccessibilityTarget(JObject value)
        {
            Exact(value, "role", "name");
            return new DittoAccessibilityTarget(
                AccessibilityRole(Field(value, "role")),
                String(Field(value, "name"))
            );
        }

        private static SemanticRole AccessibilityRole(JToken value) =>
            String(value) switch
            {
                "button" => SemanticRole.Button,
                "checkbox" => SemanticRole.Checkbox,
                "switch" => SemanticRole.Switch,
                "radio" => SemanticRole.Radio,
                "radio-group" => SemanticRole.RadioGroup,
                "slider" => SemanticRole.Slider,
                "progress" => SemanticRole.Progress,
                "disclosure" => SemanticRole.Disclosure,
                "scroll-area" => SemanticRole.ScrollArea,
                "tab" => SemanticRole.Tab,
                "tab-list" => SemanticRole.TabList,
                "tab-panel" => SemanticRole.TabPanel,
                "dialog" => SemanticRole.Dialog,
                "heading" => SemanticRole.Heading,
                "image" => SemanticRole.Image,
                "static-text" => SemanticRole.StaticText,
                "group" => SemanticRole.Group,
                "list-box" => SemanticRole.ListBox,
                "option" => SemanticRole.Option,
                "table" => SemanticRole.Table,
                "row" => SemanticRole.Row,
                "column-header" => SemanticRole.ColumnHeader,
                "row-header" => SemanticRole.RowHeader,
                "cell" => SemanticRole.Cell,
                "link" => SemanticRole.Link,
                "navigation" => SemanticRole.Navigation,
                "region" => SemanticRole.Region,
                var unknown => throw new JsonSerializationException(
                    $"Unknown accessibility role {unknown}."
                ),
            };

        private static AccessibilityAction AccessibilityActionValue(JToken value) =>
            String(value) switch
            {
                "activate" => new AccessibilityAction.Activate(),
                "increment" => new AccessibilityAction.Increment(),
                "decrement" => new AccessibilityAction.Decrement(),
                "dismiss" => new AccessibilityAction.Dismiss(),
                "scroll-forward" => new AccessibilityAction.Scroll(
                    AccessibilityScrollDirection.Forward
                ),
                "scroll-backward" => new AccessibilityAction.Scroll(
                    AccessibilityScrollDirection.Backward
                ),
                var unknown => throw new JsonSerializationException(
                    $"Unknown accessibility action {unknown}."
                ),
            };

        private static DittoStepAction.Click Click(JObject value)
        {
            Exact(value, "target", "settle");
            return new DittoStepAction.Click(
                Target(Field(value, "target")),
                Boolean(Field(value, "settle"))
            );
        }

        private static DittoInputTarget TargetBody(JObject value)
        {
            Exact(value, "target");
            return Target(Field(value, "target"));
        }

        private static DittoStepAction Drag(JObject value)
        {
            Exact(value, "from", "to");
            return new DittoStepAction.Drag(
                Target(Field(value, "from")),
                Target(Field(value, "to"))
            );
        }

        private static DittoStepAction Key(JObject value)
        {
            Exact(value, "key", "action");
            return new DittoStepAction.Key(
                String(Field(value, "key")),
                KeyAction(Field(value, "action"))
            );
        }

        private static DittoInputTarget Target(JToken value)
        {
            if (value.Type == JTokenType.String)
            {
                return new DittoInputTarget.Object(String(value));
            }
            JArray coordinates = JsonArray(value, "input target");
            if (coordinates.Count != 2)
            {
                throw new JsonSerializationException("Input coordinates require two values.");
            }
            return new DittoInputTarget.Coordinates(Number(coordinates[0]), Number(coordinates[1]));
        }

        private static DittoWait Wait(JObject value)
        {
            if (value.Property("frames") is not null)
            {
                Exact(value, "frames");
                return new DittoWait.Frames(UInt32(Field(value, "frames")));
            }
            return new DittoWait.Object(Condition(value));
        }

        private static DittoObjectCondition Condition(JObject value)
        {
            Exact(value, "object", "state");
            return new DittoObjectCondition(
                String(Field(value, "object")),
                ObjectState(Field(value, "state"))
            );
        }

        private static DittoScreenshot Screenshot(JObject value)
        {
            Exact(value, "name", "comparison");
            return new DittoScreenshot(
                String(Field(value, "name")),
                Comparison(Object(Field(value, "comparison"), "comparison"))
            );
        }

        private static DittoComparison Comparison(JObject value)
        {
            Exact(value, "threshold", "anti_alias", "max_changed_percent");
            return new DittoComparison(
                String(Field(value, "threshold")),
                Boolean(Field(value, "anti_alias")),
                String(Field(value, "max_changed_percent"))
            );
        }

        private static DittoVideo Video(JObject value)
        {
            string action = String(Field(value, "action"));
            if (action == "stop")
            {
                Exact(value, "action");
                return new DittoVideo.Stop();
            }
            if (action != "start")
            {
                throw new JsonSerializationException($"Unknown video action {action}.");
            }
            Exact(value, "action", "name", "motion", "max_duration_ms");
            return new DittoVideo.Start(
                String(Field(value, "name")),
                Motion(Field(value, "motion")),
                UInt64(Field(value, "max_duration_ms"))
            );
        }

        private static IReadOnlyList<T> Array<T>(
            JObject value,
            string field,
            Func<JToken, T> convert
        ) => JsonArray(Field(value, field), field).Select(convert).ToArray();

        private static JObject Object(JToken value, string field) =>
            value as JObject ?? throw new JsonSerializationException($"{field} must be an object.");

        private static JArray JsonArray(JToken value, string field) =>
            value as JArray ?? throw new JsonSerializationException($"{field} must be an array.");

        private static JToken Field(JObject value, string field) =>
            value[field]
            ?? throw new JsonSerializationException($"Missing required field {field}.");

        private static string String(JToken value) =>
            value.Type == JTokenType.String
                ? value.Value<string>()!
                : throw new JsonSerializationException("Expected a string.");

        private static bool Boolean(JToken value) =>
            value.Type == JTokenType.Boolean
                ? value.Value<bool>()
                : throw new JsonSerializationException("Expected a Boolean.");

        private static uint UInt32(JToken value)
        {
            ulong parsed = UInt64(value);
            return parsed <= uint.MaxValue
                ? (uint)parsed
                : throw new JsonSerializationException("Integer exceeds UInt32.");
        }

        private static ulong UInt64(JToken value) =>
            value.Type == JTokenType.Integer
                ? value.ToObject<ulong>()
                : throw new JsonSerializationException("Expected an unsigned integer.");

        private static double Number(JToken value) =>
            value.Type is JTokenType.Integer or JTokenType.Float
                ? value.ToObject<double>()
                : throw new JsonSerializationException("Expected a number.");

        private static void Exact(JObject value, params string[] fields)
        {
            var expected = new HashSet<string>(fields, StringComparer.Ordinal);
            string? unknown = value
                .Properties()
                .Select(property => property.Name)
                .FirstOrDefault(name => !expected.Contains(name));
            if (unknown is not null)
            {
                throw new JsonSerializationException($"Unknown field {unknown}.");
            }
            string? missing = fields.FirstOrDefault(field => value.Property(field) is null);
            if (missing is not null)
            {
                throw new JsonSerializationException($"Missing required field {missing}.");
            }
        }

        private static void ExactOptional(JObject value, string optional, params string[] fields)
        {
            var expected = new HashSet<string>(fields, StringComparer.Ordinal) { optional };
            string? unknown = value
                .Properties()
                .Select(property => property.Name)
                .FirstOrDefault(name => !expected.Contains(name));
            if (unknown is not null)
            {
                throw new JsonSerializationException($"Unknown field {unknown}.");
            }
            string? missing = fields.FirstOrDefault(field => value.Property(field) is null);
            if (missing is not null)
            {
                throw new JsonSerializationException($"Missing required field {missing}.");
            }
        }

        private static DittoCommand Command(JToken value) =>
            String(value) switch
            {
                "run" => DittoCommand.Run,
                "capture" => DittoCommand.Capture,
                string other => throw new JsonSerializationException($"Unknown command {other}."),
            };

        private static DittoPlatform Platform(JToken value) =>
            String(value) switch
            {
                "macos" => DittoPlatform.Macos,
                "webgl" => DittoPlatform.Webgl,
                "ios-simulator" => DittoPlatform.IosSimulator,
                string other => throw new JsonSerializationException($"Unknown platform {other}."),
            };

        private static DittoOrientation Orientation(JToken value) =>
            String(value) switch
            {
                "portrait" => DittoOrientation.Portrait,
                "portrait-upside-down" => DittoOrientation.PortraitUpsideDown,
                "landscape-left" => DittoOrientation.LandscapeLeft,
                "landscape-right" => DittoOrientation.LandscapeRight,
                string other => throw new JsonSerializationException(
                    $"Unknown orientation {other}."
                ),
            };

        private static DittoCapability Capability(JToken value) =>
            String(value) switch
            {
                "click" => DittoCapability.Click,
                "hover" => DittoCapability.Hover,
                "drag" => DittoCapability.Drag,
                "key" => DittoCapability.Key,
                "png" => DittoCapability.Png,
                "video" => DittoCapability.Video,
                string other => throw new JsonSerializationException(
                    $"Unknown capability {other}."
                ),
            };

        private static DittoMotion Motion(JToken value) =>
            String(value) switch
            {
                "instant" => DittoMotion.Instant,
                "controlled" => DittoMotion.Controlled,
                "real-time" => DittoMotion.RealTime,
                string other => throw new JsonSerializationException($"Unknown motion {other}."),
            };

        private static DittoKeyAction KeyAction(JToken value) =>
            String(value) switch
            {
                "down" => DittoKeyAction.Down,
                "up" => DittoKeyAction.Up,
                "tap" => DittoKeyAction.Tap,
                string other => throw new JsonSerializationException(
                    $"Unknown key action {other}."
                ),
            };

        private static DittoObjectState ObjectState(JToken value) =>
            String(value) switch
            {
                "exists" => DittoObjectState.Exists,
                "absent" => DittoObjectState.Absent,
                "visible" => DittoObjectState.Visible,
                "hidden" => DittoObjectState.Hidden,
                "enabled" => DittoObjectState.Enabled,
                "disabled" => DittoObjectState.Disabled,
                string other => throw new JsonSerializationException(
                    $"Unknown object state {other}."
                ),
            };
    }
}
