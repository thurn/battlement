#nullable enable

using System;
using System.Collections.Concurrent;
using System.Linq;
using System.Reflection;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Battlement
{
    internal static class BattlementUnionRecordFactory
    {
        private static readonly ConcurrentDictionary<Type, RecordMetadata> Metadata = new();

        internal static object CreateValue(
            Type target,
            JToken payload,
            JsonSerializer serializer,
            bool directPayload
        )
        {
            RecordMetadata metadata = Metadata.GetOrAdd(target, CreateMetadata);
            ConstructorInfo? constructor = metadata.Constructor;
            ParameterInfo[] parameters = metadata.Parameters;
            if (parameters.Length == 1)
            {
                JToken argument = payload;
                if (payload is JObject objectValue)
                {
                    JToken? propertyValue = objectValue[metadata.ParameterPropertyName];
                    if (propertyValue is null && !directPayload)
                    {
                        return DeserializeObject(target, payload, serializer);
                    }

                    argument = propertyValue ?? payload;
                    if (directPayload && argument == payload)
                    {
                        argument = payload;
                    }
                }

                object converted =
                    argument.ToObject(parameters[0].ParameterType, serializer)
                    ?? throw new JsonSerializationException(
                        $"Variant payload '{target.Name}' was null."
                    );
                return constructor!.Invoke(new[] { converted });
            }

            return DeserializeObject(target, payload, serializer);
        }

        internal static object CreateValue(
            Type target,
            JsonReader reader,
            JsonSerializer serializer,
            bool directPayload
        )
        {
            RecordMetadata metadata = Metadata.GetOrAdd(target, CreateMetadata);
            if (
                metadata.Parameters.Length == 1
                && (directPayload || reader.TokenType != JsonToken.StartObject)
            )
            {
                object converted =
                    serializer.Deserialize(reader, metadata.Parameters[0].ParameterType)
                    ?? throw new JsonSerializationException(
                        $"Variant payload '{target.Name}' was null."
                    );
                return metadata.Constructor!.Invoke(new[] { converted });
            }

            return DeserializeObject(target, reader, serializer);
        }

        internal static object DeserializeObject(
            Type target,
            JToken payload,
            JsonSerializer serializer
        )
        {
            try
            {
                BattlementUnionTypeGuard.Add(target);
                return payload.ToObject(target, serializer)
                    ?? throw new JsonSerializationException(
                        $"Variant payload '{target.Name}' was null."
                    );
            }
            catch (JsonSerializationException exception)
            {
                throw new JsonSerializationException(
                    $"Variant payload '{target.FullName}' could not be decoded: "
                        + exception.Message,
                    exception
                );
            }
            finally
            {
                BattlementUnionTypeGuard.Remove(target);
            }
        }

        private static object DeserializeObject(
            Type target,
            JsonReader reader,
            JsonSerializer serializer
        )
        {
            try
            {
                BattlementUnionTypeGuard.Add(target);
                return serializer.Deserialize(reader, target)
                    ?? throw new JsonSerializationException(
                        $"Variant payload '{target.Name}' was null."
                    );
            }
            catch (JsonSerializationException exception)
            {
                throw new JsonSerializationException(
                    $"Variant payload '{target.FullName}' could not be decoded: "
                        + exception.Message,
                    exception
                );
            }
            finally
            {
                BattlementUnionTypeGuard.Remove(target);
            }
        }

        private static RecordMetadata CreateMetadata(Type target)
        {
            ConstructorInfo? constructor = target
                .GetConstructors()
                .OrderByDescending(candidate => candidate.GetParameters().Length)
                .FirstOrDefault();
            ParameterInfo[] parameters =
                constructor?.GetParameters() ?? Array.Empty<ParameterInfo>();
            if (parameters.Length != 1)
            {
                return new RecordMetadata(constructor, parameters, string.Empty);
            }

            PropertyInfo? property = target
                .GetProperties(BindingFlags.Instance | BindingFlags.Public)
                .FirstOrDefault(value =>
                    string.Equals(
                        value.Name,
                        parameters[0].Name,
                        StringComparison.OrdinalIgnoreCase
                    )
                );
            string propertyName = property is null
                ? BattlementUnionPayload.ToSnakeCase(parameters[0].Name ?? string.Empty)
                : BattlementUnionPayload.GetWirePropertyName(property);
            return new RecordMetadata(constructor, parameters, propertyName);
        }

        private sealed class RecordMetadata
        {
            public RecordMetadata(
                ConstructorInfo? constructor,
                ParameterInfo[] parameters,
                string parameterPropertyName
            )
            {
                Constructor = constructor;
                Parameters = parameters;
                ParameterPropertyName = parameterPropertyName;
            }

            public ConstructorInfo? Constructor { get; }

            public ParameterInfo[] Parameters { get; }

            public string ParameterPropertyName { get; }
        }
    }
}
