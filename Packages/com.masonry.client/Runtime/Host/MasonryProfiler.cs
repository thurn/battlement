#nullable enable

using Unity.Profiling;

namespace Masonry
{
    internal static class MasonryProfiler
    {
        internal static readonly ProfilerMarker Frame = new("Masonry.Frame");
        internal static readonly ProfilerMarker Poll = new("Masonry.Poll");
        internal static readonly ProfilerMarker Serialization = new("Masonry.Serialization");
        internal static readonly ProfilerMarker Transport = new("Masonry.Transport");
        internal static readonly ProfilerMarker ResponseParsing = new("Masonry.Response.Parse");
        internal static readonly ProfilerMarker ResponseApplication = new("Masonry.Response.Apply");
        internal static readonly ProfilerMarker CustomHandler = new("Masonry.CustomHandler");
    }
}
