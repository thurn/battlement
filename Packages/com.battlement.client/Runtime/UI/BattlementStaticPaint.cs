#nullable enable

using System.Linq;
using Newtonsoft.Json.Linq;

namespace Battlement.UI
{
    internal static class BattlementStaticPaint
    {
        public static bool Owns(MotionProperty property) =>
            property
                is MotionProperty.BackgroundColor
                    or MotionProperty.BackgroundGradient
                    or MotionProperty.ClipPolygon
                    or MotionProperty.BoxShadow;

        public static bool IsOnlyChange(MotionDescriptor next, MotionDescriptor previous)
        {
            if (next.Generation != previous.Generation)
                return false;
            if (Equal(next.StaticBaseline, previous.StaticBaseline))
                return false;
            return Equal(next, previous with { StaticBaseline = next.StaticBaseline })
                && Equal(
                    next.StaticBaseline.Where(value => !Owns(value.Property)).ToArray(),
                    previous.StaticBaseline.Where(value => !Owns(value.Property)).ToArray()
                );
        }

        private static bool Equal(object left, object right) =>
            JToken.DeepEquals(JToken.FromObject(left), JToken.FromObject(right));
    }
}
