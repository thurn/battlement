#nullable enable

using System;
using System.Collections.Generic;
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

        [Test]
        public void CommonPropertiesApplyBeforeAttachmentAndUpdateAtomically()
        {
            ObjectId documentId = Id("6deab132-95be-4144-abfb-8400d0cea735");
            ObjectId rootId = Id("89a74403-2228-4ee9-b90c-1c570dd1fdd8");
            ObjectId elementId = Id("ad58f5df-ea46-4eea-91d6-bce8ac117a93");
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            var documents = new BattlementUiDocuments();
            try
            {
                documents.Replace(
                    new[] { new UiDocument(documentId, rootId) },
                    id => id == documentId ? owned : null
                );
                documents.Create(
                    new CommandBody.VisualElement.Create(
                        rootId,
                        new UiNode(
                            elementId,
                            new UiBox
                            {
                                Name = "common-state",
                                Enabled = false,
                                PickingMode = UiPickingMode.Ignore,
                                LanguageDirection = UiLanguageDirection.Rtl,
                                Focusable = true,
                                TabIndex = 3,
                                DelegatesFocus = true,
                                Classes = new[] { "first", "second" },
                                UsageHints = new[]
                                {
                                    UiUsageHint.DynamicTransform,
                                    UiUsageHint.DynamicColor,
                                },
                            }
                        )
                    )
                );

                Assert.That(documents.TryGet(elementId, out VisualElement? value), Is.True);
                Assert.That(value!.name, Is.EqualTo("common-state"));
                Assert.That(value.enabledSelf, Is.False);
                Assert.That(value.pickingMode, Is.EqualTo(PickingMode.Ignore));
                Assert.That(value.languageDirection, Is.EqualTo(LanguageDirection.RTL));
                Assert.That(value.focusable, Is.True);
                Assert.That(value.tabIndex, Is.EqualTo(3));
                Assert.That(value.delegatesFocus, Is.True);
                Assert.That(value.ClassListContains("first"), Is.True);
                Assert.That(
                    value.usageHints,
                    Is.EqualTo(UsageHints.DynamicTransform | UsageHints.DynamicColor)
                );

                BattlementUiException failure = Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Properties(
                                elementId,
                                new UiBox
                                {
                                    Name = "not-applied",
                                    UsageHints = new[] { UiUsageHint.MaskContainer },
                                }
                            )
                        )
                    )
                )!;
                Assert.That(failure.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
                Assert.That(value.name, Is.EqualTo("common-state"));
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void RejectedHierarchyAndIdentityOperationsMutateNothing()
        {
            ObjectId firstDocumentId = Id("71d2bb7e-91ae-43a6-8543-b43ea3a82d70");
            ObjectId firstRootId = Id("10e81d38-2112-4366-adaf-7231265e04c9");
            ObjectId firstParentId = Id("76d9434a-1998-46df-82a2-1f6193b5f617");
            ObjectId childId = Id("c659ee18-71b5-41b2-a31a-4a06bd6bb216");
            ObjectId secondDocumentId = Id("311f037f-8574-4313-b048-41493ea09738");
            ObjectId secondRootId = Id("40711ca0-2e45-4bab-991d-09f75c0c1bb8");
            ObjectId secondParentId = Id("62264b0d-9b1e-4657-a028-8e30aa113444");
            ObjectId detachedId = Id("96ec5201-b3cd-4382-b605-15b99b682b74");
            GameObject firstOwned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(firstRootId)
            );
            GameObject secondOwned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(secondRootId)
            );
            var documents = new BattlementUiDocuments();
            try
            {
                documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            firstDocumentId,
                            firstRootId,
                            Children: new UiNode[]
                            {
                                new(
                                    firstParentId,
                                    new UiBox(),
                                    new UiNode[] { new(childId, new UiBox()) }
                                ),
                            }
                        ),
                        new UiDocument(
                            secondDocumentId,
                            secondRootId,
                            Children: new UiNode[] { new(secondParentId, new UiBox()) }
                        ),
                    },
                    id => id == firstDocumentId ? firstOwned : secondOwned
                );
                documents.TryGet(childId, out VisualElement? child);
                documents.TryGet(firstParentId, out VisualElement? firstParent);

                Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Parent(childId, secondParentId)
                        )
                    )
                );
                Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Parent(firstParentId, childId)
                        )
                    )
                );
                Assert.That(child!.parent, Is.SameAs(firstParent));

                Assert.Throws<BattlementUiException>(() =>
                    documents.Create(
                        new CommandBody.VisualElement.Create(
                            firstParentId,
                            new UiNode(
                                detachedId,
                                new UiBox(),
                                new UiNode[] { new(childId, new UiLabel { Text = "duplicate" }) }
                            )
                        )
                    )
                );
                Assert.That(documents.TryGet(detachedId, out _), Is.False);
                Assert.That(firstParent!.childCount, Is.EqualTo(1));

                documents.Destroy(new CommandBody.VisualElement.Destroy(firstParentId));
                Assert.That(documents.TryGet(firstParentId, out _), Is.False);
                Assert.That(documents.TryGet(childId, out _), Is.False);
            }
            finally
            {
                Object.DestroyImmediate(firstOwned);
                Object.DestroyImmediate(secondOwned);
            }
        }

        [Test]
        public void CrossDomainIdentitiesAreRejectedBeforeUiMutation()
        {
            ObjectId documentId = Id("e291b456-ac25-4662-aa10-4c2c486a6b01");
            ObjectId rootId = Id("89f83a78-8db5-40ad-bf50-427baa0a4ec8");
            ObjectId childId = Id("24f052f7-1678-4e9f-8529-80a6f6acb9c5");
            ObjectId worldId = Id("37dd8d3f-4a73-4087-938c-a1896c028c87");
            var used = new HashSet<Guid> { worldId.Value };
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            var documents = new BattlementUiDocuments(
                containsWorldObject: id => id == worldId.Value,
                reserveUiIdentities: ids =>
                {
                    foreach (Guid id in ids)
                    {
                        if (!used.Add(id))
                            throw new BattlementUiException(
                                CoreErrorCode.DuplicateId,
                                "Identity already belongs to a world object."
                            );
                    }
                }
            );
            try
            {
                documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            documentId,
                            rootId,
                            Children: new UiNode[] { new(childId, new UiBox()) }
                        ),
                    },
                    id => id == documentId ? owned : null
                );

                BattlementUiException duplicate = Assert.Throws<BattlementUiException>(() =>
                    documents.Create(
                        new CommandBody.VisualElement.Create(
                            rootId,
                            new UiNode(worldId, new UiBox())
                        )
                    )
                )!;
                Assert.That(duplicate.ErrorCode, Is.EqualTo(CoreErrorCode.DuplicateId));
                Assert.That(documents.TryGet(worldId, out _), Is.False);

                BattlementUiException wrongKind = Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Parent(childId, worldId)
                        )
                    )
                )!;
                Assert.That(wrongKind.ErrorCode, Is.EqualTo(CoreErrorCode.ComponentMissing));
                Assert.That(documents.TryGet(childId, out VisualElement? child), Is.True);
                Assert.That(
                    child!.parent,
                    Is.SameAs(owned.GetComponent<UIDocument>().rootVisualElement)
                );
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
