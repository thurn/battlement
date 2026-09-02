#nullable enable

using System.Collections.Generic;

namespace Battlement
{
    internal enum DittoCommand
    {
        Run,
        Capture,
    }

    internal enum DittoPlatform
    {
        Macos,
        Webgl,
        IosSimulator,
    }

    internal enum DittoOrientation
    {
        Portrait,
        PortraitUpsideDown,
        LandscapeLeft,
        LandscapeRight,
    }

    internal enum DittoCapability
    {
        Click,
        Hover,
        Drag,
        Key,
        Png,
        Video,
    }

    internal enum DittoMotion
    {
        Instant,
        Controlled,
        RealTime,
    }

    internal enum DittoKeyAction
    {
        Down,
        Up,
        Tap,
    }

    internal enum DittoObjectState
    {
        Exists,
        Absent,
        Visible,
        Hidden,
        Enabled,
        Disabled,
    }

    internal sealed record DittoJob(
        string JobId,
        string RunId,
        ulong RemainingRunTimeoutMs,
        IReadOnlyList<string> LogRedactions,
        DittoCommand Command,
        DittoResolvedProfile Profile,
        IReadOnlyList<DittoResolvedScenario> Scenarios
    );

    internal sealed record DittoResolvedProfile(
        string Name,
        DittoPlatform Platform,
        DittoDisplay Display,
        string BuildFingerprint,
        string SourceFingerprint,
        IReadOnlyList<DittoCapability> Capabilities
    );

    internal sealed record DittoDisplay(
        uint Width,
        uint Height,
        double Scale,
        DittoOrientation? Orientation,
        IReadOnlyList<uint> SafeArea
    );

    internal sealed record DittoResolvedScenario(
        string Id,
        uint RunIndex,
        string Name,
        string? Fixture,
        DittoMotion Motion,
        ulong TimeoutMs,
        IReadOnlyList<DittoResolvedStep> Steps
    );

    internal sealed record DittoResolvedStep(
        uint Index,
        string? Name,
        ulong TimeoutMs,
        DittoStepAction Action
    );

    internal abstract record DittoStepAction
    {
        internal sealed record Click(DittoInputTarget Target, bool Settle = true) : DittoStepAction;

        internal sealed record Hover(DittoInputTarget Target) : DittoStepAction;

        internal sealed record Drag(DittoInputTarget From, DittoInputTarget To) : DittoStepAction;

        internal sealed record Key(string Value, DittoKeyAction Action) : DittoStepAction;

        internal sealed record Wait(DittoWait Value) : DittoStepAction;

        internal sealed record Assert(DittoObjectCondition Condition) : DittoStepAction;

        internal sealed record AccessibilityAssert(DittoAccessibilityAssertion Value)
            : DittoStepAction;

        internal sealed record AccessibilityAction(
            DittoAccessibilityTarget Target,
            global::Battlement.AccessibilityAction Action
        ) : DittoStepAction;

        internal sealed record Screenshot(DittoScreenshot Value) : DittoStepAction;

        internal sealed record Video(DittoVideo Value) : DittoStepAction;
    }

    internal sealed record DittoAccessibilityTarget(SemanticRole Role, string Name);

    internal sealed record DittoAccessibilityAssertion(
        DittoAccessibilityTarget Target,
        SemanticRole Role,
        string Name,
        bool? Selected = null,
        bool? Disabled = null,
        bool? CurrentPage = null,
        DittoAccessibilityTarget? Parent = null
    );

    internal abstract record DittoInputTarget
    {
        internal sealed record Object(string Id) : DittoInputTarget;

        internal sealed record Coordinates(double X, double Y) : DittoInputTarget;
    }

    internal abstract record DittoWait
    {
        internal sealed record Frames(uint Count) : DittoWait;

        internal sealed record Object(DittoObjectCondition Condition) : DittoWait;
    }

    internal sealed record DittoObjectCondition(string Object, DittoObjectState State);

    internal sealed record DittoScreenshot(string Name, DittoComparison Comparison);

    internal sealed record DittoComparison(
        string Threshold,
        bool AntiAlias,
        string MaxChangedPercent
    );

    internal abstract record DittoVideo
    {
        internal sealed record Start(string Name, DittoMotion Motion, ulong MaxDurationMs)
            : DittoVideo;

        internal sealed record Stop : DittoVideo;
    }
}
