#nullable enable

using System;
using System.Linq;
using System.Text;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Battlement
{
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
