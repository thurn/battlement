#nullable enable

using System;
using System.Reflection;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class BattlementUiPanelClockTests
    {
        [Test]
        public void NativeButtonFeedbackSettlesOnlyWhenItsPresentationClockAdvances()
        {
            var documentId = new ObjectId(Guid.NewGuid());
            var rootId = new ObjectId(Guid.NewGuid());
            var buttonId = new ObjectId(Guid.NewGuid());
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            TimeSpan elapsed = TimeSpan.Zero;
            try
            {
                using var documents = new BattlementUiDocuments(
                    now: () => elapsed,
                    controlledTime: () => true
                );
                documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            documentId,
                            rootId,
                            Children: new[]
                            {
                                new UiNode(buttonId, new UiElement.Button { Text = "Inspect" }),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Assert.That(documents.TryGet(buttonId, out VisualElement? element), Is.True);
                var button = (Button)element!;
                documents.Advance();
                int activations = 0;
                button.clicked += () => activations++;

                for (int cycle = 0; cycle < 2; cycle++)
                {
                    using (NavigationSubmitEvent submit = NavigationSubmitEvent.GetPooled())
                    {
                        submit.target = button;
                        button.SendEvent(submit);
                    }
                    Assert.That(activations, Is.EqualTo(cycle + 1));
                    Assert.That(documents.DittoHasTimedSettlement, Is.True);
                    Assert.That(documents.DittoHasPendingDeferredWork, Is.True);
                    Tick(button.panel);
                    Assert.That(documents.DittoHasTimedSettlement, Is.True);
                    elapsed += TimeSpan.FromMilliseconds(99);
                    Tick(button.panel);
                    Assert.That(documents.DittoHasTimedSettlement, Is.True);
                    elapsed += TimeSpan.FromMilliseconds(1);
                    Tick(button.panel);
                    Assert.That(documents.DittoHasTimedSettlement, Is.False);
                }

                using (NavigationSubmitEvent submit = NavigationSubmitEvent.GetPooled())
                {
                    submit.target = button;
                    button.SendEvent(submit);
                }
                documents.Clear();
                Assert.That(documents.DittoHasTimedSettlement, Is.False);
                Assert.That(activations, Is.EqualTo(3));
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        private static void Tick(IPanel panel) =>
            panel
                .GetType()
                .GetMethod("TickSchedulingUpdaters", BindingFlags.Instance | BindingFlags.Public)!
                .Invoke(panel, null);
    }
}
