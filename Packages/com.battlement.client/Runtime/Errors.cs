#nullable enable

using System;

namespace Battlement
{
    internal static class Errors
    {
        public static T CheckNotNull<T>(T? value, string parameterName)
            where T : class => value ?? throw new ArgumentNullException(parameterName);
    }
}
