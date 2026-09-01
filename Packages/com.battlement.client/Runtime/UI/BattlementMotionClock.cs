#nullable enable

using System;

namespace Battlement.UI
{
    internal static class BattlementMotionClock
    {
        public static ulong ToMicros(double seconds)
        {
            if (!double.IsFinite(seconds) || seconds < 0)
                throw new BattlementUiException(
                    CoreErrorCode.InvalidProperty,
                    "A motion clock returned invalid time."
                );
            return checked((ulong)Math.Round(seconds * 1_000_000d));
        }
    }
}
