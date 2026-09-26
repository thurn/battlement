#nullable enable

namespace Battlement
{
    /// <summary>Host-reported preference for reducing nonessential motion.</summary>
    public enum ReducedMotionPreference
    {
        Unavailable,
        Reduce,
        NoPreference,
    }

    /// <summary>The latest application focus and pause observations.</summary>
    public sealed record ApplicationState(bool Focused = true, bool Paused = false);

    public abstract partial record CommandBody
    {
        /// <summary>Requests the platform's external handler for an absolute URL.</summary>
        public sealed record ApplicationOpenUrl(string Url) : CommandBody;

        /// <summary>Apply pacing without changing the window or display preview.</summary>
        public sealed record ApplicationSetFramePacing(FramePacing Value) : CommandBody;
    }
}
