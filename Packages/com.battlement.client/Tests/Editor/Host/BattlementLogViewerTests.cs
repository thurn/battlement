#nullable enable

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
    }
}
