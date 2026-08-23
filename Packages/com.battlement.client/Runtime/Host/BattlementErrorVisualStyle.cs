#nullable enable

using UnityColor = UnityEngine.Color;

namespace Battlement
{
    internal static class BattlementErrorVisualStyle
    {
        public const float CloseButtonInset = 14;
        public const float CloseButtonSize = 36;
        public const float ContentInset = 30;
        public static readonly UnityColor CloseButtonColor = new(0.76f, 0.09f, 0.13f, 1);
        public static readonly UnityColor DialogBackgroundColor = new(0.11f, 0.12f, 0.14f, 1);
        public static readonly UnityColor NeutralButtonColor = new(0.2f, 0.22f, 0.26f, 1);
    }
}
