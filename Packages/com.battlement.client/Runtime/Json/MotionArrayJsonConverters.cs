#nullable enable

using System;
using System.Collections.Generic;
using Newtonsoft.Json;

namespace Battlement
{
    internal sealed class MotionTargetDescriptorJsonConverter : JsonConverter
    {
        public override bool CanConvert(Type objectType) =>
            objectType == typeof(MotionTargetDescriptor);

        public override object ReadJson(
            JsonReader reader,
            Type objectType,
            object? existingValue,
            JsonSerializer serializer
        )
        {
            if (reader.TokenType == JsonToken.Null)
            {
                return null!;
            }
            MotionArrayJson.RequireStart(reader, nameof(MotionTargetDescriptor));
            IReadOnlyList<TransitionDefinition> transitions = MotionArrayJson.Read<
                IReadOnlyList<TransitionDefinition>
            >(reader, serializer);
            MotionArrayJson.Advance(reader, "motion property tracks");
            MotionArrayJson.RequireStart(reader, "motion property tracks");
            var tracks = new List<MotionPropertyTrack>();
            while (reader.Read() && reader.TokenType != JsonToken.EndArray)
            {
                MotionArrayJson.RequireStart(reader, nameof(MotionPropertyTrack));
                MotionProperty property = MotionArrayJson.Read<MotionProperty>(reader, serializer);
                IReadOnlyList<MotionValue> values = MotionArrayJson.Read<
                    IReadOnlyList<MotionValue>
                >(reader, serializer);
                IReadOnlyList<double>? times = MotionArrayJson.ReadNullable<IReadOnlyList<double>>(
                    reader,
                    serializer
                );
                int transition = MotionArrayJson.Read<int>(reader, serializer);
                MotionArrayJson.RequireEnd(reader, nameof(MotionPropertyTrack));
                if (transition < 0 || transition >= transitions.Count)
                {
                    throw new JsonSerializationException(
                        "A motion track transition index is out of range."
                    );
                }
                tracks.Add(
                    new MotionPropertyTrack(property, values, transitions[transition], times)
                );
            }
            if (reader.TokenType != JsonToken.EndArray)
            {
                throw new JsonSerializationException("Motion property tracks were not terminated.");
            }
            IReadOnlyList<MotionPropertyValue> transitionEnd = MotionArrayJson.Read<
                IReadOnlyList<MotionPropertyValue>
            >(reader, serializer);
            MotionArrayJson.RequireEnd(reader, nameof(MotionTargetDescriptor));
            return new MotionTargetDescriptor(tracks, transitionEnd);
        }

        public override void WriteJson(JsonWriter writer, object? value, JsonSerializer serializer)
        {
            var target =
                value as MotionTargetDescriptor
                ?? throw new JsonSerializationException("A motion target descriptor is required.");
            var transitions = new List<TransitionDefinition>();
            foreach (MotionPropertyTrack track in target.Tracks)
            {
                if (!transitions.Contains(track.Transition))
                {
                    transitions.Add(track.Transition);
                }
            }
            writer.WriteStartArray();
            serializer.Serialize(writer, transitions);
            writer.WriteStartArray();
            foreach (MotionPropertyTrack track in target.Tracks)
            {
                writer.WriteStartArray();
                serializer.Serialize(writer, track.Property);
                serializer.Serialize(writer, track.Values);
                serializer.Serialize(writer, track.Times);
                writer.WriteValue(transitions.IndexOf(track.Transition));
                writer.WriteEndArray();
            }
            writer.WriteEndArray();
            serializer.Serialize(writer, target.TransitionEnd);
            writer.WriteEndArray();
        }
    }

    internal sealed class TransitionDefinitionJsonConverter : JsonConverter
    {
        public override bool CanConvert(Type objectType) =>
            objectType == typeof(TransitionDefinition);

        public override object ReadJson(
            JsonReader reader,
            Type objectType,
            object? existingValue,
            JsonSerializer serializer
        )
        {
            MotionArrayJson.RequireStart(reader, nameof(TransitionDefinition));
            TransitionGenerator generator = MotionArrayJson.Read<TransitionGenerator>(
                reader,
                serializer
            );
            long delayMicros = MotionArrayJson.Read<long>(reader, serializer);
            MotionRepeat repeat = MotionArrayJson.Read<MotionRepeat>(reader, serializer);
            ulong repeatDelayMicros = MotionArrayJson.Read<ulong>(reader, serializer);
            MotionRepeatType repeatType = MotionArrayJson.Read<MotionRepeatType>(
                reader,
                serializer
            );
            MotionArrayJson.RequireEnd(reader, nameof(TransitionDefinition));
            return new TransitionDefinition(
                generator,
                delayMicros,
                repeat,
                repeatDelayMicros,
                repeatType
            );
        }

        public override void WriteJson(JsonWriter writer, object? value, JsonSerializer serializer)
        {
            var transition =
                value as TransitionDefinition
                ?? throw new JsonSerializationException("A transition definition is required.");
            writer.WriteStartArray();
            serializer.Serialize(writer, transition.Generator);
            writer.WriteValue(transition.DelayMicros);
            serializer.Serialize(writer, transition.Repeat);
            writer.WriteValue(transition.RepeatDelayMicros);
            serializer.Serialize(writer, transition.RepeatType);
            writer.WriteEndArray();
        }
    }

    internal static class MotionArrayJson
    {
        internal static void RequireStart(JsonReader reader, string type)
        {
            if (reader.TokenType != JsonToken.StartArray)
            {
                throw new JsonSerializationException(
                    $"A {type} must be a JSON array, not {reader.TokenType}."
                );
            }
        }

        internal static T Read<T>(JsonReader reader, JsonSerializer serializer)
        {
            Advance(reader, typeof(T).Name);
            T? value = serializer.Deserialize<T>(reader);
            return value
                ?? throw new JsonSerializationException($"A {typeof(T).Name} value is required.");
        }

        internal static T? ReadNullable<T>(JsonReader reader, JsonSerializer serializer)
            where T : class
        {
            Advance(reader, typeof(T).Name);
            return serializer.Deserialize<T>(reader);
        }

        internal static object? ReadObject(JsonReader reader, Type type, JsonSerializer serializer)
        {
            Advance(reader, type.Name);
            return serializer.Deserialize(reader, type);
        }

        internal static void RequireEnd(JsonReader reader, string type)
        {
            if (!reader.Read() || reader.TokenType != JsonToken.EndArray)
            {
                throw new JsonSerializationException($"A {type} has an invalid item count.");
            }
        }

        internal static void Advance(JsonReader reader, string item)
        {
            if (!reader.Read() || reader.TokenType == JsonToken.EndArray)
            {
                throw new JsonSerializationException($"A required {item} value is missing.");
            }
        }
    }
}
