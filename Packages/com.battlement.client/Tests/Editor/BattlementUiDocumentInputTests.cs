#nullable enable

using System;
using System.Collections.Generic;
using System.Reflection;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using static Battlement.Tests.BattlementUiDocumentSupport;
using Object = UnityEngine.Object;
using UiButton = Battlement.UiElement.Button;
using UiRepeatButton = Battlement.UiElement.RepeatButton;
using UiVisualElement = Battlement.UiElement.VisualElement;

namespace Battlement.Tests
{
    public sealed class BattlementUiDocumentInputTests
    {
        [Test]
        public void SemanticClickAndRepeatTimingUseOneForwardingRoute()
        {
            ObjectId documentId = Id("f4208d7a-c0ad-4345-84fc-e12f50612e04");
            ObjectId rootId = Id("67bbd0b2-cdcc-4e97-b45a-2ada85cfaf3a");
            ObjectId containerId = Id("bbba2aef-cd90-477e-8d57-70935a0baa32");
            ObjectId buttonId = Id("c8b7d514-53b4-40aa-97cc-fc75a24da37d");
            ObjectId repeatId = Id("e103f40c-f5e0-45c6-94f3-e6726133cd38");
            var events = new List<UiEvent>();
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            var documents = new BattlementUiDocuments(value =>
            {
                events.Add(value);
                return UiEventDisposition.Continue;
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
                                    containerId,
                                    new UiVisualElement
                                    {
                                        Events = new[] { UiEventKind.Click },
                                        EventSubscriptions = new[]
                                        {
                                            new UiEventSubscription(
                                                UiEventKind.Click,
                                                UiEventPhase.Bubble
                                            ),
                                        },
                                    },
                                    new UiNode[]
                                    {
                                        new(buttonId, new UiButton { Text = "Confirm" }),
                                        new(
                                            repeatId,
                                            new UiRepeatButton
                                            {
                                                Text = "Hold",
                                                DelayMs = 300,
                                                IntervalMs = 100,
                                            }
                                        ),
                                    }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Assert.That(documents.TryGet(buttonId, out VisualElement? button), Is.True);
                Assert.That(button, Is.TypeOf<Button>());
                Assert.That(
                    documents.DispatchSemanticActivation(buttonId, out string? diagnostic),
                    Is.True,
                    diagnostic
                );
                Assert.That(events, Has.Count.EqualTo(1));
                Assert.That(events[0].TargetId, Is.EqualTo(buttonId));
                Assert.That(events[0].Body, Is.TypeOf<UiEventBody.Click>());
                Assert.That(
                    ((UiEventBody.Click)events[0].Body).Value,
                    Is.TypeOf<Battlement.ClickEvent.NavigationSubmit>()
                );

                FieldInfo controlsField = typeof(BattlementUiDocuments).GetField(
                    "repeatControls",
                    BindingFlags.Instance | BindingFlags.NonPublic
                )!;
                object repeatControls = controlsField.GetValue(documents)!;
                FieldInfo actionsField = repeatControls
                    .GetType()
                    .GetField("actions", BindingFlags.Instance | BindingFlags.NonPublic)!;
                var actions =
                    (Dictionary<Guid, System.Action>)actionsField.GetValue(repeatControls)!;
                System.Action retained = actions[repeatId.Value];
                retained();
                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            repeatId,
                            new UiRepeatButton { DelayMs = 200, IntervalMs = 80 }
                        )
                    )
                );
                Assert.That(actions[repeatId.Value], Is.SameAs(retained));
                retained();
                Assert.That(events, Has.Count.EqualTo(3));
                Assert.That(events[1].TargetId, Is.EqualTo(containerId));
                Assert.That(
                    ((UiEventBody.Click)events[1].Body).Value,
                    Is.TypeOf<Battlement.ClickEvent.Repeat>()
                );
                Assert.That(
                    ((UiEventBody.Click)events[2].Body).Value,
                    Is.TypeOf<Battlement.ClickEvent.Repeat>()
                );
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void SyntheticHoverLeavesThePreviousTargetAndSupportsEnterOnlyRoutes()
        {
            ObjectId documentId = Id("68a6965d-894e-44b0-a8c7-56e87b58f7da");
            ObjectId rootId = Id("5e11956a-3240-4f51-b2d8-6f7811554cce");
            ObjectId firstId = Id("a667781b-7945-46af-98b6-bf495c1c814a");
            ObjectId secondId = Id("ca0e1666-e364-4d13-812e-bbc8dff4e57b");
            var observed = new List<UiEvent>();
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            var documents = new BattlementUiDocuments(value =>
            {
                observed.Add(value);
                return UiEventDisposition.Continue;
            });
            try
            {
                UiEventKind[] hoverEvents = { UiEventKind.PointerEnter, UiEventKind.PointerLeave };
                documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            documentId,
                            rootId,
                            Children: new UiNode[]
                            {
                                new(firstId, new UiButton { Events = hoverEvents }),
                                new(secondId, new UiButton { Events = hoverEvents }),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );

                Assert.That(
                    documents.DispatchSyntheticHover(
                        firstId,
                        new Vector2(0, 0),
                        out string? firstDiagnostic
                    ),
                    Is.True,
                    firstDiagnostic
                );
                Assert.That(
                    documents.DispatchSyntheticHover(
                        secondId,
                        new Vector2(0, 0),
                        out string? secondDiagnostic
                    ),
                    Is.True,
                    secondDiagnostic
                );
                Assert.That(observed, Has.Count.EqualTo(3));
                Assert.That(observed[0].TargetId, Is.EqualTo(firstId));
                Assert.That(observed[0].Body, Is.TypeOf<UiEventBody.PointerEnter>());
                Assert.That(observed[1].TargetId, Is.EqualTo(firstId));
                Assert.That(observed[1].Body, Is.TypeOf<UiEventBody.PointerLeave>());
                Assert.That(observed[2].TargetId, Is.EqualTo(secondId));
                Assert.That(observed[2].Body, Is.TypeOf<UiEventBody.PointerEnter>());
            }
            finally
            {
                documents.Clear();
                Object.DestroyImmediate(owned);
            }
        }
    }
}
