#nullable enable

using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal static class BattlementUiContainerProperties
    {
        public static void ApplyCreate(
            UnityEngine.UIElements.VisualElement target,
            UiElement value
        ) => ApplyUpdate(target, value);

        public static void ApplyUpdate(UnityEngine.UIElements.VisualElement target, UiElement value)
        {
            if (value is UiElement.GroupBox group)
            {
                if (group.Text.IsSet)
                    ((GroupBox)target).text = group.Text.Value;
                else if (group.Text.IsReset)
                    ((GroupBox)target).text = new GroupBox().text;
            }
            if (value is not UiElement.PopupWindow popup)
                return;
            BattlementUiTypographyProperties.Apply((TextElement)target, popup);
        }
    }
}
