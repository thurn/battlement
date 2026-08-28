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
            ImageSource? source =
                value.Source.IsSet ? value.Source.Value
                : value.Source.IsReset ? null
                : currentSource;
            ProtocolRect? mergedSourceRect =
                value.SourceRect.IsSet ? value.SourceRect.Value
                : value.SourceRect.IsReset ? null
                : currentSourceRect;
            if (source is ImageSource.Sprite && mergedSourceRect is not null)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "A sprite image cannot also specify a source rectangle."
                );
            if (value.SourceRect.IsSet)
                ValidateRect(value.SourceRect.Value, normalized: false, "image source rectangle");
            if (value.Uv.IsSet)
                ValidateRect(value.Uv.Value, normalized: true, "image UV rectangle");
            if (value.TintColor.IsSet)
            {
                Color tint = value.TintColor.Value;
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
                CommitSourceRect(objectId.Value, value.SourceRect);
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
            Commit(objectId.Value, value.Source, staged);
            CommitSourceRect(objectId.Value, value.SourceRect);
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

        private IBattlementUiAssetLease? Stage(Prop<ImageSource> source)
        {
            if (!source.IsSet)
                return null;
            if (assets is null)
                throw Failure(CoreErrorCode.AssetNotPrepared, "No UI asset lookup is configured.");
            IBattlementUiAssetLease lease = assets.Acquire(Prepared(source.Value));
            try
            {
                RequireValueType(source.Value, lease.Value);
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
            if (!value.Source.IsUnset)
            {
                target.image = null;
                target.sprite = null;
                target.vectorImage = null;
                if (value.Source.IsSet)
                {
                    switch (value.Source.Value)
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
            }
            UnityImage defaults = new();
            Apply(
                value.SourceRect,
                item => target.sourceRect = ToUnity(item),
                () => target.sourceRect = defaults.sourceRect
            );
            Apply(
                value.TintColor,
                item => target.tintColor = ToUnity(item),
                () => target.tintColor = defaults.tintColor
            );
            Apply(
                value.ScaleMode,
                item => target.scaleMode = ToUnity(item),
                () => target.scaleMode = defaults.scaleMode
            );
            Apply(value.Uv, item => target.uv = ToUnity(item), () => target.uv = defaults.uv);
        }

        private void Commit(
            Guid objectId,
            Prop<ImageSource> source,
            IBattlementUiAssetLease? replacement
        )
        {
            if (source.IsUnset)
                return;
            leases.Remove(objectId, out ImageLease previous);
            if (source.IsSet)
                leases.Add(objectId, new ImageLease(source.Value, replacement!));
            previous?.Lease.Dispose();
        }

        private void CommitSourceRect(Guid objectId, Prop<ProtocolRect> value)
        {
            if (value.IsSet)
                sourceRects[objectId] = value.Value;
            else if (value.IsReset)
                sourceRects.Remove(objectId);
        }

        private static void Apply<T>(Prop<T> value, System.Action<T> set, System.Action resetValue)
        {
            if (value.IsSet)
                set(value.Value);
            else if (value.IsReset)
                resetValue();
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

        private static ScaleMode ToUnity(ImageScaleMode value) =>
            value switch
            {
                ImageScaleMode.ScaleAndCrop => ScaleMode.ScaleAndCrop,
                ImageScaleMode.StretchToFill => ScaleMode.StretchToFill,
                _ => ScaleMode.ScaleToFit,
            };

        private static UnityEngine.Color ToUnity(Color value) =>
            new((float)value.Red, (float)value.Green, (float)value.Blue, (float)value.Alpha);

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
