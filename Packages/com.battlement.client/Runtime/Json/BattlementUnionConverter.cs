#nullable enable

using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Battlement
{
    internal sealed class BattlementUnionConverter : JsonConverter
    {
        private static readonly ConcurrentDictionary<Type, bool> ConvertibleTypes = new();

        public override bool CanConvert(Type objectType)
        {
            if (BattlementUnionTypeGuard.IsDisabled(objectType))
            {
                return false;
            }

            return ConvertibleTypes.GetOrAdd(objectType, ComputeCanConvert);
        }

        private static bool ComputeCanConvert(Type objectType)
        {
            if (
                objectType == typeof(ICommand)
                || objectType == typeof(CommandBody)
                || objectType == typeof(ActionBody)
                || objectType == typeof(DiagnosticsCommand)
                || BattlementUnionCaseCatalog.Cases.ContainsKey(objectType)
                || IsGenericUnion(objectType)
            )
            {
                return true;
            }

            if (objectType != typeof(Command) && typeof(ICommand).IsAssignableFrom(objectType))
            {
                return true;
            }

            if (
                BattlementUnionCaseCatalog.Cases.Values.Any(values =>
                    values.Values.Contains(objectType)
                )
            )
            {
                return true;
            }

            for (
                Type? baseType = objectType.BaseType;
                baseType is not null;
                baseType = baseType.BaseType
            )
            {
                if (IsGenericUnion(baseType))
                {
                    return true;
                }
            }

            return false;
        }

        public override object ReadJson(
            JsonReader reader,
            Type objectType,
            object? existingValue,
            JsonSerializer serializer
        )
        {
            if (objectType == typeof(ICommand))
            {
                throw new JsonSerializationException(
                    "Custom command decoding requires a registered command payload handler."
                );
            }

            if (reader.TokenType == JsonToken.Null)
            {
                if (objectType.IsValueType)
                {
                    throw new JsonSerializationException(
                        $"Protocol union {objectType.Name} cannot be null."
                    );
                }

                return null!;
            }

            if (reader.TokenType == JsonToken.String)
            {
                string unitTag =
                    (string?)reader.Value
                    ?? throw new JsonSerializationException("A union tag cannot be null.");
                Type unitTarget = ResolveCase(objectType, unitTag);
                if (!BattlementUnionPayload.IsUnit(unitTarget))
                {
                    throw new JsonSerializationException(
                        $"Variant '{unitTag}' requires a payload."
                    );
                }

                return Activator.CreateInstance(unitTarget)!;
            }

            if (reader.TokenType != JsonToken.StartObject || !reader.Read())
            {
                throw new JsonSerializationException(
                    $"Externally tagged union {objectType.Name} requires one JSON property."
                );
            }

            if (reader.TokenType != JsonToken.PropertyName || reader.Value is not string tag)
            {
                throw new JsonSerializationException(
                    $"Externally tagged union {objectType.Name} requires one JSON property."
                );
            }

            Type target = ResolveCase(objectType, tag);
            if (!reader.Read())
            {
                throw new JsonSerializationException($"Variant '{tag}' has no payload.");
            }

            object value;
            if (BattlementUnionPayload.IsUnit(target))
            {
                if (reader.TokenType != JsonToken.Null)
                {
                    throw new JsonSerializationException($"Unit variant '{tag}' has a payload.");
                }

                value = Activator.CreateInstance(target)!;
            }
            else if (BattlementUnionPayload.IsPropertyCommand(target))
            {
                JToken payload = BattlementUnionPayload.FlattenPropertyPayload(JToken.Load(reader));
                value = BattlementUnionRecordFactory.CreateValue(
                    target,
                    payload,
                    serializer,
                    directPayload: false
                );
            }
            else
            {
                bool directPayload =
                    IsWrapperUnion(objectType)
                    || BattlementUnionPayload.IsScalarUnion(objectType)
                    || BattlementUnionPayload.IsDirectPayload(target);
                value = TryReadNumericValue(target, reader, out object? numericValue)
                    ? numericValue
                    : BattlementUnionRecordFactory.CreateValue(
                        target,
                        reader,
                        serializer,
                        directPayload
                    );
            }

            if (!reader.Read() || reader.TokenType != JsonToken.EndObject)
            {
                throw new JsonSerializationException(
                    $"Externally tagged union {objectType.Name} requires one JSON property."
                );
            }

            return value;
        }

        private static bool TryReadNumericValue(Type target, JsonReader reader, out object value)
        {
            if (reader.TokenType != JsonToken.Integer && reader.TokenType != JsonToken.Float)
            {
                value = null!;
                return false;
            }

            object raw = reader.Value!;
            if (target == typeof(UiLength.Px))
            {
                value = new UiLength.Px(Convert.ToSingle(raw, CultureInfo.InvariantCulture));
                return true;
            }

            if (target == typeof(UiLength.Percent))
            {
                value = new UiLength.Percent(Convert.ToSingle(raw, CultureInfo.InvariantCulture));
                return true;
            }

            if (target == typeof(UiLengthOrAuto.Px))
            {
                value = new UiLengthOrAuto.Px(Convert.ToSingle(raw, CultureInfo.InvariantCulture));
                return true;
            }

            if (target == typeof(UiLengthOrAuto.Percent))
            {
                value = new UiLengthOrAuto.Percent(
                    Convert.ToSingle(raw, CultureInfo.InvariantCulture)
                );
                return true;
            }

            if (target == typeof(MotionValue.Scalar))
            {
                value = new MotionValue.Scalar(Convert.ToDouble(raw, CultureInfo.InvariantCulture));
                return true;
            }

            if (target == typeof(MotionValue.Angle))
            {
                value = new MotionValue.Angle(Convert.ToDouble(raw, CultureInfo.InvariantCulture));
                return true;
            }

            value = null!;
            return false;
        }

        public override void WriteJson(JsonWriter writer, object? value, JsonSerializer serializer)
        {
            if (value is null)
            {
                throw new JsonSerializationException("A protocol union cannot be null.");
            }

            Type baseType = FindBaseType(value.GetType());
            if (baseType == typeof(ICommand))
            {
                if (value is Command)
                {
                    BattlementUnionPayload.WriteTagged(writer, "Core", value, serializer);
                    return;
                }

                if (value is ICustomCommand)
                {
                    BattlementUnionPayload.WriteTagged(writer, "Custom", value, serializer);
                    return;
                }
            }

            string tag = FindTag(baseType, value.GetType());
            if (BattlementUnionPayload.IsUnit(value.GetType()))
            {
                writer.WriteValue(tag);
                return;
            }

            JToken payload = BattlementUnionPayload.SerializeRecordPayload(value, serializer);
            if (BattlementUnionPayload.IsPropertyCommand(value.GetType()))
            {
                payload = BattlementUnionPayload.NestPropertyPayload(payload);
            }
            else if (BattlementUnionPayload.IsScalarUnion(baseType))
            {
                payload = BattlementUnionPayload.GetSinglePayload(payload, value, serializer);
            }
            else if (IsWrapperUnion(baseType))
            {
                payload = BattlementUnionPayload.GetSinglePayload(payload, value, serializer);
            }
            else if (BattlementUnionPayload.IsDirectPayload(value.GetType()))
            {
                payload = BattlementUnionPayload.GetSinglePayload(payload, value, serializer);
            }

            BattlementUnionPayload.WriteTagged(writer, tag, payload, serializer);
        }

        private static bool IsGenericUnion(Type type) =>
            type.IsGenericType
            && (
                type.GetGenericTypeDefinition() == typeof(ResponseMessage<>)
                || type.GetGenericTypeDefinition() == typeof(ClientMessage<,>)
            );

        private static bool IsWrapperUnion(Type type) =>
            type.IsGenericType
            && (
                type.GetGenericTypeDefinition() == typeof(ResponseMessage<>)
                || type.GetGenericTypeDefinition() == typeof(ClientMessage<,>)
            );

        private static Type FindBaseType(Type runtimeType)
        {
            if (typeof(ICommand).IsAssignableFrom(runtimeType))
            {
                return typeof(ICommand);
            }

            foreach (Type baseType in BattlementUnionCaseCatalog.Cases.Keys)
            {
                if (BattlementUnionCaseCatalog.Cases[baseType].Values.Contains(runtimeType))
                {
                    return baseType;
                }
            }

            Type? genericBase = runtimeType.BaseType;
            while (genericBase is not null)
            {
                if (IsGenericUnion(genericBase))
                {
                    return genericBase;
                }

                genericBase = genericBase.BaseType;
            }

            throw new JsonSerializationException(
                $"Unknown protocol union type {runtimeType.Name}."
            );
        }

        private static Type ResolveCase(Type baseType, string tag)
        {
            if (IsGenericUnion(baseType))
            {
                Type nestedType =
                    baseType.GetNestedType(tag + "Message")
                    ?? throw new JsonSerializationException(
                        $"Unknown {baseType.Name} variant '{tag}'."
                    );
                return nestedType.ContainsGenericParameters
                    ? nestedType.MakeGenericType(baseType.GetGenericArguments())
                    : nestedType;
            }

            if (
                BattlementUnionCaseCatalog.Cases.TryGetValue(
                    baseType,
                    out IReadOnlyDictionary<string, Type>? cases
                ) && cases.TryGetValue(tag, out Type? target)
            )
            {
                return target;
            }

            throw new JsonSerializationException($"Unknown {baseType.Name} variant '{tag}'.");
        }

        private static string FindTag(Type baseType, Type runtimeType)
        {
            if (IsGenericUnion(baseType))
            {
                string name = runtimeType.Name.Split('`')[0];
                return name.EndsWith("Message", StringComparison.Ordinal)
                    ? name[..^"Message".Length]
                    : name;
            }

            if (
                BattlementUnionCaseCatalog.Cases.TryGetValue(
                    baseType,
                    out IReadOnlyDictionary<string, Type>? cases
                )
            )
            {
                KeyValuePair<string, Type> match = cases.FirstOrDefault(pair =>
                    pair.Value == runtimeType
                );
                if (match.Value is not null)
                {
                    return match.Key;
                }
            }

            throw new JsonSerializationException(
                $"Unknown {baseType.Name} variant type {runtimeType.Name}."
            );
        }
    }
}
