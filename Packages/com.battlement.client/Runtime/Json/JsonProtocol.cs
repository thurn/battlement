#nullable enable

using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;
using Newtonsoft.Json;
using Newtonsoft.Json.Converters;
using Newtonsoft.Json.Serialization;

namespace Battlement
{
    internal static class JsonProtocol
    {
        private static readonly UTF8Encoding StrictUtf8 = new(false, true);

        private static readonly JsonSerializerSettings Settings = new()
        {
            ContractResolver = new CanonicalConstructorContractResolver
            {
                NamingStrategy = new SnakeCaseNamingStrategy(),
            },
            Culture = CultureInfo.InvariantCulture,
            DateParseHandling = DateParseHandling.None,
            DefaultValueHandling = DefaultValueHandling.Ignore,
            MaxDepth = 128,
            MissingMemberHandling = MissingMemberHandling.Ignore,
            NullValueHandling = NullValueHandling.Ignore,
            TypeNameHandling = TypeNameHandling.None,
        };

        static JsonProtocol()
        {
            Settings.Converters.Add(new ProtocolScalarConverter());
            Settings.Converters.Add(new ProtocolByteConverter());
            Settings.Converters.Add(new ProtocolColorConverter());
            Settings.Converters.Add(new PropJsonConverter());
            Settings.Converters.Add(new UiStyleJsonConverter());
            Settings.Converters.Add(new UiStyleValueConverter());
            Settings.Converters.Add(new MotionTargetDescriptorJsonConverter());
            Settings.Converters.Add(new TransitionDefinitionJsonConverter());
            Settings.Converters.Add(new BattlementUnionConverter());
            Settings.Converters.Add(new StringEnumConverter { AllowIntegerValues = false });
        }

        public static byte[] Serialize<T>(T value, params JsonConverter[] converters)
        {
            var serializer = JsonSerializer.Create(Settings);
            AddConverters(serializer, converters);
            var builder = new StringBuilder();
            using (var writer = new StringWriter(builder, CultureInfo.InvariantCulture))
            {
                serializer.Serialize(writer, value);
            }

            return StrictUtf8.GetBytes(builder.ToString());
        }

        public static T Deserialize<T>(
            ReadOnlyMemory<byte> bytes,
            params JsonConverter[] converters
        )
        {
            try
            {
                string text = StrictUtf8.GetString(bytes.Span);
                ValidateTokens(text);
                var serializer = JsonSerializer.Create(Settings);
                AddConverters(serializer, converters);
                using var stringReader = new StringReader(text);
                using var reader = new JsonTextReader(stringReader)
                {
                    DateParseHandling = DateParseHandling.None,
                    MaxDepth = 128,
                };
                T? value = serializer.Deserialize<T>(reader);
                if (reader.Read())
                {
                    throw new JsonSerializationException(
                        "A Battlement buffer must contain exactly one JSON value."
                    );
                }

                return value
                    ?? throw new JsonSerializationException(
                        "A required Battlement value was null."
                    );
            }
            catch (JsonSerializationException)
            {
                throw;
            }
            catch (Exception exception)
            {
                throw new JsonSerializationException(
                    "The buffer is not a valid Battlement JSON value: " + exception.Message,
                    exception
                );
            }
        }

        private static void AddConverters(
            JsonSerializer serializer,
            IEnumerable<JsonConverter> converters
        )
        {
            foreach (JsonConverter converter in converters)
            {
                serializer.Converters.Insert(0, converter);
            }
        }

        private static void ValidateTokens(string text)
        {
            using var stringReader = new StringReader(text);
            using var reader = new JsonTextReader(stringReader)
            {
                DateParseHandling = DateParseHandling.None,
                MaxDepth = 128,
            };
            var properties = new Stack<HashSet<string>?>();

            while (reader.Read())
            {
                if (reader.TokenType == JsonToken.StartObject)
                {
                    properties.Push(new HashSet<string>(StringComparer.Ordinal));
                }
                else if (reader.TokenType == JsonToken.StartArray)
                {
                    properties.Push(null);
                }
                else if (
                    reader.TokenType == JsonToken.EndObject
                    || reader.TokenType == JsonToken.EndArray
                )
                {
                    properties.Pop();
                }
                else if (reader.TokenType == JsonToken.PropertyName)
                {
                    HashSet<string>? names = properties.Peek();
                    string name = (string)reader.Value!;
                    if (names is not null && !names.Add(name))
                    {
                        throw new JsonSerializationException(
                            $"Property with the name '{name}' already exists "
                                + "in the current JSON object."
                        );
                    }
                }
            }
        }
    }
}
