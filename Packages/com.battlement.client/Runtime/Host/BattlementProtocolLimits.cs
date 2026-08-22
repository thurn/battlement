#nullable enable

using System;

namespace Battlement
{
    internal static class BattlementProtocolLimits
    {
        public const int MaximumMessageBytes = 16 * 1024 * 1024;

        public static readonly TimeSpan MaximumDuration = TimeSpan.FromDays(1);

        public static TimeSpan RequireDuration(TimeSpan value, string name, bool allowZero = true)
        {
            if (value < TimeSpan.Zero || (!allowZero && value == TimeSpan.Zero))
            {
                string requirement = allowZero ? "nonnegative" : "positive";
                throw new BattlementCommandException(
                    CoreErrorCode.InvalidProperty,
                    $"{name} must be {requirement}."
                );
            }

            if (value > MaximumDuration)
            {
                throw new BattlementCommandException(
                    CoreErrorCode.LimitExceeded,
                    $"{name} cannot exceed {MaximumDuration.TotalMilliseconds} milliseconds."
                );
            }

            return value;
        }
    }
}
