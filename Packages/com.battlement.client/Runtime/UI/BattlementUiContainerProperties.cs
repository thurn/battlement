#nullable enable

using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal static class BattlementUiContainerProperties
    {
        public static void ApplyCreate(UnityEngine.UIElements.VisualElement target, UiElement value)
        {
            if (value is UiElement.GroupBox group)
                ((GroupBox)target).text = group.Text ?? string.Empty;
            if (value is UiElement.PopupWindow popup)
                BattlementUiTypographyProperties.Apply((TextElement)target, popup);
        }

        public static void ApplyUpdate(UnityEngine.UIElements.VisualElement target, UiElement value)
        {
            if (value is UiElement.GroupBox group && group.Text is string groupText)
                ((GroupBox)target).text = groupText;
            if (value is not UiElement.PopupWindow popup)
                return;
            BattlementUiTypographyProperties.Apply((TextElement)target, popup);
            if (popup.Text is string popupText)
                ((PopupWindow)target).text = popupText;
        }
    }
}
