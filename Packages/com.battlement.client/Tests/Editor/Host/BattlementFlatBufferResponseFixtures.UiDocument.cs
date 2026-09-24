#nullable enable

using System;
using System.Collections.Generic;
using Google.FlatBuffers;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement.Tests
{
    internal static partial class BattlementFlatBufferResponseFixtures
    {
        private static Offset<Wire.UiDocument> WriteUiDocument(
            FlatBufferBuilder builder,
            UiDocument value
        )
        {
            var root = new UiElement.VisualElement
            {
                Name = value.Name,
                Enabled = value.Enabled,
                PickingMode = value.PickingMode,
                LanguageDirection = value.LanguageDirection,
                Focusable = value.Focusable,
                TabIndex = value.TabIndex,
                DelegatesFocus = value.DelegatesFocus,
                AutoFocus = value.AutoFocus,
                Inert = value.Inert,
                Classes = value.Classes,
                Style = value.Style,
                Events = value.Events,
                EventSubscriptions = value.EventSubscriptions,
            };
            Offset<Wire.UiElement> rootElement = WriteElement(builder, root);
            IReadOnlyList<UiNode> children = value.Children ?? Array.Empty<UiNode>();
            VectorOffset rootChildren = WriteUuidVector(builder, children);
            VectorOffset nodeVector = WriteNodeVector(builder, children);
            Wire.UiDocument.StartUiDocument(builder);
            Wire.UiDocument.AddNodes(builder, nodeVector);
            Wire.UiDocument.AddRootChildIds(builder, rootChildren);
            Wire.UiDocument.AddRootElement(builder, rootElement);
            Wire.UiDocument.AddRootId(builder, Uuid(builder, value.RootId.Value));
            Wire.UiDocument.AddDocumentId(builder, Uuid(builder, value.DocumentId.Value));
            return Wire.UiDocument.EndUiDocument(builder);
        }

        private static VectorOffset WriteNodeVector(
            FlatBufferBuilder builder,
            IReadOnlyList<UiNode> roots
        )
        {
            var flattened = new List<UiNode>();
            var stack = new Stack<UiNode>();
            for (int index = roots.Count - 1; index >= 0; index--)
                stack.Push(roots[index]);
            while (stack.Count > 0)
            {
                UiNode node = stack.Pop();
                flattened.Add(node);
                IReadOnlyList<UiNode> descendants = node.Children ?? Array.Empty<UiNode>();
                for (int index = descendants.Count - 1; index >= 0; index--)
                    stack.Push(descendants[index]);
            }
            var nodes = new int[flattened.Count];
            for (int index = 0; index < nodes.Length; index++)
                nodes[index] = WriteNode(builder, flattened[index]).Value;
            return OffsetVector(builder, nodes);
        }

        private static Offset<Wire.UiNode> WriteNode(FlatBufferBuilder builder, UiNode value)
        {
            Offset<Wire.UiElement> element = WriteElement(builder, value.Element);
            VectorOffset children = WriteUuidVector(
                builder,
                value.Children ?? Array.Empty<UiNode>()
            );
            Wire.UiNode.StartUiNode(builder);
            Wire.UiNode.AddChildIds(builder, children);
            Wire.UiNode.AddElement(builder, element);
            Wire.UiNode.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            return Wire.UiNode.EndUiNode(builder);
        }

        private static Offset<Wire.UiElement> WriteElement(
            FlatBufferBuilder builder,
            UiElement value
        )
        {
            var properties = new List<int>();
            AddText(builder, properties, Wire.UiPropertyKey.Name, value.Name);
            AddBool(builder, properties, Wire.UiPropertyKey.Enabled, value.Enabled);
            AddEnum(
                builder,
                properties,
                Wire.UiPropertyKey.PickingMode,
                Wire.UiEnumCatalog.PickingMode,
                value.PickingMode
            );
            AddEnum(
                builder,
                properties,
                Wire.UiPropertyKey.LanguageDirection,
                Wire.UiEnumCatalog.LanguageDirection,
                value.LanguageDirection
            );
            AddBool(builder, properties, Wire.UiPropertyKey.Focusable, value.Focusable);
            AddInt(builder, properties, Wire.UiPropertyKey.TabIndex, value.TabIndex);
            AddBool(builder, properties, Wire.UiPropertyKey.DelegatesFocus, value.DelegatesFocus);
            AddBool(builder, properties, Wire.UiPropertyKey.AutoFocus, value.AutoFocus);
            AddBool(builder, properties, Wire.UiPropertyKey.Inert, value.Inert);
            AddTextList(builder, properties, Wire.UiPropertyKey.Classes, value.Classes);
            AddEvents(builder, properties, value.Events);
            AddStyle(builder, properties, value.Style);
            switch (value)
            {
                case UiElement.Label label:
                    AddText(builder, properties, Wire.UiPropertyKey.Text, label.Text);
                    break;
                case UiElement.TextElement text:
                    AddText(builder, properties, Wire.UiPropertyKey.Text, text.Text);
                    break;
                case UiElement.Button button:
                    AddText(builder, properties, Wire.UiPropertyKey.Text, button.Text);
                    break;
                case UiElement.Toggle toggle:
                    AddText(builder, properties, Wire.UiPropertyKey.Label, toggle.Label);
                    AddText(builder, properties, Wire.UiPropertyKey.Text, toggle.Text);
                    AddBool(builder, properties, Wire.UiPropertyKey.Value, toggle.Value);
                    break;
                default:
                    break;
            }
            VectorOffset subscriptions = WriteSubscriptions(
                builder,
                properties,
                value.EventSubscriptions
            );
            VectorOffset propertyVector = OffsetVector(builder, properties.ToArray());
            VectorOffset empty = OffsetVector(builder, Array.Empty<int>());
            uint usageHints = 0;
            foreach (UiUsageHint hint in value.UsageHints ?? Array.Empty<UiUsageHint>())
                usageHints |= 1u << (int)hint;
            return Wire.UiElement.CreateUiElement(
                builder,
                ElementKind(value),
                usageHints,
                propertyVector,
                subscriptions,
                empty
            );
        }

        private static Wire.UiElementKind ElementKind(UiElement value) =>
            value switch
            {
                UiElement.VisualElement => Wire.UiElementKind.VisualElement,
                UiElement.Flex => Wire.UiElementKind.Flex,
                UiElement.Grid => Wire.UiElementKind.Grid,
                UiElement.Stack => Wire.UiElementKind.Stack,
                UiElement.Box => Wire.UiElementKind.Box,
                UiElement.Label => Wire.UiElementKind.Label,
                UiElement.TextElement => Wire.UiElementKind.TextElement,
                UiElement.TextField => Wire.UiElementKind.TextField,
                UiElement.Toggle => Wire.UiElementKind.Toggle,
                UiElement.RadioButton => Wire.UiElementKind.RadioButton,
                UiElement.RadioButtonGroup => Wire.UiElementKind.RadioButtonGroup,
                UiElement.ToggleButtonGroup => Wire.UiElementKind.ToggleButtonGroup,
                UiElement.DropdownField => Wire.UiElementKind.DropdownField,
                UiElement.Button => Wire.UiElementKind.Button,
                UiElement.RepeatButton => Wire.UiElementKind.RepeatButton,
                UiElement.GroupBox => Wire.UiElementKind.GroupBox,
                UiElement.PopupWindow => Wire.UiElementKind.PopupWindow,
                UiElement.ScrollView => Wire.UiElementKind.ScrollView,
                UiElement.Scroller => Wire.UiElementKind.Scroller,
                UiElement.Slider => Wire.UiElementKind.Slider,
                UiElement.SliderInt => Wire.UiElementKind.SliderInt,
                UiElement.MinMaxSlider => Wire.UiElementKind.MinMaxSlider,
                UiElement.ProgressBar => Wire.UiElementKind.ProgressBar,
                UiElement.Tab => Wire.UiElementKind.Tab,
                UiElement.TabView => Wire.UiElementKind.TabView,
                UiElement.Image => Wire.UiElementKind.Image,
                _ => throw Unsupported(value),
            };

        private static void AddBool(
            FlatBufferBuilder builder,
            List<int> properties,
            Wire.UiPropertyKey key,
            Prop<bool> prop
        )
        {
            if (prop.IsUnset)
                return;
            int offset = prop.IsSet
                ? Wire.BoolPropertyValue.CreateBoolPropertyValue(builder, prop.Value).Value
                : 0;
            properties.Add(
                Property(builder, key, prop.State, Wire.UiPropertyValue.BoolPropertyValue, offset)
            );
        }

        private static void AddInt(
            FlatBufferBuilder builder,
            List<int> properties,
            Wire.UiPropertyKey key,
            Prop<int> prop
        )
        {
            if (prop.IsUnset)
                return;
            int offset = prop.IsSet
                ? Wire.IntPropertyValue.CreateIntPropertyValue(builder, prop.Value).Value
                : 0;
            properties.Add(
                Property(builder, key, prop.State, Wire.UiPropertyValue.IntPropertyValue, offset)
            );
        }

        private static void AddText(
            FlatBufferBuilder builder,
            List<int> properties,
            Wire.UiPropertyKey key,
            Prop<string> prop
        )
        {
            if (prop.IsUnset)
                return;
            int offset = prop.IsSet
                ? Wire
                    .TextPropertyValue.CreateTextPropertyValue(
                        builder,
                        builder.CreateString(prop.Value)
                    )
                    .Value
                : 0;
            properties.Add(
                Property(builder, key, prop.State, Wire.UiPropertyValue.TextPropertyValue, offset)
            );
        }

        private static void AddTextList(
            FlatBufferBuilder builder,
            List<int> properties,
            Wire.UiPropertyKey key,
            Prop<IReadOnlyList<string>> prop
        )
        {
            if (prop.IsUnset)
                return;
            int offset = 0;
            if (prop.IsSet)
            {
                var strings = new int[prop.Value.Count];
                for (int index = 0; index < strings.Length; index++)
                    strings[index] = builder.CreateString(prop.Value[index]).Value;
                VectorOffset values = OffsetVector(builder, strings);
                offset = Wire
                    .TextListPropertyValue.CreateTextListPropertyValue(builder, values)
                    .Value;
            }
            properties.Add(
                Property(
                    builder,
                    key,
                    prop.State,
                    Wire.UiPropertyValue.TextListPropertyValue,
                    offset
                )
            );
        }

        private static void AddEnum<T>(
            FlatBufferBuilder builder,
            List<int> properties,
            Wire.UiPropertyKey key,
            Wire.UiEnumCatalog catalog,
            Prop<T> prop
        )
            where T : Enum
        {
            if (prop.IsUnset)
                return;
            int offset = prop.IsSet
                ? Wire
                    .EnumPropertyValue.CreateEnumPropertyValue(
                        builder,
                        catalog,
                        Convert.ToUInt32(prop.Value)
                    )
                    .Value
                : 0;
            properties.Add(
                Property(builder, key, prop.State, Wire.UiPropertyValue.EnumPropertyValue, offset)
            );
        }

        private static void AddEvents(
            FlatBufferBuilder builder,
            List<int> properties,
            Prop<IReadOnlyList<UiEventKind>> prop
        )
        {
            if (prop.IsUnset)
                return;
            int offset = 0;
            if (prop.IsSet)
            {
                var events = new uint[prop.Value.Count];
                for (int index = 0; index < events.Length; index++)
                    events[index] = (uint)prop.Value[index];
                VectorOffset values = WriteUIntVector(builder, events);
                offset = Wire
                    .UIntListPropertyValue.CreateUIntListPropertyValue(builder, values)
                    .Value;
            }
            properties.Add(
                Property(
                    builder,
                    Wire.UiPropertyKey.Events,
                    prop.State,
                    Wire.UiPropertyValue.UIntListPropertyValue,
                    offset
                )
            );
        }

        private static VectorOffset WriteSubscriptions(
            FlatBufferBuilder builder,
            List<int> properties,
            Prop<IReadOnlyList<UiEventSubscription>> prop
        )
        {
            if (prop.IsUnset)
                return OffsetVector(builder, Array.Empty<int>());
            properties.Add(
                Property(
                    builder,
                    Wire.UiPropertyKey.EventSubscriptions,
                    prop.State,
                    Wire.UiPropertyValue.NONE,
                    0
                )
            );
            if (!prop.IsSet)
                return OffsetVector(builder, Array.Empty<int>());
            var values = new int[prop.Value.Count];
            for (int index = 0; index < values.Length; index++)
            {
                UiEventSubscription subscription = prop.Value[index];
                byte phase = subscription.Phase switch
                {
                    UiEventPhase.Trickle => 1,
                    UiEventPhase.Target => 2,
                    UiEventPhase.Bubble => 4,
                    _ => throw Unsupported(subscription),
                };
                values[index] = Wire
                    .UiEventSubscriptionValue.CreateUiEventSubscriptionValue(
                        builder,
                        (Wire.UiSubscriptionKind)subscription.Kind,
                        phase
                    )
                    .Value;
            }
            return OffsetVector(builder, values);
        }

        private static int Property(
            FlatBufferBuilder builder,
            Wire.UiPropertyKey key,
            PropState state,
            Wire.UiPropertyValue type,
            int offset,
            Wire.StyleValueKind styleKind = Wire.StyleValueKind.Value
        ) =>
            Wire
                .UiProperty.CreateUiProperty(
                    builder,
                    key,
                    (Wire.PropState)state,
                    styleKind,
                    state == PropState.Set ? type : Wire.UiPropertyValue.NONE,
                    state == PropState.Set ? offset : 0
                )
                .Value;

        private static VectorOffset WriteUuidVector(
            FlatBufferBuilder builder,
            IReadOnlyList<UiNode> nodes
        )
        {
            builder.StartVector(16, nodes.Count, 1);
            for (int index = nodes.Count - 1; index >= 0; index--)
                Uuid(builder, nodes[index].ObjectId.Value);
            return builder.EndVector();
        }

        private static VectorOffset WriteUIntVector(FlatBufferBuilder builder, uint[] values)
        {
            builder.StartVector(4, values.Length, 4);
            for (int index = values.Length - 1; index >= 0; index--)
                builder.AddUint(values[index]);
            return builder.EndVector();
        }
    }
}
