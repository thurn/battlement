#nullable enable

using System;
using System.Collections.Generic;
using System.Reflection;
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
        public void StyleProtocolPropertiesTargetWritableIStyleMembers()
        {
            foreach (PropertyInfo property in typeof(UiStyle).GetProperties())
            {
                string name = char.ToLowerInvariant(property.Name[0]) + property.Name.Substring(1);
                PropertyInfo? target = typeof(IStyle).GetProperty(name);
                Assert.That(target, Is.Not.Null, $"UiStyle.{property.Name} has no IStyle target.");
                Assert.That(target!.CanWrite, Is.True, $"IStyle.{name} is not writable.");
            }
        }

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
        public void LayoutStylesMapToPublicInlineStateAndRejectInvalidUpdatesAtomically()
        {
            ObjectId documentId = Id("d6a598b1-fee0-408f-8f33-3241ced17a10");
            ObjectId rootId = Id("9d4b926d-e913-4789-8fe9-9e075a25de93");
            ObjectId elementId = Id("98454d7b-5736-4952-9503-a2588be2912d");
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
                                    elementId,
                                    new UiBox
                                    {
                                        Name = "layout-target",
                                        Style = new UiStyle(
                                            AlignContent: UiAlign.Center,
                                            AlignItems: UiAlign.Stretch,
                                            AlignSelf: UiAlign.FlexEnd,
                                            AspectRatio: new UiAspectRatio.Ratio(16, 9),
                                            FlexBasis: new UiLengthOrAuto.Auto(),
                                            FlexDirection: UiFlexDirection.RowReverse,
                                            FlexGrow: 2,
                                            FlexShrink: 1,
                                            FlexWrap: UiFlexWrap.Wrap,
                                            Height: new UiLengthOrAuto.Px(240),
                                            JustifyContent: UiJustify.SpaceEvenly,
                                            MarginLeft: new UiLengthOrAuto.Auto(),
                                            PaddingTop: new UiLength.Percent(5),
                                            Position: UiPosition.Absolute,
                                            Right: new UiLengthOrAuto.Percent(10),
                                            Top: new UiLengthOrAuto.Px(20),
                                            Width: new UiLengthOrAuto.Percent(75)
                                        ),
                                    }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );

                Assert.That(documents.TryGet(elementId, out VisualElement? target), Is.True);
                IStyle style = target!.style;
                Assert.That(style.alignContent.value, Is.EqualTo(Align.Center));
                Assert.That(style.flexDirection.value, Is.EqualTo(FlexDirection.RowReverse));
                Assert.That(style.flexWrap.value, Is.EqualTo(Wrap.Wrap));
                Assert.That(style.justifyContent.value, Is.EqualTo(Justify.SpaceEvenly));
                Assert.That(style.position.value, Is.EqualTo(Position.Absolute));
                Assert.That(style.width.value.unit, Is.EqualTo(LengthUnit.Percent));
                Assert.That(style.width.value.value, Is.EqualTo(75).Within(0.001));
                Assert.That(style.paddingTop.value.unit, Is.EqualTo(LengthUnit.Percent));
                Assert.That(style.flexGrow.value, Is.EqualTo(2).Within(0.001));

                BattlementUiException invalid = Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Properties(
                                elementId,
                                new UiBox
                                {
                                    Name = "not-applied",
                                    Style = new UiStyle(PaddingLeft: new UiLength.Px(-1)),
                                }
                            )
                        )
                    )
                )!;
                Assert.That(invalid.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
                Assert.That(target.name, Is.EqualTo("layout-target"));

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            elementId,
                            new UiBox
                            {
                                Style = new UiStyle(
                                    Width: new UiStyleValue<UiLengthOrAuto>(
                                        null!,
                                        UiInlineKeyword.Initial
                                    )
                                ),
                            }
                        )
                    )
                );
                Assert.That(style.width.keyword, Is.EqualTo(StyleKeyword.Initial));
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void AppearanceStylesMapToPublicInlineStateAndRejectInvalidUpdatesAtomically()
        {
            ObjectId documentId = Id("94ca9bdc-df82-42f8-967e-e2545fcb7e93");
            ObjectId rootId = Id("19b90f99-739a-44cc-a770-1e53fd89b82b");
            ObjectId elementId = Id("06b5592c-85f7-474d-9cdd-bbe350574f42");
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
                                    elementId,
                                    new UiBox
                                    {
                                        Name = "appearance-target",
                                        Style = new UiStyle(
                                            BackgroundColor: new Battlement.Color(
                                                0.04,
                                                0.08,
                                                0.12,
                                                1
                                            ),
                                            BorderBottomColor: new Battlement.Color(
                                                0.2,
                                                0.8,
                                                0.9,
                                                1
                                            ),
                                            BorderBottomLeftRadius: new UiLength.Percent(25),
                                            BorderBottomWidth: 3,
                                            BorderLeftColor: new Battlement.Color(0.9, 0.6, 0.2, 1),
                                            BorderLeftWidth: 5,
                                            BorderRightColor: new Battlement.Color(
                                                0.2,
                                                0.8,
                                                0.9,
                                                1
                                            ),
                                            BorderRightWidth: 7,
                                            BorderTopColor: new Battlement.Color(0.9, 0.6, 0.2, 1),
                                            BorderTopLeftRadius: new UiLength.Px(18),
                                            BorderTopRightRadius: new UiLength.Px(8),
                                            BorderTopWidth: 2,
                                            Color: new Battlement.Color(0.9, 0.95, 1, 1),
                                            Display: UiDisplay.Flex,
                                            Opacity: 0.65f,
                                            Overflow: UiOverflow.Hidden,
                                            UnityBackgroundImageTintColor: new Battlement.Color(
                                                0.5,
                                                0.75,
                                                1,
                                                0.8
                                            ),
                                            UnityOverflowClipBox: UiOverflowClipBox.ContentBox,
                                            UnitySliceBottom: 4,
                                            UnitySliceLeft: 5,
                                            UnitySliceRight: 6,
                                            UnitySliceScale: 2,
                                            UnitySliceTop: 7,
                                            UnitySliceType: UiSliceType.Tiled,
                                            Visibility: UiVisibility.Hidden
                                        ),
                                    }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );

                Assert.That(documents.TryGet(elementId, out VisualElement? target), Is.True);
                IStyle style = target!.style;
                Assert.That(style.borderLeftWidth.value, Is.EqualTo(5).Within(0.001));
                Assert.That(
                    style.borderBottomLeftRadius.value.unit,
                    Is.EqualTo(LengthUnit.Percent)
                );
                Assert.That(style.opacity.value, Is.EqualTo(0.65f).Within(0.001));
                Assert.That(style.display.value, Is.EqualTo(DisplayStyle.Flex));
                Assert.That(style.overflow.value, Is.EqualTo(Overflow.Hidden));
                Assert.That(
                    style.unityOverflowClipBox.value,
                    Is.EqualTo(OverflowClipBox.ContentBox)
                );
                Assert.That(style.unitySliceScale.value, Is.EqualTo(2).Within(0.001));
                Assert.That(style.unitySliceType.value, Is.EqualTo(SliceType.Tiled));
                Assert.That(style.visibility.value, Is.EqualTo(Visibility.Hidden));

                BattlementUiException invalid = Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Properties(
                                elementId,
                                new UiBox
                                {
                                    Name = "not-applied",
                                    Style = new UiStyle(Opacity: 1.1f),
                                }
                            )
                        )
                    )
                )!;
                Assert.That(invalid.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
                Assert.That(target.name, Is.EqualTo("appearance-target"));

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            elementId,
                            new UiBox
                            {
                                Style = new UiStyle(
                                    Visibility: new UiStyleValue<UiVisibility>(
                                        default,
                                        UiInlineKeyword.Initial
                                    )
                                ),
                            }
                        )
                    )
                );
                Assert.That(style.visibility.keyword, Is.EqualTo(StyleKeyword.Initial));
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
