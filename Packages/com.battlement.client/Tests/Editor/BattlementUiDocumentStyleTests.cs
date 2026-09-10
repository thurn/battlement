#nullable enable

using System.Collections.Generic;
using System.Reflection;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using static Battlement.Tests.BattlementUiDocumentSupport;
using Object = UnityEngine.Object;
using UiBox = Battlement.UiElement.Box;
using UiVisualElement = Battlement.UiElement.VisualElement;

namespace Battlement.Tests
{
    public sealed class BattlementUiDocumentStyleTests
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
        public void SharedVisualUpdatesSetOmitAndRestoreConstructorState()
        {
            ObjectId documentId = Id("0f30630f-cb20-45cc-a6d7-e8007d0940cc");
            ObjectId rootId = Id("1fb8daab-482f-408f-9254-efc3c58520aa");
            ObjectId elementId = Id("b88e09cc-106b-4f0b-8395-f2b89607755a");
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            int emitted = 0;
            var documents = new BattlementUiDocuments(_ =>
            {
                emitted++;
                return UiEventDisposition.Continue;
            });
            try
            {
                documents.Replace(
                    new[] { new UiDocument(documentId, rootId) },
                    id => id == documentId ? owned : null
                );
                documents.Create(
                    new CommandBody.VisualElement.Create(
                        rootId,
                        new UiNode(elementId, new UiVisualElement())
                    )
                );
                Assert.That(documents.TryGet(elementId, out VisualElement? value), Is.True);
                Assert.That(value!.name, Is.Empty);
                Assert.That(value.enabledSelf, Is.True);
                Assert.That(value.pickingMode, Is.EqualTo(PickingMode.Position));
                Assert.That(value.languageDirection, Is.EqualTo(LanguageDirection.Inherit));
                Assert.That(value.focusable, Is.False);
                Assert.That(value.tabIndex, Is.EqualTo(0));
                Assert.That(value.delegatesFocus, Is.False);
                Assert.That(value.ClassListContains("changed"), Is.False);

                UpdateShared(
                    documents,
                    elementId,
                    new UiVisualElement
                    {
                        Name = "changed",
                        Enabled = false,
                        PickingMode = UiPickingMode.Ignore,
                        LanguageDirection = UiLanguageDirection.Rtl,
                        Focusable = true,
                        TabIndex = 7,
                        DelegatesFocus = true,
                        Classes = new[] { "changed" },
                        Events = new[] { UiEventKind.PointerDown },
                    }
                );
                Assert.That(value.name, Is.EqualTo("changed"));
                Assert.That(value.enabledSelf, Is.False);
                Assert.That(value.pickingMode, Is.EqualTo(PickingMode.Ignore));
                Assert.That(value.languageDirection, Is.EqualTo(LanguageDirection.RTL));
                Assert.That(value.focusable, Is.True);
                Assert.That(value.tabIndex, Is.EqualTo(7));
                Assert.That(value.delegatesFocus, Is.True);
                Assert.That(value.ClassListContains("changed"), Is.True);
                SendPointerDown(value);
                Assert.That(emitted, Is.EqualTo(1));

                UpdateShared(documents, elementId, new UiVisualElement());
                Assert.That(value.enabledSelf, Is.False);
                Assert.That(value.name, Is.EqualTo("changed"));
                Assert.That(value.ClassListContains("changed"), Is.True);
                SendPointerDown(value);
                Assert.That(emitted, Is.EqualTo(2));

                UpdateShared(
                    documents,
                    elementId,
                    new UiVisualElement
                    {
                        Name = Prop<string>.Reset(),
                        Enabled = Prop<bool>.Reset(),
                        PickingMode = Prop<UiPickingMode>.Reset(),
                        LanguageDirection = Prop<UiLanguageDirection>.Reset(),
                        Focusable = Prop<bool>.Reset(),
                        TabIndex = Prop<int>.Reset(),
                        DelegatesFocus = Prop<bool>.Reset(),
                        Classes = Prop<IReadOnlyList<string>>.Reset(),
                        Events = Prop<IReadOnlyList<UiEventKind>>.Reset(),
                    }
                );
                Assert.That(value.name, Is.Empty);
                Assert.That(value.enabledSelf, Is.True);
                Assert.That(value.pickingMode, Is.EqualTo(PickingMode.Position));
                Assert.That(value.languageDirection, Is.EqualTo(LanguageDirection.Inherit));
                Assert.That(value.focusable, Is.False);
                Assert.That(value.tabIndex, Is.EqualTo(0));
                Assert.That(value.delegatesFocus, Is.False);
                Assert.That(value.ClassListContains("changed"), Is.False);
                SendPointerDown(value);
                Assert.That(emitted, Is.EqualTo(2));
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        private static void UpdateShared(
            BattlementUiDocuments documents,
            ObjectId elementId,
            UiVisualElement value
        ) =>
            documents.Update(
                new CommandBody.VisualElement.Update(
                    new VisualElementUpdate.Properties(elementId, value)
                )
            );

        private static void SendPointerDown(VisualElement target)
        {
            using PointerDownEvent value = PointerDownEvent.GetPooled(
                new Event { type = EventType.MouseDown, button = 0 }
            );
            value.target = target;
            target.SendEvent(value);
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
                                            AlignContent: UiStyle.Set(UiAlign.Center),
                                            AlignItems: UiStyle.Set(UiAlign.Stretch),
                                            AlignSelf: UiStyle.Set(UiAlign.FlexEnd),
                                            AspectRatio: UiStyle.Set<UiAspectRatio>(
                                                new UiAspectRatio.Ratio(16, 9)
                                            ),
                                            BorderBottomWidth: UiStyle.Set(1f),
                                            BorderLeftWidth: UiStyle.Set(2f),
                                            BorderRightWidth: UiStyle.Set(3f),
                                            BorderTopWidth: UiStyle.Set(4f),
                                            Bottom: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(5)
                                            ),
                                            Display: UiStyle.Set(UiDisplay.Flex),
                                            FlexBasis: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Auto()
                                            ),
                                            FlexDirection: UiStyle.Set(UiFlexDirection.RowReverse),
                                            FlexGrow: UiStyle.Set(2f),
                                            FlexShrink: UiStyle.Set(1f),
                                            FlexWrap: UiStyle.Set(UiFlexWrap.Wrap),
                                            Height: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(240)
                                            ),
                                            JustifyContent: UiStyle.Set(UiJustify.SpaceEvenly),
                                            Left: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(7)
                                            ),
                                            MarginBottom: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(8)
                                            ),
                                            MarginLeft: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Auto()
                                            ),
                                            MarginRight: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(9)
                                            ),
                                            MarginTop: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(10)
                                            ),
                                            MaxHeight: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(400)
                                            ),
                                            MaxWidth: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(500)
                                            ),
                                            MinHeight: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(40)
                                            ),
                                            MinWidth: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(50)
                                            ),
                                            Overflow: UiStyle.Set(UiOverflow.Hidden),
                                            PaddingBottom: UiStyle.Set<UiLength>(
                                                new UiLength.Px(11)
                                            ),
                                            PaddingLeft: UiStyle.Set<UiLength>(new UiLength.Px(12)),
                                            PaddingRight: UiStyle.Set<UiLength>(
                                                new UiLength.Px(13)
                                            ),
                                            PaddingTop: UiStyle.Set<UiLength>(
                                                new UiLength.Percent(5)
                                            ),
                                            Position: UiStyle.Set(UiPosition.Absolute),
                                            Right: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Percent(10)
                                            ),
                                            Top: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(20)
                                            ),
                                            Width: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Percent(75)
                                            )
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

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            elementId,
                            new UiBox { Style = new UiStyle(FlexGrow: UiStyle.Set(3f)) }
                        )
                    )
                );
                Assert.That(style.flexGrow.value, Is.EqualTo(3).Within(0.001));
                Assert.That(style.width.value.unit, Is.EqualTo(LengthUnit.Percent));
                Assert.That(style.width.value.value, Is.EqualTo(75).Within(0.001));
                Assert.That(style.position.value, Is.EqualTo(Position.Absolute));

                BattlementUiException invalid = Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Properties(
                                elementId,
                                new UiBox
                                {
                                    Name = "not-applied",
                                    Style = new UiStyle(
                                        PaddingLeft: UiStyle.Set<UiLength>(new UiLength.Px(-1))
                                    ),
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
                            new UiBox { Style = ResetLayoutStyle() }
                        )
                    )
                );
                Assert.That(style.alignContent.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.alignItems.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.alignSelf.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.aspectRatio.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.borderBottomWidth.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.borderLeftWidth.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.borderRightWidth.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.borderTopWidth.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.bottom.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.display.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.flexBasis.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.flexDirection.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.flexGrow.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.flexShrink.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.flexWrap.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.height.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.justifyContent.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.left.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.marginBottom.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.marginLeft.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.marginRight.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.marginTop.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.maxHeight.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.maxWidth.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.minHeight.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.minWidth.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.overflow.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.paddingBottom.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.paddingLeft.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.paddingRight.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.paddingTop.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.position.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.right.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.top.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.width.keyword, Is.EqualTo(StyleKeyword.Null));
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void BackgroundGeometryMapsToPublicInlineStateAndRejectsInvalidAxesAtomically()
        {
            ObjectId documentId = Id("5cfe41bd-f8d6-4a24-802d-4cd75c89ddad");
            ObjectId rootId = Id("91aaa06b-c360-47c7-a3a9-99025c221387");
            ObjectId elementId = Id("8691e1f1-8598-4548-96e8-012a80347890");
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
                                        Style = new UiStyle(
                                            BackgroundPositionX: UiStyle.Set(
                                                new UiBackgroundPosition(
                                                    UiBackgroundPositionKeyword.Right,
                                                    new UiLength.Percent(12)
                                                )
                                            ),
                                            BackgroundPositionY: UiStyle.Set(
                                                new UiBackgroundPosition(
                                                    UiBackgroundPositionKeyword.Bottom,
                                                    new UiLength.Px(8)
                                                )
                                            ),
                                            BackgroundRepeat: UiStyle.Set(
                                                new UiBackgroundRepeat(
                                                    UiBackgroundRepeatMode.Space,
                                                    UiBackgroundRepeatMode.Round
                                                )
                                            ),
                                            BackgroundSize: UiStyle.Set<UiBackgroundSize>(
                                                new UiBackgroundSize.Axes(
                                                    new UiLengthOrAuto.Percent(45),
                                                    new UiLengthOrAuto.Px(72)
                                                )
                                            )
                                        ),
                                    }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Assert.That(documents.TryGet(elementId, out VisualElement? target), Is.True);
                Assert.That(
                    target!.style.backgroundPositionX.value.keyword,
                    Is.EqualTo(BackgroundPositionKeyword.Right)
                );
                Assert.That(target.style.backgroundPositionX.value.offset.value, Is.EqualTo(12));
                Assert.That(
                    target.style.backgroundRepeat.value.x,
                    Is.EqualTo(UnityEngine.UIElements.Repeat.Space)
                );
                Assert.That(
                    target.style.backgroundRepeat.value.y,
                    Is.EqualTo(UnityEngine.UIElements.Repeat.Round)
                );
                Assert.That(target.style.backgroundSize.value.x.value, Is.EqualTo(45));
                Assert.That(target.style.backgroundSize.value.y.value, Is.EqualTo(72));

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            elementId,
                            new UiBox
                            {
                                Style = new UiStyle(
                                    BackgroundSize: UiStyle.Set<UiBackgroundSize>(
                                        new UiBackgroundSize.Contain()
                                    )
                                ),
                            }
                        )
                    )
                );
                Assert.That(
                    target.style.backgroundPositionX.value.keyword,
                    Is.EqualTo(BackgroundPositionKeyword.Right)
                );
                Assert.That(
                    target.style.backgroundSize.value.sizeType,
                    Is.EqualTo(BackgroundSizeType.Contain)
                );

                BattlementUiException failure = Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Properties(
                                elementId,
                                new UiBox
                                {
                                    Name = "not-applied",
                                    Style = new UiStyle(
                                        BackgroundPositionX: UiStyle.Set(
                                            new UiBackgroundPosition(
                                                UiBackgroundPositionKeyword.Top,
                                                new UiLength.Px(0)
                                            )
                                        )
                                    ),
                                }
                            )
                        )
                    )
                )!;
                Assert.That(failure.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
                Assert.That(target.name, Is.Empty);
                Assert.That(
                    target.style.backgroundPositionX.value.keyword,
                    Is.EqualTo(BackgroundPositionKeyword.Right)
                );

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            elementId,
                            new UiBox
                            {
                                Style = new UiStyle(
                                    BackgroundPositionX: UiStyle.Reset<UiBackgroundPosition>(),
                                    BackgroundPositionY: UiStyle.Reset<UiBackgroundPosition>(),
                                    BackgroundRepeat: UiStyle.Reset<UiBackgroundRepeat>(),
                                    BackgroundSize: UiStyle.Reset<UiBackgroundSize>()
                                ),
                            }
                        )
                    )
                );
                Assert.That(
                    target.style.backgroundPositionX.keyword,
                    Is.EqualTo(StyleKeyword.Null)
                );
                Assert.That(
                    target.style.backgroundPositionY.keyword,
                    Is.EqualTo(StyleKeyword.Null)
                );
                Assert.That(target.style.backgroundRepeat.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(target.style.backgroundSize.keyword, Is.EqualTo(StyleKeyword.Null));
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
                                            BackgroundColor: UiStyle.Set(
                                                new Battlement.Color(0.04, 0.08, 0.12, 1)
                                            ),
                                            BorderBottomColor: UiStyle.Set(
                                                new Battlement.Color(0.2, 0.8, 0.9, 1)
                                            ),
                                            BorderBottomLeftRadius: UiStyle.Set<UiLength>(
                                                new UiLength.Percent(25)
                                            ),
                                            BorderBottomWidth: UiStyle.Set(3f),
                                            BorderLeftColor: UiStyle.Set(
                                                new Battlement.Color(0.9, 0.6, 0.2, 1)
                                            ),
                                            BorderLeftWidth: UiStyle.Set(5f),
                                            BorderRightColor: UiStyle.Set(
                                                new Battlement.Color(0.2, 0.8, 0.9, 1)
                                            ),
                                            BorderRightWidth: UiStyle.Set(7f),
                                            BorderTopColor: UiStyle.Set(
                                                new Battlement.Color(0.9, 0.6, 0.2, 1)
                                            ),
                                            BorderTopLeftRadius: UiStyle.Set<UiLength>(
                                                new UiLength.Px(18)
                                            ),
                                            BorderTopRightRadius: UiStyle.Set<UiLength>(
                                                new UiLength.Px(8)
                                            ),
                                            BorderTopWidth: UiStyle.Set(2f),
                                            Color: UiStyle.Set(
                                                new Battlement.Color(0.9, 0.95, 1, 1)
                                            ),
                                            Display: UiStyle.Set(UiDisplay.Flex),
                                            Opacity: UiStyle.Set(0.65f),
                                            Overflow: UiStyle.Set(UiOverflow.Hidden),
                                            UnityBackgroundImageTintColor: UiStyle.Set(
                                                new Battlement.Color(0.5, 0.75, 1, 0.8)
                                            ),
                                            UnityOverflowClipBox: UiStyle.Set(
                                                UiOverflowClipBox.ContentBox
                                            ),
                                            UnitySliceBottom: UiStyle.Set(4),
                                            UnitySliceLeft: UiStyle.Set(5),
                                            UnitySliceRight: UiStyle.Set(6),
                                            UnitySliceScale: UiStyle.Set(2f),
                                            UnitySliceTop: UiStyle.Set(7),
                                            UnitySliceType: UiStyle.Set(UiSliceType.Tiled),
                                            Visibility: UiStyle.Set(UiVisibility.Hidden)
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
                                    Style = new UiStyle(Opacity: UiStyle.Set(1.1f)),
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
                            new UiBox { Style = ResetAppearanceStyle() }
                        )
                    )
                );
                Assert.That(style.backgroundColor.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.borderBottomColor.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.borderBottomLeftRadius.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.borderLeftColor.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.borderRightColor.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.borderTopColor.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.borderTopLeftRadius.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.borderTopRightRadius.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.color.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.opacity.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(
                    style.unityBackgroundImageTintColor.keyword,
                    Is.EqualTo(StyleKeyword.Null)
                );
                Assert.That(style.unityOverflowClipBox.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.unitySliceBottom.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.unitySliceLeft.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.unitySliceRight.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.unitySliceScale.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.unitySliceTop.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.unitySliceType.keyword, Is.EqualTo(StyleKeyword.Null));

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            elementId,
                            new UiBox
                            {
                                Style = new UiStyle(
                                    Visibility: Prop<UiStyleValue<UiVisibility>>.Set(
                                        new UiStyleValue<UiVisibility>(
                                            default,
                                            UiInlineKeyword.Initial
                                        )
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

        private static UiStyle ResetLayoutStyle() =>
            new(
                AlignContent: UiStyle.Reset<UiAlign>(),
                AlignItems: UiStyle.Reset<UiAlign>(),
                AlignSelf: UiStyle.Reset<UiAlign>(),
                AspectRatio: UiStyle.Reset<UiAspectRatio>(),
                BorderBottomWidth: UiStyle.Reset<float>(),
                BorderLeftWidth: UiStyle.Reset<float>(),
                BorderRightWidth: UiStyle.Reset<float>(),
                BorderTopWidth: UiStyle.Reset<float>(),
                Bottom: UiStyle.Reset<UiLengthOrAuto>(),
                Display: UiStyle.Reset<UiDisplay>(),
                FlexBasis: UiStyle.Reset<UiLengthOrAuto>(),
                FlexDirection: UiStyle.Reset<UiFlexDirection>(),
                FlexGrow: UiStyle.Reset<float>(),
                FlexShrink: UiStyle.Reset<float>(),
                FlexWrap: UiStyle.Reset<UiFlexWrap>(),
                Height: UiStyle.Reset<UiLengthOrAuto>(),
                JustifyContent: UiStyle.Reset<UiJustify>(),
                Left: UiStyle.Reset<UiLengthOrAuto>(),
                MarginBottom: UiStyle.Reset<UiLengthOrAuto>(),
                MarginLeft: UiStyle.Reset<UiLengthOrAuto>(),
                MarginRight: UiStyle.Reset<UiLengthOrAuto>(),
                MarginTop: UiStyle.Reset<UiLengthOrAuto>(),
                MaxHeight: UiStyle.Reset<UiLengthOrAuto>(),
                MaxWidth: UiStyle.Reset<UiLengthOrAuto>(),
                MinHeight: UiStyle.Reset<UiLengthOrAuto>(),
                MinWidth: UiStyle.Reset<UiLengthOrAuto>(),
                Overflow: UiStyle.Reset<UiOverflow>(),
                PaddingBottom: UiStyle.Reset<UiLength>(),
                PaddingLeft: UiStyle.Reset<UiLength>(),
                PaddingRight: UiStyle.Reset<UiLength>(),
                PaddingTop: UiStyle.Reset<UiLength>(),
                Position: UiStyle.Reset<UiPosition>(),
                Right: UiStyle.Reset<UiLengthOrAuto>(),
                Top: UiStyle.Reset<UiLengthOrAuto>(),
                Width: UiStyle.Reset<UiLengthOrAuto>()
            );

        private static UiStyle ResetAppearanceStyle() =>
            new(
                BackgroundColor: UiStyle.Reset<Battlement.Color>(),
                BorderBottomColor: UiStyle.Reset<Battlement.Color>(),
                BorderBottomLeftRadius: UiStyle.Reset<UiLength>(),
                BorderBottomRightRadius: UiStyle.Reset<UiLength>(),
                BorderLeftColor: UiStyle.Reset<Battlement.Color>(),
                BorderRightColor: UiStyle.Reset<Battlement.Color>(),
                BorderTopColor: UiStyle.Reset<Battlement.Color>(),
                BorderTopLeftRadius: UiStyle.Reset<UiLength>(),
                BorderTopRightRadius: UiStyle.Reset<UiLength>(),
                Color: UiStyle.Reset<Battlement.Color>(),
                Opacity: UiStyle.Reset<float>(),
                UnityBackgroundImageTintColor: UiStyle.Reset<Battlement.Color>(),
                UnityOverflowClipBox: UiStyle.Reset<UiOverflowClipBox>(),
                UnitySliceBottom: UiStyle.Reset<int>(),
                UnitySliceLeft: UiStyle.Reset<int>(),
                UnitySliceRight: UiStyle.Reset<int>(),
                UnitySliceScale: UiStyle.Reset<float>(),
                UnitySliceTop: UiStyle.Reset<int>(),
                UnitySliceType: UiStyle.Reset<UiSliceType>()
            );
    }
}
