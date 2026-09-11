#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using Newtonsoft.Json;
using Newtonsoft.Json.Serialization;

namespace Battlement
{
    internal sealed class UiStyleJsonConverter : JsonConverter
    {
        private static readonly ConstructorInfo Constructor = typeof(UiStyle)
            .GetConstructors()
            .Single(value => value.GetParameters().Length > 0);
        private static readonly ParameterInfo[] Parameters = Constructor.GetParameters();
        private static readonly object?[] Defaults = Parameters.Select(DefaultValue).ToArray();
        private static readonly IReadOnlyDictionary<string, int> ParameterIndices = Parameters
            .Select(
                (parameter, index) =>
                    new KeyValuePair<string, int>(
                        new SnakeCaseNamingStrategy().GetPropertyName(parameter.Name!, false),
                        index
                    )
            )
            .ToDictionary(pair => pair.Key, pair => pair.Value, StringComparer.Ordinal);

        public override bool CanConvert(Type objectType) => objectType == typeof(UiStyle);

        public override object ReadJson(
            JsonReader reader,
            Type objectType,
            object? existingValue,
            JsonSerializer serializer
        )
        {
            if (reader.TokenType != JsonToken.StartObject)
            {
                throw new JsonSerializationException("A UI style must be a JSON object.");
            }

            object?[] arguments = (object?[])Defaults.Clone();
            while (reader.Read() && reader.TokenType != JsonToken.EndObject)
            {
                if (reader.TokenType != JsonToken.PropertyName || reader.Value is not string name)
                {
                    throw new JsonSerializationException(
                        "A UI style property name must be a string."
                    );
                }

                if (!reader.Read())
                {
                    throw new JsonSerializationException(
                        $"UI style property '{name}' has no value."
                    );
                }

                if (!ParameterIndices.TryGetValue(name, out int index))
                {
                    reader.Skip();
                    continue;
                }

                arguments[index] = serializer.Deserialize(reader, Parameters[index].ParameterType);
            }

            if (reader.TokenType != JsonToken.EndObject)
            {
                throw new JsonSerializationException("A UI style object was not terminated.");
            }

            return Constructor.Invoke(arguments);
        }

        public override void WriteJson(
            JsonWriter writer,
            object? value,
            JsonSerializer serializer
        ) => throw new NotSupportedException();

        public override bool CanWrite => false;

        private static object? DefaultValue(ParameterInfo parameter)
        {
            if (parameter.HasDefaultValue && parameter.DefaultValue is not Missing)
            {
                object? declared = parameter.DefaultValue;
                if (declared is not null || !parameter.ParameterType.IsValueType)
                {
                    return declared;
                }
            }

            return parameter.ParameterType.IsValueType
                ? Activator.CreateInstance(parameter.ParameterType)
                : null;
        }
    }
}
