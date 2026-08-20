#nullable enable

using System;
using UnityEngine;

namespace Masonry
{
    internal static class MasonryImageTextCommands
    {
        public static IMasonryCommandOperation? SetTexture(
            CommandBody.Image.SetTexture command,
            MasonryWorld world,
            MasonryPreparedAssets preparedAssets
        )
        {
            MasonryImage image = RequireImage(command.ObjectId, world);
            IMasonryAssetLease lease = preparedAssets.Acquire(
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

        public static IMasonryCommandOperation? SetSize(
            CommandBody.Image.SetSize command,
            MasonryWorld world
        )
        {
            RequireImage(command.ObjectId, world).SetSize(command.Width, command.Height);
            return null;
        }

        public static IMasonryCommandOperation? SetFit(
            CommandBody.Image.SetFit command,
            MasonryWorld world
        )
        {
            RequireImage(command.ObjectId, world).SetFit(command.Fit);
            return null;
        }

        public static IMasonryCommandOperation? SetTint(
            CommandBody.Image.SetTint command,
            MasonryWorld world
        )
        {
            RequireImage(command.ObjectId, world).SetTint(command.Tint);
            return null;
        }

        public static IMasonryCommandOperation? TweenTint(
            CommandBody.Image.TweenTint command,
            MasonryWorld world,
            MasonryTweenAdapter tweens,
            TimeSpan now
        )
        {
            MasonryImage image = RequireImage(command.ObjectId, world);
            UnityEngine.Color start = image.Color;
            UnityEngine.Color end = MasonryImage.ConvertTint(command.Tint);
            return tweens.Color(image.transform, start, end, command.Tween, now, image.ApplyTint);
        }

        public static IMasonryCommandOperation? SetOpacity(
            CommandBody.Image.SetOpacity command,
            MasonryWorld world
        )
        {
            RequireImage(command.ObjectId, world).SetOpacity(command.Opacity);
            return null;
        }

        public static IMasonryCommandOperation? TweenOpacity(
            CommandBody.Image.TweenOpacity command,
            MasonryWorld world,
            MasonryTweenAdapter tweens,
            TimeSpan now
        )
        {
            MasonryImage image = RequireImage(command.ObjectId, world);
            float start = image.Color.a;
            float end = MasonryImage.ConvertOpacity(command.Opacity);
            return tweens.Float(
                image.transform,
                start,
                end,
                command.Tween,
                now,
                image.ApplyOpacity
            );
        }

        public static IMasonryCommandOperation? SetImageFaceCamera(
            CommandBody.Image.SetFaceCamera command,
            MasonryWorld world
        )
        {
            RequireImage(command.ObjectId, world).SetFaceCamera(command.FacesCamera);
            return null;
        }

        public static IMasonryCommandOperation? SetContent(
            CommandBody.Text.SetContent command,
            MasonryWorld world
        )
        {
            RequireText(command.ObjectId, world).SetContent(command.Content);
            return null;
        }

        public static IMasonryCommandOperation? SetFont(
            CommandBody.Text.SetFont command,
            MasonryWorld world,
            MasonryPreparedAssets preparedAssets
        )
        {
            MasonryText text = RequireText(command.ObjectId, world);
            IMasonryAssetLease lease = preparedAssets.Acquire(
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

        public static IMasonryCommandOperation? SetTextSize(
            CommandBody.Text.SetSize command,
            MasonryWorld world
        )
        {
            RequireText(command.ObjectId, world).SetSize(command.Size);
            return null;
        }

        public static IMasonryCommandOperation? TweenTextSize(
            CommandBody.Text.TweenSize command,
            MasonryWorld world,
            MasonryTweenAdapter tweens,
            TimeSpan now
        )
        {
            MasonryText text = RequireText(command.ObjectId, world);
            float start = text.Size;
            float end = MasonryStandardComponents.RequirePositive(command.Size, "Text size");
            return tweens.Float(text.transform, start, end, command.Tween, now, text.ApplySize);
        }

        public static IMasonryCommandOperation? SetTextColor(
            CommandBody.Text.SetColor command,
            MasonryWorld world
        )
        {
            RequireText(command.ObjectId, world).SetColor(command.Color);
            return null;
        }

        public static IMasonryCommandOperation? TweenTextColor(
            CommandBody.Text.TweenColor command,
            MasonryWorld world,
            MasonryTweenAdapter tweens,
            TimeSpan now
        )
        {
            MasonryText text = RequireText(command.ObjectId, world);
            UnityEngine.Color start = text.Color;
            UnityEngine.Color end = MasonryStandardComponents.ConvertColor(
                command.Color,
                "Text color"
            );
            return tweens.Color(text.transform, start, end, command.Tween, now, text.ApplyColor);
        }

        public static IMasonryCommandOperation? SetAlignment(
            CommandBody.Text.SetAlignment command,
            MasonryWorld world
        )
        {
            RequireText(command.ObjectId, world).SetAlignment(command.Horizontal, command.Vertical);
            return null;
        }

        public static IMasonryCommandOperation? SetWrapping(
            CommandBody.Text.SetWrapping command,
            MasonryWorld world
        )
        {
            RequireText(command.ObjectId, world).SetWrapping(command.WrapWidth);
            return null;
        }

        public static IMasonryCommandOperation? SetRichText(
            CommandBody.Text.SetRichText command,
            MasonryWorld world
        )
        {
            RequireText(command.ObjectId, world).SetRichText(command.IsRichText);
            return null;
        }

        public static IMasonryCommandOperation? SetTextFaceCamera(
            CommandBody.Text.SetFaceCamera command,
            MasonryWorld world
        )
        {
            RequireText(command.ObjectId, world).SetFaceCamera(command.FacesCamera);
            return null;
        }

        private static MasonryImage RequireImage(ObjectId objectId, MasonryWorld world)
        {
            GameObject gameObject = world.RequireObject(objectId);
            return gameObject.TryGetComponent(out MasonryImage image)
                ? image
                : throw Missing(objectId, "image");
        }

        private static MasonryText RequireText(ObjectId objectId, MasonryWorld world)
        {
            GameObject gameObject = world.RequireObject(objectId);
            return gameObject.TryGetComponent(out MasonryText text)
                ? text
                : throw Missing(objectId, "text");
        }

        private static MasonryWorldException Missing(ObjectId objectId, string kind) =>
            new(
                CoreErrorCode.ComponentMissing,
                $"Object {objectId} does not have a Masonry {kind} component."
            );
    }
}
