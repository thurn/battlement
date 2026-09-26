#nullable enable

namespace Battlement
{
    public abstract record DisplayCommand
    {
        public sealed record Preview(DisplayConfiguration Configuration) : DisplayCommand;

        public sealed record Confirm(CommandId PreviewId) : DisplayCommand;

        public sealed record Cancel(CommandId PreviewId) : DisplayCommand;
    }

    public abstract partial record CommandBody
    {
        public sealed record ApplicationDisplay(DisplayCommand Value) : CommandBody;
    }

    public enum DisplayPreviewState : byte
    {
        Applying,
        Confirmable,
        Reverting,
    }

    /// <summary>The host owns both the transaction identity and its real-time deadline.</summary>
    public sealed record DisplayPreview(
        CommandId RequestId,
        DisplayPreviewState State,
        uint RemainingSeconds
    );
}
