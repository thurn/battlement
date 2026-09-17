#nullable enable

namespace Battlement
{
    /// <summary>Whether ordering applies to a visual group or a single renderer.</summary>
    public enum RenderOrderKind : byte
    {
        Group,
        Layer,
    }

    /// <summary>Visual order relative to the nearest enclosing sorting group.</summary>
    public readonly struct RenderOrder
    {
        public RenderOrder(RenderOrderKind kind, short order) => (Kind, Order) = (kind, order);

        public RenderOrderKind Kind { get; }
        public short Order { get; }
    }
}
