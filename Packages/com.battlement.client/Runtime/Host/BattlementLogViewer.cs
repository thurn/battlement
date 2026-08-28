#nullable enable

using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text;
using UnityEngine;
using UnityEngine.UIElements;
using NativePanelScaleMode = UnityEngine.UIElements.PanelScaleMode;
using Object = UnityEngine.Object;

namespace Battlement
{
    internal sealed class BattlementLogViewer : IDisposable
    {
        private readonly BattlementLogDialog dialog;
        private readonly List<BattlementLogEntry> records = new();
        private ulong renderedVersion = ulong.MaxValue;
        private bool refreshRequested;
        private float nextRefresh;

        public BattlementLogViewer(Transform parent)
        {
            dialog = new BattlementLogDialog(parent);
            dialog.Close.clicked += dialog.Hide;
            dialog.SourceFilter.RegisterValueChangedCallback(_ => Render());
            dialog.SeverityFilter.RegisterValueChangedCallback(_ => Render());
            dialog.Search.RegisterValueChangedCallback(_ => Render());
        }

        public void Toggle()
        {
            SetVisible(!dialog.IsVisible);
        }

        public bool IsVisible => dialog.IsVisible;

        public void SetVisible(bool visible)
        {
            if (!visible)
            {
                dialog.Hide();
                return;
            }

            if (dialog.IsVisible)
            {
                return;
            }

            renderedVersion = ulong.MaxValue;
            dialog.Show();
            Refresh();
        }

        public void RequestRefresh() => refreshRequested = true;

        public void Update()
        {
            if (!dialog.IsVisible)
            {
                return;
            }
            if (!refreshRequested && Time.realtimeSinceStartup < nextRefresh)
            {
                return;
            }

            Refresh();
        }

        public void Dispose() => dialog.Dispose();

        private void Refresh()
        {
            refreshRequested = false;
            nextRefresh = Time.realtimeSinceStartup + 0.5f;
            BattlementLogEntry[] snapshot = BattlementLogStore.Snapshot(out ulong version);
            if (version == renderedVersion)
            {
                return;
            }

            renderedVersion = version;
            records.Clear();
            records.AddRange(snapshot);
            UpdateChoices();
            Render();
        }

        private void UpdateChoices()
        {
            UpdateChoices(dialog.SourceFilter, records.Select(record => record.Source));
            UpdateChoices(
                dialog.SeverityFilter,
                records.Select(record => SeverityName(record.Record.Severity))
            );
        }

        private void Render()
        {
            string source = dialog.SourceFilter.value;
            string severity = dialog.SeverityFilter.value;
            string search = dialog.Search.value ?? string.Empty;
            BattlementLogEntry[] visible = records
                .Where(record => source == "All" || record.Source == source)
                .Where(record =>
                    severity == "All" || SeverityName(record.Record.Severity) == severity
                )
                .Where(record => MatchesSearch(record, search))
                .ToArray();
            var text = new StringBuilder();
            foreach (BattlementLogEntry record in visible)
            {
                AppendRecord(text, record);
            }

            dialog.Details.text = text.Length == 0 ? "No matching log records." : text.ToString();
            dialog.Status.text = $"{visible.Length} of {records.Count} records";
            dialog.Status.tooltip = "Most recent records from this run.";
            dialog.ScrollToBottom();
        }

        private static void UpdateChoices(DropdownField field, IEnumerable<string?> values)
        {
            string selected = field.value;
            List<string> choices = values
                .Where(value => !string.IsNullOrWhiteSpace(value))
                .Select(value => value!)
                .Distinct(StringComparer.Ordinal)
                .OrderBy(value => value, StringComparer.Ordinal)
                .Prepend("All")
                .ToList();
            field.choices = choices;
            field.value = choices.Contains(selected) ? selected : "All";
        }

        private static bool MatchesSearch(BattlementLogEntry entry, string search)
        {
            if (search.Length == 0)
            {
                return true;
            }

            BattlementLogRecord record = entry.Record;
            string searchable =
                $"{entry.Source} {record.Severity} {record.EventName} {record.Message} "
                + string.Join(
                    " ",
                    record.Fields?.Select(field => $"{field.Key} {field.Value}")
                        ?? Enumerable.Empty<string>()
                )
                + $" {record.Exception} {record.StackTrace}";
            return searchable.IndexOf(search, StringComparison.OrdinalIgnoreCase) >= 0;
        }

        private static void AppendRecord(StringBuilder text, BattlementLogEntry entry)
        {
            BattlementLogRecord record = entry.Record;
            text.Append(
                entry
                    .OccurredAt.ToLocalTime()
                    .ToString("HH:mm:ss.fff", CultureInfo.InvariantCulture)
            );
            text.Append("  #");
            text.Append(entry.Sequence.ToString("D6"));
            text.Append("  [");
            text.Append(entry.Source);
            text.Append('/');
            text.Append(SeverityName(record.Severity));
            text.Append("]  ");
            text.Append(record.EventName);
            text.Append("\n  ");
            text.AppendLine(record.Message);

            if (record.Fields is not null)
            {
                foreach (
                    KeyValuePair<string, string> field in record.Fields.OrderBy(field => field.Key)
                )
                {
                    text.Append("    ");
                    text.Append(field.Key);
                    text.Append(": ");
                    text.AppendLine(field.Value);
                }
            }

            AppendDiagnostic(text, "exception", record.Exception?.ToString());
            AppendDiagnostic(text, "stack trace", record.StackTrace);
            text.AppendLine();
        }

        private static string SeverityName(BattlementLogSeverity severity) =>
            severity.ToString().ToLowerInvariant();

        private static void AppendDiagnostic(StringBuilder text, string label, string? value)
        {
            if (string.IsNullOrWhiteSpace(value))
            {
                return;
            }

            text.Append("    ");
            text.Append(label);
            text.AppendLine(":");
            foreach (string line in value.Split('\n'))
            {
                text.Append("      ");
                text.AppendLine(line.TrimEnd('\r'));
            }
        }
    }

    internal sealed class BattlementLogDialog : IDisposable
    {
        private const string PanelSettingsResource = "BattlementErrorPanelSettings";
        private readonly GameObject host;
        private readonly PanelSettings panelSettings;
        private readonly VisualElement root;
        private readonly ScrollView scroll;

        public BattlementLogDialog(Transform parent)
        {
            PanelSettings template = Resources.Load<PanelSettings>(PanelSettingsResource);
            if (template == null)
            {
                throw new InvalidOperationException(
                    "Battlement log viewer panel settings are missing."
                );
            }
            panelSettings = Object.Instantiate(template);
            panelSettings.scaleMode = NativePanelScaleMode.ConstantPixelSize;

            host = new GameObject("Battlement Log Viewer");
            host.SetActive(false);
            host.transform.SetParent(parent, false);
            UIDocument document = host.AddComponent<UIDocument>();
            document.panelSettings = panelSettings;
            document.sortingOrder = 9_998;
            host.SetActive(true);

            root = document.rootVisualElement;
            root.AddToClassList("battlement-log-overlay");
            VisualElement content = Add<VisualElement>(root, "battlement-log-dialog");
            VisualElement header = Add<VisualElement>(content, "battlement-log-header");
            Label title = Add<Label>(header, "battlement-log-title");
            title.text = "Battlement logs";
            Close = Add<Button>(header, "battlement-log-close");
            Close.text = "×";
            Close.tooltip = "Close";

            VisualElement toolbar = Add<VisualElement>(content, "battlement-log-toolbar");
            Search = new TextField("Search");
            Search.AddToClassList("battlement-log-search");
            toolbar.Add(Search);
            VisualElement options = Add<VisualElement>(toolbar, "battlement-log-options");
            SourceFilter = new DropdownField("Source", new List<string> { "All" }, 0);
            SourceFilter.AddToClassList("battlement-log-filter");
            options.Add(SourceFilter);
            SeverityFilter = new DropdownField("Severity", new List<string> { "All" }, 0);
            SeverityFilter.AddToClassList("battlement-log-filter");
            options.Add(SeverityFilter);
            AutoScroll = new Toggle("Auto-scroll");
            AutoScroll.AddToClassList("battlement-log-auto-scroll");
            AutoScroll.RegisterValueChangedCallback(change =>
            {
                if (change.newValue)
                {
                    ScrollToBottom();
                }
            });
            options.Add(AutoScroll);

            scroll = new ScrollView(ScrollViewMode.VerticalAndHorizontal);
            scroll.AddToClassList("battlement-log-scroll");
            HideScrollerButtons(scroll.horizontalScroller);
            HideScrollerButtons(scroll.verticalScroller);
            scroll.RegisterCallback<WheelEvent>(_ => StopFollowing());
            scroll.verticalScroller.RegisterCallback<PointerDownEvent>(_ => StopFollowing());
            content.Add(scroll);
            Details = Add<Label>(scroll, "battlement-log-details");
            Details.RegisterCallback<GeometryChangedEvent>(_ => ScheduleScrollToBottom());
            Status = Add<Label>(content, "battlement-log-status");
            Hide();
        }

        public Button Close { get; }

        public DropdownField SourceFilter { get; }

        public DropdownField SeverityFilter { get; }

        public TextField Search { get; }

        public Toggle AutoScroll { get; }

        public Label Details { get; }

        public Label Status { get; }

        public bool IsVisible { get; private set; }

        public void Show()
        {
            root.style.display = DisplayStyle.Flex;
            root.BringToFront();
            AutoScroll.SetValueWithoutNotify(true);
            IsVisible = true;
        }

        public void Hide()
        {
            root.style.display = DisplayStyle.None;
            IsVisible = false;
        }

        public void Dispose()
        {
            if (Application.isPlaying)
            {
                Object.Destroy(host);
                Object.Destroy(panelSettings);
                return;
            }

            Object.DestroyImmediate(host);
            Object.DestroyImmediate(panelSettings);
        }

        public void ScrollToBottom()
        {
            if (AutoScroll.value)
            {
                scroll.verticalScroller.value = scroll.verticalScroller.highValue;
            }
        }

        private void ScheduleScrollToBottom() => scroll.schedule.Execute(ScrollToBottom);

        private void StopFollowing() => AutoScroll.SetValueWithoutNotify(false);

        private static void HideScrollerButtons(Scroller scroller)
        {
            scroller.lowButton.style.display = DisplayStyle.None;
            scroller.highButton.style.display = DisplayStyle.None;
            scroller.style.marginTop = 0;
            scroller.style.marginRight = 0;
            scroller.style.marginBottom = 0;
            scroller.style.marginLeft = 0;
            scroller.slider.style.marginTop = 0;
            scroller.slider.style.marginRight = 0;
            scroller.slider.style.marginBottom = 0;
            scroller.slider.style.marginLeft = 0;
        }

        private static T Add<T>(VisualElement parent, string className)
            where T : VisualElement, new()
        {
            T element = new();
            element.AddToClassList(className);
            parent.Add(element);
            return element;
        }
    }
}
