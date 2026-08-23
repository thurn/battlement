#nullable enable

using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;
using Newtonsoft.Json;
using Newtonsoft.Json.Converters;
using Newtonsoft.Json.Linq;
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
            Settings.Converters.Add(new ProtocolColorConverter());
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
                var serializer = JsonSerializer.Create(Settings);
                AddConverters(serializer, converters);
                using var stringReader = new StringReader(text);
                using var reader = new JsonTextReader(stringReader)
                {
                    DateParseHandling = DateParseHandling.None,
                    MaxDepth = 128,
                };
                JToken token = JToken.ReadFrom(reader);
                if (reader.Read())
                {
                    throw new JsonSerializationException(
                        "A Battlement buffer must contain exactly one JSON value."
                    );
                }

                return token.ToObject<T>(serializer)
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
    }
}
