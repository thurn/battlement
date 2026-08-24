#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;

namespace Battlement.Errors
{
    internal static class BattlementCSharpExceptionText
    {
        private const int MaximumVisibleFrames = 32;
        private const string ApplicationColor = "#FBBF24";
        private const string ExceptionColor = "#FF6B6B";
        private const string FrameworkColor = "#9CA3AF";
        private const string SourceColor = "#67E8F9";

        private static readonly Regex AsyncMethod = new(
            @"^(?<owner>.+)\.<(?<method>[^>]+)>d__\d+\.MoveNext\(\)$",
            RegexOptions.CultureInvariant
        );
        private static readonly Regex LambdaMethod = new(
            @"^(?<owner>.+)\.<(?<method>[^>]+)>b__[^.()]+\(\)$",
            RegexOptions.CultureInvariant
        );
        private static readonly Regex LocalMethod = new(
            @"^(?<owner>.+)\.<(?<method>[^>]+)>g__(?<local>[^|]+)\|[^.()]+\(\)$",
            RegexOptions.CultureInvariant
        );
        private static readonly Regex UnityAsyncMethod = new(
            @"^(?<owner>.+)\.<(?<method>[^>]+)>d__\d+:MoveNext\(\)$",
            RegexOptions.CultureInvariant
        );

        public static BattlementFormattedText Format(
            Exception? exception,
            string? stackTrace,
            string fallbackMessage
        )
        {
            var plain = new StringBuilder();
            var rich = new StringBuilder();
            int nextFrame = 0;
            if (exception is null)
            {
                AppendRawException(plain, rich, stackTrace, fallbackMessage, ref nextFrame);
            }
            else
            {
                AppendException(plain, rich, exception, new List<Exception>(), 0, ref nextFrame);
            }

            return new BattlementFormattedText(plain.ToString(), rich.ToString());
        }

        private static void AppendRawException(
            StringBuilder plain,
            StringBuilder rich,
            string? stackTrace,
            string fallbackMessage,
            ref int nextFrame
        )
        {
            AppendHeading(plain, rich, FirstLine(fallbackMessage));
            if (string.IsNullOrWhiteSpace(stackTrace))
            {
                AppendFrames(plain, rich, null, ref nextFrame);
                return;
            }

            var frames = new StringBuilder();
            foreach (
                string line in stackTrace.Split(
                    new[] { '\r', '\n' },
                    StringSplitOptions.RemoveEmptyEntries
                )
            )
            {
                string value = line.Trim();
                if (!value.StartsWith("Rethrow as ", StringComparison.Ordinal))
                {
                    frames.AppendLine(line);
                    continue;
                }

                AppendFrames(plain, rich, frames.ToString(), ref nextFrame);
                frames.Clear();
                plain.AppendLine().AppendLine("Rethrow as:");
                rich.AppendLine().AppendLine($"<color={FrameworkColor}>Rethrow as:</color>");
                AppendHeading(plain, rich, value.Substring("Rethrow as ".Length));
            }
            AppendFrames(plain, rich, frames.ToString(), ref nextFrame);
        }

        private static void AppendException(
            StringBuilder plain,
            StringBuilder rich,
            Exception exception,
            List<Exception> visited,
            int depth,
            ref int nextFrame
        )
        {
            if (depth == 8 || visited.Any(value => ReferenceEquals(value, exception)))
            {
                AppendMutedLine(plain, rich, "Further inner exceptions omitted.");
                return;
            }

            if (depth > 0)
            {
                plain.AppendLine().AppendLine("Caused by:");
                rich.AppendLine().AppendLine($"<color={FrameworkColor}>Caused by:</color>");
            }
            visited.Add(exception);
            AppendHeading(
                plain,
                rich,
                $"{ShortTypeName(exception.GetType())}: {exception.Message}"
            );
            AppendFrames(plain, rich, exception.StackTrace, ref nextFrame);

            if (exception is AggregateException aggregate)
            {
                foreach (Exception inner in aggregate.InnerExceptions)
                {
                    AppendException(plain, rich, inner, visited, depth + 1, ref nextFrame);
                }
                return;
            }
            if (exception.InnerException is not null)
            {
                AppendException(
                    plain,
                    rich,
                    exception.InnerException,
                    visited,
                    depth + 1,
                    ref nextFrame
                );
            }
        }

        private static void AppendHeading(StringBuilder plain, StringBuilder rich, string heading)
        {
            string normalized = heading.Trim();
            int separator = normalized.IndexOf(':');
            string type = separator < 0 ? normalized : normalized.Substring(0, separator);
            string message = separator < 0 ? string.Empty : normalized.Substring(separator + 1);
            plain.Append(type);
            rich.Append(
                $"<color={ExceptionColor}><b>{BattlementAnsiText.Escape(type)}</b></color>"
            );
            if (!string.IsNullOrWhiteSpace(message))
            {
                plain.Append(':').Append(message);
                rich.Append(':').Append(BattlementAnsiText.Escape(message));
            }
            plain.AppendLine();
            rich.AppendLine();
        }

        private static void AppendFrames(
            StringBuilder plain,
            StringBuilder rich,
            string? stackTrace,
            ref int nextFrame
        )
        {
            List<CSharpFrame> frames = ParseFrames(stackTrace);
            if (frames.Count == 0)
            {
                AppendMutedLine(plain, rich, "Stack trace unavailable.");
                return;
            }

            bool onlyFrameworkFrames = frames.All(frame => frame.IsFramework);
            int hidden = 0;
            int omitted = 0;
            foreach (CSharpFrame frame in frames)
            {
                if (frame.IsFramework && !onlyFrameworkFrames)
                {
                    hidden++;
                    continue;
                }
                if (nextFrame == MaximumVisibleFrames)
                {
                    omitted++;
                    continue;
                }

                AppendHiddenFrames(plain, rich, ref hidden);
                AppendFrame(plain, rich, frame, nextFrame++, onlyFrameworkFrames);
            }
            AppendHiddenFrames(plain, rich, ref hidden);
            if (omitted > 0)
            {
                AppendMutedLine(
                    plain,
                    rich,
                    $": {omitted} additional {(omitted == 1 ? "frame" : "frames")} omitted :"
                );
            }
        }

        private static void AppendFrame(
            StringBuilder plain,
            StringBuilder rich,
            CSharpFrame frame,
            int number,
            bool framework
        )
        {
            string prefix = $"{number, 2}: ";
            plain.Append(prefix).AppendLine(frame.Method);
            string color = framework ? FrameworkColor : ApplicationColor;
            rich.Append($"<color={color}>");
            if (!framework)
            {
                rich.Append("<b>");
            }
            rich.Append(BattlementAnsiText.Escape(prefix + frame.Method));
            if (!framework)
            {
                rich.Append("</b>");
            }
            rich.AppendLine("</color>");

            if (frame.Source is null)
            {
                return;
            }
            string source = $"    at {frame.Source}";
            plain.AppendLine(source);
            rich.AppendLine($"<color={SourceColor}>{BattlementAnsiText.Escape(source)}</color>");
        }

        private static void AppendHiddenFrames(
            StringBuilder plain,
            StringBuilder rich,
            ref int count
        )
        {
            if (count == 0)
            {
                return;
            }

            string line = $": {count} framework {(count == 1 ? "frame" : "frames")} hidden :";
            plain.AppendLine(line);
            rich.AppendLine($"<color={SourceColor}>{line}</color>");
            count = 0;
        }

        private static void AppendMutedLine(StringBuilder plain, StringBuilder rich, string value)
        {
            plain.AppendLine(value);
            rich.AppendLine($"<color={FrameworkColor}>{BattlementAnsiText.Escape(value)}</color>");
        }

        private static List<CSharpFrame> ParseFrames(string? stackTrace) =>
            string.IsNullOrWhiteSpace(stackTrace)
                ? new List<CSharpFrame>()
                : stackTrace
                    .Split(new[] { '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries)
                    .Select(ParseFrame)
                    .Where(frame => frame is not null)
                    .Cast<CSharpFrame>()
                    .ToList();

        private static CSharpFrame? ParseFrame(string line)
        {
            string value = line.Trim();
            if (value.StartsWith("at ", StringComparison.Ordinal))
            {
                value = value.Substring(3);
            }
            string? source = null;
            int sourceStart = value.LastIndexOf(" (at ", StringComparison.Ordinal);
            if (sourceStart >= 0 && value.EndsWith(")", StringComparison.Ordinal))
            {
                source = value.Substring(sourceStart + 5, value.Length - sourceStart - 6);
                value = value.Substring(0, sourceStart);
            }
            else
            {
                int inStart = value.LastIndexOf(" in ", StringComparison.Ordinal);
                if (inStart >= 0)
                {
                    source = value.Substring(inStart + 4);
                    value = value.Substring(0, inStart);
                }
            }

            if (source is null && !LooksLikeFrame(value))
            {
                return null;
            }

            int offset = value.IndexOf(" [0x", StringComparison.Ordinal);
            if (offset >= 0)
            {
                value = value.Substring(0, offset);
            }
            string method = NormalizeMethod(value.Trim());
            string? normalizedSource = NormalizeSource(source);
            return new CSharpFrame(
                method,
                normalizedSource,
                IsFrameworkFrame(method, normalizedSource)
            );
        }

        private static string NormalizeMethod(string value)
        {
            string method = value.Replace(" ()", "()").Replace('+', '.');
            Match unityAsync = UnityAsyncMethod.Match(method);
            if (unityAsync.Success)
            {
                return $"{unityAsync.Groups["owner"].Value}."
                    + $"{unityAsync.Groups["method"].Value}() [async]";
            }
            Match async = AsyncMethod.Match(method);
            if (async.Success)
            {
                return $"{async.Groups["owner"].Value}.{async.Groups["method"].Value}() [async]";
            }
            Match lambda = LambdaMethod.Match(method);
            if (lambda.Success)
            {
                return $"{lambda.Groups["owner"].Value}.{lambda.Groups["method"].Value}() [lambda]";
            }
            Match local = LocalMethod.Match(method);
            if (local.Success)
            {
                return $"{local.Groups["owner"].Value}.{local.Groups["method"].Value}()"
                    + $" [{local.Groups["local"].Value}]";
            }
            return method;
        }

        private static bool LooksLikeFrame(string value) =>
            value.Contains("()", StringComparison.Ordinal)
            || value.Contains(" ()", StringComparison.Ordinal)
            || value.Contains(" [0x", StringComparison.Ordinal);

        private static string? NormalizeSource(string? value)
        {
            if (string.IsNullOrWhiteSpace(value))
            {
                return null;
            }

            string source = value.Replace('\\', '/').Replace(":line ", ":");
            if (source.StartsWith("<", StringComparison.Ordinal))
            {
                return null;
            }
            int assets = source.LastIndexOf("/Assets/", StringComparison.Ordinal);
            if (assets >= 0)
            {
                return source.Substring(assets + 1);
            }
            int packages = source.LastIndexOf("/Packages/", StringComparison.Ordinal);
            if (packages >= 0)
            {
                return source.Substring(packages + 1);
            }

            int separator = source.LastIndexOf(':');
            if (separator > 0 && int.TryParse(source.Substring(separator + 1), out _))
            {
                return $"{Path.GetFileName(source.Substring(0, separator))}"
                    + source.Substring(separator);
            }
            return Path.GetFileName(source);
        }

        private static bool IsFrameworkFrame(string method, string? source)
        {
            string[] prefixes =
            {
                "Microsoft.",
                "Mono.",
                "System.",
                "TMPro.",
                "Unity.",
                "UnityEditor.",
                "UnityEngine.",
            };
            if (prefixes.Any(prefix => method.StartsWith(prefix, StringComparison.Ordinal)))
            {
                return true;
            }
            if (IsBattlementSource(source))
            {
                return true;
            }

            string[] battlementBoundaries =
            {
                "Battlement.BattlementBatch",
                "Battlement.BattlementCommand",
                "Battlement.Errors.BattlementError",
                "Battlement.BattlementOperations",
                "Battlement.BattlementRunner",
                "Battlement.BattlementSession",
                "Battlement.Errors.BattlementUnityErrors",
            };
            return battlementBoundaries.Any(prefix =>
                method.StartsWith(prefix, StringComparison.Ordinal)
            );
        }

        private static bool IsBattlementSource(string? source) =>
            source?.StartsWith("Packages/com.battlement.client/Runtime/", StringComparison.Ordinal)
                == true
            || source?.StartsWith(
                "Packages/com.battlement.client/Editor/",
                StringComparison.Ordinal
            ) == true;

        private static string ShortTypeName(Type type) => type.Name;

        private static string FirstLine(string value) =>
            value
                .Split(new[] { '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries)
                .FirstOrDefault()
            ?? "C# exception";

        private sealed class CSharpFrame
        {
            public CSharpFrame(string method, string? source, bool isFramework) =>
                (Method, Source, IsFramework) = (method, source, isFramework);

            public bool IsFramework { get; }

            public string Method { get; }

            public string? Source { get; }
        }
    }
}
