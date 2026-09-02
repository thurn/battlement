#nullable enable

using Newtonsoft.Json;

namespace Battlement
{
    /// <summary>The latest application focus and pause observations.</summary>
    public sealed record ApplicationState(
        [property: JsonProperty(Required = Required.Always)] bool Focused = true,
        [property: JsonProperty(Required = Required.Always)] bool Paused = false
    );

    public abstract partial record CommandBody
    {
        /// <summary>Requests the platform's external handler for an absolute URL.</summary>
        public sealed record ApplicationOpenUrl(string Url) : CommandBody;
    }
}
