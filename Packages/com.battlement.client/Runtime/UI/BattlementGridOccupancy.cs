#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;

namespace Battlement.UI
{
    internal readonly struct BattlementGridPlacement
    {
        public BattlementGridPlacement(int row, int column, int rowSpan, int columnSpan)
        {
            Row = row;
            Column = column;
            RowSpan = rowSpan;
            ColumnSpan = columnSpan;
        }

        public int Row { get; }

        public int Column { get; }

        public int RowSpan { get; }

        public int ColumnSpan { get; }
    }

    internal readonly struct BattlementGridPlacementResult
    {
        public BattlementGridPlacementResult(
            IReadOnlyList<BattlementGridPlacement> items,
            int rows,
            int columns
        )
        {
            Items = items;
            Rows = rows;
            Columns = columns;
        }

        public IReadOnlyList<BattlementGridPlacement> Items { get; }

        public int Rows { get; }

        public int Columns { get; }
    }

    internal static class BattlementGridOccupancy
    {
        public static BattlementGridPlacementResult Place(
            IReadOnlyList<GridItem> items,
            int explicitRows,
            int explicitColumns,
            GridAutoFlow flow
        )
        {
            bool rowFlow = flow == GridAutoFlow.Row;
            AxisItem[] axisItems = items.Select(item => ToAxis(item, rowFlow)).ToArray();
            var occupied = new HashSet<(int Major, int Minor)>();
            var placements = new AxisPlacement?[items.Count];
            int explicitMajor = rowFlow ? explicitRows : explicitColumns;
            int explicitMinor = rowFlow ? explicitColumns : explicitRows;
            int minorExtent = Math.Max(1, explicitMinor);
            foreach (AxisItem item in axisItems)
            {
                minorExtent = Math.Max(minorExtent, item.MinorSpan);
                if (item.Minor is int minor)
                    minorExtent = Math.Max(minorExtent, checked(minor + item.MinorSpan));
            }

            for (int index = 0; index < axisItems.Length; index++)
            {
                AxisItem item = axisItems[index];
                if (item.Major is not int major || item.Minor is not int minor)
                    continue;
                placements[index] = new AxisPlacement(major, minor, item.MajorSpan, item.MinorSpan);
                Reserve(occupied, placements[index]!.Value);
            }

            for (int index = 0; index < axisItems.Length; index++)
            {
                AxisItem item = axisItems[index];
                if (placements[index].HasValue || item.Major is not int major)
                    continue;
                int minor = 0;
                while (
                    minor + item.MinorSpan <= minorExtent
                    && !IsFree(
                        occupied,
                        new AxisPlacement(major, minor, item.MajorSpan, item.MinorSpan)
                    )
                )
                    minor++;
                if (minor + item.MinorSpan > minorExtent)
                {
                    minor = minorExtent;
                    minorExtent = checked(minorExtent + item.MinorSpan);
                }
                placements[index] = new AxisPlacement(major, minor, item.MajorSpan, item.MinorSpan);
                Reserve(occupied, placements[index]!.Value);
            }

            int cursorMajor = 0;
            int cursorMinor = 0;
            for (int index = 0; index < axisItems.Length; index++)
            {
                if (placements[index].HasValue)
                    continue;
                AxisItem item = axisItems[index];
                if (item.Minor is int fixedMinor)
                {
                    if (fixedMinor < cursorMinor)
                        cursorMajor++;
                    cursorMinor = fixedMinor;
                    AxisPlacement candidate = new(
                        cursorMajor,
                        cursorMinor,
                        item.MajorSpan,
                        item.MinorSpan
                    );
                    while (!IsFree(occupied, candidate))
                    {
                        cursorMajor++;
                        candidate = new AxisPlacement(
                            cursorMajor,
                            cursorMinor,
                            item.MajorSpan,
                            item.MinorSpan
                        );
                    }
                    placements[index] = candidate;
                }
                else
                {
                    while (true)
                    {
                        if (cursorMinor + item.MinorSpan > minorExtent)
                        {
                            cursorMajor++;
                            cursorMinor = 0;
                        }
                        AxisPlacement candidate = new(
                            cursorMajor,
                            cursorMinor,
                            item.MajorSpan,
                            item.MinorSpan
                        );
                        if (IsFree(occupied, candidate))
                        {
                            placements[index] = candidate;
                            break;
                        }
                        cursorMinor++;
                    }
                }
                AxisPlacement placed = placements[index]!.Value;
                Reserve(occupied, placed);
                cursorMinor = placed.Minor + placed.MinorSpan;
                if (cursorMinor >= minorExtent)
                {
                    cursorMajor++;
                    cursorMinor = 0;
                }
            }

            AxisPlacement[] resolved = placements.Select(value => value!.Value).ToArray();
            int placedMajor =
                resolved.Length == 0 ? 0 : resolved.Max(value => value.Major + value.MajorSpan);
            int majorExtent = Math.Max(1, Math.Max(explicitMajor, placedMajor));
            BattlementGridPlacement[] physical = resolved
                .Select(value => ToPhysical(value, rowFlow))
                .ToArray();
            return rowFlow
                ? new BattlementGridPlacementResult(physical, majorExtent, minorExtent)
                : new BattlementGridPlacementResult(physical, minorExtent, majorExtent);
        }

        private static bool IsFree(HashSet<(int Major, int Minor)> occupied, AxisPlacement value)
        {
            for (int major = value.Major; major < value.Major + value.MajorSpan; major++)
            for (int minor = value.Minor; minor < value.Minor + value.MinorSpan; minor++)
                if (occupied.Contains((major, minor)))
                    return false;
            return true;
        }

        private static void Reserve(HashSet<(int Major, int Minor)> occupied, AxisPlacement value)
        {
            for (int major = value.Major; major < value.Major + value.MajorSpan; major++)
            for (int minor = value.Minor; minor < value.Minor + value.MinorSpan; minor++)
                occupied.Add((major, minor));
        }

        private static AxisItem ToAxis(GridItem value, bool rowFlow)
        {
            int? row = value.Row.HasValue ? checked((int)value.Row.Value - 1) : null;
            int? column = value.Column.HasValue ? checked((int)value.Column.Value - 1) : null;
            int rowSpan = checked((int)value.RowSpan);
            int columnSpan = checked((int)value.ColumnSpan);
            return rowFlow
                ? new AxisItem(row, column, rowSpan, columnSpan)
                : new AxisItem(column, row, columnSpan, rowSpan);
        }

        private static BattlementGridPlacement ToPhysical(AxisPlacement value, bool rowFlow) =>
            rowFlow
                ? new BattlementGridPlacement(
                    value.Major,
                    value.Minor,
                    value.MajorSpan,
                    value.MinorSpan
                )
                : new BattlementGridPlacement(
                    value.Minor,
                    value.Major,
                    value.MinorSpan,
                    value.MajorSpan
                );

        private readonly struct AxisItem
        {
            public AxisItem(int? major, int? minor, int majorSpan, int minorSpan)
            {
                Major = major;
                Minor = minor;
                MajorSpan = majorSpan;
                MinorSpan = minorSpan;
            }

            public int? Major { get; }

            public int? Minor { get; }

            public int MajorSpan { get; }

            public int MinorSpan { get; }
        }

        private readonly struct AxisPlacement
        {
            public AxisPlacement(int major, int minor, int majorSpan, int minorSpan)
            {
                Major = major;
                Minor = minor;
                MajorSpan = majorSpan;
                MinorSpan = minorSpan;
            }

            public int Major { get; }

            public int Minor { get; }

            public int MajorSpan { get; }

            public int MinorSpan { get; }
        }
    }
}
