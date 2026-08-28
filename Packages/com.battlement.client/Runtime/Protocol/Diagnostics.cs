#nullable enable

using System;

namespace Battlement
{
    /// <summary>Commands supported by Battlement's Unity Diagnostics module.</summary>
    public abstract record DiagnosticsCommand
    {
        private DiagnosticsCommand() { }

        /// <summary>Sets one metadata value, or clears the key when value is null.</summary>
        public sealed record SetMetadata(string Key, string? Value = null) : DiagnosticsCommand;
    }

    /// <summary>Unity Diagnostics metadata bounds shared by protocol clients.</summary>
    public static class DiagnosticsProtocol
    {
        public const int MaximumMetadataKeyLength = 255;
        public const int MaximumMetadataValueLength = 1024;

        /// <summary>Validates a Diagnostics command and returns a stable error code.</summary>
        public static CoreErrorCode? Validate(DiagnosticsCommand command)
        {
            if (command is null)
                return CoreErrorCode.InvalidEncoding;
            return command switch
            {
                DiagnosticsCommand.SetMetadata value
                    when !ValidKey(value.Key) || !ValidValue(value.Value) =>
                    CoreErrorCode.DiagnosticsMetadataInvalid,
                _ => null,
            };
        }

        private static bool ValidKey(string? key)
        {
            if (
                string.IsNullOrEmpty(key)
                || !TryCountScalars(key, out int length)
                || length > MaximumMetadataKeyLength
            )
                return false;
            if (char.IsWhiteSpace(key[0]) || char.IsWhiteSpace(key[^1]))
                return false;
            foreach (char character in key)
            {
                if (char.IsControl(character))
                    return false;
            }
            return true;
        }

        private static bool ValidValue(string? value)
        {
            if (value is null)
                return true;
            if (!TryCountScalars(value, out int length) || length > MaximumMetadataValueLength)
                return false;
            return !value.Contains("\0", StringComparison.Ordinal);
        }

        private static bool TryCountScalars(string value, out int count)
        {
            count = 0;
            for (int index = 0; index < value.Length; index++)
            {
                char character = value[index];
                if (char.IsHighSurrogate(character))
                {
                    if (index + 1 >= value.Length || !char.IsLowSurrogate(value[index + 1]))
                        return false;
                    index++;
                }
                else if (char.IsLowSurrogate(character))
                {
                    return false;
                }
                count++;
            }
            return true;
        }
    }

    public abstract partial record CommandBody
    {
        /// <summary>Enriches future Unity Diagnostics reports with custom metadata.</summary>
        public sealed record Diagnostics(DiagnosticsCommand Command) : CommandBody;
    }
}
