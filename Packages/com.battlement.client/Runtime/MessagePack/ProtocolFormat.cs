#nullable enable

using System;
using System.Buffers;
using System.Collections.Generic;
using MessagePack;

namespace Battlement
{
    internal static partial class ProtocolFormat
    {
        private static void WriteArrayHeader(ref MessagePackWriter writer, int count) =>
            writer.WriteArrayHeader(count);

        private static void ReadArrayHeader(ref MessagePackReader reader, int expected)
        {
            int actual = reader.ReadArrayHeader();
            if (actual != expected)
            {
                throw new MessagePackSerializationException(
                    $"Expected a {expected}-element array, received {actual}."
                );
            }
        }

        private static void WriteVariant(ref MessagePackWriter writer, string variant) =>
            writer.Write(variant);

        private static void WriteVariantHeader(ref MessagePackWriter writer, string variant)
        {
            writer.WriteMapHeader(1);
            writer.Write(variant);
        }

        private static string ReadVariant(ref MessagePackReader reader) =>
            reader.ReadString()
            ?? throw new MessagePackSerializationException("A variant name cannot be nil.");

        private static string ReadVariantHeader(ref MessagePackReader reader)
        {
            int count = reader.ReadMapHeader();
            if (count != 1)
            {
                throw new MessagePackSerializationException(
                    $"Expected a single-entry variant map, received {count} entries."
                );
            }

            return ReadVariant(ref reader);
        }

        private static void WriteString(ref MessagePackWriter writer, string value) =>
            writer.Write(value);

        private static string ReadString(ref MessagePackReader reader) =>
            reader.ReadString()
            ?? throw new MessagePackSerializationException("A required string cannot be nil.");

        private static void WriteStrings(ref MessagePackWriter writer, IReadOnlyList<string> values)
        {
            writer.WriteArrayHeader(values.Count);
            foreach (string value in values)
            {
                writer.Write(value);
            }
        }

        private static IReadOnlyList<string> ReadStrings(ref MessagePackReader reader)
        {
            int count = reader.ReadArrayHeader();
            var values = new string[count];
            for (int index = 0; index < count; index++)
            {
                values[index] = ReadString(ref reader);
            }

            return values;
        }

        private static void WriteOptionalString(ref MessagePackWriter writer, string? value)
        {
            if (value is null)
            {
                writer.WriteNil();
            }
            else
            {
                writer.Write(value);
            }
        }

        private static string? ReadOptionalString(ref MessagePackReader reader) =>
            reader.TryReadNil() ? null : ReadString(ref reader);

        private static void WriteGuid(ref MessagePackWriter writer, Guid value)
        {
            if (value == Guid.Empty)
            {
                throw new MessagePackSerializationException("The all-zero UUID is not valid.");
            }

            byte[] bytes = value.ToByteArray();
            SwapGuidByteOrder(bytes);
            writer.Write(bytes);
        }

        private static Guid ReadGuid(ref MessagePackReader reader)
        {
            ReadOnlySequence<byte>? sequence = reader.ReadBytes();
            if (sequence is null || sequence.Value.Length != 16)
            {
                throw new MessagePackSerializationException(
                    "A Battlement ID must contain 16 bytes."
                );
            }

            var bytes = new byte[16];
            sequence.Value.CopyTo(bytes);
            SwapGuidByteOrder(bytes);
            var value = new Guid(bytes);
            if (value == Guid.Empty)
            {
                throw new MessagePackSerializationException("The all-zero UUID is not valid.");
            }

            return value;
        }

        private static void SwapGuidByteOrder(byte[] bytes)
        {
            (bytes[0], bytes[3]) = (bytes[3], bytes[0]);
            (bytes[1], bytes[2]) = (bytes[2], bytes[1]);
            (bytes[4], bytes[5]) = (bytes[5], bytes[4]);
            (bytes[6], bytes[7]) = (bytes[7], bytes[6]);
        }

        private static ulong Milliseconds(TimeSpan value)
        {
            if (value < TimeSpan.Zero || value.Ticks % TimeSpan.TicksPerMillisecond != 0)
            {
                throw new MessagePackSerializationException(
                    "Battlement durations must be nonnegative whole milliseconds."
                );
            }

            return checked((ulong)(value.Ticks / TimeSpan.TicksPerMillisecond));
        }

        private static TimeSpan ReadDuration(ref MessagePackReader reader)
        {
            ulong milliseconds = reader.ReadUInt64();
            if (milliseconds > (ulong)(TimeSpan.MaxValue.Ticks / TimeSpan.TicksPerMillisecond))
            {
                throw new MessagePackSerializationException(
                    "A duration exceeds TimeSpan.MaxValue."
                );
            }

            return TimeSpan.FromTicks((long)milliseconds * TimeSpan.TicksPerMillisecond);
        }

        private static MessagePackSerializationException UnknownVariant(
            string type,
            string variant
        ) => new($"Unknown {type} variant '{variant}'.");

        private static void WriteEnum(
            ref MessagePackWriter writer,
            int value,
            IReadOnlyList<string> variants,
            string type
        )
        {
            if ((uint)value >= (uint)variants.Count)
            {
                throw new MessagePackSerializationException($"Unknown {type} value {value}.");
            }

            writer.Write(variants[value]);
        }

        private static int ReadEnum(
            ref MessagePackReader reader,
            IReadOnlyList<string> variants,
            string type
        )
        {
            string variant = ReadVariant(ref reader);
            for (int index = 0; index < variants.Count; index++)
            {
                if (variants[index] == variant)
                {
                    return index;
                }
            }

            throw UnknownVariant(type, variant);
        }
    }
}
