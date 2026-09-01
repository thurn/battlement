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
        public void UnavailableLayoutStateIsRejectedWithoutChangingTheNativeTree()
        {
            ObjectId documentId = Id("7d175052-06d3-46a6-87e4-d12a711db84c");
            ObjectId rootId = Id("34cd0664-cbaf-41f5-aa2b-31c0a102fc56");
            ObjectId layoutId = Id("831053d0-6a4f-49f7-9750-e98f5207cdba");
            ObjectId ordinaryId = Id("c1dcd487-b065-44db-8864-280f365db147");
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
                BattlementUiException? createFailure = Assert.Throws<BattlementUiException>(() =>
                    documents.Create(
                        new CommandBody.VisualElement.Create(
                            rootId,
                            new UiNode(layoutId, new UiElement.Stack())
                        )
                    )
                );
                Assert.That(createFailure!.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
                StringAssert.Contains("not enabled", createFailure.Message);
                Assert.That(documents.TryGet(layoutId, out _), Is.False);

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
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
