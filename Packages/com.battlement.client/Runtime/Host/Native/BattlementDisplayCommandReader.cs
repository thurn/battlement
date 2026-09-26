#nullable enable

using System.IO;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement
{
    internal static class BattlementDisplayCommandReader
    {
        public static DisplayCommand Read(Wire.DisplayCommandPayload value)
        {
            if (value.Operation == Wire.DisplayOperation.Preview)
            {
                if (value.PreviewId.HasValue || !value.Configuration.HasValue)
                    throw new InvalidDataException("Invalid display preview payload.");
                Wire.DisplayConfiguration configuration = value.Configuration.Value;
                Wire.DisplayResolution resolution =
                    configuration.Resolution
                    ?? throw new InvalidDataException("Display resolution is missing.");
                if ((byte)configuration.Mode > (byte)DisplayMode.Fullscreen)
                    throw new InvalidDataException("Unknown display mode.");
                if (resolution.Width is < 1 or > 32768 || resolution.Height is < 1 or > 32768)
                    throw new InvalidDataException("Invalid display dimensions.");
                if (resolution.RefreshDenominator == 0)
                    throw new InvalidDataException("Invalid display refresh ratio.");
                return new DisplayCommand.Preview(
                    new DisplayConfiguration(
                        (DisplayMode)configuration.Mode,
                        new DisplayResolution(
                            resolution.Width,
                            resolution.Height,
                            resolution.RefreshNumerator,
                            resolution.RefreshDenominator
                        )
                    )
                );
            }
            if (value.Configuration.HasValue)
                throw new InvalidDataException("Display control cannot contain a configuration.");
            var id = new CommandId(
                BattlementFlatBufferCore.ReadUuid(value.PreviewId, "display preview")
            );
            return value.Operation switch
            {
                Wire.DisplayOperation.Confirm => new DisplayCommand.Confirm(id),
                Wire.DisplayOperation.Cancel => new DisplayCommand.Cancel(id),
                _ => throw new InvalidDataException("Unknown display operation."),
            };
        }
    }
}
