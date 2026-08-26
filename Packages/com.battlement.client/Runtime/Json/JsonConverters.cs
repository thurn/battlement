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
            typeof(FontAddress),
            typeof(UiFontAddress),
            typeof(UnityFontAddress),
        };

        public override bool CanConvert(Type objectType)
        {
            Type scalarType = Nullable.GetUnderlyingType(objectType) ?? objectType;
            return scalarType == typeof(TimeSpan)
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
            (string tag, JToken payload) = ReadTag(token);
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
                payload = GetSinglePayload(payload, value.GetType());
            }
            else if (IsWrapperUnion(baseType))
            {
                payload = GetSinglePayload(payload, value.GetType());
            }
            else if (IsDirectPayload(value.GetType()))
            {
                payload = GetSinglePayload(payload, value.GetType());
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

        private static (string Tag, JToken Payload) ReadTag(JToken token)
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
                    "Externally tagged unions require one JSON property."
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

        private static JToken GetSinglePayload(JToken payload, Type target)
        {
            if (payload is not JObject objectValue)
            {
                return payload;
            }

            PropertyInfo[] properties = target.GetProperties(
                BindingFlags.Instance | BindingFlags.Public
            );
            if (properties.Length != 1)
            {
                return payload;
            }

            string propertyName = GetWirePropertyName(properties[0]);
            return objectValue[propertyName] ?? payload;
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
            type == typeof(CommandBody.VisualElement.Update);

        private static bool IsScalarUnion(Type baseType) =>
            baseType == typeof(PreparedAsset)
            || baseType == typeof(BackgroundSource)
            || baseType == typeof(UiBackgroundSize)
            || baseType == typeof(UiCursor)
            || baseType == typeof(UiFilterFunction)
            || baseType == typeof(ImageSource)
            || baseType == typeof(IconSource)
            || baseType == typeof(UiValue)
            || baseType == typeof(ParentScene)
            || baseType == typeof(ParticleSpawnLocation)
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
                    ("Font", typeof(PreparedAsset.Font)),
                    ("UiFont", typeof(PreparedAsset.UiFont)),
                    ("UnityFont", typeof(PreparedAsset.UnityFont))
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
                [typeof(UiElement)] = Fixed(
                    ("VisualElement", typeof(UiElement.VisualElement)),
                    ("Box", typeof(UiElement.Box)),
                    ("Label", typeof(UiElement.Label)),
                    ("TextElement", typeof(UiElement.TextElement)),
                    ("Button", typeof(UiElement.Button)),
                    ("RepeatButton", typeof(UiElement.RepeatButton)),
                    ("GroupBox", typeof(UiElement.GroupBox)),
                    ("PopupWindow", typeof(UiElement.PopupWindow)),
                    ("ScrollView", typeof(UiElement.ScrollView)),
                    ("Scroller", typeof(UiElement.Scroller)),
                    ("Image", typeof(UiElement.Image))
                ),
                [typeof(UiEventBody)] = Fixed(
                    ("Click", typeof(UiEventBody.Click)),
                    ("NavigationSubmit", typeof(UiEventBody.NavigationSubmit)),
                    ("TransitionStart", typeof(UiEventBody.TransitionStart)),
                    ("TransitionEnd", typeof(UiEventBody.TransitionEnd)),
                    ("TransitionCancel", typeof(UiEventBody.TransitionCancel)),
                    ("ValueChanging", typeof(UiEventBody.ValueChanging)),
                    ("ValueCommitted", typeof(UiEventBody.ValueCommitted)),
                    ("ScrollSettled", typeof(UiEventBody.ScrollSettled)),
                    ("ScrollChanged", typeof(UiEventBody.ScrollChanged))
                ),
                [typeof(UiValue)] = Fixed(("F32", typeof(UiValue.F32))),
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
                    "VisualElement"
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
                ("VisualElementCreate", typeof(CommandBody.VisualElement.Create)),
                ("VisualElementUpdate", typeof(CommandBody.VisualElement.Update)),
                ("VisualElementDestroy", typeof(CommandBody.VisualElement.Destroy)),
                ("VisualElementPerformAction", typeof(CommandBody.VisualElement.PerformAction))
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
