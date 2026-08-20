#nullable enable

using System;

namespace Masonry.VisualCapture
{
    internal static class CaptureArguments
    {
        internal static string? Value(string name)
        {
            string[] arguments = Environment.GetCommandLineArgs();
            for (int index = 0; index < arguments.Length - 1; index++)
            {
                if (arguments[index] == name)
                {
                    return arguments[index + 1];
                }
            }
            return null;
        }

        internal static bool Has(string name) =>
            Array.IndexOf(Environment.GetCommandLineArgs(), name) >= 0;
    }
}
