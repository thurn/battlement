#nullable enable

using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class BattlementLogViewerTests
    {
        [Test]
        public void LogDialogFollowsGrowthUntilTheScrollbarIsUsed()
        {
            var parent = new GameObject("Battlement Log Viewer Test");
            try
            {
                using var dialog = new BattlementLogDialog(parent.transform);
                ScrollView scroll = dialog.Details.GetFirstAncestorOfType<ScrollView>();
                dialog.Show();

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
                scroll.verticalScroller.value = 40;
                scroll.verticalScroller.highValue = 200;
                dialog.ScrollToBottom();
                Assert.That(scroll.verticalScroller.value, Is.EqualTo(40));

                dialog.Hide();
                dialog.Show();
                dialog.ScrollToBottom();
                Assert.That(scroll.verticalScroller.value, Is.EqualTo(200));
            }
            finally
            {
                Object.DestroyImmediate(parent);
            }
        }
    }
}
