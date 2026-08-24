#nullable enable

using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text;
using Newtonsoft.Json.Linq;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

namespace Battlement
{
    internal sealed class BattlementLogViewer : IDisposable
    {
        private readonly BattlementLogDialog dialog;
        private readonly List<JObject> records = new();
        private ulong offset;
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
            if (dialog.IsVisible)
            {
                dialog.Hide();
                return;
            }

            records.Clear();
            offset = 0;
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
            if (!BattlementFileLogging.IsActive)
            {
                dialog.Details.text =
                    "File logging is unavailable.\n\n"
                    + (BattlementFileLogging.Failure ?? "Initialization did not complete.");
                dialog.Status.text = "Native log unavailable";
                return;
            }

            bool changed = false;
            while (true)
            {
                string chunk = BattlementFileLogging.Read(ref offset);
                if (chunk.Length == 0)
                {
                    break;
                }

                foreach (string line in chunk.Split('\n'))
                {
                    if (string.IsNullOrWhiteSpace(line))
                    {
                        continue;
                    }

                    try
                    {
                        records.Add(JObject.Parse(line));
                        changed = true;
                    }
                    catch
                    {
                        records.Add(
                            new JObject
                            {
                                ["source"] = "viewer",
                                ["severity"] = "error",
                                ["event_name"] = "battlement.log.invalid_record",
                                ["message"] = line,
                            }
                        );
                        changed = true;
                    }
                }
            }

            if (changed || records.Count == 0)
            {
                UpdateChoices();
                Render();
            }
        }

        private void UpdateChoices()
        {
            UpdateChoices(
                dialog.SourceFilter,
                records.Select(record => record.Value<string>("source"))
            );
            UpdateChoices(
                dialog.SeverityFilter,
                records.Select(record => record.Value<string>("severity"))
            );
        }

        private void Render()
        {
            string source = dialog.SourceFilter.value;
            string severity = dialog.SeverityFilter.value;
            string search = dialog.Search.value ?? string.Empty;
            JObject[] visible = records
                .Where(record => Matches(record, "source", source))
                .Where(record => Matches(record, "severity", severity))
                .Where(record => MatchesSearch(record, search))
                .ToArray();
            var text = new StringBuilder();
            foreach (JObject record in visible)
            {
                AppendRecord(text, record);
            }

            dialog.Details.text = text.Length == 0 ? "No matching log records." : text.ToString();
            dialog.Status.text =
                $"{visible.Length} of {records.Count} records  •  {BattlementFileLogging.LogPath}";
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

        private static bool Matches(JObject record, string name, string selected) =>
            selected == "All"
            || string.Equals(record.Value<string>(name), selected, StringComparison.Ordinal);

        private static bool MatchesSearch(JObject record, string search) =>
            search.Length == 0
            || record.ToString().IndexOf(search, StringComparison.OrdinalIgnoreCase) >= 0;

        private static void AppendRecord(StringBuilder text, JObject record)
        {
            long timestamp = record.Value<long?>("timestamp_unix_us") ?? 0;
            DateTimeOffset occurredAt = DateTimeOffset.FromUnixTimeMilliseconds(timestamp / 1000);
            text.Append(
                occurredAt.ToLocalTime().ToString("HH:mm:ss.fff", CultureInfo.InvariantCulture)
            );
            text.Append("  #");
            text.Append((record.Value<ulong?>("sequence") ?? 0).ToString("D6"));
            text.Append("  [");
            text.Append(record.Value<string>("source") ?? "unknown");
            text.Append('/');
            text.Append(record.Value<string>("severity") ?? "unknown");
            text.Append("]  ");
            text.Append(record.Value<string>("event_name") ?? "unknown");
            text.Append("\n  ");
            text.AppendLine(record.Value<string>("message") ?? string.Empty);

            if (record["fields"] is JObject fields)
            {
                foreach (JProperty field in fields.Properties().OrderBy(field => field.Name))
                {
                    text.Append("    ");
                    text.Append(field.Name);
                    text.Append(": ");
                    text.AppendLine(field.Value.ToString());
                }
            }

            AppendDiagnostic(text, "exception", record.Value<string>("exception"));
            AppendDiagnostic(text, "stack trace", record.Value<string>("stack_trace"));
            text.AppendLine();
        }

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
        private readonly VisualElement root;

        public BattlementLogDialog(Transform parent)
        {
            PanelSettings panelSettings = Resources.Load<PanelSettings>(PanelSettingsResource);
            if (panelSettings == null)
            {
                throw new InvalidOperationException(
                    "Battlement log viewer panel settings are missing."
                );
            }

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
            Label title = Add<Label>(content, "battlement-log-title");
            title.text = "Battlement logs";
            Close = Add<Button>(content, "battlement-log-close");
            Close.text = "×";
            Close.tooltip = "Close";

            VisualElement toolbar = Add<VisualElement>(content, "battlement-log-toolbar");
            SourceFilter = new DropdownField("Source", new List<string> { "All" }, 0);
            SourceFilter.AddToClassList("battlement-log-filter");
            toolbar.Add(SourceFilter);
            SeverityFilter = new DropdownField("Severity", new List<string> { "All" }, 0);
            SeverityFilter.AddToClassList("battlement-log-filter");
            toolbar.Add(SeverityFilter);
            Search = new TextField("Search");
            Search.AddToClassList("battlement-log-search");
            toolbar.Add(Search);

            var scroll = new ScrollView(ScrollViewMode.VerticalAndHorizontal);
            scroll.AddToClassList("battlement-log-scroll");
            content.Add(scroll);
            Details = Add<Label>(scroll, "battlement-log-details");
            Status = Add<Label>(content, "battlement-log-status");
            Hide();
        }

        public Button Close { get; }

        public DropdownField SourceFilter { get; }

        public DropdownField SeverityFilter { get; }

        public TextField Search { get; }

        public Label Details { get; }

        public Label Status { get; }

        public bool IsVisible { get; private set; }

        public void Show()
        {
            root.style.display = DisplayStyle.Flex;
            root.BringToFront();
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
                return;
            }

            Object.DestroyImmediate(host);
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
