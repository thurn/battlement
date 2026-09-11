#nullable enable

using System;
using System.Collections.Generic;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using static Battlement.Tests.BattlementUiDocumentSupport;
using Object = UnityEngine.Object;
using UiBox = Battlement.UiElement.Box;
using UiGroupBox = Battlement.UiElement.GroupBox;
using UiLabel = Battlement.UiElement.Label;
using UiPopupWindow = Battlement.UiElement.PopupWindow;

namespace Battlement.Tests
{
    public sealed class BattlementUiDocumentHierarchyTests
    {
        [Test]
        public void GroupAndPopupContentSurvivesConditionalTitleUpdates()
        {
            ObjectId documentId = Id("2517c5f9-a2fa-479c-a15d-7994cf349d15");
            ObjectId rootId = Id("8a60b9d6-7ef0-4b7d-badc-0763980fef88");
            ObjectId groupId = Id("9e5d40fa-b659-4fcb-8366-5f64695d16c8");
            ObjectId groupChildId = Id("7a2a0dc3-838d-4457-ab6f-bf6cc6a55b71");
            ObjectId popupId = Id("cd077d6c-9d6d-40c8-a098-589ba9c7851e");
            ObjectId popupChildId = Id("31564214-2881-41f2-822d-2e84917e443c");
            ObjectId popupSecondChildId = Id("ffbddffb-35ed-4664-b263-df0e65f263ee");
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
                                new(
                                    groupId,
                                    new UiGroupBox { Text = "Settings" },
                                    new UiNode[]
                                    {
                                        new(groupChildId, new UiLabel { Text = "Music" }),
                                    }
                                ),
                                new(
                                    popupId,
                                    new UiPopupWindow
                                    {
                                        Text = "<b>Deployment</b>",
                                        EnableRichText = true,
                                    },
                                    new UiNode[]
                                    {
                                        new(popupChildId, new UiLabel { Text = "Ready" }),
                                        new(popupSecondChildId, new UiLabel { Text = "04:20" }),
                                    }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );

                Assert.That(documents.TryGet(groupId, out VisualElement? groupValue), Is.True);
                var group = (GroupBox)groupValue!;
                Assert.That(GroupTitle(group), Is.Not.Null);
                Assert.That(GroupTitle(group)!.text, Is.EqualTo("Settings"));
                Assert.That(documents.TryGet(groupChildId, out VisualElement? groupChild), Is.True);
                Assert.That(groupChild!.parent, Is.SameAs(group.contentContainer));

                Assert.That(documents.TryGet(popupId, out VisualElement? popupValue), Is.True);
                var popup = (PopupWindow)popupValue!;
                Assert.That(popup.text, Is.EqualTo("<b>Deployment</b>"));
                Assert.That(documents.TryGet(popupChildId, out VisualElement? popupChild), Is.True);
                Assert.That(popupChild!.parent, Is.SameAs(popup.contentContainer));
                Assert.That(
                    documents.TryGet(popupSecondChildId, out VisualElement? popupSecondChild),
                    Is.True
                );

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Index(popupSecondChildId, 0)
                    )
                );
                Assert.That(popup.contentContainer[0], Is.SameAs(popupSecondChild));
                Assert.That(popup.contentContainer[1], Is.SameAs(popupChild));

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(groupId, new UiGroupBox { Text = "" })
                    )
                );
                Assert.That(GroupTitle(group), Is.Null);
                Assert.That(groupChild.parent, Is.SameAs(group.contentContainer));

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            groupId,
                            new UiGroupBox { Text = "Advanced" }
                        )
                    )
                );
                Assert.That(GroupTitle(group), Is.Not.Null);
                Assert.That(GroupTitle(group)!.text, Is.EqualTo("Advanced"));
                Assert.That(groupChild.parent, Is.SameAs(group.contentContainer));

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(popupId, new UiPopupWindow { Text = "" })
                    )
                );
                Assert.That(popup.text, Is.Empty);
                Assert.That(popupChild.parent, Is.SameAs(popup.contentContainer));
                Assert.That(popupSecondChild!.parent, Is.SameAs(popup.contentContainer));

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            groupId,
                            new UiGroupBox { Text = Prop<string>.Reset() }
                        )
                    )
                );
                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            popupId,
                            new UiPopupWindow
                            {
                                Text = Prop<string>.Reset(),
                                EnableRichText = Prop<bool>.Reset(),
                            }
                        )
                    )
                );
                Assert.That(documents.TryGet(groupId, out VisualElement? resetGroup), Is.True);
                Assert.That(resetGroup, Is.SameAs(group));
                Assert.That(GroupTitle(group), Is.Null);
                Assert.That(documents.TryGet(popupId, out VisualElement? resetPopup), Is.True);
                Assert.That(resetPopup, Is.SameAs(popup));
                Assert.That(popup.text, Is.Empty);
                Assert.That(popup.enableRichText, Is.EqualTo(new PopupWindow().enableRichText));
                Assert.That(popup.contentContainer[0], Is.SameAs(popupSecondChild));
                Assert.That(popup.contentContainer[1], Is.SameAs(popupChild));
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void TitledGroupBoxPreservesLogicalChildIndices()
        {
            ObjectId documentId = Id("7483c8da-82db-44bd-9ad4-48c50e9801e2");
            ObjectId rootId = Id("4b7f54d7-c7fd-4a02-9b6f-5c0a5fa8c1f9");
            ObjectId groupId = Id("d2fdbbb1-2a9b-4d4a-8f49-4e5bc9d8dc76");
            ObjectId firstId = Id("bb71cd2a-5c97-4823-9a98-e4c9b5b3f0b2");
            ObjectId secondId = Id("550a8c17-9e4c-42dd-bc7b-6a1bfbb58b3e");
            ObjectId outsideId = Id("f45be75b-5b17-4c2b-8d4c-f8eaf9e0ed8f");
            ObjectId addedId = Id("8ec40a93-cb15-42f1-a41e-1e7da8d0b3ef");
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
                                new(
                                    groupId,
                                    new UiGroupBox { Text = "Settings" },
                                    new UiNode[]
                                    {
                                        new(firstId, new UiLabel { Text = "First" }),
                                        new(secondId, new UiLabel { Text = "Second" }),
                                    }
                                ),
                                new(outsideId, new UiLabel { Text = "Outside" }),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );

                Assert.That(documents.TryGet(groupId, out VisualElement? groupValue), Is.True);
                var group = (GroupBox)groupValue!;
                documents.Update(
                    new CommandBody.VisualElement.Update(new VisualElementUpdate.Index(secondId, 0))
                );
                Assert.That(
                    group.contentContainer[1],
                    Is.SameAs(documents.TryGet(secondId, out VisualElement? second) ? second : null)
                );
                Assert.That(
                    group.contentContainer[2],
                    Is.SameAs(documents.TryGet(firstId, out VisualElement? first) ? first : null)
                );
                Assert.That(group.contentContainer[0], Is.SameAs(GroupTitle(group)));
                Assert.That(GroupTitle(group)!.text, Is.EqualTo("Settings"));

                documents.Create(
                    new CommandBody.VisualElement.Create(
                        groupId,
                        new UiNode(addedId, new UiLabel { Text = "Added" }),
                        0
                    )
                );
                Assert.That(
                    group.contentContainer[1],
                    Is.SameAs(documents.TryGet(addedId, out VisualElement? added) ? added : null)
                );

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Parent(outsideId, groupId, 0)
                    )
                );
                documents.Update(
                    new CommandBody.VisualElement.Update(new VisualElementUpdate.Index(firstId, 1))
                );
                Assert.That(
                    group.contentContainer[1],
                    Is.SameAs(
                        documents.TryGet(outsideId, out VisualElement? outside) ? outside : null
                    )
                );
                Assert.That(
                    group.contentContainer[2],
                    Is.SameAs(
                        documents.TryGet(firstId, out VisualElement? reordered) ? reordered : null
                    )
                );
                Assert.That(
                    group.contentContainer[3],
                    Is.SameAs(
                        documents.TryGet(addedId, out VisualElement? stillAdded) ? stillAdded : null
                    )
                );
                Assert.That(
                    group.contentContainer[4],
                    Is.SameAs(
                        documents.TryGet(secondId, out VisualElement? stillSecond)
                            ? stillSecond
                            : null
                    )
                );
                Assert.That(group.contentContainer[0], Is.SameAs(GroupTitle(group)));
                Assert.That(GroupTitle(group)!.text, Is.EqualTo("Settings"));
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

        private static Label? GroupTitle(GroupBox value) =>
            value.Q<Label>(className: GroupBox.labelUssClassName);
    }
}
