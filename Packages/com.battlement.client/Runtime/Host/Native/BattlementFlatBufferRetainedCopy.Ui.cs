#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement
{
    internal static partial class BattlementFlatBufferRetainedCopy
    {
        internal static UiDocument UiDocumentRoot(Wire.UiDocument value)
        {
            UiElement root = UiElement(
                value.RootElement ?? throw Missing("UI document root element")
            );
            if (root is not Battlement.UiElement.VisualElement)
                throw new InvalidDataException("A UI document root must be a visual element.");
            if (
                root.UsageHints is not null
                || !root.Paint.IsUnset
                || !root.Motion.IsUnset
                || !root.GridItem.IsUnset
                || !root.StackItem.IsUnset
                || !root.Sticky.IsUnset
                || !root.OverlayPlacement.IsUnset
            )
                throw new InvalidDataException(
                    "A UI document root contains properties that its document representation "
                        + "cannot retain."
                );
            return new UiDocument(
                new ObjectId(Uuid(value.DocumentId, "UI document")),
                new ObjectId(Uuid(value.RootId, "UI document root")),
                root.Name,
                root.Enabled,
                root.PickingMode,
                root.LanguageDirection,
                root.Focusable,
                root.TabIndex,
                root.DelegatesFocus,
                root.Classes,
                root.Style,
                root.Events,
                Array.Empty<UiNode>(),
                root.EventSubscriptions,
                root.AutoFocus,
                root.Inert
            );
        }

        internal static UiElement UiElement(Wire.UiElement value)
        {
            // Concrete sparse property retention is centralized at this lifetime boundary.
            var properties = new UiProperties(value);
            UiElement element = properties.Concrete(value.Kind);
            return properties.Common(element, value.UsageHints);
        }

        private sealed partial class UiProperties
        {
            private readonly Dictionary<Wire.UiPropertyKey, Wire.UiProperty> values;
            private readonly Wire.UiElement source;

            internal UiProperties(Wire.UiElement element)
            {
                source = element;
                values = new Dictionary<Wire.UiPropertyKey, Wire.UiProperty>(
                    element.PropertiesLength
                );
                for (int index = 0; index < element.PropertiesLength; index++)
                {
                    Wire.UiProperty property =
                        element.Properties(index) ?? throw Missing("UI property");
                    if (!values.TryAdd(property.Key, property))
                        throw new InvalidDataException("A UI element repeats a property key.");
                }
            }

            private UiProperties(
                Wire.UiElement element,
                Dictionary<Wire.UiPropertyKey, Wire.UiProperty> properties
            )
            {
                source = element;
                values = properties;
            }

            internal UiElement Concrete(Wire.UiElementKind kind) =>
                kind switch
                {
                    Wire.UiElementKind.VisualElement => new UiElement.VisualElement(),
                    Wire.UiElementKind.Flex => new UiElement.Flex
                    {
                        Direction = Enum<UiFlexDirection>(
                            Wire.UiPropertyKey.Direction,
                            Wire.UiEnumCatalog.FlexDirection
                        ),
                        Wrap = Enum<UiFlexWrap>(
                            Wire.UiPropertyKey.Wrap,
                            Wire.UiEnumCatalog.FlexWrap
                        ),
                        AlignItems = Enum<UiAlign>(
                            Wire.UiPropertyKey.AlignItems,
                            Wire.UiEnumCatalog.Align
                        ),
                        JustifyContent = Enum<UiJustify>(
                            Wire.UiPropertyKey.JustifyContent,
                            Wire.UiEnumCatalog.Justify
                        ),
                        RowGap = Float(Wire.UiPropertyKey.RowGap),
                        ColumnGap = Float(Wire.UiPropertyKey.ColumnGap),
                    },
                    Wire.UiElementKind.Grid => new UiElement.Grid
                    {
                        Columns = GridTracks(Wire.UiPropertyKey.Columns),
                        Rows = GridTracks(Wire.UiPropertyKey.Rows),
                        AutoColumns = GridTrack(Wire.UiPropertyKey.AutoColumns),
                        AutoRows = GridTrack(Wire.UiPropertyKey.AutoRows),
                        AutoFlow = Enum<GridAutoFlow>(
                            Wire.UiPropertyKey.AutoFlow,
                            Wire.UiEnumCatalog.GridAutoFlow
                        ),
                        RowGap = Float(Wire.UiPropertyKey.RowGap),
                        ColumnGap = Float(Wire.UiPropertyKey.ColumnGap),
                        AlignItems = Enum<UiAlign>(
                            Wire.UiPropertyKey.AlignItems,
                            Wire.UiEnumCatalog.Align
                        ),
                        JustifyItems = Enum<UiAlign>(
                            Wire.UiPropertyKey.JustifyItems,
                            Wire.UiEnumCatalog.Align
                        ),
                    },
                    Wire.UiElementKind.Stack => new UiElement.Stack
                    {
                        AlignItems = Enum<UiAlign>(
                            Wire.UiPropertyKey.AlignItems,
                            Wire.UiEnumCatalog.Align
                        ),
                        JustifyItems = Enum<UiAlign>(
                            Wire.UiPropertyKey.JustifyItems,
                            Wire.UiEnumCatalog.Align
                        ),
                    },
                    Wire.UiElementKind.Box => new UiElement.Box(),
                    Wire.UiElementKind.Label => Textual(new UiElement.Label()),
                    Wire.UiElementKind.TextElement => Textual(new UiElement.TextElement()),
                    Wire.UiElementKind.TextField => new UiElement.TextField
                    {
                        Label = Text(Wire.UiPropertyKey.Label),
                        Value = Text(Wire.UiPropertyKey.Value),
                        Multiline = Bool(Wire.UiPropertyKey.Multiline),
                        VerticalScrollerVisibility = Enum<UiScrollerVisibility>(
                            Wire.UiPropertyKey.VerticalScrollerVisibility,
                            Wire.UiEnumCatalog.ScrollerVisibility
                        ),
                        Password = Bool(Wire.UiPropertyKey.Password),
                        ReadOnly = Bool(Wire.UiPropertyKey.ReadOnly),
                        Placeholder = Text(Wire.UiPropertyKey.Placeholder),
                        HidePlaceholderOnFocus = Bool(Wire.UiPropertyKey.HidePlaceholderOnFocus),
                        CursorIndex = UInt(Wire.UiPropertyKey.CursorIndex),
                        SelectIndex = UInt(Wire.UiPropertyKey.SelectIndex),
                        SelectAllOnFocus = Bool(Wire.UiPropertyKey.SelectAllOnFocus),
                        SelectAllOnMouseUp = Bool(Wire.UiPropertyKey.SelectAllOnMouseUp),
                    },
                    Wire.UiElementKind.Toggle => new UiElement.Toggle
                    {
                        Label = Text(Wire.UiPropertyKey.Label),
                        Text = Text(Wire.UiPropertyKey.Text),
                        Value = Bool(Wire.UiPropertyKey.Value),
                    },
                    Wire.UiElementKind.RadioButton => new UiElement.RadioButton
                    {
                        Label = Text(Wire.UiPropertyKey.Label),
                        Text = Text(Wire.UiPropertyKey.Text),
                        Value = Bool(Wire.UiPropertyKey.Value),
                    },
                    Wire.UiElementKind.RadioButtonGroup => new UiElement.RadioButtonGroup
                    {
                        Label = Text(Wire.UiPropertyKey.Label),
                        Choices = TextList(Wire.UiPropertyKey.Choices),
                        SelectedIndex = UInt(Wire.UiPropertyKey.SelectedIndex),
                    },
                    Wire.UiElementKind.ToggleButtonGroup => new UiElement.ToggleButtonGroup
                    {
                        Label = Text(Wire.UiPropertyKey.Label),
                        MultipleSelection = Bool(Wire.UiPropertyKey.MultipleSelection),
                        AllowEmptySelection = Bool(Wire.UiPropertyKey.AllowEmptySelection),
                        SelectedIndices = UIntList(Wire.UiPropertyKey.SelectedIndices),
                    },
                    Wire.UiElementKind.DropdownField => new UiElement.DropdownField
                    {
                        Label = Text(Wire.UiPropertyKey.Label),
                        ShowMixedValue = Bool(Wire.UiPropertyKey.ShowMixedValue),
                        Choices = TextList(Wire.UiPropertyKey.Choices),
                        Selection = Choice(Wire.UiPropertyKey.Selection),
                    },
                    Wire.UiElementKind.Button => new UiElement.Button
                    {
                        Text = Text(Wire.UiPropertyKey.Text),
                        EnableRichText = Bool(Wire.UiPropertyKey.EnableRichText),
                        EmojiFallbackSupport = Bool(Wire.UiPropertyKey.EmojiFallbackSupport),
                        ParseEscapeSequences = Bool(Wire.UiPropertyKey.ParseEscapeSequences),
                        DisplayTooltipWhenElided = Bool(
                            Wire.UiPropertyKey.DisplayTooltipWhenElided
                        ),
                        Icon = Icon(Wire.UiPropertyKey.Icon),
                    },
                    Wire.UiElementKind.RepeatButton => new UiElement.RepeatButton
                    {
                        Text = Text(Wire.UiPropertyKey.Text),
                        DelayMs = UInt(Wire.UiPropertyKey.DelayMs),
                        IntervalMs = UInt(Wire.UiPropertyKey.IntervalMs),
                        EnableRichText = Bool(Wire.UiPropertyKey.EnableRichText),
                        EmojiFallbackSupport = Bool(Wire.UiPropertyKey.EmojiFallbackSupport),
                        ParseEscapeSequences = Bool(Wire.UiPropertyKey.ParseEscapeSequences),
                        DisplayTooltipWhenElided = Bool(
                            Wire.UiPropertyKey.DisplayTooltipWhenElided
                        ),
                    },
                    Wire.UiElementKind.GroupBox => new UiElement.GroupBox
                    {
                        Text = Text(Wire.UiPropertyKey.Text),
                    },
                    Wire.UiElementKind.PopupWindow => Textual(new UiElement.PopupWindow()),
                    Wire.UiElementKind.ScrollView => new UiElement.ScrollView
                    {
                        Mode = Enum<UiScrollViewMode>(
                            Wire.UiPropertyKey.Mode,
                            Wire.UiEnumCatalog.ScrollViewMode
                        ),
                        NestedInteraction = Enum<UiNestedInteraction>(
                            Wire.UiPropertyKey.NestedInteraction,
                            Wire.UiEnumCatalog.NestedInteraction
                        ),
                        HorizontalScrollerVisibility = Enum<UiScrollerVisibility>(
                            Wire.UiPropertyKey.HorizontalScrollerVisibility,
                            Wire.UiEnumCatalog.ScrollerVisibility
                        ),
                        VerticalScrollerVisibility = Enum<UiScrollerVisibility>(
                            Wire.UiPropertyKey.VerticalScrollerVisibility,
                            Wire.UiEnumCatalog.ScrollerVisibility
                        ),
                        ScrollOffset = Vector(Wire.UiPropertyKey.ScrollOffset),
                        HorizontalPageSize = Float(Wire.UiPropertyKey.HorizontalPageSize),
                        VerticalPageSize = Float(Wire.UiPropertyKey.VerticalPageSize),
                        MouseWheelScrollSize = Float(Wire.UiPropertyKey.MouseWheelScrollSize),
                        TouchScrollBehavior = Enum<UiTouchScrollBehavior>(
                            Wire.UiPropertyKey.TouchScrollBehavior,
                            Wire.UiEnumCatalog.TouchScrollBehavior
                        ),
                        ScrollDecelerationRate = Float(Wire.UiPropertyKey.ScrollDecelerationRate),
                        Elasticity = Float(Wire.UiPropertyKey.Elasticity),
                        ElasticAnimationInterval = UInt(
                            Wire.UiPropertyKey.ElasticAnimationInterval
                        ),
                    },
                    Wire.UiElementKind.Scroller => new UiElement.Scroller
                    {
                        LowValue = Float(Wire.UiPropertyKey.LowValue),
                        HighValue = Float(Wire.UiPropertyKey.HighValue),
                        Direction = Enum<UiSliderDirection>(
                            Wire.UiPropertyKey.Direction,
                            Wire.UiEnumCatalog.SliderDirection
                        ),
                        Value = Float(Wire.UiPropertyKey.Value),
                    },
                    Wire.UiElementKind.Slider => new UiElement.Slider
                    {
                        Label = Text(Wire.UiPropertyKey.Label),
                        LowValue = Float(Wire.UiPropertyKey.LowValue),
                        HighValue = Float(Wire.UiPropertyKey.HighValue),
                        Value = Float(Wire.UiPropertyKey.Value),
                        Fill = Bool(Wire.UiPropertyKey.Fill),
                        PageSize = Float(Wire.UiPropertyKey.PageSize),
                        ShowInputField = Bool(Wire.UiPropertyKey.ShowInputField),
                        Direction = Enum<UiSliderDirection>(
                            Wire.UiPropertyKey.Direction,
                            Wire.UiEnumCatalog.SliderDirection
                        ),
                        Inverted = Bool(Wire.UiPropertyKey.Inverted),
                    },
                    Wire.UiElementKind.SliderInt => new UiElement.SliderInt
                    {
                        Label = Text(Wire.UiPropertyKey.Label),
                        LowValue = Int(Wire.UiPropertyKey.LowValue),
                        HighValue = Int(Wire.UiPropertyKey.HighValue),
                        Value = Int(Wire.UiPropertyKey.Value),
                        Fill = Bool(Wire.UiPropertyKey.Fill),
                        PageSize = Float(Wire.UiPropertyKey.PageSize),
                        ShowInputField = Bool(Wire.UiPropertyKey.ShowInputField),
                        Direction = Enum<UiSliderDirection>(
                            Wire.UiPropertyKey.Direction,
                            Wire.UiEnumCatalog.SliderDirection
                        ),
                        Inverted = Bool(Wire.UiPropertyKey.Inverted),
                    },
                    Wire.UiElementKind.MinMaxSlider => new UiElement.MinMaxSlider
                    {
                        Label = Text(Wire.UiPropertyKey.Label),
                        MinValue = Float(Wire.UiPropertyKey.MinValue),
                        MaxValue = Float(Wire.UiPropertyKey.MaxValue),
                        LowLimit = LowerLimit(Wire.UiPropertyKey.LowLimit),
                        HighLimit = UpperLimit(Wire.UiPropertyKey.HighLimit),
                    },
                    Wire.UiElementKind.ProgressBar => new UiElement.ProgressBar
                    {
                        LowValue = Float(Wire.UiPropertyKey.LowValue),
                        HighValue = Float(Wire.UiPropertyKey.HighValue),
                        Value = Float(Wire.UiPropertyKey.Value),
                        Title = Text(Wire.UiPropertyKey.Title),
                    },
                    Wire.UiElementKind.Tab => new UiElement.Tab
                    {
                        Text = Text(Wire.UiPropertyKey.Text),
                        Icon = Icon(Wire.UiPropertyKey.Icon),
                        Closeable = Bool(Wire.UiPropertyKey.Closeable),
                    },
                    Wire.UiElementKind.TabView => new UiElement.TabView
                    {
                        SelectedTabIndex = UInt(Wire.UiPropertyKey.SelectedTabIndex),
                        Reorderable = Bool(Wire.UiPropertyKey.Reorderable),
                    },
                    Wire.UiElementKind.Image => new UiElement.Image
                    {
                        Source = ImageSource(Wire.UiPropertyKey.Source),
                        SourceRect = Rect(Wire.UiPropertyKey.SourceRect),
                        TintColor = Color(Wire.UiPropertyKey.TintColor),
                        ScaleMode = Enum<ImageScaleMode>(
                            Wire.UiPropertyKey.ScaleMode,
                            Wire.UiEnumCatalog.ImageScaleMode
                        ),
                        Uv = Rect(Wire.UiPropertyKey.Uv),
                    },
                    _ => throw new InvalidDataException("Unknown UI element kind."),
                };

            private UiElement.Label Textual(UiElement.Label value) =>
                value with
                {
                    Text = Text(Wire.UiPropertyKey.Text),
                    EnableRichText = Bool(Wire.UiPropertyKey.EnableRichText),
                    EmojiFallbackSupport = Bool(Wire.UiPropertyKey.EmojiFallbackSupport),
                    ParseEscapeSequences = Bool(Wire.UiPropertyKey.ParseEscapeSequences),
                    DisplayTooltipWhenElided = Bool(Wire.UiPropertyKey.DisplayTooltipWhenElided),
                    Selectable = Bool(Wire.UiPropertyKey.Selectable),
                    DoubleClickSelectsWord = Bool(Wire.UiPropertyKey.DoubleClickSelectsWord),
                    TripleClickSelectsLine = Bool(Wire.UiPropertyKey.TripleClickSelectsLine),
                    SelectAllOnFocus = Bool(Wire.UiPropertyKey.SelectAllOnFocus),
                    SelectAllOnMouseUp = Bool(Wire.UiPropertyKey.SelectAllOnMouseUp),
                };

            private UiElement.TextElement Textual(UiElement.TextElement value) =>
                value with
                {
                    Text = Text(Wire.UiPropertyKey.Text),
                    EnableRichText = Bool(Wire.UiPropertyKey.EnableRichText),
                    EmojiFallbackSupport = Bool(Wire.UiPropertyKey.EmojiFallbackSupport),
                    ParseEscapeSequences = Bool(Wire.UiPropertyKey.ParseEscapeSequences),
                    DisplayTooltipWhenElided = Bool(Wire.UiPropertyKey.DisplayTooltipWhenElided),
                    Selectable = Bool(Wire.UiPropertyKey.Selectable),
                    DoubleClickSelectsWord = Bool(Wire.UiPropertyKey.DoubleClickSelectsWord),
                    TripleClickSelectsLine = Bool(Wire.UiPropertyKey.TripleClickSelectsLine),
                    SelectAllOnFocus = Bool(Wire.UiPropertyKey.SelectAllOnFocus),
                    SelectAllOnMouseUp = Bool(Wire.UiPropertyKey.SelectAllOnMouseUp),
                };

            private UiElement.PopupWindow Textual(UiElement.PopupWindow value) =>
                value with
                {
                    Text = Text(Wire.UiPropertyKey.Text),
                    EnableRichText = Bool(Wire.UiPropertyKey.EnableRichText),
                    EmojiFallbackSupport = Bool(Wire.UiPropertyKey.EmojiFallbackSupport),
                    ParseEscapeSequences = Bool(Wire.UiPropertyKey.ParseEscapeSequences),
                    DisplayTooltipWhenElided = Bool(Wire.UiPropertyKey.DisplayTooltipWhenElided),
                    Selectable = Bool(Wire.UiPropertyKey.Selectable),
                    DoubleClickSelectsWord = Bool(Wire.UiPropertyKey.DoubleClickSelectsWord),
                    TripleClickSelectsLine = Bool(Wire.UiPropertyKey.TripleClickSelectsLine),
                    SelectAllOnFocus = Bool(Wire.UiPropertyKey.SelectAllOnFocus),
                    SelectAllOnMouseUp = Bool(Wire.UiPropertyKey.SelectAllOnMouseUp),
                };

            internal UiElement Common(UiElement element, uint usageHints)
            {
                element = element with
                {
                    Name = Text(Wire.UiPropertyKey.Name),
                    Enabled = Bool(Wire.UiPropertyKey.Enabled),
                    PickingMode = Enum<UiPickingMode>(
                        Wire.UiPropertyKey.PickingMode,
                        Wire.UiEnumCatalog.PickingMode
                    ),
                    LanguageDirection = Enum<UiLanguageDirection>(
                        Wire.UiPropertyKey.LanguageDirection,
                        Wire.UiEnumCatalog.LanguageDirection
                    ),
                    Focusable = Bool(Wire.UiPropertyKey.Focusable),
                    TabIndex = Int(Wire.UiPropertyKey.TabIndex),
                    DelegatesFocus = Bool(Wire.UiPropertyKey.DelegatesFocus),
                    AutoFocus = Bool(Wire.UiPropertyKey.AutoFocus),
                    Inert = Bool(Wire.UiPropertyKey.Inert),
                    Classes = TextList(Wire.UiPropertyKey.Classes),
                    UsageHints = UsageHints(usageHints),
                    Style = Style(),
                    Paint = PaintProperty(Wire.UiPropertyKey.Paint),
                    Events = EventKinds(Wire.UiPropertyKey.Events),
                    EventSubscriptions = EventSubscriptions(),
                    Motion = MotionProperty(Wire.UiPropertyKey.Motion),
                    GridItem = GridItemProperty(Wire.UiPropertyKey.GridItem),
                    StackItem = StackItemProperty(Wire.UiPropertyKey.StackItem),
                    Sticky = StickyProperty(Wire.UiPropertyKey.Sticky),
                    OverlayPlacement = OverlayPlacementProperty(
                        Wire.UiPropertyKey.OverlayPlacement
                    ),
                };
                element = Parts(element);
                if (values.Count != 0)
                {
                    foreach (Wire.UiPropertyKey key in values.Keys)
                    {
                        throw new InvalidDataException(
                            $"UI property {key} is not admitted by its element kind."
                        );
                    }
                }
                return element;
            }

            private UiElement Parts(UiElement element)
            {
                if (source.PartStylesLength == 0)
                    return element;
                var parts = new UiPartStyle[source.PartStylesLength];
                for (int index = 0; index < parts.Length; index++)
                {
                    Wire.PartStyle sourcePart =
                        source.PartStyles(index) ?? throw Missing("UI part style");
                    var partValues = new Dictionary<Wire.UiPropertyKey, Wire.UiProperty>(
                        sourcePart.PropertiesLength
                    );
                    for (
                        int propertyIndex = 0;
                        propertyIndex < sourcePart.PropertiesLength;
                        propertyIndex++
                    )
                    {
                        Wire.UiProperty property =
                            sourcePart.Properties(propertyIndex)
                            ?? throw Missing("UI part property");
                        if (!partValues.TryAdd(property.Key, property))
                            throw new InvalidDataException("A UI part repeats a style property.");
                    }
                    var reader = new UiProperties(default, partValues);
                    UiStyle style = reader.Style() ?? new UiStyle();
                    if (reader.values.Count != 0)
                        throw new InvalidDataException("A UI part contains a non-style property.");
                    parts[index] = new UiPartStyle((UiPart)(ushort)sourcePart.Part, style)
                    {
                        Index = sourcePart.Index,
                    };
                }
                return element switch
                {
                    UiElement.TextField value => value with { Parts = parts },
                    UiElement.Toggle value => value with { Parts = parts },
                    UiElement.RadioButton value => value with { Parts = parts },
                    UiElement.RadioButtonGroup value => value with { Parts = parts },
                    UiElement.ToggleButtonGroup value => value with { Parts = parts },
                    UiElement.DropdownField value => value with { Parts = parts },
                    UiElement.Button value => value with { Parts = parts },
                    UiElement.GroupBox value => value with { Parts = parts },
                    UiElement.PopupWindow value => value with { Parts = parts },
                    UiElement.ScrollView value => value with { Parts = parts },
                    UiElement.Scroller value => value with { Parts = parts },
                    UiElement.Slider value => value with { Parts = parts },
                    UiElement.SliderInt value => value with { Parts = parts },
                    UiElement.MinMaxSlider value => value with { Parts = parts },
                    UiElement.ProgressBar value => value with { Parts = parts },
                    UiElement.Tab value => value with { Parts = parts },
                    UiElement.TabView value => value with { Parts = parts },
                    _ => throw new InvalidDataException(
                        "This UI element kind cannot own part styles."
                    ),
                };
            }

            private static IReadOnlyList<UiUsageHint>? UsageHints(uint mask)
            {
                if (mask == 0)
                    return null;
                if ((mask & ~0x0fU) != 0)
                    throw new InvalidDataException("UI usage hints contain unknown bits.");
                var result = new List<UiUsageHint>(4);
                for (int bit = 0; bit < 4; bit++)
                    if ((mask & (1U << bit)) != 0)
                        result.Add((UiUsageHint)bit);
                return result;
            }

            private Prop<bool> Bool(Wire.UiPropertyKey key) =>
                Read(
                    key,
                    Wire.UiPropertyValue.BoolPropertyValue,
                    value => value.ValueAsBoolPropertyValue().Value
                );

            private Prop<int> Int(Wire.UiPropertyKey key) =>
                Read(
                    key,
                    Wire.UiPropertyValue.IntPropertyValue,
                    value => value.ValueAsIntPropertyValue().Value
                );

            private Prop<uint> UInt(Wire.UiPropertyKey key) =>
                Read(
                    key,
                    Wire.UiPropertyValue.UIntPropertyValue,
                    value => value.ValueAsUIntPropertyValue().Value
                );

            private Prop<float> Float(Wire.UiPropertyKey key) =>
                Read(
                    key,
                    Wire.UiPropertyValue.FloatPropertyValue,
                    value => value.ValueAsFloatPropertyValue().Value
                );

            private Prop<string> Text(Wire.UiPropertyKey key) =>
                Read(
                    key,
                    Wire.UiPropertyValue.TextPropertyValue,
                    value => Required(value.ValueAsTextPropertyValue().Value, "UI text property")
                );

            private Prop<IReadOnlyList<string>> TextList(Wire.UiPropertyKey key) =>
                Read(
                    key,
                    Wire.UiPropertyValue.TextListPropertyValue,
                    value =>
                    {
                        Wire.TextListPropertyValue list = value.ValueAsTextListPropertyValue();
                        var result = new string[list.ValuesLength];
                        for (int index = 0; index < result.Length; index++)
                            result[index] = Required(list.Values(index), "UI text-list item");
                        return (IReadOnlyList<string>)result;
                    }
                );

            private Prop<IReadOnlyList<uint>> UIntList(Wire.UiPropertyKey key) =>
                Read(
                    key,
                    Wire.UiPropertyValue.UIntListPropertyValue,
                    value =>
                    {
                        Wire.UIntListPropertyValue list = value.ValueAsUIntListPropertyValue();
                        var result = new uint[list.ValuesLength];
                        for (int index = 0; index < result.Length; index++)
                            result[index] = list.Values(index);
                        return (IReadOnlyList<uint>)result;
                    }
                );

            private Prop<Vector> Vector(Wire.UiPropertyKey key) =>
                Read(
                    key,
                    Wire.UiPropertyValue.Vector2PropertyValue,
                    value =>
                    {
                        Wire.Vector2d item =
                            value.ValueAsVector2PropertyValue().Value
                            ?? throw Missing("UI vector property");
                        return new Vector((float)item.X, (float)item.Y);
                    }
                );

            private Prop<Rect> Rect(Wire.UiPropertyKey key) =>
                Read(
                    key,
                    Wire.UiPropertyValue.RectPropertyValue,
                    value =>
                    {
                        Wire.Rectd item =
                            value.ValueAsRectPropertyValue().Value
                            ?? throw Missing("UI rectangle property");
                        return new Rect(item.X, item.Y, item.Width, item.Height);
                    }
                );

            private Prop<Color> Color(Wire.UiPropertyKey key) =>
                Read(
                    key,
                    Wire.UiPropertyValue.ColorPropertyValue,
                    value =>
                        Rgba(
                            value.ValueAsColorPropertyValue().Value
                                ?? throw Missing("UI color property")
                        )
                );

            private Prop<DropdownChoice> Choice(Wire.UiPropertyKey key) =>
                Read(
                    key,
                    Wire.UiPropertyValue.ChoicePropertyValue,
                    value =>
                    {
                        Wire.ChoicePropertyValue item = value.ValueAsChoicePropertyValue();
                        return item.Kind switch
                        {
                            Wire.ChoiceKind.None => DropdownChoice.None(),
                            Wire.ChoiceKind.Index => DropdownChoice.Selected(
                                item.Index,
                                Required(item.Value, "dropdown value")
                            ),
                            _ => throw new InvalidDataException("Unknown dropdown choice kind."),
                        };
                    }
                );

            private Prop<LowerLimit> LowerLimit(Wire.UiPropertyKey key) =>
                Read<LowerLimit>(
                    key,
                    Wire.UiPropertyValue.LimitPropertyValue,
                    value =>
                    {
                        Wire.LimitPropertyValue item = value.ValueAsLimitPropertyValue();
                        return item.Kind switch
                        {
                            Wire.LimitKind.Unbounded => new LowerLimit.Unbounded(),
                            Wire.LimitKind.Inclusive => new LowerLimit.Inclusive(item.Value),
                            _ => throw new InvalidDataException("Unknown lower-limit kind."),
                        };
                    }
                );

            private Prop<UpperLimit> UpperLimit(Wire.UiPropertyKey key) =>
                Read<UpperLimit>(
                    key,
                    Wire.UiPropertyValue.LimitPropertyValue,
                    value =>
                    {
                        Wire.LimitPropertyValue item = value.ValueAsLimitPropertyValue();
                        return item.Kind switch
                        {
                            Wire.LimitKind.Unbounded => new UpperLimit.Unbounded(),
                            Wire.LimitKind.Inclusive => new UpperLimit.Inclusive(item.Value),
                            _ => throw new InvalidDataException("Unknown upper-limit kind."),
                        };
                    }
                );

            private Prop<IReadOnlyList<GridTrack>> GridTracks(Wire.UiPropertyKey key) =>
                Read(
                    key,
                    Wire.UiPropertyValue.GridTracksPropertyValue,
                    value =>
                    {
                        Wire.GridTracksPropertyValue list = value.ValueAsGridTracksPropertyValue();
                        var result = new GridTrack[list.ValuesLength];
                        for (int index = 0; index < result.Length; index++)
                        {
                            result[index] = GridTrack(
                                list.Values(index) ?? throw Missing("grid track")
                            );
                        }
                        return (IReadOnlyList<GridTrack>)result;
                    }
                );

            private Prop<GridTrack> GridTrack(Wire.UiPropertyKey key) =>
                Read(
                    key,
                    Wire.UiPropertyValue.GridTracksPropertyValue,
                    value =>
                    {
                        Wire.GridTracksPropertyValue list = value.ValueAsGridTracksPropertyValue();
                        if (list.ValuesLength != 1)
                            throw new InvalidDataException(
                                "A grid-track property must contain one track."
                            );
                        return GridTrack(list.Values(0) ?? throw Missing("grid track"));
                    }
                );

            private static GridTrack GridTrack(Wire.GridTrackValue value) =>
                value.Kind switch
                {
                    Wire.GridTrackKind.Pixels => new GridTrack.Px(value.Value),
                    Wire.GridTrackKind.Fraction => new GridTrack.Fraction(value.Value),
                    Wire.GridTrackKind.Auto => new GridTrack.Auto(),
                    _ => throw new InvalidDataException("Unknown grid-track kind."),
                };

            private Prop<IconSource> Icon(Wire.UiPropertyKey key) =>
                Read<IconSource>(
                    key,
                    Wire.UiPropertyValue.AssetPropertyValue,
                    value =>
                    {
                        Wire.AssetPropertyValue item = value.ValueAsAssetPropertyValue();
                        string address = Required(item.Address, "icon address");
                        return item.Kind switch
                        {
                            Wire.AssetSourceKind.Texture => new IconSource.Texture(
                                new TextureAddress(address)
                            ),
                            Wire.AssetSourceKind.Sprite => new IconSource.Sprite(
                                new SpriteAddress(address)
                            ),
                            Wire.AssetSourceKind.VectorImage => new IconSource.VectorImage(
                                new VectorImageAddress(address)
                            ),
                            Wire.AssetSourceKind.RenderTexture => new IconSource.RenderTexture(
                                new RenderTextureAddress(address)
                            ),
                            _ => throw new InvalidDataException("Invalid icon asset kind."),
                        };
                    }
                );

            private Prop<ImageSource> ImageSource(Wire.UiPropertyKey key) =>
                Read<ImageSource>(
                    key,
                    Wire.UiPropertyValue.AssetPropertyValue,
                    value =>
                    {
                        Wire.AssetPropertyValue item = value.ValueAsAssetPropertyValue();
                        string address = Required(item.Address, "image source address");
                        return item.Kind switch
                        {
                            Wire.AssetSourceKind.Texture => new ImageSource.Texture(
                                new TextureAddress(address)
                            ),
                            Wire.AssetSourceKind.Sprite => new ImageSource.Sprite(
                                new SpriteAddress(address)
                            ),
                            Wire.AssetSourceKind.VectorImage => new ImageSource.VectorImage(
                                new VectorImageAddress(address)
                            ),
                            Wire.AssetSourceKind.RenderTexture => new ImageSource.RenderTexture(
                                new RenderTextureAddress(address)
                            ),
                            _ => throw new InvalidDataException("Invalid image asset kind."),
                        };
                    }
                );

            private Prop<T> Enum<T>(Wire.UiPropertyKey key, Wire.UiEnumCatalog catalog)
                where T : struct, System.Enum =>
                Read(
                    key,
                    Wire.UiPropertyValue.EnumPropertyValue,
                    value =>
                    {
                        Wire.EnumPropertyValue item = value.ValueAsEnumPropertyValue();
                        if (item.Catalog != catalog)
                            throw new InvalidDataException(
                                "A UI enum property uses the wrong catalog."
                            );
                        return (T)System.Enum.ToObject(typeof(T), item.Value);
                    }
                );

            private Prop<T> Read<T>(
                Wire.UiPropertyKey key,
                Wire.UiPropertyValue expected,
                Func<Wire.UiProperty, T> read
            )
            {
                if (!values.Remove(key, out Wire.UiProperty value))
                    return default;
                if (value.State == Wire.PropState.Reset)
                {
                    if (value.ValueType != Wire.UiPropertyValue.NONE)
                        throw new InvalidDataException("A reset UI property carries a value.");
                    return Prop<T>.Reset();
                }
                if (value.State != Wire.PropState.Set || value.ValueType != expected)
                    throw new InvalidDataException("A UI property state and value do not match.");
                return Prop<T>.Set(read(value));
            }

            private Prop<IReadOnlyList<UiEventSubscription>> EventSubscriptions()
            {
                if (
                    !values.Remove(
                        Wire.UiPropertyKey.EventSubscriptions,
                        out Wire.UiProperty marker
                    )
                )
                {
                    if (source.EventSubscriptionsLength != 0)
                        throw new InvalidDataException("UI subscriptions have no property marker.");
                    return default;
                }
                if (marker.ValueType != Wire.UiPropertyValue.NONE)
                    throw new InvalidDataException("A UI subscription marker carries a value.");
                if (marker.State == Wire.PropState.Reset)
                {
                    if (source.EventSubscriptionsLength != 0)
                        throw new InvalidDataException("Reset UI subscriptions carry entries.");
                    return Prop<IReadOnlyList<UiEventSubscription>>.Reset();
                }
                if (marker.State != Wire.PropState.Set)
                    throw new InvalidDataException(
                        "A UI subscription marker has an invalid state."
                    );
                var result = new List<UiEventSubscription>(source.EventSubscriptionsLength);
                for (int index = 0; index < source.EventSubscriptionsLength; index++)
                {
                    Wire.UiEventSubscriptionValue item =
                        source.EventSubscriptions(index) ?? throw Missing("UI event subscription");
                    if (item.Phases == 0 || (item.Phases & ~0x07) != 0)
                        throw new InvalidDataException(
                            "A UI event subscription contains invalid phase bits."
                        );
                    for (int bit = 0; bit < 3; bit++)
                    {
                        if ((item.Phases & (1 << bit)) != 0)
                        {
                            result.Add(
                                new UiEventSubscription(
                                    (UiEventKind)(byte)item.Kind,
                                    (UiEventPhase)bit
                                )
                            );
                        }
                    }
                }
                return Prop<IReadOnlyList<UiEventSubscription>>.Set(result);
            }

            private Prop<GridItem> GridItemProperty(Wire.UiPropertyKey key) =>
                Read(
                    key,
                    Wire.UiPropertyValue.GridItemPropertyValue,
                    value =>
                    {
                        Wire.GridItemPropertyValue item = value.ValueAsGridItemPropertyValue();
                        return new GridItem(
                            item.Row,
                            item.Column,
                            item.RowSpan,
                            item.ColumnSpan,
                            (UiAlign)item.AlignSelf,
                            (UiAlign)item.JustifySelf
                        );
                    }
                );

            private Prop<StackItem> StackItemProperty(Wire.UiPropertyKey key) =>
                Read(
                    key,
                    Wire.UiPropertyValue.StackItemPropertyValue,
                    value =>
                    {
                        Wire.StackItemPropertyValue item = value.ValueAsStackItemPropertyValue();
                        return new StackItem(
                            item.Order,
                            (UiAlign)item.AlignSelf,
                            (UiAlign)item.JustifySelf,
                            item.Top,
                            item.Right,
                            item.Bottom,
                            item.Left,
                            item.ContributesToSize
                        );
                    }
                );

            private Prop<Sticky> StickyProperty(Wire.UiPropertyKey key) =>
                Read(
                    key,
                    Wire.UiPropertyValue.StickyPropertyValue,
                    value =>
                    {
                        Wire.StickyPropertyValue item = value.ValueAsStickyPropertyValue();
                        return new Sticky(item.Top, item.Right, item.Bottom, item.Left, item.Order);
                    }
                );

            private Prop<OverlayPlacement> OverlayPlacementProperty(Wire.UiPropertyKey key) =>
                Read<OverlayPlacement>(
                    key,
                    Wire.UiPropertyValue.OverlayPlacementPropertyValue,
                    value =>
                    {
                        Wire.OverlayPlacementPropertyValue item =
                            value.ValueAsOverlayPlacementPropertyValue();
                        return item.Kind switch
                        {
                            Wire.OverlayPlacementKind.PopoverLayer => new OverlayPlacement.Layer(
                                OverlayLayer.Popover
                            ),
                            Wire.OverlayPlacementKind.ModalLayer => new OverlayPlacement.Layer(
                                OverlayLayer.Modal
                            ),
                            Wire.OverlayPlacementKind.Popover when item.Anchor.HasValue =>
                                new OverlayPlacement.Popover(
                                    ObjectId(item.Anchor),
                                    new PopoverPlacement(
                                        (PlacementSide)(byte)item.Side,
                                        (PlacementAlign)(byte)item.Align,
                                        item.MainOffset,
                                        item.CrossOffset,
                                        item.CollisionPadding,
                                        item.Flip,
                                        item.Shift
                                    )
                                ),
                            Wire.OverlayPlacementKind.Modal => new OverlayPlacement.Modal(
                                OptionalObjectId(item.InitialFocus),
                                OptionalObjectId(item.RestoreFocus)
                            ),
                            _ => throw new InvalidDataException("Invalid overlay placement."),
                        };
                    }
                );

            private Prop<IReadOnlyList<UiEventKind>> EventKinds(Wire.UiPropertyKey key) =>
                Read(
                    key,
                    Wire.UiPropertyValue.UIntListPropertyValue,
                    value =>
                    {
                        Wire.UIntListPropertyValue list = value.ValueAsUIntListPropertyValue();
                        var result = new UiEventKind[list.ValuesLength];
                        for (int index = 0; index < result.Length; index++)
                            result[index] = (UiEventKind)list.Values(index);
                        return (IReadOnlyList<UiEventKind>)result;
                    }
                );
        }
    }
}
