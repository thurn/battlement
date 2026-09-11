#nullable enable

using System;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal sealed class BattlementPreparedUiPropertyUpdate : IDisposable
    {
        private readonly BattlementUiElementProperties owner;
        private IBattlementUiAssetLease? image;
        private IBattlementUiAssetLease? icon;
        private IBattlementUiAssetLease? background;
        private IBattlementUiAssetLease? cursor;
        private IBattlementUiAssetLease? material;
        private BattlementUiStyleFontProperties.FontLeases? fonts;
        private bool committed;
        private bool disposed;

        internal BattlementPreparedUiPropertyUpdate(
            BattlementUiElementProperties owner,
            VisualElement target,
            ObjectId objectId,
            UiElement value,
            BattlementUiElementDefaults initial,
            IBattlementUiAssetLease? image,
            IBattlementUiAssetLease? icon,
            IBattlementUiAssetLease? background,
            IBattlementUiAssetLease? cursor,
            IBattlementUiAssetLease? material,
            BattlementUiStyleFontProperties.FontLeases? fonts
        )
        {
            this.owner = owner;
            Target = target;
            ObjectId = objectId;
            Value = value;
            Initial = initial;
            this.image = image;
            this.icon = icon;
            this.background = background;
            this.cursor = cursor;
            this.material = material;
            this.fonts = fonts;
        }

        internal VisualElement Target { get; }
        internal ObjectId ObjectId { get; }
        internal UiElement Value { get; }
        internal BattlementUiElementDefaults Initial { get; }
        internal IBattlementUiAssetLease? Image => image;
        internal IBattlementUiAssetLease? Icon => icon;
        internal IBattlementUiAssetLease? Background => background;
        internal IBattlementUiAssetLease? Cursor => cursor;
        internal IBattlementUiAssetLease? Material => material;
        internal BattlementUiStyleFontProperties.FontLeases? Fonts => fonts;

        public void Commit()
        {
            if (committed || disposed)
                throw new InvalidOperationException("UI property update was already completed.");
            owner.ApplyPreparedUpdate(this);
            committed = true;
        }

        internal void TakeImage() => image = null;

        internal void TakeIcon() => icon = null;

        internal void TakeBackground() => background = null;

        internal void TakeCursor() => cursor = null;

        internal void TakeMaterial() => material = null;

        internal void TakeFonts() => fonts = null;

        public void Dispose()
        {
            if (committed || disposed)
                return;
            disposed = true;
            image?.Dispose();
            icon?.Dispose();
            background?.Dispose();
            cursor?.Dispose();
            material?.Dispose();
            fonts?.Dispose();
            image = null;
            icon = null;
            background = null;
            cursor = null;
            material = null;
            fonts = null;
        }
    }
}
