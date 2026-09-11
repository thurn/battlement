#nullable enable

using System;
using System.Linq;
using System.Text;
using Battlement.UI;
using Newtonsoft.Json.Linq;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class LayoutProtocolTests
    {
        [Test]
        public void EveryLayoutVariantRoundTripsThroughCanonicalJson()
        {
            ObjectId anchor = Id("17608312-6e18-421e-be92-b677cec12c42");
            ObjectId initialFocus = Id("68641395-a531-479f-9606-aef0acf6acbb");
            ObjectId restoreFocus = Id("2075f62f-45f0-44c8-b65a-aa9c1e716e28");
            UiElement[] elements =
            {
                new UiElement.Flex
                {
                    Direction = UiFlexDirection.RowReverse,
                    Wrap = UiFlexWrap.WrapReverse,
                    AlignItems = UiAlign.Center,
                    JustifyContent = UiJustify.SpaceEvenly,
                    RowGap = 2,
                    ColumnGap = 3,
                },
                new UiElement.Grid
                {
                    Columns = new GridTrack[]
                    {
                        new GridTrack.Px(12),
                        new GridTrack.Fraction(2),
                        new GridTrack.Auto(),
                    },
                    Rows = Array.Empty<GridTrack>(),
                    AutoColumns = new GridTrack.Fraction(1),
                    AutoRows = new GridTrack.Auto(),
                    AutoFlow = GridAutoFlow.Column,
                    RowGap = 4,
                    ColumnGap = 5,
                    AlignItems = UiAlign.FlexStart,
                    JustifyItems = UiAlign.FlexEnd,
                },
                new UiElement.Stack { AlignItems = UiAlign.Stretch, JustifyItems = UiAlign.Center },
                new UiElement.VisualElement
                {
                    GridItem = new GridItem(1, 2, 3, 4, UiAlign.Auto, UiAlign.Center),
                    StackItem = new StackItem(
                        -7,
                        UiAlign.FlexEnd,
                        UiAlign.Stretch,
                        1,
                        2,
                        3,
                        4,
                        false
                    ),
                    Sticky = new Sticky(-3, 4, null, null, 8),
                    OverlayPlacement = new OverlayPlacement.Popover(
                        anchor,
                        new PopoverPlacement(
                            PlacementSide.Left,
                            PlacementAlign.End,
                            -2,
                            3,
                            9,
                            false,
                            false
                        )
                    ),
                },
                new UiElement.VisualElement
                {
                    OverlayPlacement = new OverlayPlacement.Layer(OverlayLayer.Popover),
                },
                new UiElement.VisualElement
                {
                    OverlayPlacement = new OverlayPlacement.Modal(initialFocus, restoreFocus),
                },
            };

            SessionId sessionId = new(Guid.NewGuid());
            ObjectId rootId = new(Guid.NewGuid());
            Command[] commands = elements
                .Select(element => new Command(
                    new CommandId(Guid.NewGuid()),
                    new CommandBody.VisualElement.Create(
                        rootId,
                        new UiNode(new ObjectId(Guid.NewGuid()), element)
                    )
                ))
                .ToArray();
            var response = new Response(
                sessionId,
                new ResponseMessage<Command>[]
                {
                    new ResponseMessage<Command>.BatchMessage(
                        new Batch(
                            new BatchId(Guid.NewGuid()),
                            sessionId,
                            new[] { new ParallelCommandGroup<Command>(commands) }
                        )
                    ),
                }
            );
            byte[] encoded = BattlementJson.SerializeResponse(response);
            Response decoded = BattlementJson.DeserializeResponse(encoded);

            Assert.That(
                JToken.Parse(Encoding.UTF8.GetString(BattlementJson.SerializeResponse(decoded))),
                Is.EqualTo(JToken.Parse(Encoding.UTF8.GetString(encoded)))
            );
        }

        [Test]
        public void InvalidLayoutNumbersAreRejectedBeforeTheAvailabilityGate()
        {
            BattlementUiException? trackFailure = Assert.Throws<BattlementUiException>(() =>
                BattlementUiElementProperties.Validate(
                    new UiElement.Grid { Columns = new GridTrack[] { new GridTrack.Px(-1) } },
                    allowUsageHints: true
                )
            );
            Assert.That(trackFailure!.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
            StringAssert.Contains("nonnegative", trackFailure.Message);

            BattlementUiException? stickyFailure = Assert.Throws<BattlementUiException>(() =>
                BattlementUiElementProperties.Validate(
                    new UiElement.VisualElement { Sticky = new Sticky(null, 0, null, 0, 0) },
                    allowUsageHints: true
                )
            );
            Assert.That(stickyFailure!.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
            StringAssert.Contains("contradictory", stickyFailure.Message);
        }

        [Test]
        public void InvalidLayoutEnumsAreRejectedBeforeTheAvailabilityGate()
        {
            ObjectId anchor = new(Guid.NewGuid());
            UiElement[] invalid =
            {
                new UiElement.Flex { Direction = (UiFlexDirection)99 },
                new UiElement.Flex { Wrap = (UiFlexWrap)99 },
                new UiElement.Flex { AlignItems = (UiAlign)99 },
                new UiElement.Flex { JustifyContent = (UiJustify)99 },
                new UiElement.Grid { AutoFlow = (GridAutoFlow)99 },
                new UiElement.VisualElement
                {
                    GridItem = new GridItem(1, 1, 1, 1, (UiAlign)99, UiAlign.Auto),
                },
                new UiElement.VisualElement
                {
                    StackItem = new StackItem(
                        0,
                        UiAlign.Auto,
                        (UiAlign)99,
                        null,
                        null,
                        null,
                        null,
                        true
                    ),
                },
                new UiElement.VisualElement
                {
                    OverlayPlacement = new OverlayPlacement.Layer((OverlayLayer)99),
                },
                new UiElement.VisualElement
                {
                    OverlayPlacement = new OverlayPlacement.Popover(
                        anchor,
                        new PopoverPlacement(
                            (PlacementSide)99,
                            PlacementAlign.Start,
                            0,
                            0,
                            0,
                            true,
                            true
                        )
                    ),
                },
                new UiElement.VisualElement
                {
                    OverlayPlacement = new OverlayPlacement.Popover(
                        anchor,
                        new PopoverPlacement(
                            PlacementSide.Top,
                            (PlacementAlign)99,
                            0,
                            0,
                            0,
                            true,
                            true
                        )
                    ),
                },
            };

            foreach (UiElement element in invalid)
            {
                BattlementUiException? failure = Assert.Throws<BattlementUiException>(() =>
                    BattlementUiElementProperties.Validate(element, allowUsageHints: true)
                );
                Assert.That(failure!.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
                StringAssert.Contains("not recognized", failure.Message);
            }
        }

        [Test]
        public void InvalidPlacementIsRejectedWithoutChangingTheNativeTree()
        {
            ObjectId documentId = Id("7d175052-06d3-46a6-87e4-d12a711db84c");
            ObjectId rootId = Id("34cd0664-cbaf-41f5-aa2b-31c0a102fc56");
            ObjectId layoutId = Id("831053d0-6a4f-49f7-9750-e98f5207cdba");
            ObjectId ordinaryId = Id("c1dcd487-b065-44db-8864-280f365db147");
            ObjectId stackChildId = Id("f3f62f22-a0d3-44a0-8873-bf9be707b9a6");
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
                        new UiNode(layoutId, new UiElement.Stack())
                    )
                );
                Assert.That(documents.TryGet(layoutId, out _), Is.True);

                documents.Create(
                    new CommandBody.VisualElement.Create(
                        rootId,
                        new UiNode(ordinaryId, new UiElement.VisualElement { Name = "unchanged" })
                    )
                );
                Assert.That(documents.TryGet(ordinaryId, out VisualElement? ordinary), Is.True);
                BattlementUiException? updateFailure = Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Properties(
                                ordinaryId,
                                new UiElement.VisualElement
                                {
                                    Name = "not-applied",
                                    GridItem = new GridItem(
                                        null,
                                        1,
                                        1,
                                        1,
                                        UiAlign.Auto,
                                        UiAlign.Auto
                                    ),
                                }
                            )
                        )
                    )
                );
                Assert.That(updateFailure!.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
                Assert.That(ordinary!.name, Is.EqualTo("unchanged"));

                BattlementUiException? stackItemFailure = Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Properties(
                                ordinaryId,
                                new UiElement.VisualElement
                                {
                                    Name = "also-not-applied",
                                    StackItem = new StackItem(
                                        1,
                                        UiAlign.Auto,
                                        UiAlign.Auto,
                                        null,
                                        null,
                                        null,
                                        null,
                                        true
                                    ),
                                }
                            )
                        )
                    )
                );
                Assert.That(stackItemFailure!.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
                Assert.That(ordinary.name, Is.EqualTo("unchanged"));

                documents.Create(
                    new CommandBody.VisualElement.Create(
                        layoutId,
                        new UiNode(
                            stackChildId,
                            new UiElement.VisualElement { Name = "stack-child" }
                        )
                    )
                );
                Assert.That(documents.TryGet(stackChildId, out VisualElement? stackChild), Is.True);
                BattlementUiException? styleFailure = Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Properties(
                                stackChildId,
                                new UiElement.VisualElement
                                {
                                    Name = "style-not-applied",
                                    Style = new UiStyle(Position: UiStyle.Set(UiPosition.Absolute)),
                                }
                            )
                        )
                    )
                );
                Assert.That(styleFailure!.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
                Assert.That(stackChild!.name, Is.EqualTo("stack-child"));
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void ModalAdmissionResolvesEffectiveFocusBeforeChangingDocumentState()
        {
            ObjectId documentId = Id("b8a0c51e-5c9f-4ad2-8bb8-5e0ebcda4a51");
            ObjectId rootId = Id("74482db8-a56c-42f6-a97d-a4a9d8fa48e0");
            ObjectId stackId = Id("b69d865e-04cc-465e-96cf-430974fe60bd");
            ObjectId hostId = Id("87fe9b1b-4bd6-4b18-99e0-82b25d2ba3db");
            ObjectId targetId = Id("e19594e9-2ea9-4ff1-99e0-5e6cfec4db31");
            ObjectId disabledId = Id("80d11708-0e29-4f93-a178-e1dcac6ea26a");
            ObjectId inertId = Id("3be5a6e4-7fc7-4935-9f90-4b8b7278042d");
            ObjectId resetId = Id("63fd15fa-e7be-4c1c-953b-0ac1eb18fe4c");
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
                                    stackId,
                                    new UiElement.Stack(),
                                    new UiNode[]
                                    {
                                        new(
                                            hostId,
                                            new UiElement.Stack
                                            {
                                                PickingMode = UiPickingMode.Ignore,
                                                StackItem = new StackItem(
                                                    int.MaxValue,
                                                    UiAlign.Stretch,
                                                    UiAlign.Stretch,
                                                    null,
                                                    null,
                                                    null,
                                                    null,
                                                    false
                                                ),
                                                Style = new UiStyle
                                                {
                                                    Overflow = UiStyle.Set(UiOverflow.Visible),
                                                },
                                            },
                                            new UiNode[]
                                            {
                                                new(targetId, new UiElement.Box { Name = "prior" }),
                                                new(
                                                    disabledId,
                                                    new UiElement.Box
                                                    {
                                                        Name = "disabled",
                                                        Enabled = false,
                                                        Focusable = true,
                                                        TabIndex = -1,
                                                    }
                                                ),
                                                new(
                                                    inertId,
                                                    new UiElement.Box
                                                    {
                                                        Name = "inert",
                                                        Focusable = true,
                                                        TabIndex = -1,
                                                        Inert = true,
                                                    }
                                                ),
                                                new(
                                                    resetId,
                                                    new UiElement.Box
                                                    {
                                                        Name = "reset",
                                                        Enabled = false,
                                                        Focusable = true,
                                                        TabIndex = -1,
                                                        Inert = true,
                                                    }
                                                ),
                                            }
                                        ),
                                    }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );

                Assert.That(documents.TryGet(targetId, out VisualElement? target), Is.True);
                Assert.That(target!.focusable, Is.False);
                Assert.That(target.tabIndex, Is.EqualTo(0));
                Assert.That(BattlementOverlayItems.HasAuthored(target), Is.False);
                Assert.That(documents.TryGet(disabledId, out VisualElement? disabled), Is.True);
                Assert.That(documents.TryGet(inertId, out VisualElement? inert), Is.True);
                Assert.That(documents.TryGet(resetId, out VisualElement? reset), Is.True);

                BattlementUiException unsetFailure = Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Properties(
                                targetId,
                                new UiElement.Box
                                {
                                    Name = "not-applied",
                                    OverlayPlacement = new OverlayPlacement.Modal(null, null),
                                }
                            )
                        )
                    )
                )!;
                Assert.That(unsetFailure.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
                Assert.That(target.name, Is.EqualTo("prior"));
                Assert.That(target.focusable, Is.False);
                Assert.That(target.tabIndex, Is.EqualTo(0));
                Assert.That(BattlementOverlayItems.HasAuthored(target), Is.False);

                BattlementUiException disabledFailure = Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Properties(
                                disabledId,
                                new UiElement.Box
                                {
                                    Name = "disabled-not-applied",
                                    OverlayPlacement = new OverlayPlacement.Modal(null, null),
                                }
                            )
                        )
                    )
                )!;
                Assert.That(disabledFailure.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
                Assert.That(disabled!.name, Is.EqualTo("disabled"));
                Assert.That(disabled.enabledSelf, Is.False);
                Assert.That(disabled.focusable, Is.True);
                Assert.That(disabled.tabIndex, Is.EqualTo(-1));
                Assert.That(BattlementOverlayItems.HasAuthored(disabled), Is.False);

                BattlementUiException inertFailure = Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Properties(
                                inertId,
                                new UiElement.Box
                                {
                                    Name = "inert-not-applied",
                                    OverlayPlacement = new OverlayPlacement.Modal(null, null),
                                }
                            )
                        )
                    )
                )!;
                Assert.That(inertFailure.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
                Assert.That(inert!.name, Is.EqualTo("inert"));
                Assert.That(inert.enabledSelf, Is.True);
                Assert.That(inert.focusable, Is.False);
                Assert.That(inert.tabIndex, Is.EqualTo(-1));
                Assert.That(BattlementOverlayItems.HasAuthored(inert), Is.False);

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            targetId,
                            new UiElement.Box
                            {
                                Name = "modal",
                                Focusable = true,
                                TabIndex = -1,
                                OverlayPlacement = new OverlayPlacement.Modal(null, null),
                            }
                        )
                    )
                );
                Assert.That(target.name, Is.EqualTo("modal"));
                Assert.That(target.focusable, Is.True);
                Assert.That(target.tabIndex, Is.EqualTo(-1));
                Assert.That(
                    BattlementOverlayItems.Get(target),
                    Is.EqualTo(new OverlayPlacement.Modal(null, null))
                );

                BattlementUiException resetFailure = Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Properties(
                                targetId,
                                new UiElement.Box
                                {
                                    Focusable = Prop<bool>.Reset(),
                                    TabIndex = Prop<int>.Reset(),
                                }
                            )
                        )
                    )
                )!;
                Assert.That(resetFailure.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
                Assert.That(target.focusable, Is.True);
                Assert.That(target.tabIndex, Is.EqualTo(-1));
                Assert.That(
                    BattlementOverlayItems.Get(target),
                    Is.EqualTo(new OverlayPlacement.Modal(null, null))
                );

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            resetId,
                            new UiElement.Box
                            {
                                Name = "reset-modal",
                                Enabled = Prop<bool>.Reset(),
                                Inert = Prop<bool>.Reset(),
                                OverlayPlacement = new OverlayPlacement.Modal(null, null),
                            }
                        )
                    )
                );
                Assert.That(reset!.name, Is.EqualTo("reset-modal"));
                Assert.That(reset.enabledSelf, Is.True);
                Assert.That(reset.focusable, Is.True);
                Assert.That(reset.tabIndex, Is.EqualTo(-1));
                Assert.That(
                    BattlementOverlayItems.Get(reset),
                    Is.EqualTo(new OverlayPlacement.Modal(null, null))
                );
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void StickyAdmissionResolvesPositionResetToTheConstructorStyle()
        {
            ObjectId documentId = Id("b00f3e79-5f10-46b6-a3e2-5b453c33b2ae");
            ObjectId rootId = Id("94eb8b42-9a48-4efb-8b38-d49daac0cc1d");
            ObjectId stackId = Id("2200357a-2bb9-4cb8-94df-3c8cde5ee13e");
            ObjectId scrollId = Id("5b1e54cd-6cb7-45b8-aefb-8faef8a532e6");
            ObjectId childId = Id("d4b77044-1aad-4861-9ec0-fdbed5f37973");
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
                                    stackId,
                                    new UiElement.Stack(),
                                    new UiNode[]
                                    {
                                        new(
                                            scrollId,
                                            new UiElement.ScrollView(),
                                            new UiNode[]
                                            {
                                                new(
                                                    childId,
                                                    new UiElement.Box
                                                    {
                                                        Style = new UiStyle
                                                        {
                                                            Position = UiStyle.Set(
                                                                UiPosition.Absolute
                                                            ),
                                                        },
                                                    }
                                                ),
                                            }
                                        ),
                                    }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );

                Assert.That(documents.TryGet(childId, out VisualElement? child), Is.True);
                Assert.That(
                    child!.style.position.value,
                    Is.EqualTo(UnityEngine.UIElements.Position.Absolute)
                );

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            childId,
                            new UiElement.Box
                            {
                                Style = new UiStyle { Position = UiStyle.Reset<UiPosition>() },
                                Sticky = new Sticky(0, null, null, null, 0),
                            }
                        )
                    )
                );

                Assert.That(
                    child.style.position.value,
                    Is.EqualTo(UnityEngine.UIElements.Position.Relative)
                );
                Assert.That(BattlementStickyItems.HasAuthored(child), Is.True);
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
