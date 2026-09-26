#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    /// <summary>Runs native panel timers on the same clock as controlled presentation.</summary>
    internal sealed class BattlementUiPanelClock
    {
        private readonly Func<TimeSpan> now;
        private readonly Func<bool> controlled;
        private readonly Dictionary<IPanel, object?> previous = new();
        private Delegate? timeSource;

        internal BattlementUiPanelClock(Func<TimeSpan> now, Func<bool> controlled)
        {
            this.now = now;
            this.controlled = controlled;
        }

        [UnityEngine.Scripting.Preserve]
        // Unity truncates seconds to milliseconds; preserve exact millisecond boundaries.
        private double Seconds()
        {
            double seconds = now().Ticks / (double)TimeSpan.TicksPerSecond;
            return BitConverter.Int64BitsToDouble(BitConverter.DoubleToInt64Bits(seconds) + 1);
        }

        internal void Bind(IPanel? panel)
        {
            if (!controlled() || panel is null)
                return;
            if (previous.ContainsKey(panel))
                return;
            timeSource ??= Delegate.CreateDelegate(
                Native.TimeSource.PropertyType,
                this,
                nameof(Seconds)
            );
            previous.Add(panel, Native.TimeSource.GetValue(panel));
            Native.TimeSource.SetValue(panel, timeSource);
        }

        internal bool HasPendingActivation(IEnumerable<VisualElement> elements) =>
            controlled()
            && elements
                .OfType<Button>()
                .Any(button =>
                    button.panel is not null
                    && Native.PendingActivation.GetValue(button.clickable) is not null
                );

        internal void Clear()
        {
            foreach ((IPanel panel, object? source) in previous)
                Native.TimeSource.SetValue(panel, source);
            previous.Clear();
        }

        private static class Native
        {
            private const BindingFlags Instance = BindingFlags.Instance | BindingFlags.NonPublic;
            internal static readonly PropertyInfo TimeSource =
                typeof(VisualElement)
                    .Assembly.GetType("UnityEngine.UIElements.BaseVisualElementPanel")!
                    .GetProperty("TimeSinceStartupFunc", Instance)
                ?? throw new InvalidOperationException(
                    "Unity's native panel clock is unavailable."
                );
            internal static readonly FieldInfo PendingActivation =
                typeof(Clickable).GetField("m_PendingActivePseudoStateReset", Instance)
                ?? throw new InvalidOperationException(
                    "Unity's native button feedback is unavailable."
                );
        }
    }
}
