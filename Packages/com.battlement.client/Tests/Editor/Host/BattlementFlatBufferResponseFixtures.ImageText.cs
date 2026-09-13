#nullable enable

using Google.FlatBuffers;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement.Tests
{
    internal static partial class BattlementFlatBufferResponseFixtures
    {
        private static Payload Address(
            FlatBufferBuilder builder,
            Battlement.ObjectId objectId,
            string address,
            Wire.CoreCommandKind kind,
            Wire.CoreCommandPayload payloadType,
            bool texture
        )
        {
            StringOffset encoded = builder.CreateString(address);
            if (texture)
            {
                Wire.SetTexturePayload.StartSetTexturePayload(builder);
                Wire.SetTexturePayload.AddAddress(builder, encoded);
                Wire.SetTexturePayload.AddObjectId(builder, Uuid(builder, objectId.Value));
                return new(
                    kind,
                    payloadType,
                    Wire.SetTexturePayload.EndSetTexturePayload(builder).Value
                );
            }
            Wire.SetFontPayload.StartSetFontPayload(builder);
            Wire.SetFontPayload.AddAddress(builder, encoded);
            Wire.SetFontPayload.AddObjectId(builder, Uuid(builder, objectId.Value));
            return new(kind, payloadType, Wire.SetFontPayload.EndSetFontPayload(builder).Value);
        }

        private static Payload ImageSize(FlatBufferBuilder builder, CommandBody.Image.SetSize value)
        {
            Wire.ImageSizePayload.StartImageSizePayload(builder);
            Wire.ImageSizePayload.AddHeight(builder, value.Height);
            Wire.ImageSizePayload.AddWidth(builder, value.Width);
            Wire.ImageSizePayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            return new(
                Wire.CoreCommandKind.ImageSetSize,
                Wire.CoreCommandPayload.ImageSizePayload,
                Wire.ImageSizePayload.EndImageSizePayload(builder).Value
            );
        }

        private static Payload ImageFit(FlatBufferBuilder builder, CommandBody.Image.SetFit value)
        {
            Wire.ImageFitPayload.StartImageFitPayload(builder);
            Wire.ImageFitPayload.AddFit(builder, (Wire.ImageFit)value.Fit);
            Wire.ImageFitPayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            return new(
                Wire.CoreCommandKind.ImageSetFit,
                Wire.CoreCommandPayload.ImageFitPayload,
                Wire.ImageFitPayload.EndImageFitPayload(builder).Value
            );
        }

        private static Payload Tint(
            FlatBufferBuilder builder,
            Battlement.ObjectId objectId,
            RgbColor value,
            ConflictPolicy conflict,
            Wire.CoreCommandKind kind
        )
        {
            Wire.TintPayload.StartTintPayload(builder);
            Wire.TintPayload.AddTint(
                builder,
                Wire.RgbColor.CreateRgbColor(builder, value.Red, value.Green, value.Blue)
            );
            Wire.TintPayload.AddObjectId(builder, Uuid(builder, objectId.Value));
            Wire.TintPayload.AddOnConflict(builder, Conflict(conflict));
            return new(
                kind,
                Wire.CoreCommandPayload.TintPayload,
                Wire.TintPayload.EndTintPayload(builder).Value
            );
        }

        private static Payload TweenTint(
            FlatBufferBuilder builder,
            CommandBody.Image.TweenTint value
        )
        {
            Wire.TweenTintPayload.StartTweenTintPayload(builder);
            Wire.TweenTintPayload.AddTween(builder, Tween(builder, value.Tween));
            Wire.TweenTintPayload.AddTint(
                builder,
                Wire.RgbColor.CreateRgbColor(
                    builder,
                    value.Tint.Red,
                    value.Tint.Green,
                    value.Tint.Blue
                )
            );
            Wire.TweenTintPayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            Wire.TweenTintPayload.AddOnConflict(builder, Conflict(value.OnConflict));
            return new(
                Wire.CoreCommandKind.ImageTweenTint,
                Wire.CoreCommandPayload.TweenTintPayload,
                Wire.TweenTintPayload.EndTweenTintPayload(builder).Value
            );
        }

        private static Payload Opacity(
            FlatBufferBuilder builder,
            Battlement.ObjectId objectId,
            double value,
            ConflictPolicy conflict,
            Wire.CoreCommandKind kind
        )
        {
            Wire.OpacityPayload.StartOpacityPayload(builder);
            Wire.OpacityPayload.AddOpacity(builder, value);
            Wire.OpacityPayload.AddObjectId(builder, Uuid(builder, objectId.Value));
            Wire.OpacityPayload.AddOnConflict(builder, Conflict(conflict));
            return new(
                kind,
                Wire.CoreCommandPayload.OpacityPayload,
                Wire.OpacityPayload.EndOpacityPayload(builder).Value
            );
        }

        private static Payload TweenOpacity(
            FlatBufferBuilder builder,
            CommandBody.Image.TweenOpacity value
        )
        {
            Wire.TweenOpacityPayload.StartTweenOpacityPayload(builder);
            Wire.TweenOpacityPayload.AddTween(builder, Tween(builder, value.Tween));
            Wire.TweenOpacityPayload.AddOpacity(builder, value.Opacity);
            Wire.TweenOpacityPayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            Wire.TweenOpacityPayload.AddOnConflict(builder, Conflict(value.OnConflict));
            return new(
                Wire.CoreCommandKind.ImageTweenOpacity,
                Wire.CoreCommandPayload.TweenOpacityPayload,
                Wire.TweenOpacityPayload.EndTweenOpacityPayload(builder).Value
            );
        }

        private static Payload TextContent(
            FlatBufferBuilder builder,
            CommandBody.Text.SetContent value
        )
        {
            StringOffset content = builder.CreateString(value.Content);
            Wire.TextContentPayload.StartTextContentPayload(builder);
            Wire.TextContentPayload.AddContent(builder, content);
            Wire.TextContentPayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            return new(
                Wire.CoreCommandKind.TextSetContent,
                Wire.CoreCommandPayload.TextContentPayload,
                Wire.TextContentPayload.EndTextContentPayload(builder).Value
            );
        }

        private static Payload TextSize(
            FlatBufferBuilder builder,
            Battlement.ObjectId objectId,
            double value,
            ConflictPolicy conflict,
            Wire.CoreCommandKind kind
        )
        {
            Wire.TextSizePayload.StartTextSizePayload(builder);
            Wire.TextSizePayload.AddSize(builder, value);
            Wire.TextSizePayload.AddObjectId(builder, Uuid(builder, objectId.Value));
            Wire.TextSizePayload.AddOnConflict(builder, Conflict(conflict));
            return new(
                kind,
                Wire.CoreCommandPayload.TextSizePayload,
                Wire.TextSizePayload.EndTextSizePayload(builder).Value
            );
        }

        private static Payload TweenTextSize(
            FlatBufferBuilder builder,
            CommandBody.Text.TweenSize value
        )
        {
            Wire.TweenTextSizePayload.StartTweenTextSizePayload(builder);
            Wire.TweenTextSizePayload.AddTween(builder, Tween(builder, value.Tween));
            Wire.TweenTextSizePayload.AddSize(builder, value.Size);
            Wire.TweenTextSizePayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            Wire.TweenTextSizePayload.AddOnConflict(builder, Conflict(value.OnConflict));
            return new(
                Wire.CoreCommandKind.TextTweenSize,
                Wire.CoreCommandPayload.TweenTextSizePayload,
                Wire.TweenTextSizePayload.EndTweenTextSizePayload(builder).Value
            );
        }

        private static Payload TextAlignment(
            FlatBufferBuilder builder,
            CommandBody.Text.SetAlignment value
        )
        {
            Wire.TextAlignmentPayload.StartTextAlignmentPayload(builder);
            Wire.TextAlignmentPayload.AddVertical(builder, (Wire.VerticalAlignment)value.Vertical);
            Wire.TextAlignmentPayload.AddHorizontal(
                builder,
                (Wire.HorizontalAlignment)value.Horizontal
            );
            Wire.TextAlignmentPayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            return new(
                Wire.CoreCommandKind.TextSetAlignment,
                Wire.CoreCommandPayload.TextAlignmentPayload,
                Wire.TextAlignmentPayload.EndTextAlignmentPayload(builder).Value
            );
        }

        private static Payload TextWrapping(
            FlatBufferBuilder builder,
            CommandBody.Text.SetWrapping value
        )
        {
            Wire.TextWrappingPayload.StartTextWrappingPayload(builder);
            Wire.TextWrappingPayload.AddWrapWidth(builder, value.WrapWidth);
            Wire.TextWrappingPayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            return new(
                Wire.CoreCommandKind.TextSetWrapping,
                Wire.CoreCommandPayload.TextWrappingPayload,
                Wire.TextWrappingPayload.EndTextWrappingPayload(builder).Value
            );
        }
    }
}
