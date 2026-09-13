#nullable enable

using System;
using System.Collections.Generic;
using Google.FlatBuffers;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement.Tests
{
    internal static partial class BattlementFlatBufferResponseFixtures
    {
        private static Payload AccessibilityUpdate(
            FlatBufferBuilder builder,
            CommandBody.AccessibilityUpdate command
        )
        {
            AccessibilityUpdatePayload value = command.Value;
            Offset<Wire.AccessibilitySnapshot>? snapshot = value.Snapshot is null
                ? null
                : WriteAccessibilitySnapshot(builder, value.Snapshot);
            var announcements = new int[value.Announcements.Count];
            for (int index = 0; index < announcements.Length; index++)
                announcements[index] = builder.CreateString(value.Announcements[index]).Value;
            VectorOffset announcementVector = OffsetVector(builder, announcements);
            Wire.AccessibilityUpdate.StartAccessibilityUpdate(builder);
            Wire.AccessibilityUpdate.AddAnnouncements(builder, announcementVector);
            if (snapshot.HasValue)
                Wire.AccessibilityUpdate.AddSnapshot(builder, snapshot.Value);
            Offset<Wire.AccessibilityUpdate> payload =
                Wire.AccessibilityUpdate.EndAccessibilityUpdate(builder);
            return new Payload(
                Wire.CoreCommandKind.AccessibilityUpdate,
                Wire.CoreCommandPayload.AccessibilityUpdate,
                payload.Value
            );
        }

        private static Offset<Wire.AccessibilitySnapshot> WriteAccessibilitySnapshot(
            FlatBufferBuilder builder,
            AccessibilitySnapshot value
        )
        {
            VectorOffset roots = WriteObjectIdVector(builder, value.Roots);
            var nodes = new int[value.Nodes.Count];
            for (int index = 0; index < nodes.Length; index++)
                nodes[index] = WriteAccessibilityNode(builder, value.Nodes[index]).Value;
            return Wire.AccessibilitySnapshot.CreateAccessibilitySnapshot(
                builder,
                value.CommitSequence,
                roots,
                OffsetVector(builder, nodes)
            );
        }

        private static Offset<Wire.AccessibilityNodeSnapshot> WriteAccessibilityNode(
            FlatBufferBuilder builder,
            AccessibilityNodeSnapshot value
        )
        {
            VectorOffset children = WriteObjectIdVector(builder, value.Children);
            StringOffset? label = value.Label is null ? null : builder.CreateString(value.Label);
            StringOffset? hint = value.Hint is null ? null : builder.CreateString(value.Hint);
            Offset<Wire.SemanticState> state = Wire.SemanticState.CreateSemanticState(
                builder,
                value.State.Disabled,
                value.State.Checked is null ? null : (Wire.CheckedState?)value.State.Checked.Value,
                value.State.Selected,
                value.State.Expanded,
                value.State.Popup is null ? null : Wire.PopupKind.ListBox,
                value.State.Busy,
                value.State.Current is null ? null : Wire.CurrentPage.Page
            );
            Offset<Wire.AccessibilityRangeValue>? range = value.Value is null
                ? null
                : Wire.AccessibilityRangeValue.CreateAccessibilityRangeValue(
                    builder,
                    value.Value.Current,
                    value.Value.Minimum,
                    value.Value.Maximum,
                    value.Value.Text is null ? default : builder.CreateString(value.Value.Text)
                );
            IReadOnlyList<AccessibilityScrollDirection> scroll =
                value.Actions.Scroll ?? Array.Empty<AccessibilityScrollDirection>();
            var scrollBytes = new byte[scroll.Count];
            for (int index = 0; index < scrollBytes.Length; index++)
                scrollBytes[index] = (byte)scroll[index];
            Offset<Wire.AccessibilityActionSet> actions =
                Wire.AccessibilityActionSet.CreateAccessibilityActionSet(
                    builder,
                    value.Actions.Activate,
                    value.Actions.Increment,
                    value.Actions.Decrement,
                    value.Actions.Dismiss,
                    ByteVector(builder, scrollBytes)
                );
            Wire.AccessibilityNodeSnapshot.StartAccessibilityNodeSnapshot(builder);
            Wire.AccessibilityNodeSnapshot.AddScrollAxis(
                builder,
                value.ScrollAxis is null
                    ? null
                    : (Wire.AccessibilityScrollAxis?)value.ScrollAxis.Value
            );
            Wire.AccessibilityNodeSnapshot.AddHeadingLevel(builder, value.HeadingLevel);
            Wire.AccessibilityNodeSnapshot.AddActions(builder, actions);
            if (range.HasValue)
                Wire.AccessibilityNodeSnapshot.AddValue(builder, range.Value);
            Wire.AccessibilityNodeSnapshot.AddState(builder, state);
            if (hint is StringOffset hintValue)
                Wire.AccessibilityNodeSnapshot.AddHint(builder, hintValue);
            if (label is StringOffset labelValue)
                Wire.AccessibilityNodeSnapshot.AddLabel(builder, labelValue);
            Wire.AccessibilityNodeSnapshot.AddRole(builder, (Wire.SemanticRole)value.Role);
            Wire.AccessibilityNodeSnapshot.AddChildren(builder, children);
            if (value.ParentId is ObjectId parentId)
                Wire.AccessibilityNodeSnapshot.AddParentId(builder, Uuid(builder, parentId.Value));
            Wire.AccessibilityNodeSnapshot.AddObjectId(
                builder,
                Uuid(builder, value.ObjectId.Value)
            );
            return Wire.AccessibilityNodeSnapshot.EndAccessibilityNodeSnapshot(builder);
        }

        private static VectorOffset WriteObjectIdVector(
            FlatBufferBuilder builder,
            IReadOnlyList<ObjectId> values
        )
        {
            builder.StartVector(16, values.Count, 1);
            for (int index = values.Count - 1; index >= 0; index--)
                Uuid(builder, values[index].Value);
            return builder.EndVector();
        }
    }
}
