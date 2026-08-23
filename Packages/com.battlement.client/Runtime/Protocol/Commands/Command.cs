#nullable enable

using Newtonsoft.Json;

namespace Battlement
{
    /// <summary>Shared identity and scheduling behavior for every command.</summary>
    public interface ICommand
    {
        /// <summary>Gets the command and operation identity.</summary>
        CommandId Id { get; }

        /// <summary>Gets whether later groups wait for this command to finish.</summary>
        bool IsBlocking { get; }
    }

    /// <summary>A game-owned command identified by an explicitly registered type.</summary>
    public interface ICustomCommand : ICommand
    {
        /// <summary>Gets the namespaced game command discriminator.</summary>
        string Type { get; }
    }

    /// <summary>A fully typed Battlement core command.</summary>
    /// <param name="Id">Identifier for the command and any operation it starts.</param>
    /// <param name="Body">Exact core command type, conflict behavior, and data.</param>
    /// <param name="IsBlocking">Whether later groups wait for this command to finish.</param>
    public sealed record Command(
        [property: JsonProperty("command_id")] CommandId Id,
        CommandBody Body,
        [property: JsonProperty("blocking")] bool IsBlocking = true
    ) : ICommand
    {
        /// <summary>Returns a copy marked as nonblocking.</summary>
        public Command Nonblocking() => this with { IsBlocking = false };
    }

    /// <summary>A custom game command using Battlement's shared command format.</summary>
    /// <typeparam name="TPayload">Game-owned command payload type.</typeparam>
    /// <param name="Id">Session-unique command and operation identity.</param>
    /// <param name="Type">Game-owned namespaced command discriminator.</param>
    /// <param name="Payload">Game-specific payload.</param>
    /// <param name="IsBlocking">Whether later groups wait for the operation.</param>
    public sealed record CustomCommand<TPayload>(
        [property: JsonProperty("command_id")] CommandId Id,
        [property: JsonProperty("command_type")] string Type,
        TPayload Payload,
        [property: JsonProperty("blocking")] bool IsBlocking = true
    ) : ICustomCommand
    {
        /// <summary>Returns a copy marked as nonblocking.</summary>
        public CustomCommand<TPayload> Nonblocking() => this with { IsBlocking = false };
    }

    /// <summary>A command body that participates in property conflict handling.</summary>
    public interface IPropertyCommandBody
    {
        /// <summary>Gets how an operation already controlling the property is handled.</summary>
        ConflictPolicy OnConflict { get; }
    }

    /// <summary>The exact union of built-in Battlement command bodies.</summary>
    public abstract partial record CommandBody
    {
        private CommandBody() { }
    }
}
