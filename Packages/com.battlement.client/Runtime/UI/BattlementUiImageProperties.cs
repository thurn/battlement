#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;
using ProtocolImage = Battlement.UiElement.Image;
using ProtocolRect = Battlement.Rect;
using UnityImage = UnityEngine.UIElements.Image;

namespace Battlement.UI
{
    internal sealed class BattlementUiImageProperties
    {
        private readonly IBattlementUiAssetLookup? assets;
        private readonly Dictionary<Guid, ImageLease> leases = new();
        private readonly Dictionary<Guid, ProtocolRect?> sourceRects = new();

        public BattlementUiImageProperties(IBattlementUiAssetLookup? assets) =>
            this.assets = assets;

        public static void Validate(
            ProtocolImage value,
            ImageSource? currentSource = null,
            ProtocolRect? currentSourceRect = null
        )
        {
            ImageSource? source = value.Source ?? currentSource;
            ProtocolRect? mergedSourceRect = value.SourceRect ?? currentSourceRect;
            if (source is ImageSource.Sprite && mergedSourceRect is not null)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "A sprite image cannot also specify a source rectangle."
                );
            if (value.SourceRect is ProtocolRect sourceRect)
                ValidateRect(sourceRect, normalized: false, "image source rectangle");
            if (value.Uv is ProtocolRect uv)
                ValidateRect(uv, normalized: true, "image UV rectangle");
            if (value.TintColor is Color tint)
            {
                double[] channels = { tint.Red, tint.Green, tint.Blue, tint.Alpha };
                foreach (double channel in channels)
                {
                    if (!double.IsFinite(channel) || channel < 0 || channel > 1)
                        throw Failure(CoreErrorCode.InvalidProperty, "Image tint is invalid.");
                }
            }
        }

        public void Apply(UnityImage target, ObjectId objectId, ProtocolImage value)
        {
            IBattlementUiAssetLease? staged = Stage(value.Source);
            try
            {
                ApplyNative(target, value, staged);
                Commit(objectId.Value, value.Source, staged);
                sourceRects[objectId.Value] = value.SourceRect;
                staged = null;
            }
            finally
            {
                staged?.Dispose();
            }
        }

        public IBattlementUiAssetLease? StageUpdate(ObjectId objectId, ProtocolImage value)
        {
            ImageSource? current = leases.TryGetValue(objectId.Value, out ImageLease existing)
                ? existing.Source
                : null;
            sourceRects.TryGetValue(objectId.Value, out ProtocolRect? currentSourceRect);
            Validate(value, current, currentSourceRect);
            return Stage(value.Source);
        }

        public void ApplyUpdate(
            UnityImage target,
            ObjectId objectId,
            ProtocolImage value,
            IBattlementUiAssetLease? staged
        )
        {
            ApplyNative(target, value, staged);
            if (value.Source is not null)
                Commit(objectId.Value, value.Source, staged!);
            if (value.SourceRect is ProtocolRect sourceRect)
                sourceRects[objectId.Value] = sourceRect;
        }

        public void Remove(Guid objectId)
        {
            sourceRects.Remove(objectId);
            if (leases.Remove(objectId, out ImageLease retained))
                retained.Lease.Dispose();
        }

        public void Clear()
        {
            foreach (ImageLease retained in leases.Values)
                retained.Lease.Dispose();
            leases.Clear();
            sourceRects.Clear();
        }

        private IBattlementUiAssetLease? Stage(ImageSource? source)
        {
            if (source is null)
                return null;
            if (assets is null)
                throw Failure(CoreErrorCode.AssetNotPrepared, "No UI asset lookup is configured.");
            IBattlementUiAssetLease lease = assets.Acquire(Prepared(source));
            try
            {
                RequireValueType(source, lease.Value);
                return lease;
            }
            catch
            {
                lease.Dispose();
                throw;
            }
        }

        private static void ApplyNative(
            UnityImage target,
            ProtocolImage value,
            IBattlementUiAssetLease? staged
        )
        {
            if (value.Source is ImageSource source)
            {
                target.image = null;
                target.sprite = null;
                target.vectorImage = null;
                switch (source)
                {
                    case ImageSource.Texture:
                        target.image = (Texture2D)staged!.Value;
                        break;
                    case ImageSource.Sprite:
                        target.sprite = (Sprite)staged!.Value;
                        break;
                    case ImageSource.VectorImage:
                        target.vectorImage = (VectorImage)staged!.Value;
                        break;
                    case ImageSource.RenderTexture:
                        target.image = (RenderTexture)staged!.Value;
                        break;
                    default:
                        throw Failure(CoreErrorCode.UnknownAsset, "Unknown image source kind.");
                }
            }
            if (value.SourceRect is ProtocolRect sourceRect)
                target.sourceRect = ToUnity(sourceRect);
            if (value.TintColor is Color tint)
                target.tintColor = new UnityEngine.Color(
                    (float)tint.Red,
                    (float)tint.Green,
                    (float)tint.Blue,
                    (float)tint.Alpha
                );
            if (value.ScaleMode is ImageScaleMode scaleMode)
                target.scaleMode = scaleMode switch
                {
                    ImageScaleMode.ScaleAndCrop => ScaleMode.ScaleAndCrop,
                    ImageScaleMode.StretchToFill => ScaleMode.StretchToFill,
                    _ => ScaleMode.ScaleToFit,
                };
            if (value.Uv is ProtocolRect uv)
                target.uv = ToUnity(uv);
        }

        private void Commit(
            Guid objectId,
            ImageSource? source,
            IBattlementUiAssetLease? replacement
        )
        {
            if (source is null || replacement is null)
                return;
            leases.Remove(objectId, out ImageLease previous);
            leases.Add(objectId, new ImageLease(source, replacement));
            previous?.Lease.Dispose();
        }

        private static PreparedAsset Prepared(ImageSource source) =>
            source switch
            {
                ImageSource.Texture value => new PreparedAsset.Texture(value.Address),
                ImageSource.Sprite value => new PreparedAsset.Sprite(value.Address),
                ImageSource.VectorImage value => new PreparedAsset.VectorImage(value.Address),
                ImageSource.RenderTexture value => new PreparedAsset.RenderTexture(value.Address),
                _ => throw Failure(CoreErrorCode.UnknownAsset, "Unknown image source kind."),
            };

        private static void RequireValueType(ImageSource source, object value)
        {
            bool valid = source switch
            {
                ImageSource.Texture => value is Texture2D,
                ImageSource.Sprite => value is Sprite,
                ImageSource.VectorImage => value is VectorImage,
                ImageSource.RenderTexture => value is RenderTexture,
                _ => false,
            };
            if (!valid)
                throw Failure(
                    CoreErrorCode.AssetTypeMismatch,
                    $"Prepared image asset '{Address(source)}' has the wrong Unity type."
                );
        }

        private static string Address(ImageSource source) =>
            source switch
            {
                ImageSource.Texture value => value.Address.Value,
                ImageSource.Sprite value => value.Address.Value,
                ImageSource.VectorImage value => value.Address.Value,
                ImageSource.RenderTexture value => value.Address.Value,
                _ => throw Failure(CoreErrorCode.UnknownAsset, "Unknown image source kind."),
            };

        private static void ValidateRect(ProtocolRect value, bool normalized, string description)
        {
            double[] fields = { value.X, value.Y, value.Width, value.Height };
            foreach (double field in fields)
            {
                if (!double.IsFinite(field))
                    throw Failure(
                        CoreErrorCode.InvalidProperty,
                        $"The {description} is nonfinite."
                    );
            }
            if (value.X < 0 || value.Y < 0 || value.Width < 0 || value.Height < 0)
                throw Failure(CoreErrorCode.InvalidProperty, $"The {description} is negative.");
            if (normalized && (value.X + value.Width > 1 || value.Y + value.Height > 1))
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    $"The {description} exceeds its source."
                );
        }

        private static UnityEngine.Rect ToUnity(ProtocolRect value) =>
            new((float)value.X, (float)value.Y, (float)value.Width, (float)value.Height);

        private static BattlementUiException Failure(CoreErrorCode code, string message) =>
            new(code, message);

        private sealed class ImageLease
        {
            public ImageLease(ImageSource source, IBattlementUiAssetLease lease)
            {
                Source = source;
                Lease = lease;
            }

            public ImageSource Source { get; }

            public IBattlementUiAssetLease Lease { get; }
        }
    }
}
