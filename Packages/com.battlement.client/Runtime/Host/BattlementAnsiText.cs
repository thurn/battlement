#nullable enable

using System.Text;
using System.Text.RegularExpressions;

namespace Battlement.Errors
{
    internal readonly struct BattlementFormattedText
    {
        public BattlementFormattedText(string plainText, string richText) =>
            (PlainText, RichText) = (plainText, richText);

        public string PlainText { get; }

        public string RichText { get; }
    }

    internal static class BattlementAnsiText
    {
        private static readonly Regex EscapeSequence = new(
            "\\u001b\\[([0-9;]*)m",
            RegexOptions.CultureInvariant
        );

        public static BattlementFormattedText Format(string? value)
        {
            string source = value ?? string.Empty;
            var rich = new StringBuilder(source.Length + 64);
            var style = new AnsiStyle();
            int offset = 0;
            foreach (Match match in EscapeSequence.Matches(source))
            {
                rich.Append(Escape(source.Substring(offset, match.Index - offset)));
                CloseStyle(rich, style);
                ApplyCodes(ref style, match.Groups[1].Value);
                OpenStyle(rich, style);
                offset = match.Index + match.Length;
            }

            rich.Append(Escape(source.Substring(offset)));
            CloseStyle(rich, style);
            return new BattlementFormattedText(
                EscapeSequence.Replace(source, string.Empty),
                rich.ToString()
            );
        }

        public static string Escape(string value) => value.Replace("<", "<noparse><</noparse>");

        private static void ApplyCodes(ref AnsiStyle style, string value)
        {
            string[] codes = string.IsNullOrEmpty(value) ? new[] { "0" } : value.Split(';');
            for (int index = 0; index < codes.Length; index++)
            {
                if (!int.TryParse(codes[index], out int code))
                {
                    continue;
                }

                if (code == 38 && TryReadIndexedColor(codes, ref index, out int indexedColor))
                {
                    style.Color = Color(indexedColor);
                    continue;
                }

                switch (code)
                {
                    case 0:
                        style = new AnsiStyle();
                        break;
                    case 1:
                        style.Bold = true;
                        break;
                    case 22:
                        style.Bold = false;
                        break;
                    case 39:
                        style.Color = null;
                        break;
                    case >= 30 and <= 37:
                        style.Color = Color(code - 30);
                        break;
                    case >= 90 and <= 97:
                        style.Color = Color(code - 90 + 8);
                        break;
                    default:
                        break;
                }
            }
        }

        private static bool TryReadIndexedColor(string[] codes, ref int index, out int color)
        {
            color = 0;
            if (index + 2 >= codes.Length)
            {
                return false;
            }
            if (codes[index + 1] != "5")
            {
                return false;
            }
            if (!int.TryParse(codes[index + 2], out color))
            {
                return false;
            }

            index += 2;
            return true;
        }

        private static string? Color(int code) =>
            code switch
            {
                1 => "#FF6B6B",
                2 => "#9CA3AF",
                5 => "#93C5FD",
                6 => "#67E8F9",
                7 => "#F4F4F5",
                8 => "#6B7280",
                9 => "#FBBF24",
                10 => "#CBD5E1",
                13 => "#BFDBFE",
                14 => "#A5F3FC",
                15 => "#FFFFFF",
                _ => null,
            };

        private static void OpenStyle(StringBuilder text, AnsiStyle style)
        {
            if (style.Color is not null)
            {
                text.Append($"<color={style.Color}>");
            }
            if (style.Bold)
            {
                text.Append("<b>");
            }
        }

        private static void CloseStyle(StringBuilder text, AnsiStyle style)
        {
            if (style.Bold)
            {
                text.Append("</b>");
            }
            if (style.Color is not null)
            {
                text.Append("</color>");
            }
        }

        private struct AnsiStyle
        {
            public bool Bold;
            public string? Color;
        }
    }
}
