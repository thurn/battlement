#nullable enable

using System;
using System.Collections.Generic;
using System.Reflection;
using System.Text;
using Battlement.UI;
using Newtonsoft.Json.Linq;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;
using UiTextElement = Battlement.UiElement.TextElement;
using UnityPointerOutLinkTagEvent = UnityEngine.UIElements.Experimental.PointerOutLinkTagEvent;
using UnityPointerOverLinkTagEvent = UnityEngine.UIElements.Experimental.PointerOverLinkTagEvent;

namespace Battlement.Tests
{
    public sealed class BattlementUiLifecycleEventTests
    {
        [Test]
        public void LinkLeaveRestoresTheMatchingEnteredIdentityAndDropsAnUnmatchedLeave()
        {
            using var fixture = new Fixture(
                UiEventKind.LinkEnter,
                UiEventKind.LinkLeave,
                UiEventKind.LinkDown,
                UiEventKind.LinkUp
            );
            using PointerMoveEvent pointer = PointerMoveEvent.GetPooled(
                new Event { type = EventType.MouseMove, mousePosition = new Vector2(23, 41) }
            );
            using UnityPointerOverLinkTagEvent enter = UnityPointerOverLinkTagEvent.GetPooled(
                pointer,
                "field-guide",
                "Field guide"
            );
            enter.target = fixture.Target;
            fixture.Target.SendEvent(enter);
            using UnityPointerOutLinkTagEvent leave = UnityPointerOutLinkTagEvent.GetPooled(
                pointer,
                "ignored-native-id"
            );
            leave.target = fixture.Target;
            fixture.Target.SendEvent(leave);
            using UnityPointerOutLinkTagEvent unmatched = UnityPointerOutLinkTagEvent.GetPooled(
                pointer,
                "ignored-native-id"
            );
            unmatched.target = fixture.Target;
            fixture.Target.SendEvent(unmatched);

            Assert.That(fixture.Events, Has.Count.EqualTo(2));
            var entered = (UiEventBody.LinkEnter)fixture.Events[0].Body;
            var left = (UiEventBody.LinkLeave)fixture.Events[1].Body;
            Assert.That(left.Value.LinkId, Is.EqualTo(entered.Value.LinkId));
            Assert.That(left.Value.LinkText, Is.EqualTo(entered.Value.LinkText));
            Assert.That(left.Value.Position, Is.EqualTo(new PanelPoint(23, 41)));
            Assert.That(left.Value.Button, Is.Null);
        }

        [Test]
        public void GeometryIsTargetOnlyAndPreservesOldAndNewRectangles()
        {
            using var fixture = new Fixture(UiEventKind.GeometryChanged);
            using GeometryChangedEvent value = GeometryChangedEvent.GetPooled(
                new UnityEngine.Rect(1, 2, 30, 40),
                new UnityEngine.Rect(5, 6, 70, 80)
            );
            value.target = fixture.Target;
            fixture.Target.SendEvent(value);

            Assert.That(fixture.Events, Has.Count.EqualTo(1));
            var body = (UiEventBody.GeometryChanged)fixture.Events[0].Body;
            Assert.That(body.Value.Previous, Is.EqualTo(new Battlement.Rect(1, 2, 30, 40)));
            Assert.That(body.Value.Current, Is.EqualTo(new Battlement.Rect(5, 6, 70, 80)));
        }

        [Test]
        public void SelectableTextCoalescesCursorAndAnchorCallbacksUntilAdvance()
        {
            using var fixture = new Fixture(UiEventKind.SelectionChanged);
            var selection = (ITextSelection)fixture.Target;
            selection.cursorIndex = 4;
            selection.selectIndex = 1;
            int cursorIndex = selection.cursorIndex;
            int selectionIndex = selection.selectIndex;
            Assert.That(fixture.Events, Is.Empty);
            fixture.Documents.Advance();

            Assert.That(fixture.Events, Has.Count.EqualTo(1));
            var body = (UiEventBody.SelectionChanged)fixture.Events[0].Body;
            Assert.That(body.Value.CursorIndex, Is.EqualTo(cursorIndex));
            Assert.That(body.Value.SelectionIndex, Is.EqualTo(selectionIndex));
            fixture.Documents.Advance();
            Assert.That(fixture.Events, Has.Count.EqualTo(1));
        }

        [Test]
        public void LinkJsonOmitsDefaultPointerAndButton()
        {
            byte[] bytes = BattlementJson.SerializeUiEventAction(
                new UiEventAction(
                    new ActionId(Guid.NewGuid()),
                    new SessionId(Guid.NewGuid()),
                    new UiEvent(
                        new ObjectId(Guid.NewGuid()),
                        new UiEventBody.LinkEnter(
                            new LinkEvent("field-guide", "FIELD GUIDE", new PanelPoint(12, 34))
                        )
                    )
                )
            );
            JToken payload = JObject
                .Parse(Encoding.UTF8.GetString(bytes))
                .SelectToken("event.body.LinkEnter")!;

            Assert.That(payload["pointer_id"], Is.Null);
            Assert.That(payload["button"], Is.Null);
        }

        [Test]
        public void DisabledInputCannotRepopulateClearedLinkIdentity()
        {
            using var fixture = new Fixture(UiEventKind.LinkEnter, UiEventKind.LinkLeave);
            SendEnter(fixture.Target, "before-disable", "Before disable");
            Assert.That(LinkIdentityCount(fixture.Documents), Is.EqualTo(1));
            fixture.Documents.SetInputEnabled(false);
            Assert.That(LinkIdentityCount(fixture.Documents), Is.Zero);
            fixture.Events.Clear();
            SendEnter(fixture.Target, "disabled-id", "Disabled");
            Assert.That(LinkIdentityCount(fixture.Documents), Is.Zero);
            fixture.Documents.SetInputEnabled(true);
            SendLeave(fixture.Target);

            Assert.That(fixture.Events, Is.Empty);
        }

        [Test]
        public void DetachDestructionAndReplacementClearLinkIdentity()
        {
            using var fixture = new Fixture(UiEventKind.LinkEnter, UiEventKind.LinkLeave);
            SendEnter(fixture.Target, "detach", "Detach");
            using UnityEngine.UIElements.DetachFromPanelEvent detach =
                UnityEngine.UIElements.DetachFromPanelEvent.GetPooled(null, null);
            detach.target = fixture.Target;
            fixture.Target.SendEvent(detach);
            Assert.That(LinkIdentityCount(fixture.Documents), Is.Zero);

            SendEnter(fixture.Target, "destroy", "Destroy");
            fixture.Documents.Destroy(new CommandBody.VisualElement.Destroy(fixture.TargetId));
            Assert.That(LinkIdentityCount(fixture.Documents), Is.Zero);
            SendEnter(fixture.Target, "stale", "Stale");
            Assert.That(LinkIdentityCount(fixture.Documents), Is.Zero);

            fixture.Documents.Replace(Array.Empty<UiDocument>(), _ => null);
            Assert.That(LinkIdentityCount(fixture.Documents), Is.Zero);
            SendEnter(fixture.Target, "replacement", "Replacement");
            Assert.That(LinkIdentityCount(fixture.Documents), Is.Zero);
        }

        private static void SendEnter(
            UnityEngine.UIElements.TextElement target,
            string linkId,
            string linkText
        )
        {
            using PointerMoveEvent pointer = PointerMoveEvent.GetPooled(
                new Event { type = EventType.MouseMove, mousePosition = new Vector2(23, 41) }
            );
            using UnityPointerOverLinkTagEvent enter = UnityPointerOverLinkTagEvent.GetPooled(
                pointer,
                linkId,
                linkText
            );
            enter.target = target;
            target.SendEvent(enter);
        }

        private static int LinkIdentityCount(BattlementUiDocuments documents) =>
            (int)
                typeof(BattlementUiDocuments)
                    .GetProperty(
                        "LinkIdentityCount",
                        BindingFlags.Instance | BindingFlags.NonPublic
                    )!
                    .GetValue(documents)!;

        private static void SendLeave(UnityEngine.UIElements.TextElement target)
        {
            using PointerMoveEvent pointer = PointerMoveEvent.GetPooled(
                new Event { type = EventType.MouseMove, mousePosition = new Vector2(23, 41) }
            );
            using UnityPointerOutLinkTagEvent leave = UnityPointerOutLinkTagEvent.GetPooled(
                pointer,
                "ignored-native-id"
            );
            leave.target = target;
            target.SendEvent(leave);
        }

        private sealed class Fixture : IDisposable
        {
            private readonly GameObject owner;

            public Fixture(params UiEventKind[] events)
            {
                ObjectId documentId = new(Guid.NewGuid());
                ObjectId rootId = new(Guid.NewGuid());
                TargetId = new ObjectId(Guid.NewGuid());
                owner = BattlementUiDocuments.CreateGameObject(
                    new GameObjectKind.UiDocumentState(rootId)
                );
                Documents = new BattlementUiDocuments(value =>
                {
                    Events.Add(value);
                    return UiEventDisposition.Continue;
                });
                Documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            documentId,
                            rootId,
                            Children: new[]
                            {
                                new UiNode(
                                    TargetId,
                                    new UiTextElement
                                    {
                                        Text = "Read the field guide",
                                        Selectable = true,
                                        Events = events,
                                    }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owner : null
                );
                Documents.SetInputEnabled(true);
                Assert.That(Documents.TryGet(TargetId, out VisualElement? target), Is.True);
                Target = (UnityEngine.UIElements.TextElement)target!;
                Events.Clear();
            }

            public BattlementUiDocuments Documents { get; }
            public List<UiEvent> Events { get; } = new();
            public UnityEngine.UIElements.TextElement Target { get; }
            public ObjectId TargetId { get; }

            public void Dispose()
            {
                Documents.Clear();
                Object.DestroyImmediate(owner);
            }
        }
    }
}
