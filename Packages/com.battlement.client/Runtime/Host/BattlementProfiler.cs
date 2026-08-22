#nullable enable

using Unity.Profiling;

namespace Battlement
{
    internal static class BattlementProfiler
    {
        internal static readonly ProfilerMarker Frame = new("Battlement.Frame");
        internal static readonly ProfilerMarker Poll = new("Battlement.Poll");
        internal static readonly ProfilerMarker Serialization = new("Battlement.Serialization");
        internal static readonly ProfilerMarker Transport = new("Battlement.Transport");
        internal static readonly ProfilerMarker ResponseParsing = new("Battlement.Response.Parse");
        internal static readonly ProfilerMarker ResponseApplication = new(
            "Battlement.Response.Apply"
        );
        internal static readonly ProfilerMarker CustomHandler = new("Battlement.CustomHandler");
    }
}
