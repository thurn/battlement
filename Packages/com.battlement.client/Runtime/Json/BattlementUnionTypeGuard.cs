#nullable enable

using System;
using System.Collections.Generic;

namespace Battlement
{
    internal static class BattlementUnionTypeGuard
    {
        [ThreadStatic]
        private static HashSet<Type>? disabledTypes;

        public static bool IsDisabled(Type objectType) =>
            disabledTypes?.Contains(objectType) == true;

        public static void Add(Type type) => (disabledTypes ??= new HashSet<Type>()).Add(type);

        public static void Remove(Type type) => disabledTypes!.Remove(type);
    }
}
