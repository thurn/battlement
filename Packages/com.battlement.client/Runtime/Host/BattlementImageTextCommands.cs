#nullable enable

using System;
using UnityEngine;

namespace Battlement
{
    internal static class BattlementImageTextCommands
    {
        public static IBattlementCommandOperation? Launch(
            BattlementDirectComponentCommand command,
            BattlementWorld world,
            BattlementPreparedAssets preparedAssets,
            BattlementTweenAdapter tweens,
            TimeSpan now
        ) =>
            command.Kind switch
            {
                BattlementDirectComponentCommandKind.ImageSetTexture => SetAsset(
                    command,
                    world,
                    preparedAssets,
                    true
                ),
                BattlementDirectComponentCommandKind.ImageSetSize => SetImageSize(command, world),
                BattlementDirectComponentCommandKind.ImageSetFit => SetImageFit(command, world),
                BattlementDirectComponentCommandKind.ImageSetTint => SetImageTint(command, world),
                BattlementDirectComponentCommandKind.ImageTweenTint => TweenImageTint(
                    command,
                    world,
                    tweens,
                    now
                ),
                BattlementDirectComponentCommandKind.ImageSetOpacity => SetImageOpacity(
                    command,
                    world
                ),
                BattlementDirectComponentCommandKind.ImageTweenOpacity => TweenImageOpacity(
                    command,
                    world,
                    tweens,
                    now
                ),
                BattlementDirectComponentCommandKind.ImageSetFaceCamera => SetImageFaceCamera(
                    command,
                    world
                ),
                BattlementDirectComponentCommandKind.TextSetFont => SetAsset(
                    command,
                    world,
                    preparedAssets,
                    false
                ),
                BattlementDirectComponentCommandKind.TextSetSize => SetTextSize(command, world),
                BattlementDirectComponentCommandKind.TextTweenSize => TweenTextSize(
                    command,
                    world,
                    tweens,
                    now
                ),
                BattlementDirectComponentCommandKind.TextSetColor => SetTextColor(command, world),
                BattlementDirectComponentCommandKind.TextTweenColor => TweenTextColor(
                    command,
                    world,
                    tweens,
                    now
                ),
                BattlementDirectComponentCommandKind.TextSetAlignment => SetTextAlignment(
                    command,
                    world
                ),
                BattlementDirectComponentCommandKind.TextSetWrapping => SetTextWrapping(
                    command,
                    world
                ),
                BattlementDirectComponentCommandKind.TextSetRichText => SetTextRichText(
                    command,
                    world
                ),
                BattlementDirectComponentCommandKind.TextSetFaceCamera => SetTextFaceCamera(
                    command,
                    world
                ),
                _ => throw new BattlementWorldException(
                    CoreErrorCode.InvalidProperty,
                    "An image or text command kind is unknown."
                ),
            };

        private static IBattlementCommandOperation? SetAsset(
            BattlementDirectComponentCommand command,
            BattlementWorld world,
            BattlementPreparedAssets preparedAssets,
            bool image
        )
        {
            string address =
                command.Address ?? throw Invalid("A component asset address is absent.");
            IBattlementAssetLease lease = preparedAssets.Acquire(
                image
                    ? new PreparedAsset.Texture(new TextureAddress(address))
                    : new PreparedAsset.TextMeshProFont(new TextMeshProFontAddress(address))
            );
            try
            {
                if (image)
                    RequireImage(command.ObjectId, world).SetTexture(lease);
                else
                    RequireText(command.ObjectId, world).SetFont(lease);
                return null;
            }
            catch
            {
                lease.Dispose();
                throw;
            }
        }

        private static IBattlementCommandOperation? SetImageSize(
            BattlementDirectComponentCommand command,
            BattlementWorld world
        )
        {
            RequireImage(command.ObjectId, world).SetSize(command.First, command.Second);
            return null;
        }

        private static IBattlementCommandOperation? SetImageFit(
            BattlementDirectComponentCommand command,
            BattlementWorld world
        )
        {
            RequireImage(command.ObjectId, world).SetFit((ImageFit)command.Option);
            return null;
        }

        private static IBattlementCommandOperation? SetImageTint(
            BattlementDirectComponentCommand command,
            BattlementWorld world
        )
        {
            RequireImage(command.ObjectId, world).ApplyTint(DirectColor(command, 1, "Image tint"));
            return null;
        }

        private static IBattlementCommandOperation? TweenImageTint(
            BattlementDirectComponentCommand command,
            BattlementWorld world,
            BattlementTweenAdapter tweens,
            TimeSpan now
        )
        {
            BattlementImage image = RequireImage(command.ObjectId, world);
            return tweens.Color(
                image.transform,
                image.Color,
                DirectColor(command, 1, "Image tint"),
                RequireTween(command),
                now,
                image.ApplyTint
            );
        }

        private static IBattlementCommandOperation? SetImageOpacity(
            BattlementDirectComponentCommand command,
            BattlementWorld world
        )
        {
            RequireImage(command.ObjectId, world)
                .ApplyOpacity(DirectUnit(command.First, "Image opacity"));
            return null;
        }

        private static IBattlementCommandOperation? TweenImageOpacity(
            BattlementDirectComponentCommand command,
            BattlementWorld world,
            BattlementTweenAdapter tweens,
            TimeSpan now
        )
        {
            BattlementImage image = RequireImage(command.ObjectId, world);
            return tweens.Float(
                image.transform,
                image.Color.a,
                DirectUnit(command.First, "Image opacity"),
                RequireTween(command),
                now,
                image.ApplyOpacity
            );
        }

        private static IBattlementCommandOperation? SetImageFaceCamera(
            BattlementDirectComponentCommand command,
            BattlementWorld world
        )
        {
            RequireImage(command.ObjectId, world).SetFaceCamera(command.Enabled);
            return null;
        }

        private static IBattlementCommandOperation? SetTextSize(
            BattlementDirectComponentCommand command,
            BattlementWorld world
        )
        {
            RequireText(command.ObjectId, world).SetSize(command.First);
            return null;
        }

        private static IBattlementCommandOperation? TweenTextSize(
            BattlementDirectComponentCommand command,
            BattlementWorld world,
            BattlementTweenAdapter tweens,
            TimeSpan now
        )
        {
            BattlementText text = RequireText(command.ObjectId, world);
            float size = BattlementStandardComponents.RequirePositive(command.First, "Text size");
            return tweens.Float(
                text.transform,
                text.Size,
                size,
                RequireTween(command),
                now,
                text.ApplySize
            );
        }

        private static IBattlementCommandOperation? SetTextColor(
            BattlementDirectComponentCommand command,
            BattlementWorld world
        )
        {
            RequireText(command.ObjectId, world)
                .ApplyColor(DirectColor(command, command.Fourth, "Text color"));
            return null;
        }

        private static IBattlementCommandOperation? TweenTextColor(
            BattlementDirectComponentCommand command,
            BattlementWorld world,
            BattlementTweenAdapter tweens,
            TimeSpan now
        )
        {
            BattlementText text = RequireText(command.ObjectId, world);
            return tweens.Color(
                text.transform,
                text.Color,
                DirectColor(command, command.Fourth, "Text color"),
                RequireTween(command),
                now,
                text.ApplyColor
            );
        }

        private static IBattlementCommandOperation? SetTextAlignment(
            BattlementDirectComponentCommand command,
            BattlementWorld world
        )
        {
            RequireText(command.ObjectId, world)
                .SetAlignment(
                    (HorizontalAlignment)command.Option,
                    (VerticalAlignment)(byte)command.First
                );
            return null;
        }

        private static IBattlementCommandOperation? SetTextWrapping(
            BattlementDirectComponentCommand command,
            BattlementWorld world
        )
        {
            RequireText(command.ObjectId, world)
                .SetWrapping(command.HasValue ? command.First : null);
            return null;
        }

        private static IBattlementCommandOperation? SetTextRichText(
            BattlementDirectComponentCommand command,
            BattlementWorld world
        )
        {
            RequireText(command.ObjectId, world).SetRichText(command.Enabled);
            return null;
        }

        private static IBattlementCommandOperation? SetTextFaceCamera(
            BattlementDirectComponentCommand command,
            BattlementWorld world
        )
        {
            RequireText(command.ObjectId, world).SetFaceCamera(command.Enabled);
            return null;
        }

        private static UnityEngine.Color DirectColor(
            BattlementDirectComponentCommand command,
            double alpha,
            string name
        ) =>
            BattlementStandardComponents.ConvertColor(
                command.First,
                command.Second,
                command.Third,
                alpha,
                name
            );

        private static float DirectUnit(double value, string name) =>
            BattlementStandardComponents.ConvertColor(value, 0, 0, 1, name).r;

        private static BattlementDirectTweenSettings RequireTween(
            BattlementDirectComponentCommand command
        ) => command.Tween ?? throw Invalid("A tween command has no tween settings.");

        private static BattlementWorldException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);

        public static IBattlementCommandOperation? SetContent(
            BattlementDirectTextContent command,
            BattlementWorld world
        )
        {
            string content = command.ReadContent();
            RequireText(command.ObjectId, world).SetContent(content);
            return null;
        }

        private static BattlementImage RequireImage(ObjectId objectId, BattlementWorld world)
        {
            GameObject gameObject = world.RequireObject(objectId);
            return gameObject.TryGetComponent(out BattlementImage image)
                ? image
                : throw Missing(objectId, "image");
        }

        private static BattlementText RequireText(ObjectId objectId, BattlementWorld world)
        {
            GameObject gameObject = world.RequireObject(objectId);
            return gameObject.TryGetComponent(out BattlementText text)
                ? text
                : throw Missing(objectId, "text");
        }

        private static BattlementWorldException Missing(ObjectId objectId, string kind) =>
            new(
                CoreErrorCode.ComponentMissing,
                $"Object {objectId} does not have a Battlement {kind} component."
            );
    }
}
