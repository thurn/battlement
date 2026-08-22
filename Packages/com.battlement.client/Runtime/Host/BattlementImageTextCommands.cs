#nullable enable

using System;
using UnityEngine;

namespace Battlement
{
    internal static class BattlementImageTextCommands
    {
        public static IBattlementCommandOperation? SetTexture(
            CommandBody.Image.SetTexture command,
            BattlementWorld world,
            BattlementPreparedAssets preparedAssets
        )
        {
            BattlementImage image = RequireImage(command.ObjectId, world);
            IBattlementAssetLease lease = preparedAssets.Acquire(
                new PreparedAsset.Texture(command.Address)
            );
            try
            {
                image.SetTexture(lease);
                return null;
            }
            catch
            {
                lease.Dispose();
                throw;
            }
        }

        public static IBattlementCommandOperation? SetSize(
            CommandBody.Image.SetSize command,
            BattlementWorld world
        )
        {
            RequireImage(command.ObjectId, world).SetSize(command.Width, command.Height);
            return null;
        }

        public static IBattlementCommandOperation? SetFit(
            CommandBody.Image.SetFit command,
            BattlementWorld world
        )
        {
            RequireImage(command.ObjectId, world).SetFit(command.Fit);
            return null;
        }

        public static IBattlementCommandOperation? SetTint(
            CommandBody.Image.SetTint command,
            BattlementWorld world
        )
        {
            RequireImage(command.ObjectId, world).SetTint(command.Tint);
            return null;
        }

        public static IBattlementCommandOperation? TweenTint(
            CommandBody.Image.TweenTint command,
            BattlementWorld world,
            BattlementTweenAdapter tweens,
            TimeSpan now
        )
        {
            BattlementImage image = RequireImage(command.ObjectId, world);
            UnityEngine.Color start = image.Color;
            UnityEngine.Color end = BattlementImage.ConvertTint(command.Tint);
            return tweens.Color(image.transform, start, end, command.Tween, now, image.ApplyTint);
        }

        public static IBattlementCommandOperation? SetOpacity(
            CommandBody.Image.SetOpacity command,
            BattlementWorld world
        )
        {
            RequireImage(command.ObjectId, world).SetOpacity(command.Opacity);
            return null;
        }

        public static IBattlementCommandOperation? TweenOpacity(
            CommandBody.Image.TweenOpacity command,
            BattlementWorld world,
            BattlementTweenAdapter tweens,
            TimeSpan now
        )
        {
            BattlementImage image = RequireImage(command.ObjectId, world);
            float start = image.Color.a;
            float end = BattlementImage.ConvertOpacity(command.Opacity);
            return tweens.Float(
                image.transform,
                start,
                end,
                command.Tween,
                now,
                image.ApplyOpacity
            );
        }

        public static IBattlementCommandOperation? SetImageFaceCamera(
            CommandBody.Image.SetFaceCamera command,
            BattlementWorld world
        )
        {
            RequireImage(command.ObjectId, world).SetFaceCamera(command.FacesCamera);
            return null;
        }

        public static IBattlementCommandOperation? SetContent(
            CommandBody.Text.SetContent command,
            BattlementWorld world
        )
        {
            RequireText(command.ObjectId, world).SetContent(command.Content);
            return null;
        }

        public static IBattlementCommandOperation? SetFont(
            CommandBody.Text.SetFont command,
            BattlementWorld world,
            BattlementPreparedAssets preparedAssets
        )
        {
            BattlementText text = RequireText(command.ObjectId, world);
            IBattlementAssetLease lease = preparedAssets.Acquire(
                new PreparedAsset.Font(command.Address)
            );
            try
            {
                text.SetFont(lease);
                return null;
            }
            catch
            {
                lease.Dispose();
                throw;
            }
        }

        public static IBattlementCommandOperation? SetTextSize(
            CommandBody.Text.SetSize command,
            BattlementWorld world
        )
        {
            RequireText(command.ObjectId, world).SetSize(command.Size);
            return null;
        }

        public static IBattlementCommandOperation? TweenTextSize(
            CommandBody.Text.TweenSize command,
            BattlementWorld world,
            BattlementTweenAdapter tweens,
            TimeSpan now
        )
        {
            BattlementText text = RequireText(command.ObjectId, world);
            float start = text.Size;
            float end = BattlementStandardComponents.RequirePositive(command.Size, "Text size");
            return tweens.Float(text.transform, start, end, command.Tween, now, text.ApplySize);
        }

        public static IBattlementCommandOperation? SetTextColor(
            CommandBody.Text.SetColor command,
            BattlementWorld world
        )
        {
            RequireText(command.ObjectId, world).SetColor(command.Color);
            return null;
        }

        public static IBattlementCommandOperation? TweenTextColor(
            CommandBody.Text.TweenColor command,
            BattlementWorld world,
            BattlementTweenAdapter tweens,
            TimeSpan now
        )
        {
            BattlementText text = RequireText(command.ObjectId, world);
            UnityEngine.Color start = text.Color;
            UnityEngine.Color end = BattlementStandardComponents.ConvertColor(
                command.Color,
                "Text color"
            );
            return tweens.Color(text.transform, start, end, command.Tween, now, text.ApplyColor);
        }

        public static IBattlementCommandOperation? SetAlignment(
            CommandBody.Text.SetAlignment command,
            BattlementWorld world
        )
        {
            RequireText(command.ObjectId, world).SetAlignment(command.Horizontal, command.Vertical);
            return null;
        }

        public static IBattlementCommandOperation? SetWrapping(
            CommandBody.Text.SetWrapping command,
            BattlementWorld world
        )
        {
            RequireText(command.ObjectId, world).SetWrapping(command.WrapWidth);
            return null;
        }

        public static IBattlementCommandOperation? SetRichText(
            CommandBody.Text.SetRichText command,
            BattlementWorld world
        )
        {
            RequireText(command.ObjectId, world).SetRichText(command.IsRichText);
            return null;
        }

        public static IBattlementCommandOperation? SetTextFaceCamera(
            CommandBody.Text.SetFaceCamera command,
            BattlementWorld world
        )
        {
            RequireText(command.ObjectId, world).SetFaceCamera(command.FacesCamera);
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
