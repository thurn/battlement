#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;

namespace Battlement.UI
{
    internal sealed class BattlementUiPartStyleState
    {
        private readonly Dictionary<(Guid, BattlementUiPartProperties.PartKey), UiStyle> styles =
            new();

        public List<UiPartStyle> EffectiveDeclarations(
            Guid objectId,
            UiElement value,
            IEnumerable<UiPartStyle> declarations
        )
        {
            var effective = new Dictionary<BattlementUiPartProperties.PartKey, UiPartStyle>();
            foreach (UiPartStyle declaration in declarations)
            {
                var key = new BattlementUiPartProperties.PartKey(
                    declaration.Part,
                    declaration.Index
                );
                styles.TryGetValue((objectId, key), out UiStyle current);
                effective[key] = declaration with
                {
                    Style = BattlementUiStyleMerge.Merge(
                        current ?? new UiStyle(),
                        declaration.Style
                    ),
                };
            }
            bool optionsRebuilt = value is UiElement.RadioButtonGroup { Choices: not null };
            bool allOptionsChanged = effective.Keys.Any(key =>
                key.Part == UiPart.RadioButtonGroupAllOptions
            );
            if (optionsRebuilt || allOptionsChanged)
                AddRetainedOptions(objectId, value, effective);
            return Ordered(effective.Values).ToList();
        }

        public IReadOnlyList<BattlementUiPartProperties.PartKey> RemovedParts(
            Guid objectId,
            UiElement value,
            IEnumerable<BattlementUiPartProperties.PartKey> conditional
        )
        {
            var removed = new List<BattlementUiPartProperties.PartKey>(conditional);
            if (value is UiElement.RadioButtonGroup { Choices: { } choices })
            {
                foreach (((Guid ownerId, BattlementUiPartProperties.PartKey key), _) in styles)
                    if (ownerId == objectId && IsIndexed(key.Part) && !OptionExists(choices, key))
                        removed.Add(key);
            }
            return removed;
        }

        public void Record(Guid objectId, BattlementUiPartProperties.PartKey key, UiStyle style) =>
            styles[(objectId, key)] = style;

        public void Remove(Guid objectId, BattlementUiPartProperties.PartKey key) =>
            styles.Remove((objectId, key));

        public void Remove(Guid objectId)
        {
            var keys = new List<(Guid, BattlementUiPartProperties.PartKey)>();
            foreach ((Guid, BattlementUiPartProperties.PartKey) key in styles.Keys)
                if (key.Item1 == objectId)
                    keys.Add(key);
            foreach ((Guid, BattlementUiPartProperties.PartKey) key in keys)
                styles.Remove(key);
        }

        public void Clear() => styles.Clear();

        private void AddRetainedOptions(
            Guid objectId,
            UiElement value,
            Dictionary<BattlementUiPartProperties.PartKey, UiPartStyle> effective
        )
        {
            foreach (
                ((Guid ownerId, BattlementUiPartProperties.PartKey key), UiStyle style) in styles
            )
            {
                if (ownerId != objectId || !IsRadioOptionPart(key.Part))
                    continue;
                if (!OptionExists(value, key))
                    continue;
                effective.TryAdd(key, new UiPartStyle(key.Part, style) { Index = key.Index });
            }
        }

        private static IEnumerable<UiPartStyle> Ordered(IEnumerable<UiPartStyle> declarations)
        {
            foreach (UiPartStyle declaration in declarations)
                if (declaration.Part == UiPart.RadioButtonGroupAllOptions)
                    yield return declaration;
            foreach (UiPartStyle declaration in declarations)
                if (declaration.Part != UiPart.RadioButtonGroupAllOptions)
                    yield return declaration;
        }

        private static bool IsIndexed(UiPart part) =>
            part
                is UiPart.RadioButtonGroupOption
                    or UiPart.RadioButtonGroupOptionCheckmarkBackground
                    or UiPart.RadioButtonGroupOptionCheckmark
                    or UiPart.RadioButtonGroupOptionText;

        private static bool IsRadioOptionPart(UiPart part) =>
            part == UiPart.RadioButtonGroupAllOptions || IsIndexed(part);

        private static bool OptionExists(UiElement value, BattlementUiPartProperties.PartKey key) =>
            value is not UiElement.RadioButtonGroup { Choices: { } choices }
            || OptionExists(choices, key);

        private static bool OptionExists(
            IReadOnlyList<string> choices,
            BattlementUiPartProperties.PartKey key
        ) => key.Index is not uint index || index < choices.Count;
    }
}
