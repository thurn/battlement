#nullable enable

using System;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;
using UiBox = Battlement.UiElement.Box;
using UiButton = Battlement.UiElement.Button;
using UiLabel = Battlement.UiElement.Label;
using UiVisualElement = Battlement.UiElement.VisualElement;

namespace Battlement.Tests
{
    public sealed class BattlementUiDocumentTests
    {
        [Test]
        public void PublicManagerRendersOwnedHierarchyAndLeavesAuthoredDocumentUntouched()
        {
            ObjectId documentId = Id("3b5fe431-f332-4314-a0f6-a7353fa17622");
            ObjectId rootId = Id("471834d0-8abc-4964-a3da-f8bc61de7c16");
            ObjectId containerId = Id("c21df719-965f-45d0-b018-07650a57f085");
            ObjectId boxId = Id("fc59ba64-b70c-4a20-83fd-1852b1cb4995");
            ObjectId labelId = Id("a9e0ac34-da16-4d33-8952-b6541ef075e8");
            GameObject authored = CreateAuthoredDocument();
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            var documents = new BattlementUiDocuments();
            try
            {
                UIDocument authoredDocument = authored.GetComponent<UIDocument>();
                PanelSettings authoredPanel = authoredDocument.panelSettings;
                var authoredChild = new Label("project authored");
                authoredDocument.rootVisualElement.Add(authoredChild);
                var description = new UiDocument(
                    documentId,
                    rootId,
                    "battlement-root",
                    Style: new UiStyle(BackgroundColor: new Battlement.Color(0.02, 0.05, 0.08, 1)),
                    Children: new UiNode[]
                    {
                        new(
                            containerId,
                            new UiVisualElement { Name = "plain-container" },
                            new UiNode[]
                            {
                                new(
                                    boxId,
                                    new UiBox
                                    {
                                        Name = "canvas",
                                        Style = new UiStyle(FlexDirection: UiFlexDirection.Row),
                                    },
                                    new UiNode[]
                                    {
                                        new(labelId, new UiLabel { Text = "BATTLEMENT UI" }),
                                    }
                                ),
                            }
                        ),
                    }
                );

                documents.Replace(new[] { description }, id => id == documentId ? owned : null);

                UIDocument ownedDocument = owned.GetComponent<UIDocument>();
                PanelSettings ownedPanel = ownedDocument.panelSettings;
                Assert.That(ownedDocument.rootVisualElement.childCount, Is.EqualTo(1));
                Assert.That(
                    ownedDocument.rootVisualElement[0],
                    Is.TypeOf<UnityEngine.UIElements.VisualElement>()
                );
                Assert.That(ownedDocument.rootVisualElement[0][0], Is.TypeOf<Box>());
                Assert.That(
                    ownedDocument.rootVisualElement.Q<Label>().text,
                    Is.EqualTo("BATTLEMENT UI")
                );
                Assert.That(authoredDocument.rootVisualElement[0], Is.SameAs(authoredChild));
                Assert.That(documents.TryGet(rootId, out _), Is.True);
                Assert.That(documents.TryGet(containerId, out _), Is.True);
                Assert.That(documents.TryGet(labelId, out _), Is.True);

                documents.Clear();
                Assert.That(documents.TryGet(rootId, out _), Is.False);
                Object.DestroyImmediate(owned);
                Assert.That(ownedPanel == null, Is.True);
                Assert.That(authoredDocument.panelSettings, Is.SameAs(authoredPanel));
            }
            finally
            {
                if (owned != null)
                    Object.DestroyImmediate(owned);
                Object.DestroyImmediate(authored.GetComponent<UIDocument>().panelSettings);
                Object.DestroyImmediate(authored);
            }
        }

        [Test]
        public void PublicManagerExecutesCreateUpdateParentAndDestroy()
        {
            ObjectId documentId = Id("ab6f62b4-e2e1-4d76-89f6-c6819f37b047");
            ObjectId rootId = Id("54903b68-e417-436f-a67c-fdc58ffeb6ef");
            ObjectId firstContainerId = Id("b6d20fdd-f63d-4469-97d1-05f5bb889157");
            ObjectId secondContainerId = Id("1e410dd2-c8c2-4f19-a321-914f88756942");
            ObjectId buttonId = Id("68b70492-ac1c-4f12-859c-8859b0c57fe7");
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            var documents = new BattlementUiDocuments();
            try
            {
                documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            documentId,
                            rootId,
                            Children: new UiNode[]
                            {
                                new(firstContainerId, new UiBox()),
                                new(secondContainerId, new UiBox()),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                documents.Create(
                    new CommandBody.VisualElement.Create(
                        firstContainerId,
                        new UiNode(buttonId, new UiButton { Text = "Run" })
                    )
                );
                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            buttonId,
                            new UiButton { Name = "run-command", Text = "Complete" }
                        )
                    )
                );
                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Parent(buttonId, secondContainerId)
                    )
                );

                Assert.That(documents.TryGet(buttonId, out VisualElement? value), Is.True);
                Assert.That(value, Is.TypeOf<Button>());
                Assert.That(((Button)value!).text, Is.EqualTo("Complete"));
                Assert.That(value.name, Is.EqualTo("run-command"));
                Assert.That(
                    value.parent,
                    Is.SameAs(owned.GetComponent<UIDocument>().rootVisualElement[1])
                );

                documents.Destroy(new CommandBody.VisualElement.Destroy(buttonId));
                Assert.That(documents.TryGet(buttonId, out _), Is.False);
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        private static GameObject CreateAuthoredDocument()
        {
            var gameObject = new GameObject("Project Authored Document");
            var document = gameObject.AddComponent<UIDocument>();
            document.panelSettings = ScriptableObject.CreateInstance<PanelSettings>();
            return gameObject;
        }

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
