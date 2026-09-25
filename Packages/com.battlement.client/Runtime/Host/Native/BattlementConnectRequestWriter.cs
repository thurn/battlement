#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using Google.FlatBuffers;
using WireApplicationState = Battlement.FlatBuffers.Generated.ApplicationState;
using WireConnectRequest = Battlement.FlatBuffers.Generated.ConnectRequest;
using WireMotionPreference = Battlement.FlatBuffers.Generated.ReducedMotionPreference;
using WireScreenSize = Battlement.FlatBuffers.Generated.ScreenSize;

namespace Battlement
{
    /// <summary>Constructs connect requests directly in reusable FlatBuffers storage.</summary>
    internal sealed class BattlementConnectRequestWriter
    {
        private const int InitialCapacity = 1024;
        private FlatBufferBuilder builder = new(InitialCapacity);
        private StringOffset[] customCommandOffsets = Array.Empty<StringOffset>();
        private StringOffset[] moduleOffsets = Array.Empty<StringOffset>();

        public int AllocationBytes => builder.DataBuffer.Length;

        internal void TrimOversized()
        {
            if (AllocationBytes > 1024 * 1024)
                builder = new FlatBufferBuilder(InitialCapacity);
        }

        public ReadOnlyMemory<byte> Write(Connect value)
        {
            Validate(value);
            builder.Clear();
            StringOffset platform = builder.CreateString(value.Platform);
            StringOffset unityVersion = builder.CreateString(value.UnityVersion);
            StringOffset persistentDataPath = OptionalString(value.PersistentDataPath);
            StringOffset streamingAssetsPath = OptionalString(value.StreamingAssetsPath);
            VectorOffset customCommandTypes = StringVector(
                value.CustomCommandTypes,
                ref customCommandOffsets
            );
            VectorOffset modules = StringVector(
                value.Modules ?? Array.Empty<string>(),
                ref moduleOffsets
            );
            Offset<WireApplicationState> applicationState =
                WireApplicationState.CreateApplicationState(
                    builder,
                    value.ApplicationState.Focused,
                    value.ApplicationState.Paused
                );
            var hostSettings = BattlementHostSettingsWriter.Write(builder, value.HostSettings);
            WireConnectRequest.StartConnectRequest(builder);
            WireConnectRequest.AddHostSettings(builder, hostSettings);
            WireConnectRequest.AddPlatform(builder, platform);
            WireConnectRequest.AddUnityVersion(builder, unityVersion);
            WireConnectRequest.AddScreen(
                builder,
                WireScreenSize.CreateScreenSize(builder, value.Screen.Width, value.Screen.Height)
            );
            WireConnectRequest.AddApplicationState(builder, applicationState);
            WireConnectRequest.AddReducedMotionPreference(
                builder,
                (WireMotionPreference)value.ReducedMotionPreference
            );
            WireConnectRequest.AddCustomCommandTypes(builder, customCommandTypes);
            WireConnectRequest.AddModules(builder, modules);
            if (value.PersistentDataPath is not null)
                WireConnectRequest.AddPersistentDataPath(builder, persistentDataPath);
            if (value.StreamingAssetsPath is not null)
                WireConnectRequest.AddStreamingAssetsPath(builder, streamingAssetsPath);
            Offset<WireConnectRequest> request = WireConnectRequest.EndConnectRequest(builder);
            WireConnectRequest.FinishSizePrefixedConnectRequestBuffer(builder, request);
            if (builder.Offset > BattlementProtocolLimits.MaximumMessageBytes)
            {
                throw new InvalidDataException(
                    $"A connect request cannot exceed "
                        + $"{BattlementProtocolLimits.MaximumMessageBytes} bytes."
                );
            }

            return builder.DataBuffer.ToReadOnlyMemory(builder.DataBuffer.Position, builder.Offset);
        }

        private StringOffset OptionalString(string? value) =>
            value is null ? default : builder.CreateString(value);

        private VectorOffset StringVector(IReadOnlyList<string> values, ref StringOffset[] offsets)
        {
            if (offsets.Length < values.Count)
                Array.Resize(ref offsets, values.Count);
            for (int index = 0; index < values.Count; index++)
                offsets[index] = builder.CreateString(values[index]);
            builder.StartVector(sizeof(int), values.Count, sizeof(int));
            for (int index = values.Count - 1; index >= 0; index--)
                builder.AddOffset(offsets[index].Value);
            return builder.EndVector();
        }

        private static void Validate(Connect value)
        {
            if (string.IsNullOrEmpty(value.Platform) || string.IsNullOrEmpty(value.UnityVersion))
                throw new InvalidDataException("Connect platform and Unity version must be set.");
            if (value.Screen.Width == 0 || value.Screen.Height == 0)
                throw new InvalidDataException("Connect screen dimensions must be nonzero.");
            ValidateSortedUnique(value.CustomCommandTypes, "custom command type", true);
            ValidateSortedUnique(value.Modules ?? Array.Empty<string>(), "module", false);
        }

        private static void ValidateSortedUnique(
            IReadOnlyList<string> values,
            string name,
            bool sorted
        )
        {
            var seen = new HashSet<string>(StringComparer.Ordinal);
            string? previous = null;
            foreach (string value in values)
            {
                if (string.IsNullOrEmpty(value) || !seen.Add(value))
                    throw new InvalidDataException($"Connect {name}s must be nonempty and unique.");
                if (sorted && previous is not null && string.CompareOrdinal(previous, value) >= 0)
                    throw new InvalidDataException($"Connect {name}s must be sorted.");
                previous = value;
            }
        }
    }
}
