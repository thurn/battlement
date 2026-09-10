#nullable enable

using System;
using System.Collections.Concurrent;
using System.Globalization;
using System.Linq;
using System.Reflection;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

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
        private static readonly ConcurrentDictionary<Type, IPropFactory> Factories = new();

        public override bool CanConvert(Type objectType) =>
            objectType.IsGenericType && objectType.GetGenericTypeDefinition() == typeof(Prop<>);

        public override object ReadJson(
            JsonReader reader,
            Type objectType,
            object? existingValue,
            JsonSerializer serializer
        )
        {
            Type valueType = objectType.GetGenericArguments()[0];
            IPropFactory factory = Factories.GetOrAdd(
                valueType,
                type =>
                    (IPropFactory)
                        Activator.CreateInstance(typeof(PropFactory<>).MakeGenericType(type))!
            );
            if (reader.TokenType == JsonToken.Null)
                return factory.Reset();
            if (valueType == typeof(bool) && reader.TokenType != JsonToken.Boolean)
                throw new JsonSerializationException("A Boolean property must be true or false.");
            object value =
                serializer.Deserialize(reader, valueType)
                ?? throw new JsonSerializationException("A set property value cannot be null.");
            return factory.Set(value);
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

        private interface IPropFactory
        {
            object Reset();

            object Set(object value);
        }

        private sealed class PropFactory<T> : IPropFactory
        {
            public object Reset() => Prop<T>.Reset();

            public object Set(object value) => Prop<T>.Set((T)value);
        }
    }

    internal sealed class UiStyleValueConverter : JsonConverter
    {
        private static readonly ConcurrentDictionary<Type, IUiStyleFactory> Factories = new();

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
            IUiStyleFactory factory = Factories.GetOrAdd(
                valueType,
                type =>
                    (IUiStyleFactory)
                        Activator.CreateInstance(typeof(UiStyleFactory<>).MakeGenericType(type))!
            );
            if (
                token is JObject keywordObject
                && keywordObject.Count == 1
                && keywordObject.TryGetValue("Keyword", out JToken? keywordToken)
            )
            {
                UiInlineKeyword keyword = keywordToken.ToObject<UiInlineKeyword>(serializer);
                return factory.CreateKeyword(keyword);
            }

            object value =
                token.ToObject(valueType, serializer)
                ?? throw new JsonSerializationException("A concrete UI style value was null.");
            return factory.CreateValue(value);
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

        private interface IUiStyleFactory
        {
            object CreateKeyword(UiInlineKeyword keyword);

            object CreateValue(object value);
        }

        private sealed class UiStyleFactory<T> : IUiStyleFactory
        {
            public object CreateKeyword(UiInlineKeyword keyword) =>
                new UiStyleValue<T>(default!, keyword);

            public object CreateValue(object value) => new UiStyleValue<T>((T)value);
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
}
