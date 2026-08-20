#nullable enable

using System;
using MessagePack;
using MessagePack.Formatters;

namespace Masonry.Integration
{
    /// <summary>Fixture-only custom failure code.</summary>
    public enum IntegrationFixtureError
    {
        Rejected,
    }

    /// <summary>Formats fixture custom failures for the public extension boundary.</summary>
    public sealed class IntegrationFixtureErrorFormatter
        : IMessagePackFormatter<IntegrationFixtureError>
    {
        public void Serialize(
            ref MessagePackWriter writer,
            IntegrationFixtureError value,
            MessagePackSerializerOptions options
        ) => writer.Write(value.ToString());

        public IntegrationFixtureError Deserialize(
            ref MessagePackReader reader,
            MessagePackSerializerOptions options
        ) => Enum.Parse<IntegrationFixtureError>(reader.ReadString() ?? string.Empty);
    }
}
