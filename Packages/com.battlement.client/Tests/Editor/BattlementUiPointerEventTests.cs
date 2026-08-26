#nullable enable

using System;
using System.Collections.Generic;
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

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
