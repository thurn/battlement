#nullable enable

using System;
using System.Linq;
using System.Reflection;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Battlement
{
    internal static class BattlementUnionRecordFactory
    {
        internal static object CreateValue(
            Type target,
            JToken payload,
            JsonSerializer serializer,
            bool directPayload
        )
        {
            ConstructorInfo? constructor = target
                .GetConstructors()
                .OrderByDescending(candidate => candidate.GetParameters().Length)
                .FirstOrDefault();
            ParameterInfo[] parameters =
                constructor?.GetParameters() ?? Array.Empty<ParameterInfo>();
            if (parameters.Length == 1)
            {
                JToken argument = payload;
                if (payload is JObject objectValue)
                {
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
                    JToken? propertyValue = objectValue[propertyName];
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
    }
}
