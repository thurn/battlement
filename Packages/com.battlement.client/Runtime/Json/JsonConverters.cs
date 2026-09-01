#nullable enable

using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Reflection;
using System.Text;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using Newtonsoft.Json.Serialization;

namespace Battlement
{
    internal sealed class ProtocolByteConverter : JsonConverter
    {
        public override bool CanConvert(Type objectType) => objectType == typeof(byte);

        public override object ReadJson(
            JsonReader reader,
            Type objectType,
            object? existingValue,
            JsonSerializer serializer
        )
        {
            if (reader.TokenType != JsonToken.Integer)
            {
                throw new JsonSerializationException("A byte value must be a JSON integer.");
            }
            try
            {
                return Convert.ToByte(reader.Value, CultureInfo.InvariantCulture);
            }
            catch (Exception exception) when (exception is OverflowException or FormatException)
            {
                throw new JsonSerializationException(
                    "A byte value is outside [0, 255].",
                    exception
                );
            }
        }

        public override void WriteJson(
            JsonWriter writer,
            object? value,
            JsonSerializer serializer
        ) =>
            writer.WriteValue(
                (byte)(value ?? throw new JsonSerializationException("A byte value is required."))
            );
    }

    internal sealed class PropJsonConverter : JsonConverter
    {
        public override bool CanConvert(Type objectType) =>
            objectType.IsGenericType && objectType.GetGenericTypeDefinition() == typeof(Prop<>);

        public override object ReadJson(
            JsonReader reader,
            Type objectType,
            object? existingValue,
            JsonSerializer serializer
        )
        {
            JToken token = JToken.Load(reader);
            Type valueType = objectType.GetGenericArguments()[0];
            if (token.Type == JTokenType.Null)
                return Create(nameof(ResetValue), valueType, null);
            if (valueType == typeof(bool) && token.Type != JTokenType.Boolean)
                throw new JsonSerializationException("A Boolean property must be true or false.");
            object value =
                token.ToObject(valueType, serializer)
                ?? throw new JsonSerializationException("A set property value cannot be null.");
            return Create(nameof(SetValue), valueType, value);
        }

        public override void WriteJson(JsonWriter writer, object? value, JsonSerializer serializer)
        {
            if (value is null)
                throw new JsonSerializationException("A property operation cannot be null.");
            Type type = value.GetType();
            PropState state = (PropState)
                type.GetProperty(nameof(Prop<int>.State))!.GetValue(value)!;
            if (state == PropState.Set)
            {
                serializer.Serialize(
                    writer,
                    type.GetProperty(nameof(Prop<int>.Value))!.GetValue(value)
                );
                return;
            }
            if (state == PropState.Reset)
            {
                writer.WriteNull();
                return;
            }
            throw new JsonSerializationException("An unset property must be omitted.");
        }

        private static object Create(string method, Type valueType, object? value) =>
            typeof(PropJsonConverter)
                .GetMethod(method, BindingFlags.NonPublic | BindingFlags.Static)!
                .MakeGenericMethod(valueType)
                .Invoke(null, value is null ? null : new[] { value })!;

        private static Prop<T> SetValue<T>(object value) => Prop<T>.Set((T)value);

        private static Prop<T> ResetValue<T>() => Prop<T>.Reset();
    }

    internal sealed class UiStyleValueConverter : JsonConverter
    {
        public override bool CanConvert(Type objectType) =>
            objectType.IsGenericType
            && objectType.GetGenericTypeDefinition() == typeof(UiStyleValue<>);

        public override object ReadJson(
            JsonReader reader,
            Type objectType,
            object? existingValue,
            JsonSerializer serializer
        )
        {
            JToken token = JToken.Load(reader);
            Type valueType = objectType.GetGenericArguments()[0];
            if (
                token is JObject keywordObject
                && keywordObject.Count == 1
                && keywordObject.TryGetValue("Keyword", out JToken? keywordToken)
            )
            {
                object? defaultValue = valueType.IsValueType
                    ? Activator.CreateInstance(valueType)
                    : null;
                UiInlineKeyword keyword = keywordToken.ToObject<UiInlineKeyword>(serializer);
                return Activator.CreateInstance(objectType, defaultValue, keyword)!;
            }

            object value =
                token.ToObject(valueType, serializer)
                ?? throw new JsonSerializationException("A concrete UI style value was null.");
            return Activator.CreateInstance(objectType, value, null)!;
        }

        public override void WriteJson(JsonWriter writer, object? value, JsonSerializer serializer)
        {
            if (value is null)
                throw new JsonSerializationException("A UI style value cannot be null.");
            Type type = value.GetType();
            object? keyword = type.GetProperty("Keyword")!.GetValue(value);
            if (keyword is not null)
            {
                writer.WriteStartObject();
                writer.WritePropertyName("Keyword");
                serializer.Serialize(writer, keyword);
                writer.WriteEndObject();
                return;
            }
            serializer.Serialize(writer, type.GetProperty("Value")!.GetValue(value));
        }
    }

    internal sealed class ProtocolColorConverter : JsonConverter
    {
        public override bool CanConvert(Type objectType)
        {
            Type scalarType = Nullable.GetUnderlyingType(objectType) ?? objectType;
            return scalarType == typeof(Color) || scalarType == typeof(RgbColor);
        }

        public override object ReadJson(
            JsonReader reader,
            Type objectType,
            object? existingValue,
            JsonSerializer serializer
        )
        {
            if (reader.TokenType == JsonToken.Null)
            {
                if (Nullable.GetUnderlyingType(objectType) is not null)
                {
                    return null!;
                }

                throw new JsonSerializationException($"A {objectType.Name} cannot be null.");
            }

            JObject value = JObject.Load(reader);
            double red = ReadComponent(value, "r");
            double green = ReadComponent(value, "g");
            double blue = ReadComponent(value, "b");
            Type scalarType = Nullable.GetUnderlyingType(objectType) ?? objectType;
            if (scalarType == typeof(RgbColor))
            {
                return new RgbColor(red, green, blue);
            }

            return new Color(red, green, blue, ReadComponent(value, "a", 1));
        }

        public override void WriteJson(JsonWriter writer, object? value, JsonSerializer serializer)
        {
            if (value is null)
            {
                writer.WriteNull();
                return;
            }

            writer.WriteStartObject();
            if (value is RgbColor rgb)
            {
                writer.WritePropertyName("r");
                writer.WriteValue(rgb.Red);
                writer.WritePropertyName("g");
                writer.WriteValue(rgb.Green);
                writer.WritePropertyName("b");
                writer.WriteValue(rgb.Blue);
            }
            else
            {
                Color color = (Color)value;
                writer.WritePropertyName("r");
                writer.WriteValue(color.Red);
                writer.WritePropertyName("g");
                writer.WriteValue(color.Green);
                writer.WritePropertyName("b");
                writer.WriteValue(color.Blue);
                if (!color.Alpha.Equals(1d))
                {
                    writer.WritePropertyName("a");
                    writer.WriteValue(color.Alpha);
                }
            }

            writer.WriteEndObject();
        }

        private static double ReadComponent(JObject value, string name, double? defaultValue = null)
        {
            if (!value.TryGetValue(name, out JToken? component))
            {
                return defaultValue
                    ?? throw new JsonSerializationException(
                        $"Color component '{name}' is required."
                    );
            }

            if (component.Type == JTokenType.Null)
            {
                throw new JsonSerializationException($"Color component '{name}' cannot be null.");
            }

            return component.ToObject<double>();
        }
    }

    internal sealed class ProtocolScalarConverter : JsonConverter
    {
        private static readonly Type[] IdTypes =
        {
            typeof(SessionId),
            typeof(ActionId),
            typeof(BatchId),
            typeof(CommandId),
            typeof(ObjectId),
            typeof(SceneId),
            typeof(GeometryObservationId),
        };

        private static readonly Type[] AddressTypes =
        {
            typeof(SceneAddress),
            typeof(PrefabAddress),
            typeof(ParticleEffectAddress),
            typeof(MaterialAddress),
            typeof(TextureAddress),
            typeof(SpriteAddress),
            typeof(VectorImageAddress),
            typeof(RenderTextureAddress),
            typeof(AudioClipAddress),
            typeof(TextMeshProFontAddress),
            typeof(UiFontAddress),
        };

        public override bool CanConvert(Type objectType)
        {
            Type scalarType = Nullable.GetUnderlyingType(objectType) ?? objectType;
            return scalarType == typeof(TimeSpan)
                || scalarType == typeof(InteractionLayerMask)
                || scalarType == typeof(GeometryGeneration)
                || scalarType == typeof(DisplayId)
                || scalarType == typeof(AnchorName)
                || IdTypes.Contains(scalarType)
                || AddressTypes.Contains(scalarType);
        }

        public override object ReadJson(
            JsonReader reader,
            Type objectType,
            object? existingValue,
            JsonSerializer serializer
        )
        {
            Type scalarType = Nullable.GetUnderlyingType(objectType) ?? objectType;
            if (reader.TokenType == JsonToken.Null)
            {
                if (Nullable.GetUnderlyingType(objectType) is not null)
                {
                    return null!;
                }

                throw new JsonSerializationException($"A {objectType.Name} cannot be null.");
            }

            if (scalarType == typeof(TimeSpan))
            {
                if (reader.TokenType != JsonToken.Integer)
                {
                    throw new JsonSerializationException(
                        "Battlement durations must be integer milliseconds."
                    );
                }

                long milliseconds = Convert.ToInt64(reader.Value, CultureInfo.InvariantCulture);
                if (milliseconds < 0)
                {
                    throw new JsonSerializationException(
                        "Battlement durations must be nonnegative."
                    );
                }

                try
                {
                    return TimeSpan.FromMilliseconds(milliseconds);
                }
                catch (OverflowException exception)
                {
                    throw new JsonSerializationException(
                        "A duration exceeds TimeSpan.MaxValue.",
                        exception
                    );
                }
            }

            if (scalarType == typeof(InteractionLayerMask))
            {
                if (reader.TokenType != JsonToken.Integer)
                    throw new JsonSerializationException(
                        "An interaction layer mask must be a JSON integer."
                    );
                return new InteractionLayerMask(
                    Convert.ToUInt32(reader.Value, CultureInfo.InvariantCulture)
                );
            }

            if (scalarType == typeof(GeometryGeneration) || scalarType == typeof(DisplayId))
            {
                if (reader.TokenType != JsonToken.Integer)
                    throw new JsonSerializationException("A geometry scalar must be an integer.");
                ulong number = Convert.ToUInt64(reader.Value, CultureInfo.InvariantCulture);
                if (scalarType == typeof(GeometryGeneration))
                {
                    if (number == 0)
                        throw new JsonSerializationException(
                            "A geometry generation must be nonzero."
                        );
                    return new GeometryGeneration(number);
                }
                if (number > uint.MaxValue)
                    throw new JsonSerializationException("A display ID exceeds UInt32.MaxValue.");
                return new DisplayId((uint)number);
            }

            if (scalarType == typeof(AnchorName))
            {
                if (
                    reader.TokenType != JsonToken.String
                    || reader.Value is not string name
                    || name.Length == 0
                )
                    throw new JsonSerializationException(
                        "A geometry anchor name must be nonempty."
                    );
                return new AnchorName(name);
            }

            if (reader.TokenType != JsonToken.String || reader.Value is not string text)
            {
                throw new JsonSerializationException($"A {objectType.Name} must be a JSON string.");
            }

            if (IdTypes.Contains(scalarType))
            {
                if (!Guid.TryParse(text, out Guid id) || id == Guid.Empty)
                {
                    throw new JsonSerializationException(
                        "The all-zero or invalid UUID is not valid."
                    );
                }

                return Activator.CreateInstance(scalarType, id)!;
            }

            return Activator.CreateInstance(scalarType, text)!;
        }

        public override void WriteJson(JsonWriter writer, object? value, JsonSerializer serializer)
        {
            if (value is null)
            {
                throw new JsonSerializationException("A required protocol scalar cannot be null.");
            }

            if (value is TimeSpan duration)
            {
                if (duration < TimeSpan.Zero || duration.Ticks % TimeSpan.TicksPerMillisecond != 0)
                {
                    throw new JsonSerializationException(
                        "Battlement durations must be nonnegative whole milliseconds."
                    );
                }

                writer.WriteValue(duration.Ticks / TimeSpan.TicksPerMillisecond);
                return;
            }

            if (value is InteractionLayerMask layerMask)
            {
                writer.WriteValue(layerMask.Value);
                return;
            }

            if (value is GeometryGeneration generation)
            {
                if (generation.Value == 0)
                    throw new JsonSerializationException("A geometry generation must be nonzero.");
                writer.WriteValue(generation.Value);
                return;
            }
            if (value is DisplayId displayId)
            {
                writer.WriteValue(displayId.Value);
                return;
            }
            if (value is AnchorName anchorName)
            {
                if (string.IsNullOrEmpty(anchorName.Value))
                    throw new JsonSerializationException(
                        "A geometry anchor name must be nonempty."
                    );
                writer.WriteValue(anchorName.Value);
                return;
            }

            PropertyInfo property =
                value.GetType().GetProperty("Value")
                ?? throw new JsonSerializationException("A protocol scalar has no value.");
            string text =
                property.GetValue(value)?.ToString()
                ?? throw new JsonSerializationException("A protocol scalar value cannot be null.");
            if (IdTypes.Contains(value.GetType()) && Guid.Parse(text) == Guid.Empty)
            {
                throw new JsonSerializationException("The all-zero UUID is not valid.");
            }

            writer.WriteValue(text);
        }
    }

    internal sealed class BattlementUnionConverter : JsonConverter
    {
        [ThreadStatic]
        private static HashSet<Type>? disabledTypes;

        private static readonly IReadOnlyDictionary<Type, IReadOnlyDictionary<string, Type>> Cases =
            CreateCases();

        public override bool CanConvert(Type objectType)
        {
            if (disabledTypes?.Contains(objectType) == true)
            {
                return false;
            }

            if (
                objectType == typeof(ICommand)
                || objectType == typeof(CommandBody)
                || objectType == typeof(ActionBody)
                || objectType == typeof(DiagnosticsCommand)
                || Cases.ContainsKey(objectType)
                || IsGenericUnion(objectType)
            )
            {
                return true;
            }

            if (objectType != typeof(Command) && typeof(ICommand).IsAssignableFrom(objectType))
            {
                return true;
            }

            if (Cases.Values.Any(values => values.Values.Contains(objectType)))
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

            JToken token = JToken.Load(reader);
            if (token.Type == JTokenType.Null)
            {
                if (objectType.IsValueType)
                {
                    throw new JsonSerializationException(
                        $"Protocol union {objectType.Name} cannot be null."
                    );
                }

                return null!;
            }

            (string tag, JToken payload) = ReadTag(token, objectType);
            Type target = ResolveCase(objectType, tag);
            if (target is null)
            {
                throw new JsonSerializationException($"Unknown {objectType.Name} variant '{tag}'.");
            }

            if (IsUnit(target))
            {
                if (payload.Type != JTokenType.Null)
                {
                    throw new JsonSerializationException($"Unit variant '{tag}' has a payload.");
                }

                return Activator.CreateInstance(target)!;
            }

            if (IsPropertyCommand(target))
            {
                payload = FlattenPropertyPayload(payload);
            }

            bool directPayload =
                IsWrapperUnion(objectType) || IsScalarUnion(objectType) || IsDirectPayload(target);
            return CreateValue(target, payload, serializer, directPayload);
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
                    WriteTagged(writer, "Core", value, serializer);
                    return;
                }

                if (value is ICustomCommand)
                {
                    WriteTagged(writer, "Custom", value, serializer);
                    return;
                }
            }

            string tag = FindTag(baseType, value.GetType());
            if (IsUnit(value.GetType()))
            {
                writer.WriteValue(tag);
                return;
            }

            JToken payload = SerializeRecordPayload(value, serializer);
            if (IsPropertyCommand(value.GetType()))
            {
                payload = NestPropertyPayload(payload);
            }
            else if (IsScalarUnion(baseType))
            {
                payload = GetSinglePayload(payload, value, serializer);
            }
            else if (IsWrapperUnion(baseType))
            {
                payload = GetSinglePayload(payload, value, serializer);
            }
            else if (IsDirectPayload(value.GetType()))
            {
                payload = GetSinglePayload(payload, value, serializer);
            }

            WriteTagged(writer, tag, payload, serializer);
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

            foreach (Type baseType in Cases.Keys)
            {
                if (Cases[baseType].Values.Contains(runtimeType))
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
                Cases.TryGetValue(baseType, out IReadOnlyDictionary<string, Type>? cases)
                && cases.TryGetValue(tag, out Type? target)
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

            if (Cases.TryGetValue(baseType, out IReadOnlyDictionary<string, Type>? cases))
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

        private static (string Tag, JToken Payload) ReadTag(JToken token, Type objectType)
        {
            if (token.Type == JTokenType.String)
            {
                return (
                    token.Value<string>()
                        ?? throw new JsonSerializationException("A union tag cannot be null."),
                    JValue.CreateNull()
                );
            }

            if (token is not JObject objectValue || objectValue.Count != 1)
            {
                throw new JsonSerializationException(
                    $"Externally tagged union {objectType.Name} requires one JSON property."
                );
            }

            JProperty property = objectValue.Properties().Single();
            return (property.Name, property.Value);
        }

        private static object CreateValue(
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
                        ? ToSnakeCase(parameters[0].Name ?? string.Empty)
                        : GetWirePropertyName(property);
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

        private static object DeserializeObject(
            Type target,
            JToken payload,
            JsonSerializer serializer
        )
        {
            try
            {
                (disabledTypes ??= new HashSet<Type>()).Add(target);
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
                disabledTypes!.Remove(target);
            }
        }

        private static JToken FlattenPropertyPayload(JToken payload)
        {
            if (payload is not JObject wrapper || wrapper["payload"] is not JObject nested)
            {
                throw new JsonSerializationException(
                    "Property command variants require a payload object."
                );
            }

            if (wrapper["on_conflict"] is JToken conflict)
            {
                nested["on_conflict"] = conflict;
            }
            return nested;
        }

        private static JToken NestPropertyPayload(JToken payload)
        {
            if (payload is not JObject objectValue)
            {
                throw new JsonSerializationException(
                    "A property command payload must be an object."
                );
            }

            if (objectValue["on_conflict"] is not JToken conflict)
            {
                return new JObject { ["payload"] = objectValue };
            }

            objectValue.Remove("on_conflict");
            return new JObject { ["on_conflict"] = conflict, ["payload"] = objectValue };
        }

        private static JToken GetSinglePayload(
            JToken payload,
            object value,
            JsonSerializer serializer
        )
        {
            if (payload is not JObject objectValue)
            {
                return payload;
            }

            PropertyInfo[] properties = value
                .GetType()
                .GetProperties(BindingFlags.Instance | BindingFlags.Public);
            if (properties.Length != 1)
            {
                return payload;
            }

            string propertyName = GetWirePropertyName(properties[0]);
            if (objectValue[propertyName] is JToken serialized)
                return serialized;
            object? propertyValue = properties[0].GetValue(value);
            return propertyValue is null
                ? JValue.CreateNull()
                : JToken.FromObject(propertyValue, serializer);
        }

        private static JObject SerializeRecordPayload(object value, JsonSerializer serializer)
        {
            Type type = value.GetType();
            (disabledTypes ??= new HashSet<Type>()).Add(type);
            try
            {
                return JObject.FromObject(value, serializer);
            }
            finally
            {
                disabledTypes!.Remove(type);
            }
        }

        private static string GetWirePropertyName(PropertyInfo property) =>
            property.GetCustomAttribute<JsonPropertyAttribute>()?.PropertyName
            ?? ToSnakeCase(property.Name);

        private static bool IsUnit(Type type) =>
            type.GetProperties(BindingFlags.Instance | BindingFlags.Public).Length == 0;

        private static bool IsPropertyCommand(Type type) =>
            typeof(IPropertyCommandBody).IsAssignableFrom(type);

        private static bool IsDirectPayload(Type type) =>
            type == typeof(CommandBody.VisualElement.Update)
            || type == typeof(CommandBody.GeometryObservation)
            || type == typeof(CommandBody.Diagnostics)
            || type == typeof(CommandBody.Motion.DragControl)
            || type == typeof(ActionBody.GeometryObservations)
            || type == typeof(ActionBody.MotionEvents)
            || type == typeof(CameraTarget.Object)
            || type == typeof(GeometryValue.Element)
            || type == typeof(GeometryValue.Viewport)
            || type == typeof(GeometryValue.WorldPoint)
            || type == typeof(GeometryValue.WorldBounds)
            || type == typeof(GeometryObservationResult.Current)
            || type == typeof(GeometryObservationResult.Unavailable)
            || type == typeof(MotionEasing.CubicBezier)
            || type == typeof(InertiaTarget.NearestMultiple)
            || type == typeof(InertiaTarget.FloorMultiple)
            || type == typeof(InertiaTarget.CeilingMultiple)
            || type == typeof(TransitionGenerator.Spring)
            || type == typeof(MotionExpressionOperation.Power)
            || type == typeof(MotionExpressionOperation.Modulo)
            || type == typeof(MotionValueSource.Time)
            || type == typeof(MotionDragConstraint.Bounds)
            || type == typeof(MotionDragConstraint.Element)
            || type == typeof(MotionValueCommand.Set)
            || type == typeof(MotionValueCommand.Jump)
            || type == typeof(MotionControlTarget.Target)
            || type == typeof(MotionControlTarget.Variant)
            || type == typeof(MotionControlCommand.Set)
            || type == typeof(MotionSelector.Element)
            || type == typeof(MotionSelector.Name)
            || type == typeof(MotionScopeCommand.Stop)
            || type == typeof(OverlayPlacement.Layer);

        private static bool IsScalarUnion(Type baseType) =>
            baseType == typeof(PreparedAsset)
            || baseType == typeof(BackgroundSource)
            || baseType == typeof(UiPointerButton)
            || baseType == typeof(UiFocusDirection)
            || baseType == typeof(UiBackgroundSize)
            || baseType == typeof(UiCursor)
            || baseType == typeof(UiFilterFunction)
            || baseType == typeof(ImageSource)
            || baseType == typeof(IconSource)
            || baseType == typeof(UiValue)
            || baseType == typeof(MotionValue)
            || baseType == typeof(MotionTransform)
            || baseType == typeof(MotionFilter)
            || baseType == typeof(MotionRepeat)
            || baseType == typeof(MotionClockSource)
            || baseType == typeof(MotionDragConstraint)
            || baseType == typeof(LowerLimit)
            || baseType == typeof(UpperLimit)
            || baseType == typeof(InteractionDistance)
            || baseType == typeof(ParentScene)
            || baseType == typeof(ParticleSpawnLocation)
            || baseType == typeof(GridTrack)
            || baseType == typeof(UiEventBody);

        private static string ToSnakeCase(string value) =>
            new SnakeCaseNamingStrategy().GetPropertyName(value, false);

        private static void WriteTagged(
            JsonWriter writer,
            string tag,
            object value,
            JsonSerializer serializer
        )
        {
            writer.WriteStartObject();
            writer.WritePropertyName(tag);
            (disabledTypes ??= new HashSet<Type>()).Add(value.GetType());
            try
            {
                serializer.Serialize(writer, value);
            }
            finally
            {
                disabledTypes!.Remove(value.GetType());
            }
            writer.WriteEndObject();
        }

        private static void WriteTagged(
            JsonWriter writer,
            string tag,
            JToken value,
            JsonSerializer serializer
        )
        {
            writer.WriteStartObject();
            writer.WritePropertyName(tag);
            value.WriteTo(writer);
            writer.WriteEndObject();
        }

        private static IReadOnlyDictionary<Type, IReadOnlyDictionary<string, Type>> CreateCases()
        {
            var cases = new Dictionary<Type, IReadOnlyDictionary<string, Type>>
            {
                [typeof(PreparedAsset)] = Fixed(
                    ("Scene", typeof(PreparedAsset.Scene)),
                    ("Prefab", typeof(PreparedAsset.Prefab)),
                    ("ParticleEffect", typeof(PreparedAsset.ParticleEffect)),
                    ("Material", typeof(PreparedAsset.Material)),
                    ("Texture", typeof(PreparedAsset.Texture)),
                    ("Sprite", typeof(PreparedAsset.Sprite)),
                    ("VectorImage", typeof(PreparedAsset.VectorImage)),
                    ("RenderTexture", typeof(PreparedAsset.RenderTexture)),
                    ("AudioClip", typeof(PreparedAsset.AudioClip)),
                    ("TextMeshProFont", typeof(PreparedAsset.TextMeshProFont)),
                    ("UiFont", typeof(PreparedAsset.UiFont))
                ),
                [typeof(BackgroundSource)] = Fixed(
                    ("Texture", typeof(BackgroundSource.Texture)),
                    ("Sprite", typeof(BackgroundSource.Sprite)),
                    ("VectorImage", typeof(BackgroundSource.VectorImage)),
                    ("RenderTexture", typeof(BackgroundSource.RenderTexture))
                ),
                [typeof(UiBackgroundSize)] = Fixed(
                    ("Auto", typeof(UiBackgroundSize.Auto)),
                    ("Cover", typeof(UiBackgroundSize.Cover)),
                    ("Contain", typeof(UiBackgroundSize.Contain)),
                    ("Axes", typeof(UiBackgroundSize.Axes))
                ),
                [typeof(UiTextAutoSize)] = Fixed(
                    ("None", typeof(UiTextAutoSize.None)),
                    ("BestFit", typeof(UiTextAutoSize.BestFit))
                ),
                [typeof(UiCursor)] = Fixed(
                    ("Default", typeof(UiCursor.Default)),
                    ("Texture", typeof(UiCursor.Texture))
                ),
                [typeof(ImageSource)] = Fixed(
                    ("Texture", typeof(ImageSource.Texture)),
                    ("Sprite", typeof(ImageSource.Sprite)),
                    ("VectorImage", typeof(ImageSource.VectorImage)),
                    ("RenderTexture", typeof(ImageSource.RenderTexture))
                ),
                [typeof(IconSource)] = Fixed(
                    ("Texture", typeof(IconSource.Texture)),
                    ("Sprite", typeof(IconSource.Sprite)),
                    ("VectorImage", typeof(IconSource.VectorImage)),
                    ("RenderTexture", typeof(IconSource.RenderTexture))
                ),
                [typeof(UiLength)] = Fixed(
                    ("Px", typeof(UiLength.Px)),
                    ("Percent", typeof(UiLength.Percent))
                ),
                [typeof(UiLengthOrAuto)] = Fixed(
                    ("Px", typeof(UiLengthOrAuto.Px)),
                    ("Percent", typeof(UiLengthOrAuto.Percent)),
                    ("Auto", typeof(UiLengthOrAuto.Auto))
                ),
                [typeof(UiAspectRatio)] = Fixed(
                    ("Auto", typeof(UiAspectRatio.Auto)),
                    ("Ratio", typeof(UiAspectRatio.Ratio))
                ),
                [typeof(UiFilterFunction)] = Fixed(
                    ("Tint", typeof(UiFilterFunction.Tint)),
                    ("Opacity", typeof(UiFilterFunction.Opacity)),
                    ("Invert", typeof(UiFilterFunction.Invert)),
                    ("Grayscale", typeof(UiFilterFunction.Grayscale)),
                    ("Sepia", typeof(UiFilterFunction.Sepia)),
                    ("Blur", typeof(UiFilterFunction.Blur)),
                    ("Contrast", typeof(UiFilterFunction.Contrast)),
                    ("HueRotate", typeof(UiFilterFunction.HueRotate))
                ),
                [typeof(ParentScene)] = Fixed(
                    ("PrimaryScene", typeof(ParentScene.Primary)),
                    ("Scene", typeof(ParentScene.Specific)),
                    ("Persistent", typeof(ParentScene.Persistent))
                ),
                [typeof(GameObjectKind)] = Fixed(
                    ("UiDocument", typeof(GameObjectKind.UiDocumentState)),
                    ("Empty", typeof(GameObjectKind.Empty)),
                    ("Cube", typeof(GameObjectKind.Cube)),
                    ("Sphere", typeof(GameObjectKind.Sphere)),
                    ("Capsule", typeof(GameObjectKind.Capsule)),
                    ("Cylinder", typeof(GameObjectKind.Cylinder)),
                    ("Plane", typeof(GameObjectKind.Plane)),
                    ("Quad", typeof(GameObjectKind.Quad)),
                    ("Image", typeof(GameObjectKind.Image)),
                    ("Text", typeof(GameObjectKind.Text)),
                    ("Camera", typeof(GameObjectKind.Camera)),
                    ("Light", typeof(GameObjectKind.Light)),
                    ("Prefab", typeof(GameObjectKind.Prefab))
                ),
                [typeof(MotionTransform)] = Fixed(
                    ("Translate", typeof(MotionTransform.Translate)),
                    ("Rotate", typeof(MotionTransform.Rotate)),
                    ("Skew", typeof(MotionTransform.Skew)),
                    ("Scale", typeof(MotionTransform.Scale))
                ),
                [typeof(MotionFilter)] = Fixed(
                    ("Blur", typeof(MotionFilter.Blur)),
                    ("Brightness", typeof(MotionFilter.Brightness)),
                    ("Saturate", typeof(MotionFilter.Saturate)),
                    ("Contrast", typeof(MotionFilter.Contrast)),
                    ("HueRotate", typeof(MotionFilter.HueRotate)),
                    ("Opacity", typeof(MotionFilter.Opacity)),
                    ("DropShadow", typeof(MotionFilter.DropShadow))
                ),
                [typeof(MotionGradient)] = Fixed(
                    ("Linear", typeof(MotionGradient.Linear)),
                    ("Radial", typeof(MotionGradient.Radial))
                ),
                [typeof(MotionValue)] = Fixed(
                    ("Scalar", typeof(MotionValue.Scalar)),
                    ("Length", typeof(MotionValue.Length)),
                    ("Color", typeof(MotionValue.Color)),
                    ("Vector2", typeof(MotionValue.Vector2)),
                    ("Vector3", typeof(MotionValue.Vector3)),
                    ("Angle", typeof(MotionValue.Angle)),
                    ("TransformList", typeof(MotionValue.TransformList)),
                    ("FilterList", typeof(MotionValue.FilterList)),
                    ("ShadowList", typeof(MotionValue.ShadowList)),
                    ("Gradient", typeof(MotionValue.Gradient)),
                    ("ClipInset", typeof(MotionValue.ClipInset)),
                    ("ClipPolygon", typeof(MotionValue.ClipPolygon)),
                    ("Discrete", typeof(MotionValue.Discrete))
                ),
                [typeof(MotionExpressionOperation)] = Fixed(
                    ("Add", typeof(MotionExpressionOperation.Add)),
                    ("Subtract", typeof(MotionExpressionOperation.Subtract)),
                    ("Multiply", typeof(MotionExpressionOperation.Multiply)),
                    ("Divide", typeof(MotionExpressionOperation.Divide)),
                    ("Power", typeof(MotionExpressionOperation.Power)),
                    ("SquareRoot", typeof(MotionExpressionOperation.SquareRoot)),
                    ("Absolute", typeof(MotionExpressionOperation.Absolute)),
                    ("Minimum", typeof(MotionExpressionOperation.Minimum)),
                    ("Maximum", typeof(MotionExpressionOperation.Maximum)),
                    ("Clamp", typeof(MotionExpressionOperation.Clamp)),
                    ("Modulo", typeof(MotionExpressionOperation.Modulo)),
                    ("Wrap", typeof(MotionExpressionOperation.Wrap)),
                    ("ExponentialDecay", typeof(MotionExpressionOperation.ExponentialDecay)),
                    ("Mix", typeof(MotionExpressionOperation.Mix))
                ),
                [typeof(MotionValueSource)] = Fixed(
                    ("Mutable", typeof(MotionValueSource.Mutable)),
                    ("Time", typeof(MotionValueSource.Time)),
                    ("Velocity", typeof(MotionValueSource.Velocity)),
                    ("Range", typeof(MotionValueSource.Range)),
                    ("Spring", typeof(MotionValueSource.Spring)),
                    ("Expression", typeof(MotionValueSource.Expression))
                ),
                [typeof(MotionValueCommand)] = Fixed(
                    ("Set", typeof(MotionValueCommand.Set)),
                    ("Jump", typeof(MotionValueCommand.Jump)),
                    ("Stop", typeof(MotionValueCommand.Stop)),
                    ("Animate", typeof(MotionValueCommand.Animate))
                ),
                [typeof(MotionControlTarget)] = Fixed(
                    ("Target", typeof(MotionControlTarget.Target)),
                    ("Variant", typeof(MotionControlTarget.Variant))
                ),
                [typeof(MotionControlCommand)] = Fixed(
                    ("Start", typeof(MotionControlCommand.Start)),
                    ("Set", typeof(MotionControlCommand.Set)),
                    ("Stop", typeof(MotionControlCommand.Stop)),
                    ("Clear", typeof(MotionControlCommand.Clear))
                ),
                [typeof(MotionSelector)] = Fixed(
                    ("Element", typeof(MotionSelector.Element)),
                    ("Name", typeof(MotionSelector.Name)),
                    ("ScopeRoot", typeof(MotionSelector.ScopeRoot)),
                    ("Children", typeof(MotionSelector.Children)),
                    ("Descendants", typeof(MotionSelector.Descendants))
                ),
                [typeof(MotionScopeCommand)] = Fixed(
                    ("Start", typeof(MotionScopeCommand.Start)),
                    ("Set", typeof(MotionScopeCommand.Set)),
                    ("Stop", typeof(MotionScopeCommand.Stop))
                ),
                [typeof(MotionEasing)] = Fixed(
                    ("Linear", typeof(MotionEasing.Linear)),
                    ("EaseIn", typeof(MotionEasing.EaseIn)),
                    ("EaseOut", typeof(MotionEasing.EaseOut)),
                    ("EaseInOut", typeof(MotionEasing.EaseInOut)),
                    ("CubicBezier", typeof(MotionEasing.CubicBezier)),
                    ("Steps", typeof(MotionEasing.Steps))
                ),
                [typeof(MotionRepeat)] = Fixed(
                    ("None", typeof(MotionRepeat.None)),
                    ("Count", typeof(MotionRepeat.Count)),
                    ("Forever", typeof(MotionRepeat.Forever))
                ),
                [typeof(InertiaTarget)] = Fixed(
                    ("Identity", typeof(InertiaTarget.Identity)),
                    ("NearestMultiple", typeof(InertiaTarget.NearestMultiple)),
                    ("FloorMultiple", typeof(InertiaTarget.FloorMultiple)),
                    ("CeilingMultiple", typeof(InertiaTarget.CeilingMultiple)),
                    ("Clamp", typeof(InertiaTarget.Clamp))
                ),
                [typeof(SpringConfiguration)] = Fixed(
                    ("Physical", typeof(SpringConfiguration.Physical)),
                    ("Duration", typeof(SpringConfiguration.Duration)),
                    ("VisualDuration", typeof(SpringConfiguration.VisualDuration))
                ),
                [typeof(TransitionGenerator)] = Fixed(
                    ("Immediate", typeof(TransitionGenerator.Immediate)),
                    ("Tween", typeof(TransitionGenerator.Tween)),
                    ("Spring", typeof(TransitionGenerator.Spring)),
                    ("Inertia", typeof(TransitionGenerator.Inertia))
                ),
                [typeof(MotionClockSource)] = Fixed(
                    ("Unscaled", typeof(MotionClockSource.Unscaled)),
                    ("Scaled", typeof(MotionClockSource.Scaled)),
                    ("Controlled", typeof(MotionClockSource.Controlled)),
                    ("Audio", typeof(MotionClockSource.Audio))
                ),
                [typeof(MotionDragConstraint)] = Fixed(
                    ("Bounds", typeof(MotionDragConstraint.Bounds)),
                    ("Element", typeof(MotionDragConstraint.Element))
                ),
                [typeof(MotionEventKind)] = Fixed(
                    ("Activated", typeof(MotionEventKind.Activated)),
                    ("Started", typeof(MotionEventKind.Started)),
                    ("Repeated", typeof(MotionEventKind.Repeated)),
                    ("Completed", typeof(MotionEventKind.Completed)),
                    ("Stopped", typeof(MotionEventKind.Stopped)),
                    ("Cancelled", typeof(MotionEventKind.Cancelled))
                ),
                [typeof(MotionPlaybackCommand)] = Fixed(
                    ("Play", typeof(MotionPlaybackCommand.Play)),
                    ("Pause", typeof(MotionPlaybackCommand.Pause)),
                    ("Replay", typeof(MotionPlaybackCommand.Replay)),
                    ("Stop", typeof(MotionPlaybackCommand.Stop)),
                    ("Cancel", typeof(MotionPlaybackCommand.Cancel)),
                    ("Complete", typeof(MotionPlaybackCommand.Complete)),
                    ("Seek", typeof(MotionPlaybackCommand.Seek)),
                    ("SetSpeed", typeof(MotionPlaybackCommand.SetSpeed)),
                    ("SetDirection", typeof(MotionPlaybackCommand.SetDirection))
                ),
                [typeof(MotionControlledClockCommand)] = Fixed(
                    ("Set", typeof(MotionControlledClockCommand.Set)),
                    ("Advance", typeof(MotionControlledClockCommand.Advance))
                ),
                [typeof(GridTrack)] = Fixed(
                    ("Px", typeof(GridTrack.Px)),
                    ("Fraction", typeof(GridTrack.Fraction)),
                    ("Auto", typeof(GridTrack.Auto))
                ),
                [typeof(OverlayPlacement)] = Fixed(
                    ("Layer", typeof(OverlayPlacement.Layer)),
                    ("Popover", typeof(OverlayPlacement.Popover)),
                    ("Modal", typeof(OverlayPlacement.Modal))
                ),
                [typeof(UiElement)] = Fixed(
                    ("VisualElement", typeof(UiElement.VisualElement)),
                    ("Flex", typeof(UiElement.Flex)),
                    ("Grid", typeof(UiElement.Grid)),
                    ("Stack", typeof(UiElement.Stack)),
                    ("Box", typeof(UiElement.Box)),
                    ("Label", typeof(UiElement.Label)),
                    ("TextElement", typeof(UiElement.TextElement)),
                    ("TextField", typeof(UiElement.TextField)),
                    ("Toggle", typeof(UiElement.Toggle)),
                    ("RadioButton", typeof(UiElement.RadioButton)),
                    ("RadioButtonGroup", typeof(UiElement.RadioButtonGroup)),
                    ("ToggleButtonGroup", typeof(UiElement.ToggleButtonGroup)),
                    ("DropdownField", typeof(UiElement.DropdownField)),
                    ("Button", typeof(UiElement.Button)),
                    ("RepeatButton", typeof(UiElement.RepeatButton)),
                    ("GroupBox", typeof(UiElement.GroupBox)),
                    ("PopupWindow", typeof(UiElement.PopupWindow)),
                    ("ScrollView", typeof(UiElement.ScrollView)),
                    ("Scroller", typeof(UiElement.Scroller)),
                    ("Slider", typeof(UiElement.Slider)),
                    ("SliderInt", typeof(UiElement.SliderInt)),
                    ("MinMaxSlider", typeof(UiElement.MinMaxSlider)),
                    ("ProgressBar", typeof(UiElement.ProgressBar)),
                    ("Tab", typeof(UiElement.Tab)),
                    ("TabView", typeof(UiElement.TabView)),
                    ("Image", typeof(UiElement.Image))
                ),
                [typeof(UiEventBody)] = Fixed(
                    ("PointerDown", typeof(UiEventBody.PointerDown)),
                    ("PointerMove", typeof(UiEventBody.PointerMove)),
                    ("PointerUp", typeof(UiEventBody.PointerUp)),
                    ("PointerCancel", typeof(UiEventBody.PointerCancel)),
                    ("Click", typeof(UiEventBody.Click)),
                    ("PointerEnter", typeof(UiEventBody.PointerEnter)),
                    ("PointerLeave", typeof(UiEventBody.PointerLeave)),
                    ("PointerOver", typeof(UiEventBody.PointerOver)),
                    ("PointerOut", typeof(UiEventBody.PointerOut)),
                    ("Wheel", typeof(UiEventBody.Wheel)),
                    ("PointerCapture", typeof(UiEventBody.PointerCapture)),
                    ("PointerCaptureOut", typeof(UiEventBody.PointerCaptureOut)),
                    ("KeyDown", typeof(UiEventBody.KeyDown)),
                    ("KeyUp", typeof(UiEventBody.KeyUp)),
                    ("NavigationMove", typeof(UiEventBody.NavigationMove)),
                    ("NavigationCancel", typeof(UiEventBody.NavigationCancel)),
                    ("FocusIn", typeof(UiEventBody.FocusIn)),
                    ("Focus", typeof(UiEventBody.Focus)),
                    ("FocusOut", typeof(UiEventBody.FocusOut)),
                    ("Blur", typeof(UiEventBody.Blur)),
                    ("GeometryChanged", typeof(UiEventBody.GeometryChanged)),
                    ("AttachToPanel", typeof(UiEventBody.AttachToPanel)),
                    ("DetachFromPanel", typeof(UiEventBody.DetachFromPanel)),
                    ("TransitionStart", typeof(UiEventBody.TransitionStart)),
                    ("TransitionEnd", typeof(UiEventBody.TransitionEnd)),
                    ("TransitionCancel", typeof(UiEventBody.TransitionCancel)),
                    ("ValueChanging", typeof(UiEventBody.ValueChanging)),
                    ("ValueCommitted", typeof(UiEventBody.ValueCommitted)),
                    ("Input", typeof(UiEventBody.Input)),
                    ("SelectionChanged", typeof(UiEventBody.SelectionChanged)),
                    ("LinkEnter", typeof(UiEventBody.LinkEnter)),
                    ("LinkLeave", typeof(UiEventBody.LinkLeave)),
                    ("LinkDown", typeof(UiEventBody.LinkDown)),
                    ("LinkUp", typeof(UiEventBody.LinkUp)),
                    ("ScrollSettled", typeof(UiEventBody.ScrollSettled)),
                    ("ScrollChanged", typeof(UiEventBody.ScrollChanged)),
                    ("TabSelectionRequested", typeof(UiEventBody.TabSelectionRequested)),
                    ("TabCloseRequested", typeof(UiEventBody.TabCloseRequested)),
                    ("TabReorderRequested", typeof(UiEventBody.TabReorderRequested))
                ),
                [typeof(UiPointerButton)] = Fixed(
                    ("Left", typeof(UiPointerButton.Left)),
                    ("Middle", typeof(UiPointerButton.Middle)),
                    ("Right", typeof(UiPointerButton.Right)),
                    ("Other", typeof(UiPointerButton.Other))
                ),
                [typeof(UiFocusDirection)] = Fixed(
                    ("None", typeof(UiFocusDirection.None)),
                    ("Unspecified", typeof(UiFocusDirection.Unspecified)),
                    ("Left", typeof(UiFocusDirection.Left)),
                    ("Right", typeof(UiFocusDirection.Right)),
                    ("Other", typeof(UiFocusDirection.Other))
                ),
                [typeof(UiValue)] = Fixed(
                    ("Bool", typeof(UiValue.Bool)),
                    ("Index", typeof(UiValue.Index)),
                    ("Indices", typeof(UiValue.Indices)),
                    ("Choice", typeof(UiValue.Choice)),
                    ("F32", typeof(UiValue.F32)),
                    ("I32", typeof(UiValue.I32)),
                    ("F32Range", typeof(UiValue.F32Range)),
                    ("String", typeof(UiValue.String))
                ),
                [typeof(LowerLimit)] = Fixed(
                    ("Unbounded", typeof(LowerLimit.Unbounded)),
                    ("Inclusive", typeof(LowerLimit.Inclusive))
                ),
                [typeof(UpperLimit)] = Fixed(
                    ("Unbounded", typeof(UpperLimit.Unbounded)),
                    ("Inclusive", typeof(UpperLimit.Inclusive))
                ),
                [typeof(InteractionDistance)] = Fixed(
                    ("Unbounded", typeof(InteractionDistance.Unbounded)),
                    ("Inclusive", typeof(InteractionDistance.Inclusive))
                ),
                [typeof(ClickEvent)] = Fixed(
                    ("Pointer", typeof(ClickEvent.Pointer)),
                    ("NavigationSubmit", typeof(ClickEvent.NavigationSubmit)),
                    ("Repeat", typeof(ClickEvent.Repeat))
                ),
                [typeof(VisualElementUpdate)] = Fixed(
                    ("Properties", typeof(VisualElementUpdate.Properties)),
                    ("Parent", typeof(VisualElementUpdate.Parent)),
                    ("Index", typeof(VisualElementUpdate.Index))
                ),
                [typeof(VisualElementAction)] = Fixed(
                    ("Focus", typeof(VisualElementAction.Focus)),
                    ("Blur", typeof(VisualElementAction.Blur)),
                    ("CapturePointer", typeof(VisualElementAction.CapturePointer)),
                    ("ReleasePointer", typeof(VisualElementAction.ReleasePointer)),
                    ("ScrollTo", typeof(VisualElementAction.ScrollTo)),
                    ("SelectText", typeof(VisualElementAction.SelectText))
                ),
                [typeof(ActionBody)] = Nested<ActionBody>(
                    "PointerEnter",
                    "PointerExit",
                    "PointerDown",
                    "PointerUp",
                    "PointerClick",
                    "DragStart",
                    "DragEnd",
                    "KeyDown",
                    "KeyUp",
                    "ControllerButtonDown",
                    "ControllerButtonUp",
                    "ControllerNavigate",
                    "VisualElement",
                    "GeometryObservations",
                    "MotionEvents"
                ),
                [typeof(DiagnosticsCommand)] = Nested<DiagnosticsCommand>("SetMetadata"),
                [typeof(CameraTarget)] = Nested<CameraTarget>("Input", "Object"),
                [typeof(GeometryObservationTarget)] = Nested<GeometryObservationTarget>(
                    "UiElement",
                    "Viewport",
                    "WorldOrigin",
                    "WorldAnchor",
                    "WorldRenderedBounds"
                ),
                [typeof(GeometryValue)] = Nested<GeometryValue>(
                    "Element",
                    "Viewport",
                    "WorldPoint",
                    "WorldBounds"
                ),
                [typeof(GeometryObservationResult)] = Nested<GeometryObservationResult>(
                    "Current",
                    "Unavailable"
                ),
                [typeof(ParticleSpawnLocation)] = Fixed(
                    ("GameObject", typeof(ParticleSpawnLocation.AtGameObject)),
                    ("WorldPosition", typeof(ParticleSpawnLocation.AtWorldPosition))
                ),
                [typeof(TweenRepeat)] = Fixed(
                    ("Once", typeof(TweenRepeat.Once)),
                    ("Count", typeof(TweenRepeat.Count)),
                    ("Forever", typeof(TweenRepeat.Forever))
                ),
                [typeof(CommandBody)] = CommandCases(),
            };
            return cases;
        }

        private static IReadOnlyDictionary<string, Type> CommandCases()
        {
            return Fixed(
                ("AssetsReplaceSet", typeof(CommandBody.Assets.ReplaceSet)),
                ("SceneLoad", typeof(CommandBody.Scene.Load)),
                ("SceneUnload", typeof(CommandBody.Scene.Unload)),
                ("SceneSetPrimary", typeof(CommandBody.Scene.SetPrimary)),
                ("ObjectCreate", typeof(CommandBody.Object.Create)),
                ("ObjectDestroy", typeof(CommandBody.Object.Destroy)),
                ("ObjectSetActive", typeof(CommandBody.Object.SetActive)),
                ("ObjectReparent", typeof(CommandBody.Object.Reparent)),
                ("TransformSetLocalPosition", typeof(CommandBody.Transform.SetLocalPosition)),
                ("TransformSetWorldPosition", typeof(CommandBody.Transform.SetWorldPosition)),
                ("TransformTweenLocalPosition", typeof(CommandBody.Transform.TweenLocalPosition)),
                ("TransformTweenWorldPosition", typeof(CommandBody.Transform.TweenWorldPosition)),
                ("TransformSetLocalRotation", typeof(CommandBody.Transform.SetLocalRotation)),
                ("TransformSetWorldRotation", typeof(CommandBody.Transform.SetWorldRotation)),
                ("TransformTweenLocalRotation", typeof(CommandBody.Transform.TweenLocalRotation)),
                ("TransformTweenWorldRotation", typeof(CommandBody.Transform.TweenWorldRotation)),
                ("TransformSetLocalScale", typeof(CommandBody.Transform.SetLocalScale)),
                ("TransformTweenLocalScale", typeof(CommandBody.Transform.TweenLocalScale)),
                ("RendererSetMaterial", typeof(CommandBody.Renderer.SetMaterial)),
                ("CameraSetEnabled", typeof(CommandBody.Camera.SetEnabled)),
                ("CameraSetPerspective", typeof(CommandBody.Camera.SetPerspective)),
                ("CameraTweenFieldOfView", typeof(CommandBody.Camera.TweenFieldOfView)),
                ("CameraSetOrthographic", typeof(CommandBody.Camera.SetOrthographic)),
                ("CameraTweenOrthographicSize", typeof(CommandBody.Camera.TweenOrthographicSize)),
                ("CameraSetClipping", typeof(CommandBody.Camera.SetClipping)),
                ("CameraSetClear", typeof(CommandBody.Camera.SetClear)),
                ("LightSetEnabled", typeof(CommandBody.Light.SetEnabled)),
                ("LightSetType", typeof(CommandBody.Light.SetType)),
                ("LightSetColor", typeof(CommandBody.Light.SetColor)),
                ("LightTweenColor", typeof(CommandBody.Light.TweenColor)),
                ("LightSetIntensity", typeof(CommandBody.Light.SetIntensity)),
                ("LightTweenIntensity", typeof(CommandBody.Light.TweenIntensity)),
                ("LightSetRange", typeof(CommandBody.Light.SetRange)),
                ("LightSetSpotAngle", typeof(CommandBody.Light.SetSpotAngle)),
                ("LightSetShadows", typeof(CommandBody.Light.SetShadows)),
                ("ImageSetTexture", typeof(CommandBody.Image.SetTexture)),
                ("ImageSetSize", typeof(CommandBody.Image.SetSize)),
                ("ImageSetFit", typeof(CommandBody.Image.SetFit)),
                ("ImageSetTint", typeof(CommandBody.Image.SetTint)),
                ("ImageTweenTint", typeof(CommandBody.Image.TweenTint)),
                ("ImageSetOpacity", typeof(CommandBody.Image.SetOpacity)),
                ("ImageTweenOpacity", typeof(CommandBody.Image.TweenOpacity)),
                ("ImageSetFaceCamera", typeof(CommandBody.Image.SetFaceCamera)),
                ("TextSetContent", typeof(CommandBody.Text.SetContent)),
                ("TextSetFont", typeof(CommandBody.Text.SetFont)),
                ("TextSetSize", typeof(CommandBody.Text.SetSize)),
                ("TextTweenSize", typeof(CommandBody.Text.TweenSize)),
                ("TextSetColor", typeof(CommandBody.Text.SetColor)),
                ("TextTweenColor", typeof(CommandBody.Text.TweenColor)),
                ("TextSetAlignment", typeof(CommandBody.Text.SetAlignment)),
                ("TextSetWrapping", typeof(CommandBody.Text.SetWrapping)),
                ("TextSetRichText", typeof(CommandBody.Text.SetRichText)),
                ("TextSetFaceCamera", typeof(CommandBody.Text.SetFaceCamera)),
                ("AnimatorPlay", typeof(CommandBody.Animator.Play)),
                ("AnimatorCrossFade", typeof(CommandBody.Animator.CrossFade)),
                ("AnimatorSetBool", typeof(CommandBody.Animator.SetBool)),
                ("AnimatorSetInt", typeof(CommandBody.Animator.SetInt)),
                ("AnimatorSetFloat", typeof(CommandBody.Animator.SetFloat)),
                ("AnimatorSetTrigger", typeof(CommandBody.Animator.SetTrigger)),
                ("AnimatorSetSpeed", typeof(CommandBody.Animator.SetSpeed)),
                ("ParticlePlay", typeof(CommandBody.Particle.Play)),
                ("ParticleStop", typeof(CommandBody.Particle.Stop)),
                ("ParticleSpawn", typeof(CommandBody.Particle.Spawn)),
                ("AudioPlay", typeof(CommandBody.Audio.Play)),
                ("AudioStop", typeof(CommandBody.Audio.Stop)),
                ("AudioPause", typeof(CommandBody.Audio.Pause)),
                ("AudioResume", typeof(CommandBody.Audio.Resume)),
                ("AudioSeek", typeof(CommandBody.Audio.Seek)),
                ("AudioSetBuffering", typeof(CommandBody.Audio.SetBuffering)),
                ("AudioReplace", typeof(CommandBody.Audio.Replace)),
                ("AudioSetVolume", typeof(CommandBody.Audio.SetVolume)),
                ("AudioTweenVolume", typeof(CommandBody.Audio.TweenVolume)),
                ("TimeWait", typeof(CommandBody.Time.Wait)),
                ("OperationCancel", typeof(CommandBody.Operation.Cancel)),
                ("InputSetEnabled", typeof(CommandBody.Input.SetEnabled)),
                ("InputSetCamera", typeof(CommandBody.Input.SetCamera)),
                ("InputSetPointerEvents", typeof(CommandBody.Input.SetPointerEvents)),
                ("InputSetGlobalKeys", typeof(CommandBody.Input.SetGlobalKeys)),
                ("InputSetController", typeof(CommandBody.Input.SetController)),
                ("ControllerVibrate", typeof(CommandBody.Controller.Vibrate)),
                ("DebugUi", typeof(CommandBody.DebugUi)),
                ("VisualElementCreate", typeof(CommandBody.VisualElement.Create)),
                ("VisualElementUpdate", typeof(CommandBody.VisualElement.Update)),
                ("VisualElementDestroy", typeof(CommandBody.VisualElement.Destroy)),
                ("VisualElementPerformAction", typeof(CommandBody.VisualElement.PerformAction)),
                ("MotionValue", typeof(CommandBody.Motion.ValueCommand)),
                ("MotionValuePlayback", typeof(CommandBody.Motion.ValuePlayback)),
                ("MotionPlayback", typeof(CommandBody.Motion.Playback)),
                ("MotionControlledClock", typeof(CommandBody.Motion.ControlledClock)),
                ("MotionControl", typeof(CommandBody.Motion.Control)),
                ("MotionScope", typeof(CommandBody.Motion.Scope)),
                ("MotionDragControl", typeof(CommandBody.Motion.DragControl)),
                ("GeometryObservationUpdate", typeof(CommandBody.GeometryObservation)),
                ("Diagnostics", typeof(CommandBody.Diagnostics))
            );
        }

        private static IReadOnlyDictionary<string, Type> Nested<T>(params string[] names)
        {
            var result = new Dictionary<string, Type>();
            foreach (string name in names)
            {
                result[name] =
                    typeof(T).GetNestedType(name)
                    ?? throw new InvalidOperationException(
                        $"Union case {typeof(T).Name}.{name} is missing."
                    );
            }

            return result;
        }

        private static IReadOnlyDictionary<string, Type> Fixed(
            params (string Tag, Type Type)[] values
        ) => values.ToDictionary(value => value.Tag, value => value.Type, StringComparer.Ordinal);
    }

    internal sealed class CustomCommandJsonConverter : JsonConverter
    {
        private readonly Func<CommandId, string, bool, ReadOnlyMemory<byte>, ICommand> decode;

        public CustomCommandJsonConverter(
            Func<CommandId, string, bool, ReadOnlyMemory<byte>, ICommand> decode
        ) => this.decode = decode;

        public override bool CanWrite => false;

        public override bool CanConvert(Type objectType) => objectType == typeof(ICommand);

        public override object ReadJson(
            JsonReader reader,
            Type objectType,
            object? existingValue,
            JsonSerializer serializer
        )
        {
            JToken token = JToken.Load(reader);
            if (token is JObject directCore && directCore["body"] is not null)
            {
                return token.ToObject<Command>(serializer)!;
            }

            (string tag, JToken payload) = ReadTag(token);
            if (tag == "Core")
            {
                return payload.ToObject<Command>(serializer)!;
            }

            if (tag != "Custom" || payload is not JObject custom)
            {
                throw new JsonSerializationException($"Unknown command variant '{tag}'.");
            }

            CommandId id =
                custom["command_id"]?.ToObject<CommandId>(serializer)
                ?? throw new JsonSerializationException("A custom command has no command_id.");
            string type =
                custom["command_type"]?.Value<string>()
                ?? throw new JsonSerializationException("A custom command has no command_type.");
            bool blocking = custom["blocking"]?.Value<bool>() ?? true;
            JToken payloadToken =
                custom["payload"]
                ?? throw new JsonSerializationException("A custom command has no payload.");
            byte[] payloadBytes = Encoding.UTF8.GetBytes(payloadToken.ToString(Formatting.None));
            return decode(id, type, blocking, payloadBytes);
        }

        public override void WriteJson(
            JsonWriter writer,
            object? value,
            JsonSerializer serializer
        ) => throw new NotSupportedException();

        private static (string Tag, JToken Payload) ReadTag(JToken token)
        {
            if (token is not JObject objectValue || objectValue.Count != 1)
            {
                throw new JsonSerializationException(
                    "Externally tagged unions require one JSON property."
                );
            }

            JProperty property = objectValue.Properties().Single();
            return (property.Name, property.Value);
        }
    }
}
