#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine.UIElements;
using ProtocolLanguageDirection = Battlement.UiLanguageDirection;
using ProtocolPickingMode = Battlement.UiPickingMode;

namespace Battlement.UI
{
    internal sealed record BattlementUiElementDefaults(
        string? Name,
        bool Enabled,
        PickingMode PickingMode,
        LanguageDirection LanguageDirection,
        bool Focusable,
        int TabIndex,
        bool DelegatesFocus
    )
    {
        public BattlementUiElementDefaults(VisualElement target)
            : this(
                target.name,
                target.enabledSelf,
                target.pickingMode,
                target.languageDirection,
                target.focusable,
                target.tabIndex,
                target.delegatesFocus
            ) { }

        public void Apply(
            VisualElement target,
            Prop<string> name,
            Prop<bool> enabled,
            Prop<ProtocolPickingMode> pickingMode,
            Prop<ProtocolLanguageDirection> languageDirection,
            Prop<bool> focusable,
            Prop<int> tabIndex,
            Prop<bool> delegatesFocus
        )
        {
            Apply(name, item => target.name = item, () => target.name = Name);
            Apply(enabled, target.SetEnabled, () => target.SetEnabled(Enabled));
            Apply(
                pickingMode,
                item => target.pickingMode = BattlementUiElementProperties.ToUnity(item),
                () => target.pickingMode = PickingMode
            );
            Apply(
                languageDirection,
                item => target.languageDirection = BattlementUiElementProperties.ToUnity(item),
                () => target.languageDirection = LanguageDirection
            );
            Apply(focusable, item => target.focusable = item, () => target.focusable = Focusable);
            Apply(tabIndex, item => target.tabIndex = item, () => target.tabIndex = TabIndex);
            Apply(
                delegatesFocus,
                item => target.delegatesFocus = item,
                () => target.delegatesFocus = DelegatesFocus
            );
        }

        public static HashSet<string> ApplyClasses(
            VisualElement target,
            HashSet<string> authored,
            Prop<IReadOnlyList<string>> classes
        )
        {
            if (classes.IsUnset)
                return authored;
            foreach (string className in authored)
                target.RemoveFromClassList(className);
            var replacements = new HashSet<string>();
            if (!classes.IsSet)
                return replacements;
            foreach (string className in classes.Value)
            {
                target.AddToClassList(className);
                replacements.Add(className);
            }
            return replacements;
        }

        public static IReadOnlyList<T>? Values<T>(Prop<IReadOnlyList<T>> value) =>
            value.IsSet ? value.Value
            : value.IsReset ? Array.Empty<T>()
            : null;

        private static void Apply<T>(Prop<T> value, System.Action<T> set, System.Action reset)
        {
            if (value.IsSet)
                set(value.Value);
            else if (value.IsReset)
                reset();
        }
    }
}
