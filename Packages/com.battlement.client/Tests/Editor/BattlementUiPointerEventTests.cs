#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using Battlement.UI;
using Newtonsoft.Json.Linq;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;
using UiButton = Battlement.UiElement.Button;
using UiVisualElement = Battlement.UiElement.VisualElement;

namespace Battlement.Tests
{
    public sealed class BattlementUiPointerEventTests
    {
        [Test]
        public void OneNativePointerEventCreatesOneCompleteLogicalEvent()
        {
            ObjectId documentId = Id("22000000-0000-4000-8000-000000000021");
            ObjectId rootId = Id("22000000-0000-4000-8000-000000000022");
            ObjectId panelId = Id("22000000-0000-4000-8000-000000000023");
            ObjectId targetId = Id("22000000-0000-4000-8000-000000000024");
            var emitted = new List<UiEvent>();
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            var documents = new BattlementUiDocuments(value =>
            {
                emitted.Add(value);
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
                                    panelId,
                                    new UiVisualElement
                                    {
                                        EventSubscriptions = new[]
                                        {
                                            new UiEventSubscription(
                                                UiEventKind.PointerDown,
                                                UiEventPhase.Trickle
                                            ),
                                            new UiEventSubscription(
                                                UiEventKind.PointerDown,
                                                UiEventPhase.Bubble
                                            ),
                                        },
                                    },
                                    new UiNode[]
                                    {
                                        new(
                                            targetId,
                                            new UiButton
                                            {
                                                Text = "Route",
                                                Events = new[] { UiEventKind.PointerDown },
                                            }
                                        ),
                                    }
                                ),
                            },
                            EventSubscriptions: new[]
                            {
                                new UiEventSubscription(
                                    UiEventKind.PointerDown,
                                    UiEventPhase.Trickle
                                ),
                                new UiEventSubscription(
                                    UiEventKind.PointerDown,
                                    UiEventPhase.Bubble
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Assert.That(documents.TryGet(targetId, out VisualElement? target), Is.True);

                using PointerDownEvent value = PointerDownEvent.GetPooled(
                    new Event
                    {
                        type = EventType.MouseDown,
                        button = 0,
                        mousePosition = new UnityEngine.Vector2(31, 47),
                        modifiers = EventModifiers.Shift,
                    }
                );
                value.target = target;
                target!.SendEvent(value);

                Assert.That(emitted, Has.Count.EqualTo(1));
                Assert.That(emitted[0].TargetId, Is.EqualTo(targetId));
                var body = (UiEventBody.PointerDown)emitted[0].Body;
                Assert.That(body.Value.Position, Is.EqualTo(new PanelPoint(31, 47)));
                Assert.That(body.Value.Button, Is.Null, "Left is represented by omission.");
                Assert.That(body.Value.ClickCount, Is.EqualTo(1));
                Assert.That(body.Value.Modifiers, Is.EqualTo(new[] { KeyModifier.Shift }));
            }
            finally
            {
                documents.Clear();
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void UnsubscribedPointerMoveDoesNotEmit()
        {
            ObjectId documentId = Id("22000000-0000-4000-8000-000000000031");
            ObjectId rootId = Id("22000000-0000-4000-8000-000000000032");
            ObjectId targetId = Id("22000000-0000-4000-8000-000000000033");
            int emitted = 0;
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            var documents = new BattlementUiDocuments(_ =>
            {
                emitted++;
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
                            Children: new[] { new UiNode(targetId, new UiVisualElement()) }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Assert.That(documents.TryGet(targetId, out VisualElement? target), Is.True);
                using PointerMoveEvent value = PointerMoveEvent.GetPooled(
                    new Event { type = EventType.MouseMove, mousePosition = new Vector2(10, 12) }
                );
                value.target = target;
                target!.SendEvent(value);
                Assert.That(emitted, Is.Zero);
            }
            finally
            {
                documents.Clear();
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void PointerCaptureIsForwardedOnceFromTheLogicalTarget()
        {
            ObjectId documentId = Id("22000000-0000-4000-8000-000000000041");
            ObjectId rootId = Id("22000000-0000-4000-8000-000000000042");
            ObjectId targetId = Id("22000000-0000-4000-8000-000000000043");
            ObjectId panelId = Id("22000000-0000-4000-8000-000000000044");
            var emitted = new List<UiEvent>();
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            var documents = new BattlementUiDocuments(value =>
            {
                emitted.Add(value);
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
                            Children: new[]
                            {
                                new UiNode(
                                    panelId,
                                    new UiVisualElement
                                    {
                                        EventSubscriptions = new[]
                                        {
                                            new UiEventSubscription(
                                                UiEventKind.PointerCapture,
                                                UiEventPhase.Trickle
                                            ),
                                            new UiEventSubscription(
                                                UiEventKind.PointerCapture,
                                                UiEventPhase.Bubble
                                            ),
                                        },
                                    },
                                    new[]
                                    {
                                        new UiNode(
                                            targetId,
                                            new UiVisualElement
                                            {
                                                Events = new[] { UiEventKind.PointerCapture },
                                            }
                                        ),
                                    }
                                ),
                            },
                            EventSubscriptions: new[]
                            {
                                new UiEventSubscription(
                                    UiEventKind.PointerCapture,
                                    UiEventPhase.Trickle
                                ),
                                new UiEventSubscription(
                                    UiEventKind.PointerCapture,
                                    UiEventPhase.Bubble
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Assert.That(documents.TryGet(targetId, out VisualElement? target), Is.True);

                using PointerCaptureEvent value = PointerCaptureEvent.GetPooled(target, null, 7);
                value.target = target;
                target!.SendEvent(value);

                Assert.That(emitted, Has.Count.EqualTo(1));
                Assert.That(emitted[0].TargetId, Is.EqualTo(targetId));
                var body = (UiEventBody.PointerCapture)emitted[0].Body;
                Assert.That(body.Value.PointerId, Is.EqualTo(7));
            }
            finally
            {
                documents.Clear();
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void PointerActionJsonKeepsTheEventTagAndOmitsDefaults()
        {
            byte[] bytes = BattlementJson.SerializeAction(
                new Battlement.Action(
                    new ActionId(Guid.Parse("22000000-0000-4000-8000-000000000051")),
                    new SessionId(Guid.Parse("22000000-0000-4000-8000-000000000052")),
                    new ActionBody.VisualElement(
                        Id("22000000-0000-4000-8000-000000000053"),
                        new UiEventBody.PointerDown(
                            new UiPointerButtonEvent(
                                new PanelPoint(12, 34),
                                new Battlement.Vector(0, 0)
                            )
                        )
                    )
                )
            );
            JObject root = JObject.Parse(Encoding.UTF8.GetString(bytes));
            JToken payload = root.SelectToken("Action.body.VisualElement.body.PointerDown")!;

            Assert.That(payload.SelectToken("position.x")!.Value<double>(), Is.EqualTo(12));
            Assert.That(payload["pointer_id"], Is.Null);
            Assert.That(payload["button"], Is.Null);
            Assert.That(payload["click_count"], Is.Null);
        }

        [Test]
        public void PickedCrossingsReportSiblingAncestorAndDocumentRelations()
        {
            using var fixture = new CrossingFixture();

            fixture.Cross(UiEventKind.PointerOver, fixture.Left, new Vector2(40, 40));
            fixture.Events.Clear();
            fixture.Cross(UiEventKind.PointerOut, fixture.Left, new Vector2(160, 40));
            fixture.Move(fixture.Right, new Vector2(160, 40));
            fixture.Cross(UiEventKind.PointerOver, fixture.Right, new Vector2(160, 40));
            AssertCrossing(fixture.Events[0], fixture.LeftId, fixture.RightId);
            Assert.That(fixture.Events[1].Body, Is.TypeOf<UiEventBody.PointerMove>());
            AssertCrossing(fixture.Events[2], fixture.RightId, fixture.LeftId);

            fixture.Events.Clear();
            fixture.Cross(UiEventKind.PointerOut, fixture.Right, new Vector2(230, 100));
            fixture.Cross(UiEventKind.PointerOver, fixture.Parent, new Vector2(230, 100));
            AssertCrossing(fixture.Events[0], fixture.RightId, fixture.ParentId);
            AssertCrossing(fixture.Events[1], fixture.ParentId, fixture.RightId);

            fixture.Events.Clear();
            fixture.Cross(UiEventKind.PointerOut, fixture.Parent, new Vector2(500, 500));
            AssertCrossing(fixture.Events.Single(), fixture.ParentId, null);
            TestContext.Out.WriteLine(
                string.Join(
                    Environment.NewLine,
                    fixture.Journal.Select(value =>
                        Encoding.UTF8.GetString(
                            BattlementJson.SerializeAction(
                                new Battlement.Action(
                                    new ActionId(Guid.NewGuid()),
                                    new SessionId(Guid.NewGuid()),
                                    new ActionBody.VisualElement(value.TargetId, value.Body)
                                )
                            )
                        )
                    )
                )
            );
        }

        [Test]
        public void CancelledPointerIdentityStartsItsNextCrossingOutsideTheDocument()
        {
            using var fixture = new CrossingFixture();
            fixture.Cross(UiEventKind.PointerOver, fixture.Left, new Vector2(40, 40));
            fixture.Cancel(fixture.Left, new Vector2(40, 40));

            fixture.Events.Clear();
            fixture.Cross(UiEventKind.PointerOver, fixture.Right, new Vector2(160, 40));

            AssertCrossing(fixture.Events.Single(), fixture.RightId, null);
        }

        [Test]
        public void DestroyedPickedTargetCannotBecomeARelatedTarget()
        {
            using var fixture = new CrossingFixture();
            fixture.Cross(UiEventKind.PointerOver, fixture.Left, new Vector2(40, 40));
            fixture.Destroy(fixture.LeftId);

            fixture.Events.Clear();
            fixture.Cross(UiEventKind.PointerOver, fixture.Right, new Vector2(160, 40));

            AssertCrossing(fixture.Events.Single(), fixture.RightId, null);
        }

        private static void AssertCrossing(UiEvent value, ObjectId target, ObjectId? related)
        {
            Assert.That(value.TargetId, Is.EqualTo(target));
            UiPointerCrossingEvent crossing = value.Body switch
            {
                UiEventBody.PointerOver over => over.Value,
                UiEventBody.PointerOut pointerOut => pointerOut.Value,
                _ => throw new AssertionException("Expected a pointer crossing action."),
            };
            Assert.That(crossing.RelatedTargetId, Is.EqualTo(related));
        }

        private static ObjectId Id(string value) => new(Guid.Parse(value));

        private sealed class CrossingFixture : IDisposable
        {
            private readonly BattlementUiDocuments documents;
            private readonly GameObject owned;

            public CrossingFixture()
            {
                ObjectId documentId = Id("22000000-0000-4000-8000-000000000061");
                ObjectId rootId = Id("22000000-0000-4000-8000-000000000062");
                ParentId = Id("22000000-0000-4000-8000-000000000063");
                LeftId = Id("22000000-0000-4000-8000-000000000064");
                RightId = Id("22000000-0000-4000-8000-000000000065");
                owned = BattlementUiDocuments.CreateGameObject(
                    new GameObjectKind.UiDocumentState(rootId)
                );
                documents = new BattlementUiDocuments(value =>
                {
                    Events.Add(value);
                    Journal.Add(value);
                    return true;
                });
                UiEventKind[] crossingEvents =
                {
                    UiEventKind.PointerOver,
                    UiEventKind.PointerOut,
                    UiEventKind.PointerMove,
                    UiEventKind.PointerCancel,
                };
                documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            documentId,
                            rootId,
                            Children: new[]
                            {
                                new UiNode(
                                    ParentId,
                                    new UiVisualElement { Events = crossingEvents },
                                    new[]
                                    {
                                        new UiNode(
                                            LeftId,
                                            new UiVisualElement { Events = crossingEvents }
                                        ),
                                        new UiNode(
                                            RightId,
                                            new UiVisualElement { Events = crossingEvents }
                                        ),
                                    }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Assert.That(documents.TryGet(rootId, out VisualElement? root), Is.True);
                Assert.That(documents.TryGet(ParentId, out VisualElement? parent), Is.True);
                Assert.That(documents.TryGet(LeftId, out VisualElement? left), Is.True);
                Assert.That(documents.TryGet(RightId, out VisualElement? right), Is.True);
                Root = root!;
                Parent = parent!;
                Left = left!;
                Right = right!;
                Place(Root, 0, 0, 300, 160);
                Place(Parent, 10, 10, 250, 120);
                Place(Left, 10, 10, 80, 80);
                Place(Right, 130, 10, 80, 80);
            }

            public List<UiEvent> Events { get; } = new();

            public List<UiEvent> Journal { get; } = new();

            public ObjectId ParentId { get; }

            public ObjectId LeftId { get; }

            public ObjectId RightId { get; }

            public VisualElement Root { get; }

            public VisualElement Parent { get; }

            public VisualElement Left { get; }

            public VisualElement Right { get; }

            public void Cross(UiEventKind kind, VisualElement target, Vector2 position)
            {
                using PointerMoveEvent trigger = PointerMoveEvent.GetPooled(
                    new Event { type = EventType.MouseMove, mousePosition = position }
                );
                using EventBase value = kind switch
                {
                    UiEventKind.PointerOver => PointerOverEvent.GetPooled(trigger),
                    UiEventKind.PointerOut => PointerOutEvent.GetPooled(trigger),
                    _ => throw new AssertionException("Expected a pointer crossing kind."),
                };
                value.target = target;
                target.SendEvent(value);
            }

            public void Move(VisualElement target, Vector2 position)
            {
                using PointerMoveEvent value = PointerMoveEvent.GetPooled(
                    new Event { type = EventType.MouseMove, mousePosition = position }
                );
                value.target = target;
                target.SendEvent(value);
            }

            public void Cancel(VisualElement target, Vector2 position)
            {
                using PointerMoveEvent trigger = PointerMoveEvent.GetPooled(
                    new Event { type = EventType.MouseMove, mousePosition = position }
                );
                using PointerCancelEvent value = PointerCancelEvent.GetPooled(trigger);
                value.target = target;
                target.SendEvent(value);
            }

            public void Destroy(ObjectId objectId) =>
                documents.Destroy(new CommandBody.VisualElement.Destroy(objectId));

            public void Dispose()
            {
                documents.Clear();
                Object.DestroyImmediate(owned);
            }

            private static void Place(
                VisualElement value,
                float left,
                float top,
                float width,
                float height
            )
            {
                value.style.position = Position.Absolute;
                value.style.left = left;
                value.style.top = top;
                value.style.width = width;
                value.style.height = height;
            }
        }
    }
}
