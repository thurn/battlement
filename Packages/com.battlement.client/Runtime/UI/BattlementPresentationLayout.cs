#nullable enable

namespace Battlement.UI
{
    internal sealed class BattlementPresentationLayout
    {
        private readonly BattlementStickyCoordinator sticky;
        private readonly BattlementOverlayCoordinator overlay;

        public BattlementPresentationLayout(
            BattlementStickyCoordinator sticky,
            BattlementOverlayCoordinator overlay
        )
        {
            this.sticky = sticky;
            this.overlay = overlay;
        }

        public void Refresh()
        {
            sticky.RefreshAll();
            overlay.RefreshAll();
        }
    }
}
