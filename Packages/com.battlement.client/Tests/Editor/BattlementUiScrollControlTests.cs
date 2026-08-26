#nullable enable

using System;
using System.Collections.Generic;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;
using UiScroller = Battlement.UiElement.Scroller;
using UiScrollView = Battlement.UiElement.ScrollView;

namespace Battlement.Tests
{
    public sealed class BattlementUiScrollControlTests
    {
        [Test]
        public void ScrollViewCoalescesChangesAndSettlesAtExactManualClockBoundary()
        {
            ObjectId documentId = Id("9dd25727-cb31-4d1d-9e71-4550b044db8e");
            ObjectId rootId = Id("240a5568-d871-4388-aea6-1cdb65710d8c");
            ObjectId scrollId = Id("b783381e-f530-4305-96b2-36d673fc9138");
            ObjectId childId = Id("cecf6358-4c19-46bc-91fe-90b48d977f83");
            TimeSpan now = TimeSpan.Zero;
            var events = new List<UiEvent>();
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            var documents = new BattlementUiDocuments(
                value =>
                {
                    events.Add(value);
                    return true;
                },
                now: () => now
            );
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
                                    scrollId,
                                    new UiScrollView
                                    {
                                        Mode = UiScrollViewMode.VerticalAndHorizontal,
                                        NestedInteraction = UiNestedInteraction.ForwardScrolling,
                                        HorizontalScrollerVisibility =
                                            UiScrollerVisibility.AlwaysVisible,
                                        VerticalScrollerVisibility =
                                            UiScrollerVisibility.AlwaysVisible,
                                        ScrollOffset = new Battlement.Vector(10, 20),
                                        HorizontalPageSize = 0.75f,
                                        VerticalPageSize = 1.25f,
                                        MouseWheelScrollSize = 36,
                                        TouchScrollBehavior = UiTouchScrollBehavior.Elastic,
                                        ScrollDecelerationRate = 0.135f,
                                        Elasticity = 0.1f,
                                        ElasticAnimationInterval = 16,
                                        Events = new[]
                                        {
                                            UiEventKind.ScrollChanged,
                                            UiEventKind.ScrollSettled,
                                        },
                                    },
                                    new UiNode[] { new(childId, new Battlement.UiElement.Box()) }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );

                Assert.That(documents.TryGet(scrollId, out VisualElement? value), Is.True);
                var scroll = (ScrollView)value!;
                Assert.That(scroll.mode, Is.EqualTo(ScrollViewMode.VerticalAndHorizontal));
                Assert.That(scroll.scrollOffset, Is.EqualTo(new Vector2(10, 20)));
                Assert.That(scroll.mouseWheelScrollSize, Is.EqualTo(36));
                Assert.That(documents.TryGet(childId, out VisualElement? child), Is.True);
                Assert.That(child!.parent, Is.SameAs(scroll.contentContainer));
                Assert.DoesNotThrow(() =>
                    documents.PerformAction(
                        new CommandBody.VisualElement.PerformAction(
                            scrollId,
                            new VisualElementAction.ScrollTo(childId)
                        )
                    )
                );

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            scrollId,
                            new UiScrollView { ScrollOffset = new Battlement.Vector(24, 32) }
                        )
                    )
                );
                now = TimeSpan.FromMilliseconds(100);
                documents.Advance();
                Assert.That(events, Is.Empty, "Rust offset writes must not arm settlement.");

                scroll.horizontalScroller.value += 8;
                scroll.verticalScroller.value += 12;
                documents.Advance();
                Assert.That(events, Has.Count.EqualTo(1));
                Assert.That(events[0].Body, Is.TypeOf<UiEventBody.ScrollChanged>());

                now = TimeSpan.FromMilliseconds(199);
                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            scrollId,
                            new UiScrollView { ScrollOffset = new Battlement.Vector(40, 56) }
                        )
                    )
                );
                documents.Advance();
                Assert.That(events, Has.Count.EqualTo(1));
                now = TimeSpan.FromMilliseconds(200);
                documents.Advance();
                Assert.That(
                    events,
                    Has.Count.EqualTo(1),
                    "Authored writes cancel armed settlement."
                );
                now = TimeSpan.FromMilliseconds(500);
                documents.Advance();
                Assert.That(events, Has.Count.EqualTo(1));

                SendCapture(scroll);
                scroll.horizontalScroller.value += 4;
                documents.Advance();
                Assert.That(events, Has.Count.EqualTo(2));
                now = TimeSpan.FromMilliseconds(600);
                documents.Advance();
                Assert.That(events, Has.Count.EqualTo(2), "Capture suppresses settlement.");
                SendCaptureOut(scroll);
                documents.Advance();
                Assert.That(events, Has.Count.EqualTo(3));
                Assert.That(events[2].Body, Is.TypeOf<UiEventBody.ScrollSettled>());
            }
            finally
            {
                documents.Clear();
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void ScrollerKeepsLocalProposalUntilReleaseThenRestoresCommittedValue()
        {
            ObjectId documentId = Id("33c23cd8-428d-43da-90dd-df7210aa0fa6");
            ObjectId rootId = Id("56c567c2-e601-4277-80b6-600982ae3cb7");
            ObjectId parentId = Id("78332f0c-f3d3-44a5-a03e-e31f57ef9c06");
            ObjectId scrollerId = Id("a30c4447-c294-4941-a067-fdb1b8352bd8");
            var events = new List<UiEvent>();
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            var documents = new BattlementUiDocuments(value =>
            {
                events.Add(value);
                return true;
            });
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
                                    parentId,
                                    new Battlement.UiElement.Box(),
                                    new UiNode[]
                                    {
                                        new(
                                            scrollerId,
                                            new UiScroller
                                            {
                                                LowValue = 0,
                                                HighValue = 100,
                                                Value = 25,
                                                Direction = UiSliderDirection.Horizontal,
                                                Events = new[]
                                                {
                                                    UiEventKind.ValueChanging,
                                                    UiEventKind.ValueCommitted,
                                                },
                                            }
                                        ),
                                    }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Assert.That(documents.TryGet(scrollerId, out VisualElement? value), Is.True);
                var scroller = (Scroller)value!;

                SendCapture(scroller.slider);
                scroller.value = 70;
                Assert.That(events, Has.Count.EqualTo(1));
                Assert.That(events[0].Body, Is.TypeOf<UiEventBody.ValueChanging>());
                Assert.That(
                    scroller.value,
                    Is.EqualTo(70),
                    "Proposal remains visible during drag."
                );
                SendCaptureOut(scroller.slider);
                Assert.That(events, Has.Count.EqualTo(2));
                Assert.That(events[1].Body, Is.TypeOf<UiEventBody.ValueCommitted>());
                Assert.That(scroller.value, Is.EqualTo(25));

                SendCaptureOut(scroller.slider);
                Assert.That(events, Has.Count.EqualTo(2), "Duplicate release must be ignored.");

                SendCapture(scroller.slider);
                scroller.value = 80;
                Assert.That(documents.TryGet(parentId, out VisualElement? parent), Is.True);
                parent!.SetEnabled(false);
                documents.Advance();
                Assert.That(
                    scroller.value,
                    Is.EqualTo(25),
                    "Ancestor disable rolls back proposal."
                );
                Assert.That(events, Has.Count.EqualTo(3), "Cancellation must not commit.");
                parent.SetEnabled(true);

                SendCapture(scroller.slider);
                scroller.value = 90;
                scroller.RemoveFromHierarchy();
                Assert.That(scroller.value, Is.EqualTo(25), "Detach rolls back proposal.");
                Assert.That(events, Has.Count.EqualTo(4), "Detach must not commit.");
                parent.Add(scroller);

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            scrollerId,
                            new UiScroller { Value = 68 }
                        )
                    )
                );
                Assert.That(scroller.value, Is.EqualTo(68));
                Assert.That(events, Has.Count.EqualTo(4), "Rust writes must be silent.");
            }
            finally
            {
                documents.Clear();
                Object.DestroyImmediate(owned);
            }
        }

        private static void SendCapture(VisualElement target)
        {
            using PointerCaptureEvent value = PointerCaptureEvent.GetPooled(
                target,
                null,
                PointerId.mousePointerId
            );
            target.SendEvent(value);
        }

        private static void SendCaptureOut(VisualElement target)
        {
            using PointerCaptureOutEvent value = PointerCaptureOutEvent.GetPooled(
                target,
                null,
                PointerId.mousePointerId
            );
            target.SendEvent(value);
        }

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
