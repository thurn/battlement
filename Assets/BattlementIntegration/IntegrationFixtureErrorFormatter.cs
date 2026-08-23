#nullable enable

using System;
using Newtonsoft.Json;

namespace Battlement.Integration
{
    /// <summary>Fixture-only custom failure code.</summary>
    public enum IntegrationFixtureError
    {
        Rejected,
    }

    /// <summary>Formats fixture custom failures for the public extension boundary.</summary>
    public sealed class IntegrationFixtureErrorFormatter : JsonConverter<IntegrationFixtureError>
    {
        public override void WriteJson(
            JsonWriter writer,
            IntegrationFixtureError value,
            JsonSerializer serializer
        ) => writer.WriteValue(value.ToString());

        public override IntegrationFixtureError ReadJson(
            JsonReader reader,
            Type objectType,
            IntegrationFixtureError existingValue,
            bool hasExistingValue,
            JsonSerializer serializer
        ) => Enum.Parse<IntegrationFixtureError>(reader.Value?.ToString() ?? string.Empty);
    }
}
