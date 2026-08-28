#nullable enable

using System;
using System.Text;
using Battlement.Errors;
using UnityEngine;

namespace Battlement
{
    /// <summary>How a captured failure participates in Unity exception reporting.</summary>
    public enum BattlementErrorReportingDisposition
    {
        Ignore,
        AlreadyLoggedByUnity,
        ReportCaughtFailure,
    }

    /// <summary>Reports otherwise-caught Battlement failures as Unity exceptions.</summary>
    public interface IBattlementCaughtFailureReporter
    {
        void Report(BattlementError error);
    }

    internal sealed class UnityCaughtFailureReporter : IBattlementCaughtFailureReporter
    {
        public void Report(BattlementError error) =>
            Debug.LogException(new BattlementCaughtFailureException(error));
    }

    internal sealed class BattlementCaughtFailureException : Exception
    {
        private readonly string originalStackTrace;

        public BattlementCaughtFailureException(BattlementError error)
            : base(DiagnosticMessage(error)) => originalStackTrace = DiagnosticStackTrace(error);

        public override string StackTrace => originalStackTrace;

        public override string ToString() => $"{GetType().FullName}: {Message}\n{StackTrace}";

        private static string DiagnosticMessage(BattlementError error) =>
            Bound(
                $"{Sanitize(error.EventName)}; source={error.Source}; type={error.Type}; "
                    + Sanitize(error.Message),
                512
            );

        private static string DiagnosticStackTrace(BattlementError error) =>
            BoundStack(
                SanitizeStack(error.StackTrace ?? error.Exception?.StackTrace ?? string.Empty)
            );

        private static string Sanitize(string value)
        {
            var builder = new StringBuilder(value.Length);
            foreach (char character in value)
            {
                builder.Append(char.IsControl(character) ? ' ' : character);
            }
            return builder.ToString();
        }

        private static string SanitizeStack(string value)
        {
            var builder = new StringBuilder(value.Length);
            bool escape = false;
            foreach (char character in value)
            {
                if (escape)
                {
                    if (character == 'm')
                        escape = false;
                    continue;
                }
                if (character == '\u001b')
                {
                    escape = true;
                    continue;
                }
                if (!char.IsControl(character) || character is '\n' or '\r' or '\t')
                    builder.Append(character);
            }
            return builder.ToString();
        }

        private static string Bound(string value, int maximum) =>
            value.Length <= maximum ? value : value.Substring(0, maximum);

        private static string BoundStack(string value)
        {
            const int maximum = 32768;
            if (value.Length <= maximum)
                return value;
            const string marker = "\n... omitted characters ...\n";
            int side = (maximum - marker.Length) / 2;
            return value.Substring(0, side) + marker + value.Substring(value.Length - side);
        }
    }
}
