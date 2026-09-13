#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using Google.FlatBuffers;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement
{
    internal sealed class BattlementUiEventBodyWriter
    {
        private readonly FlatBufferBuilder builder;

        internal BattlementUiEventBodyWriter(FlatBufferBuilder builder) => this.builder = builder;

        internal BattlementUiEventBodyOffset Write(UiEventBody body) =>
            body switch
            {
                UiEventBody.AccessibilityAction value => Result(
                    UiEventKind.AccessibilityAction,
                    Wire.UiEventBody.AccessibilityActionEvent,
                    Accessibility(value.Value).Value
                ),
                UiEventBody.PointerDown value => Result(
                    UiEventKind.PointerDown,
                    Wire.UiEventBody.PointerButtonEvent,
                    PointerButton(value.Value).Value
                ),
                UiEventBody.PointerMove value => Result(
                    UiEventKind.PointerMove,
                    Wire.UiEventBody.PointerMoveEvent,
                    PointerMove(value.Value).Value
                ),
                UiEventBody.PointerUp value => Result(
                    UiEventKind.PointerUp,
                    Wire.UiEventBody.PointerButtonEvent,
                    PointerButton(value.Value).Value
                ),
                UiEventBody.PointerCancel value => Result(
                    UiEventKind.PointerCancel,
                    Wire.UiEventBody.PointerCancelEvent,
                    PointerCancel(value.Value).Value
                ),
                UiEventBody.Click value => Result(
                    UiEventKind.Click,
                    Wire.UiEventBody.ClickEvent,
                    Click(value.Value).Value
                ),
                UiEventBody.PointerEnter value => Boundary(UiEventKind.PointerEnter, value.Value),
                UiEventBody.PointerLeave value => Boundary(UiEventKind.PointerLeave, value.Value),
                UiEventBody.PointerOver value => Crossing(UiEventKind.PointerOver, value.Value),
                UiEventBody.PointerOut value => Crossing(UiEventKind.PointerOut, value.Value),
                UiEventBody.Wheel value => Result(
                    UiEventKind.Wheel,
                    Wire.UiEventBody.WheelEvent,
                    Wheel(value.Value).Value
                ),
                UiEventBody.PointerCapture value => Capture(
                    UiEventKind.PointerCapture,
                    value.Value
                ),
                UiEventBody.PointerCaptureOut value => Capture(
                    UiEventKind.PointerCaptureOut,
                    value.Value
                ),
                UiEventBody.KeyDown value => Key(UiEventKind.KeyDown, value.Value),
                UiEventBody.KeyUp value => Key(UiEventKind.KeyUp, value.Value),
                UiEventBody.NavigationMove value => Result(
                    UiEventKind.NavigationMove,
                    Wire.UiEventBody.NavigationMoveEvent,
                    Navigation(value.Value).Value
                ),
                UiEventBody.NavigationCancel => Empty(UiEventKind.NavigationCancel),
                UiEventBody.FocusIn value => Focus(UiEventKind.FocusIn, value.Value),
                UiEventBody.Focus value => Focus(UiEventKind.Focus, value.Value),
                UiEventBody.FocusOut value => Focus(UiEventKind.FocusOut, value.Value),
                UiEventBody.Blur value => Focus(UiEventKind.Blur, value.Value),
                UiEventBody.GeometryChanged value => Result(
                    UiEventKind.GeometryChanged,
                    Wire.UiEventBody.GeometryEvent,
                    Geometry(value.Value).Value
                ),
                UiEventBody.AttachToPanel => Empty(UiEventKind.AttachToPanel),
                UiEventBody.DetachFromPanel => Empty(UiEventKind.DetachFromPanel),
                UiEventBody.TransitionStart value => Transition(
                    UiEventKind.TransitionStart,
                    value.Value
                ),
                UiEventBody.TransitionEnd value => Transition(
                    UiEventKind.TransitionEnd,
                    value.Value
                ),
                UiEventBody.TransitionCancel value => Transition(
                    UiEventKind.TransitionCancel,
                    value.Value
                ),
                UiEventBody.ValueChanging value => ValueChanging(value.Value),
                UiEventBody.ValueCommitted value => ValueCommitted(value.Value),
                UiEventBody.Input value => Text(UiEventKind.Input, value.Value.Value),
                UiEventBody.SelectionChanged value => Result(
                    UiEventKind.SelectionChanged,
                    Wire.UiEventBody.SelectionEvent,
                    Wire.SelectionEvent.CreateSelectionEvent(
                        builder,
                        value.Value.CursorIndex,
                        value.Value.SelectionIndex
                    ).Value
                ),
                UiEventBody.LinkEnter value => Link(UiEventKind.LinkEnter, value.Value),
                UiEventBody.LinkLeave value => Link(UiEventKind.LinkLeave, value.Value),
                UiEventBody.LinkDown value => Link(UiEventKind.LinkDown, value.Value),
                UiEventBody.LinkUp value => Link(UiEventKind.LinkUp, value.Value),
                UiEventBody.ScrollSettled value => Scroll(UiEventKind.ScrollSettled, value.Value),
                UiEventBody.ScrollChanged value => Scroll(UiEventKind.ScrollChanged, value.Value),
                UiEventBody.TabSelectionRequested value => TabSelection(value.Value),
                UiEventBody.TabCloseRequested value => TabClose(value.Value),
                UiEventBody.TabReorderRequested value => TabReorder(value.Value),
                _ => throw new InvalidDataException(
                    $"Unsupported UI event body {body.GetType().Name}."
                ),
            };

        private Offset<Wire.AccessibilityActionEvent> Accessibility(
            AccessibilityActionEvent value
        ) =>
            Wire.AccessibilityActionEvent.CreateAccessibilityActionEvent(
                builder,
                value.BackendGeneration,
                value.Action switch
                {
                    UiAccessibilityAction.Activate => Wire.AccessibilityActionKind.Activate,
                    UiAccessibilityAction.Increment => Wire.AccessibilityActionKind.Increment,
                    UiAccessibilityAction.Decrement => Wire.AccessibilityActionKind.Decrement,
                    UiAccessibilityAction.Dismiss => Wire.AccessibilityActionKind.Dismiss,
                    UiAccessibilityAction.ScrollForward =>
                        Wire.AccessibilityActionKind.ScrollForward,
                    UiAccessibilityAction.ScrollBackward =>
                        Wire.AccessibilityActionKind.ScrollBackward,
                    _ => throw new InvalidDataException("Unknown accessibility action."),
                }
            );

        private Offset<Wire.PointerButtonEvent> PointerButton(UiPointerButtonEvent value)
        {
            Offset<Wire.PointerButton> button = Button(value.Button);
            Wire.PointerButtonEvent.StartPointerButtonEvent(builder);
            Wire.PointerButtonEvent.AddPointerId(builder, value.PointerId);
            if (value.Button is not null)
                Wire.PointerButtonEvent.AddButton(builder, button);
            Wire.PointerButtonEvent.AddButtons(builder, value.Buttons);
            Wire.PointerButtonEvent.AddPressure(builder, Finite(value.Pressure));
            Wire.PointerButtonEvent.AddClickCount(builder, value.ClickCount);
            Wire.PointerButtonEvent.AddModifiers(builder, Modifiers(value.Modifiers));
            Wire.PointerButtonEvent.AddPointerType(builder, PointerType(value.PointerType));
            Wire.PointerButtonEvent.AddDelta(
                builder,
                Wire.Vector2.CreateVector2(builder, Finite(value.Delta.X), Finite(value.Delta.Y))
            );
            Wire.PointerButtonEvent.AddPosition(
                builder,
                Wire.PanelPoint.CreatePanelPoint(
                    builder,
                    Finite(value.Position.X),
                    Finite(value.Position.Y)
                )
            );
            return Wire.PointerButtonEvent.EndPointerButtonEvent(builder);
        }

        private Offset<Wire.PointerMoveEvent> PointerMove(UiPointerMoveEvent value)
        {
            Offset<Wire.PointerButton> button = Button(value.ChangedButton);
            Wire.PointerMoveEvent.StartPointerMoveEvent(builder);
            Wire.PointerMoveEvent.AddPointerId(builder, value.PointerId);
            if (value.ChangedButton is not null)
                Wire.PointerMoveEvent.AddChangedButton(builder, button);
            Wire.PointerMoveEvent.AddButtons(builder, value.Buttons);
            Wire.PointerMoveEvent.AddPressure(builder, Finite(value.Pressure));
            Wire.PointerMoveEvent.AddClickCount(builder, value.ClickCount);
            Wire.PointerMoveEvent.AddModifiers(builder, Modifiers(value.Modifiers));
            Wire.PointerMoveEvent.AddPointerType(builder, PointerType(value.PointerType));
            Wire.PointerMoveEvent.AddDelta(
                builder,
                Wire.Vector2.CreateVector2(builder, Finite(value.Delta.X), Finite(value.Delta.Y))
            );
            Wire.PointerMoveEvent.AddPosition(
                builder,
                Wire.PanelPoint.CreatePanelPoint(
                    builder,
                    Finite(value.Position.X),
                    Finite(value.Position.Y)
                )
            );
            return Wire.PointerMoveEvent.EndPointerMoveEvent(builder);
        }

        private Offset<Wire.PointerCancelEvent> PointerCancel(UiPointerCancelEvent value)
        {
            Wire.PointerCancelEvent.StartPointerCancelEvent(builder);
            Wire.PointerCancelEvent.AddPointerId(builder, value.PointerId);
            Wire.PointerCancelEvent.AddButtons(builder, value.Buttons);
            Wire.PointerCancelEvent.AddPressure(builder, Finite(value.Pressure));
            Wire.PointerCancelEvent.AddModifiers(builder, Modifiers(value.Modifiers));
            Wire.PointerCancelEvent.AddPointerType(builder, PointerType(value.PointerType));
            Wire.PointerCancelEvent.AddDelta(
                builder,
                Wire.Vector2.CreateVector2(builder, Finite(value.Delta.X), Finite(value.Delta.Y))
            );
            Wire.PointerCancelEvent.AddPosition(
                builder,
                Wire.PanelPoint.CreatePanelPoint(
                    builder,
                    Finite(value.Position.X),
                    Finite(value.Position.Y)
                )
            );
            return Wire.PointerCancelEvent.EndPointerCancelEvent(builder);
        }

        private BattlementUiEventBodyOffset Boundary(UiEventKind kind, UiPointerBoundaryEvent value)
        {
            Wire.PointerBoundaryEvent.StartPointerBoundaryEvent(builder);
            Wire.PointerBoundaryEvent.AddPointerId(builder, value.PointerId);
            Wire.PointerBoundaryEvent.AddPointerType(builder, PointerType(value.PointerType));
            Wire.PointerBoundaryEvent.AddPosition(
                builder,
                Wire.PanelPoint.CreatePanelPoint(
                    builder,
                    Finite(value.Position.X),
                    Finite(value.Position.Y)
                )
            );
            return Result(
                kind,
                Wire.UiEventBody.PointerBoundaryEvent,
                Wire.PointerBoundaryEvent.EndPointerBoundaryEvent(builder).Value
            );
        }

        private BattlementUiEventBodyOffset Crossing(UiEventKind kind, UiPointerCrossingEvent value)
        {
            Wire.PointerCrossingEvent.StartPointerCrossingEvent(builder);
            Wire.PointerCrossingEvent.AddPointerId(builder, value.PointerId);
            Wire.PointerCrossingEvent.AddPointerType(builder, PointerType(value.PointerType));
            Wire.PointerCrossingEvent.AddPosition(
                builder,
                Wire.PanelPoint.CreatePanelPoint(
                    builder,
                    Finite(value.Position.X),
                    Finite(value.Position.Y)
                )
            );
            if (value.RelatedTargetId.HasValue)
                Wire.PointerCrossingEvent.AddRelatedTargetId(
                    builder,
                    WriteUuid(value.RelatedTargetId.Value.Value)
                );
            return Result(
                kind,
                Wire.UiEventBody.PointerCrossingEvent,
                Wire.PointerCrossingEvent.EndPointerCrossingEvent(builder).Value
            );
        }

        private Offset<Wire.WheelEvent> Wheel(UiWheelEvent value)
        {
            Wire.WheelEvent.StartWheelEvent(builder);
            Wire.WheelEvent.AddModifiers(builder, Modifiers(value.Modifiers));
            Wire.WheelEvent.AddDelta(
                builder,
                Wire.Vector3.CreateVector3(
                    builder,
                    Finite(value.Delta.X),
                    Finite(value.Delta.Y),
                    Finite(value.Delta.Z)
                )
            );
            Wire.WheelEvent.AddPosition(
                builder,
                Wire.PanelPoint.CreatePanelPoint(
                    builder,
                    Finite(value.Position.X),
                    Finite(value.Position.Y)
                )
            );
            return Wire.WheelEvent.EndWheelEvent(builder);
        }

        private BattlementUiEventBodyOffset Capture(
            UiEventKind kind,
            UiPointerCaptureEvent value
        ) =>
            Result(
                kind,
                Wire.UiEventBody.PointerCaptureEvent,
                Wire.PointerCaptureEvent.CreatePointerCaptureEvent(builder, value.PointerId).Value
            );

        private BattlementUiEventBodyOffset Key(UiEventKind kind, UiKeyEvent value)
        {
            if (
                value.PhysicalKey.HasValue
                && !Enum.IsDefined(typeof(PhysicalKey), value.PhysicalKey.Value)
            )
                throw new InvalidDataException("Unknown physical key.");
            StringOffset text = builder.CreateString(
                value.Text ?? throw new InvalidDataException("Key text cannot be null.")
            );
            return Result(
                kind,
                Wire.UiEventBody.KeyEvent,
                Wire.KeyEvent.CreateKeyEvent(
                    builder,
                    value.PhysicalKey.HasValue,
                    (Wire.PhysicalKey)(value.PhysicalKey ?? 0),
                    text,
                    Modifiers(value.Modifiers)
                ).Value
            );
        }

        private Offset<Wire.NavigationMoveEvent> Navigation(UiNavigationMoveEvent value)
        {
            if (!Enum.IsDefined(typeof(UiNavigationDirection), value.Direction))
                throw new InvalidDataException("Unknown navigation direction.");
            Wire.NavigationMoveEvent.StartNavigationMoveEvent(builder);
            Wire.NavigationMoveEvent.AddDirection(
                builder,
                checked((Wire.NavigationDirection)value.Direction)
            );
            Wire.NavigationMoveEvent.AddMove(
                builder,
                Wire.Vector2.CreateVector2(builder, Finite(value.Move.X), Finite(value.Move.Y))
            );
            return Wire.NavigationMoveEvent.EndNavigationMoveEvent(builder);
        }

        private BattlementUiEventBodyOffset Focus(UiEventKind kind, UiFocusEvent value)
        {
            Wire.FocusDirectionKind direction = value.Direction switch
            {
                null => Wire.FocusDirectionKind.Absent,
                UiFocusDirection.None => Wire.FocusDirectionKind.None,
                UiFocusDirection.Unspecified => Wire.FocusDirectionKind.Unspecified,
                UiFocusDirection.Left => Wire.FocusDirectionKind.Left,
                UiFocusDirection.Right => Wire.FocusDirectionKind.Right,
                UiFocusDirection.Other => Wire.FocusDirectionKind.Other,
                _ => throw new InvalidDataException("Unknown focus direction."),
            };
            int other = value.Direction is UiFocusDirection.Other item ? item.Value : 0;
            Wire.FocusEvent.StartFocusEvent(builder);
            Wire.FocusEvent.AddDirection(builder, direction);
            Wire.FocusEvent.AddOtherDirection(builder, other);
            if (value.RelatedTargetId.HasValue)
                Wire.FocusEvent.AddRelatedTargetId(
                    builder,
                    WriteUuid(value.RelatedTargetId.Value.Value)
                );
            Offset<Wire.FocusEvent> result = Wire.FocusEvent.EndFocusEvent(builder);
            return Result(kind, Wire.UiEventBody.FocusEvent, result.Value);
        }

        private Offset<Wire.GeometryEvent> Geometry(GeometryEvent value)
        {
            Wire.GeometryEvent.StartGeometryEvent(builder);
            Wire.GeometryEvent.AddCurrent(
                builder,
                Wire.Rect.CreateRect(
                    builder,
                    Finite(value.Current.X),
                    Finite(value.Current.Y),
                    Finite(value.Current.Width),
                    Finite(value.Current.Height)
                )
            );
            Wire.GeometryEvent.AddPrevious(
                builder,
                Wire.Rect.CreateRect(
                    builder,
                    Finite(value.Previous.X),
                    Finite(value.Previous.Y),
                    Finite(value.Previous.Width),
                    Finite(value.Previous.Height)
                )
            );
            return Wire.GeometryEvent.EndGeometryEvent(builder);
        }

        private BattlementUiEventBodyOffset Transition(UiEventKind kind, TransitionEvent value)
        {
            if (value.Properties.Count == 0)
                throw new InvalidDataException("Transition properties cannot be empty.");
            var seen = new HashSet<UiTransitionProperty>();
            foreach (UiTransitionProperty property in value.Properties)
            {
                if (!Enum.IsDefined(typeof(UiTransitionProperty), property))
                    throw new InvalidDataException("Unknown transition property.");
                if (!seen.Add(property))
                    throw new InvalidDataException("Duplicate transition property.");
            }
            Wire.TransitionEvent.StartPropertiesVector(builder, value.Properties.Count);
            for (int index = value.Properties.Count - 1; index >= 0; index--)
                builder.AddUshort(checked((ushort)value.Properties[index]));
            VectorOffset properties = builder.EndVector();
            return Result(
                kind,
                Wire.UiEventBody.TransitionEvent,
                Wire.TransitionEvent.CreateTransitionEvent(
                    builder,
                    properties,
                    Finite(value.ElapsedMs)
                ).Value
            );
        }

        private BattlementUiEventBodyOffset ValueChanging(ValueChangingEvent value)
        {
            UiValueOffset proposed = UiValue(value.Proposed);
            return Result(
                UiEventKind.ValueChanging,
                Wire.UiEventBody.ValueChangingEvent,
                Wire.ValueChangingEvent.CreateValueChangingEvent(
                    builder,
                    proposed.Type,
                    proposed.Offset
                ).Value
            );
        }

        private BattlementUiEventBodyOffset ValueCommitted(ValueCommitEvent value)
        {
            UiValueOffset previous = UiValue(value.Previous);
            UiValueOffset proposed = UiValue(value.Proposed);
            return Result(
                UiEventKind.ValueCommitted,
                Wire.UiEventBody.ValueCommitEvent,
                Wire.ValueCommitEvent.CreateValueCommitEvent(
                    builder,
                    previous.Type,
                    previous.Offset,
                    proposed.Type,
                    proposed.Offset
                ).Value
            );
        }

        private UiValueOffset UiValue(Battlement.UiValue value)
        {
            return value switch
            {
                Battlement.UiValue.Bool item => new(
                    Wire.UiValue.BoolValue,
                    Wire.BoolValue.CreateBoolValue(builder, item.Value).Value
                ),
                Battlement.UiValue.Index item => Index(item.Value),
                Battlement.UiValue.Indices item => Indices(item.Value),
                Battlement.UiValue.Choice item => Choice(item.Value),
                Battlement.UiValue.F32 item => new(
                    Wire.UiValue.F32Value,
                    Wire.F32Value.CreateF32Value(builder, Finite(item.Value)).Value
                ),
                Battlement.UiValue.I32 item => new(
                    Wire.UiValue.I32Value,
                    Wire.I32Value.CreateI32Value(builder, item.Value).Value
                ),
                Battlement.UiValue.F32Range item => Range(item.Value),
                Battlement.UiValue.String item => new(
                    Wire.UiValue.StringValue,
                    Wire.StringValue.CreateStringValue(
                        builder,
                        builder.CreateString(item.Value)
                    ).Value
                ),
                _ => throw new InvalidDataException("Unknown UI value."),
            };
        }

        private UiValueOffset Index(uint? value)
        {
            Offset<Wire.OptionalIndex> optional = Wire.OptionalIndex.CreateOptionalIndex(
                builder,
                value.HasValue,
                value ?? 0
            );
            return new(
                Wire.UiValue.IndexValue,
                Wire.IndexValue.CreateIndexValue(builder, optional).Value
            );
        }

        private UiValueOffset Indices(IReadOnlyList<uint> values)
        {
            for (int index = 1; index < values.Count; index++)
            {
                if (values[index - 1] >= values[index])
                    throw new InvalidDataException("UI indices must be sorted and unique.");
            }
            Wire.IndicesValue.StartValuesVector(builder, values.Count);
            for (int index = values.Count - 1; index >= 0; index--)
                builder.AddUint(values[index]);
            return new(
                Wire.UiValue.IndicesValue,
                Wire.IndicesValue.CreateIndicesValue(builder, builder.EndVector()).Value
            );
        }

        private UiValueOffset Choice(DropdownChoice value)
        {
            if (value.Index.HasValue != (value.Value is not null))
                throw new InvalidDataException(
                    "Dropdown index and value must be present together."
                );
            StringOffset text = value.Value is null ? default : builder.CreateString(value.Value);
            Offset<Wire.DropdownChoice> choice = Wire.DropdownChoice.CreateDropdownChoice(
                builder,
                value.Index.HasValue,
                value.Index ?? 0,
                text
            );
            return new(
                Wire.UiValue.ChoiceValue,
                Wire.ChoiceValue.CreateChoiceValue(builder, choice).Value
            );
        }

        private UiValueOffset Range(FloatRange value)
        {
            if (value.Min > value.Max)
                throw new InvalidDataException("UI range endpoints must be ordered.");
            Wire.F32RangeValue.StartF32RangeValue(builder);
            Wire.F32RangeValue.AddValue(
                builder,
                Wire.FloatRange.CreateFloatRange(builder, Finite(value.Min), Finite(value.Max))
            );
            return new(
                Wire.UiValue.F32RangeValue,
                Wire.F32RangeValue.EndF32RangeValue(builder).Value
            );
        }

        private Offset<Wire.ClickEvent> Click(ClickEvent value)
        {
            if (value is ClickEvent.Pointer pointer)
            {
                Offset<Wire.PointerButton> button = Button(pointer.Button);
                Wire.PointerClickEvent.StartPointerClickEvent(builder);
                Wire.PointerClickEvent.AddClickCount(builder, pointer.ClickCount);
                Wire.PointerClickEvent.AddPointerId(builder, pointer.PointerId);
                if (pointer.Button is not null)
                    Wire.PointerClickEvent.AddButton(builder, button);
                Wire.PointerClickEvent.AddModifiers(builder, Modifiers(pointer.Modifiers));
                Wire.PointerClickEvent.AddPosition(
                    builder,
                    Wire.PanelPoint.CreatePanelPoint(
                        builder,
                        Finite(pointer.Position.X),
                        Finite(pointer.Position.Y)
                    )
                );
                Offset<Wire.PointerClickEvent> payload =
                    Wire.PointerClickEvent.EndPointerClickEvent(builder);
                return Wire.ClickEvent.CreateClickEvent(builder, Wire.ClickKind.Pointer, payload);
            }
            Wire.ClickKind kind = value switch
            {
                ClickEvent.NavigationSubmit => Wire.ClickKind.NavigationSubmit,
                ClickEvent.Repeat => Wire.ClickKind.Repeat,
                _ => throw new InvalidDataException("Unknown click event."),
            };
            return Wire.ClickEvent.CreateClickEvent(builder, kind);
        }

        private BattlementUiEventBodyOffset Text(UiEventKind kind, string value) =>
            Result(
                kind,
                Wire.UiEventBody.TextInputEvent,
                Wire.TextInputEvent.CreateTextInputEvent(builder, builder.CreateString(value)).Value
            );

        private BattlementUiEventBodyOffset Link(UiEventKind kind, LinkEvent value)
        {
            StringOffset id = builder.CreateString(value.LinkId);
            StringOffset text = builder.CreateString(value.LinkText);
            Offset<Wire.PointerButton> button = Button(value.Button);
            Wire.LinkEvent.StartLinkEvent(builder);
            Wire.LinkEvent.AddLinkId(builder, id);
            Wire.LinkEvent.AddLinkText(builder, text);
            Wire.LinkEvent.AddPointerId(builder, value.PointerId);
            if (value.Button is not null)
                Wire.LinkEvent.AddButton(builder, button);
            Wire.LinkEvent.AddPosition(
                builder,
                Wire.PanelPoint.CreatePanelPoint(
                    builder,
                    Finite(value.Position.X),
                    Finite(value.Position.Y)
                )
            );
            return Result(
                kind,
                Wire.UiEventBody.LinkEvent,
                Wire.LinkEvent.EndLinkEvent(builder).Value
            );
        }

        private BattlementUiEventBodyOffset Scroll(UiEventKind kind, ScrollEvent value)
        {
            Wire.ScrollEvent.StartScrollEvent(builder);
            Wire.ScrollEvent.AddOffset(
                builder,
                Wire.Vector2.CreateVector2(builder, Finite(value.Offset.X), Finite(value.Offset.Y))
            );
            return Result(
                kind,
                Wire.UiEventBody.ScrollEvent,
                Wire.ScrollEvent.EndScrollEvent(builder).Value
            );
        }

        private BattlementUiEventBodyOffset TabSelection(TabSelectionEvent value)
        {
            Wire.TabSelectionEvent.StartTabSelectionEvent(builder);
            Wire.TabSelectionEvent.AddPreviousIndex(builder, value.PreviousIndex);
            Wire.TabSelectionEvent.AddProposedIndex(builder, value.ProposedIndex);
            Wire.TabSelectionEvent.AddProposedTabId(builder, WriteUuid(value.ProposedTabId.Value));
            return Result(
                UiEventKind.TabSelectionRequested,
                Wire.UiEventBody.TabSelectionEvent,
                Wire.TabSelectionEvent.EndTabSelectionEvent(builder).Value
            );
        }

        private BattlementUiEventBodyOffset TabClose(TabCloseEvent value)
        {
            Wire.TabCloseEvent.StartTabCloseEvent(builder);
            Wire.TabCloseEvent.AddIndex(builder, value.Index);
            Wire.TabCloseEvent.AddTabId(builder, WriteUuid(value.TabId.Value));
            return Result(
                UiEventKind.TabCloseRequested,
                Wire.UiEventBody.TabCloseEvent,
                Wire.TabCloseEvent.EndTabCloseEvent(builder).Value
            );
        }

        private BattlementUiEventBodyOffset TabReorder(TabReorderEvent value)
        {
            Wire.TabReorderEvent.StartTabReorderEvent(builder);
            Wire.TabReorderEvent.AddPreviousIndex(builder, value.PreviousIndex);
            Wire.TabReorderEvent.AddProposedIndex(builder, value.ProposedIndex);
            Wire.TabReorderEvent.AddTabId(builder, WriteUuid(value.TabId.Value));
            return Result(
                UiEventKind.TabReorderRequested,
                Wire.UiEventBody.TabReorderEvent,
                Wire.TabReorderEvent.EndTabReorderEvent(builder).Value
            );
        }

        private BattlementUiEventBodyOffset Empty(UiEventKind kind)
        {
            Wire.EmptyEvent.StartEmptyEvent(builder);
            return Result(
                kind,
                Wire.UiEventBody.EmptyEvent,
                Wire.EmptyEvent.EndEmptyEvent(builder).Value
            );
        }

        private Offset<Wire.PointerButton> Button(UiPointerButton? value)
        {
            Wire.PointerButtonKind kind = value switch
            {
                null => Wire.PointerButtonKind.None,
                UiPointerButton.Left => Wire.PointerButtonKind.Left,
                UiPointerButton.Middle => Wire.PointerButtonKind.Middle,
                UiPointerButton.Right => Wire.PointerButtonKind.Right,
                UiPointerButton.Other custom when custom.Value > 2 => Wire.PointerButtonKind.Other,
                UiPointerButton.Other => throw new InvalidDataException(
                    "Other pointer button must exceed two."
                ),
                _ => throw new InvalidDataException("Unknown pointer button."),
            };
            return Wire.PointerButton.CreatePointerButton(
                builder,
                kind,
                value is UiPointerButton.Other other ? other.Value : 0
            );
        }

        private static Wire.PointerType PointerType(UiPointerType value) =>
            Enum.IsDefined(typeof(UiPointerType), value)
                ? checked((Wire.PointerType)value)
                : throw new InvalidDataException("Unknown pointer type.");

        private static uint Modifiers(IReadOnlyList<KeyModifier>? values)
        {
            uint result = 0;
            foreach (KeyModifier value in values ?? Array.Empty<KeyModifier>())
            {
                if ((uint)value > 6)
                    throw new InvalidDataException("Unknown key modifier.");
                uint bit = 1u << (int)value;
                if ((result & bit) != 0)
                    throw new InvalidDataException("Duplicate key modifier.");
                result |= bit;
            }
            return result;
        }

        private Offset<Wire.Uuid> WriteUuid(Guid value)
        {
            if (value == Guid.Empty)
                throw new InvalidDataException("Protocol UUIDs must be nonzero.");
            Span<byte> mixed = stackalloc byte[16];
            value.TryWriteBytes(mixed);
            ReadOnlySpan<byte> order = stackalloc byte[16]
            {
                3,
                2,
                1,
                0,
                5,
                4,
                7,
                6,
                8,
                9,
                10,
                11,
                12,
                13,
                14,
                15,
            };
            builder.Prep(1, 16);
            for (int index = 15; index >= 0; index--)
                builder.PutByte(mixed[order[index]]);
            return new Offset<Wire.Uuid>(builder.Offset);
        }

        private static float Finite(float value) =>
            float.IsFinite(value)
                ? value
                : throw new InvalidDataException("UI event numbers must be finite.");

        private static double Finite(double value) =>
            double.IsFinite(value)
                ? value
                : throw new InvalidDataException("UI event numbers must be finite.");

        private static BattlementUiEventBodyOffset Result(
            UiEventKind kind,
            Wire.UiEventBody type,
            int offset
        ) => new(kind, type, offset);

        private readonly struct UiValueOffset
        {
            internal UiValueOffset(Wire.UiValue type, int offset) =>
                (Type, Offset) = (type, offset);

            internal Wire.UiValue Type { get; }
            internal int Offset { get; }
        }
    }
}
