#nullable enable

using System.Linq;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class BattlementLogViewerTests
    {
        [Test]
        public void DevelopmentConsoleReplacesUnityConsoleAndOpensForErrors()
        {
            bool previous = Debug.developerConsoleEnabled;
            int openings = 0;
            try
            {
                using var console = new BattlementDevelopmentConsole(() => openings++);
                Assert.That(Debug.developerConsoleEnabled, Is.False);

                Debug.Log("informational message");
                console.Update();
                Assert.That(openings, Is.Zero);

                LogAssert.Expect(LogType.Error, "visible development error");
                Debug.LogError("visible development error");
                console.Update();
                Assert.That(openings, Is.EqualTo(1));
            }
            finally
            {
                Debug.developerConsoleEnabled = previous;
            }
        }

        [Test]
        public void LogDialogFollowsGrowthUntilTheScrollbarIsUsed()
        {
            var parent = new GameObject("Battlement Log Viewer Test");
            try
            {
                using var dialog = new BattlementLogDialog(parent.transform);
                UIDocument document = parent.GetComponentInChildren<UIDocument>(true);
                Assert.That(
                    document.panelSettings.scaleMode,
                    Is.EqualTo(UnityEngine.UIElements.PanelScaleMode.ConstantPhysicalSize)
                );
                Assert.That(document.panelSettings.sortingOrder, Is.GreaterThan(0));
                dialog.Show();
                ScrollView scroll = dialog.Details.GetFirstAncestorOfType<ScrollView>();
                Assert.That(dialog.AutoScroll.value, Is.True);

                scroll.verticalScroller.highValue = 100;
                dialog.ScrollToBottom();
                Assert.That(scroll.verticalScroller.value, Is.EqualTo(100));

                scroll.verticalScroller.highValue = 150;
                dialog.ScrollToBottom();
                Assert.That(scroll.verticalScroller.value, Is.EqualTo(150));

                using PointerDownEvent pointer = PointerDownEvent.GetPooled(
                    new Event { type = EventType.MouseDown, button = 0 }
                );
                pointer.target = scroll.verticalScroller;
                scroll.verticalScroller.SendEvent(pointer);
                Assert.That(dialog.AutoScroll.value, Is.False);
                scroll.verticalScroller.value = 40;
                scroll.verticalScroller.highValue = 200;
                dialog.ScrollToBottom();
                Assert.That(scroll.verticalScroller.value, Is.EqualTo(40));

                dialog.AutoScroll.value = true;
                Assert.That(scroll.verticalScroller.value, Is.EqualTo(200));

                dialog.AutoScroll.value = false;
                scroll.verticalScroller.highValue = 250;
                dialog.ScrollToBottom();
                Assert.That(scroll.verticalScroller.value, Is.EqualTo(200));

                dialog.Hide();
                dialog.Show();
                dialog.ScrollToBottom();
                Assert.That(dialog.AutoScroll.value, Is.True);
                Assert.That(scroll.verticalScroller.value, Is.EqualTo(250));
            }
            finally
            {
                Object.DestroyImmediate(parent);
            }
        }

        [Test]
        public void CopyButtonCopiesOnlyTheBottom250LinesOfTheFilteredView()
        {
            var parent = new GameObject("Battlement Log Viewer Copy Test");
            string previousClipboard = GUIUtility.systemCopyBuffer;
            BattlementLogStore.Clear();
            try
            {
                for (int index = 0; index < 100; index++)
                {
                    AddLog(BattlementLogSeverity.Warning, $"warning-{index:D3}");
                    AddLog(BattlementLogSeverity.Error, $"error-{index:D3}");
                }

                using var viewer = new BattlementLogViewer(parent.transform);
                viewer.SetVisible(true);
                UIDocument document = parent.GetComponentInChildren<UIDocument>(true);
                DropdownField severity = document
                    .rootVisualElement.Query<DropdownField>(className: "battlement-log-filter")
                    .ToList()
                    .Single(field => field.label == "Severity");
                severity.value = "error";
                Button copy = document.rootVisualElement.Q<Button>(
                    className: "battlement-log-copy"
                );

                Click(copy);

                string clipboard = GUIUtility.systemCopyBuffer;
                Assert.That(clipboard.Count(character => character == '\n'), Is.EqualTo(250));
                Assert.That(clipboard, Does.Contain("error-099"));
                Assert.That(clipboard, Does.Not.Contain("error-000"));
                Assert.That(clipboard, Does.Not.Contain("warning-"));
            }
            finally
            {
                BattlementLogStore.Clear();
                GUIUtility.systemCopyBuffer = previousClipboard;
                Object.DestroyImmediate(parent);
            }
        }

        private static void AddLog(BattlementLogSeverity severity, string eventName) =>
            BattlementLogStore.Add(
                "battlement",
                new BattlementLogRecord(severity, eventName, eventName)
            );

        private static void Click(VisualElement target)
        {
            using NavigationSubmitEvent submit = NavigationSubmitEvent.GetPooled();
            submit.target = target;
            target.SendEvent(submit);
        }
    }
}
