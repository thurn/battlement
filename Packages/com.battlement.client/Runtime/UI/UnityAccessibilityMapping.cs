#nullable enable

using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using UnityEngine.Accessibility;
using NativeRole = UnityEngine.Accessibility.AccessibilityRole;
using NativeState = UnityEngine.Accessibility.AccessibilityState;

namespace Battlement.UI
{
    /// <summary>Maps canonical semantics to the pinned Unity screen-reader surface.</summary>
    internal static class UnityAccessibilityMapping
    {
        internal static void Apply(
            AccessibilityNode target,
            AccessibilityNodeSnapshot source,
            IReadOnlyDictionary<Guid, AccessibilityNodeSnapshot> snapshots
        )
        {
            target.label = Label(source, snapshots);
            target.hint = source.Hint ?? string.Empty;
            target.value = Value(source);
            target.role = Role(source.Role);
            target.state = State(source.State);
        }

        internal static NativeRole Role(SemanticRole role) =>
            role switch
            {
                SemanticRole.Button or SemanticRole.Disclosure => NativeRole.Button,
                SemanticRole.Checkbox or SemanticRole.Switch or SemanticRole.Radio =>
                    NativeRole.Toggle,
                SemanticRole.Slider => NativeRole.Slider,
                SemanticRole.TabList => NativeRole.TabBar,
                SemanticRole.Tab => NativeRole.TabButton,
                SemanticRole.Heading => NativeRole.Header,
                SemanticRole.Image => NativeRole.Image,
                SemanticRole.StaticText or SemanticRole.Progress => NativeRole.StaticText,
                SemanticRole.ScrollArea => NativeRole.ScrollView,
                SemanticRole.Link or SemanticRole.Option => NativeRole.None,
                SemanticRole.ColumnHeader or SemanticRole.RowHeader or SemanticRole.Cell =>
                    NativeRole.None,
                _ => NativeRole.Container,
            };

        internal static string Label(
            AccessibilityNodeSnapshot node,
            IReadOnlyDictionary<Guid, AccessibilityNodeSnapshot> snapshots
        )
        {
            var parts = new List<string>();
            Add(parts, node.Label);
            if (node.Role == SemanticRole.Cell)
                AddHeaders(parts, node, snapshots);
            Add(
                parts,
                node.Role switch
                {
                    SemanticRole.ListBox => "listbox",
                    SemanticRole.Option => "option",
                    SemanticRole.Table => "table",
                    SemanticRole.Row => "row",
                    SemanticRole.ColumnHeader => "column header",
                    SemanticRole.RowHeader => "row header",
                    SemanticRole.Cell => "cell",
                    SemanticRole.Link => "link",
                    SemanticRole.Navigation => "navigation",
                    SemanticRole.Region => "region",
                    _ => null,
                }
            );
            if (node.State.Current == CurrentPage.Page)
                Add(parts, "current page");
            if (node.State.Popup == PopupKind.ListBox)
                Add(parts, "listbox popup");
            if (node.State.Expanded is bool expanded)
                Add(parts, expanded ? "expanded" : "collapsed");
            return string.Join(", ", parts);
        }

        internal static string Value(AccessibilityNodeSnapshot node) =>
            node.Value?.Text
            ?? node.Value?.Current.ToString(CultureInfo.InvariantCulture)
            ?? string.Empty;

        internal static NativeState State(SemanticState state)
        {
            NativeState result = NativeState.None;
            if (state.Disabled)
                result |= NativeState.Disabled;
            if (state.Expanded == true)
                result |= NativeState.Expanded;
            if (state.Selected == true || state.Checked == CheckedState.True)
                result |= NativeState.Selected;
            return result;
        }

        private static void AddHeaders(
            List<string> parts,
            AccessibilityNodeSnapshot cell,
            IReadOnlyDictionary<Guid, AccessibilityNodeSnapshot> snapshots
        )
        {
            if (
                cell.ParentId is not ObjectId rowId
                || !snapshots.TryGetValue(rowId.Value, out var row)
            )
                return;
            int column = row.Children.ToList().IndexOf(cell.ObjectId);
            foreach (ObjectId id in row.Children)
            {
                if (
                    snapshots.TryGetValue(id.Value, out var sibling)
                    && sibling.Role == SemanticRole.RowHeader
                )
                    Add(parts, sibling.Label);
            }
            if (
                row.ParentId is not ObjectId tableId
                || !snapshots.TryGetValue(tableId.Value, out var table)
            )
                return;
            foreach (ObjectId id in table.Children)
            {
                if (!snapshots.TryGetValue(id.Value, out var headerRow))
                    continue;
                if (column < 0 || column >= headerRow.Children.Count)
                    continue;
                if (
                    snapshots.TryGetValue(headerRow.Children[column].Value, out var header)
                    && header.Role == SemanticRole.ColumnHeader
                )
                    Add(parts, header.Label);
            }
        }

        private static void Add(List<string> parts, string? value)
        {
            if (!string.IsNullOrWhiteSpace(value))
                parts.Add(value!);
        }
    }
}
