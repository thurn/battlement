#nullable enable

using System;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class BattlementUiButtonContentTests
    {
        [Test]
        public void AuthoredChildrenSurviveCaptionUpdatesAndAcceptReparenting()
        {
            var documentId = new ObjectId(Guid.NewGuid());
            var rootId = new ObjectId(Guid.NewGuid());
            var buttonId = new ObjectId(Guid.NewGuid());
            var labelId = new ObjectId(Guid.NewGuid());
            var decorationId = new ObjectId(Guid.NewGuid());
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            try
            {
                using var documents = new BattlementUiDocuments();
                documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            documentId,
                            rootId,
                            Children: new[]
                            {
                                new UiNode(
                                    buttonId,
                                    new UiElement.Button { Text = "" },
                                    new[]
                                    {
                                        new UiNode(labelId, new UiElement.Label { Text = "High" }),
                                    }
                                ),
                                new UiNode(decorationId, new UiElement.VisualElement()),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Assert.That(documents.TryGet(buttonId, out VisualElement? host), Is.True);
                Assert.That(documents.TryGet(labelId, out VisualElement? content), Is.True);
                var button = (Button)host!;
                var label = (Label)content!;
                Assert.That(button.Contains(label), Is.True);
                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            buttonId,
                            new UiElement.Button { Text = "Quality" }
                        )
                    )
                );
                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            labelId,
                            new UiElement.Label { Text = "Low" }
                        )
                    )
                );
                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Parent(decorationId, buttonId)
                    )
                );
                Assert.That(documents.TryGet(decorationId, out VisualElement? decoration), Is.True);
                Assert.That(button.Contains(label), Is.True);
                Assert.That(button.Contains(decoration), Is.True);
                Assert.That(button.text, Is.EqualTo("Quality"));
                Assert.That(label.text, Is.EqualTo("Low"));
                Assert.That(documents.TryGet(buttonId, out VisualElement? same), Is.True);
                Assert.That(same, Is.SameAs(button));
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }
    }
}
