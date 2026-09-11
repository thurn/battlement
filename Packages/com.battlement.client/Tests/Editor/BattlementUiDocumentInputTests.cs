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

        [Test]
        public void SyntheticPointerClickIsCanceledWhenTheDocumentIsReplaced()
        {
            ObjectId documentId = Id("de7a7dc4-1871-40e5-9bc9-ae8871396f2f");
            ObjectId rootId = Id("241f200c-49ce-487e-af2b-3a9c14232ea4");
            ObjectId buttonId = Id("4657445b-8d8d-4b25-bf4f-8f4ba0702e8f");
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
                UiDocument description = new(
                    documentId,
                    rootId,
                    Children: new[]
                    {
                        new UiNode(buttonId, new UiButton { Events = new[] { UiEventKind.Click } }),
                    }
                );
                documents.Replace(new[] { description }, id => id == documentId ? owned : null);
                Assert.That(
                    documents.BeginSyntheticPointer(
                        buttonId,
                        new Vector2(17, 31),
                        out string? beginDiagnostic
                    ),
                    Is.True,
                    beginDiagnostic
                );

                documents.Replace(new[] { description }, id => id == documentId ? owned : null);
                Assert.That(
                    documents.FinishSyntheticClick(buttonId, out string? finishDiagnostic),
                    Is.False
                );
                Assert.That(finishDiagnostic, Does.Contain("no pending synthetic click"));
                Assert.That(observed, Is.Empty);
            }
            finally
            {
                documents.Clear();
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void ReplacedSyntheticPointerTargetCannotReceiveALaterClick()
        {
            ObjectId documentId = Id("f1d29b0c-18b1-463a-ae50-3f8863d7edb0");
            ObjectId rootId = Id("719d4316-cf7f-4d4d-a6c1-4e8a20dc9a3d");
            ObjectId firstId = Id("d37b23dd-cb48-41bc-9474-a3a430842c16");
            ObjectId secondId = Id("56e3cfbe-9bf9-4f71-8a7d-caa2e5547a2c");
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
                documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            documentId,
                            rootId,
                            Children: new[]
                            {
                                new UiNode(
                                    firstId,
                                    new UiButton { Events = new[] { UiEventKind.Click } }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Assert.That(
                    documents.BeginSyntheticPointer(
                        firstId,
                        new Vector2(19, 29),
                        out string? beginDiagnostic
                    ),
                    Is.True,
                    beginDiagnostic
                );

                documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            documentId,
                            rootId,
                            Children: new[]
                            {
                                new UiNode(
                                    secondId,
                                    new UiButton { Events = new[] { UiEventKind.Click } }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Assert.That(
                    documents.FinishSyntheticClick(firstId, out string? finishDiagnostic),
                    Is.False
                );
                Assert.That(finishDiagnostic, Does.Contain("no pending synthetic click"));
                Assert.That(observed, Is.Empty);
            }
            finally
            {
                documents.Clear();
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void DestroyedSyntheticPointerTargetCannotReceiveALaterClick()
        {
            ObjectId documentId = Id("19749f35-f04f-4360-a3e3-6612ac46a159");
            ObjectId rootId = Id("41309d1e-3d4a-4dc0-95b4-58df228ab72a");
            ObjectId buttonId = Id("7dca668f-6f0c-4cce-a3e6-b59f6ab0bb82");
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
                documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            documentId,
                            rootId,
                            Children: new[]
                            {
                                new UiNode(
                                    buttonId,
                                    new UiButton { Events = new[] { UiEventKind.Click } }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Assert.That(
                    documents.BeginSyntheticPointer(
                        buttonId,
                        new Vector2(23, 37),
                        out string? beginDiagnostic
                    ),
                    Is.True,
                    beginDiagnostic
                );
                documents.Destroy(new CommandBody.VisualElement.Destroy(buttonId));
                Assert.That(
                    documents.FinishSyntheticClick(buttonId, out string? missingDiagnostic),
                    Is.False
                );
                Assert.That(missingDiagnostic, Does.Contain("no pending synthetic click"));
                Assert.That(observed, Is.Empty);
                documents.Create(
                    new CommandBody.VisualElement.Create(
                        rootId,
                        new UiNode(buttonId, new UiButton { Events = new[] { UiEventKind.Click } })
                    )
                );

                Assert.That(
                    documents.FinishSyntheticClick(buttonId, out string? finishDiagnostic),
                    Is.False
                );
                Assert.That(finishDiagnostic, Does.Contain("no pending synthetic click"));
                Assert.That(observed, Is.Empty);
            }
            finally
            {
                documents.Clear();
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void SyntheticPointerClickIsCanceledWhenInputIsDisabled()
        {
            ObjectId documentId = Id("e7fd8876-7100-4a38-92de-8e0318244534");
            ObjectId rootId = Id("5ae12a54-9b07-42bd-857d-a7670cecbf00");
            ObjectId buttonId = Id("f1a03d03-ebd4-4799-ae08-44c3aa44cd2f");
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
                UiDocument description = new(
                    documentId,
                    rootId,
                    Children: new[]
                    {
                        new UiNode(buttonId, new UiButton { Events = new[] { UiEventKind.Click } }),
                    }
                );
                documents.Replace(new[] { description }, id => id == documentId ? owned : null);
                Assert.That(
                    documents.BeginSyntheticPointer(
                        buttonId,
                        new Vector2(29, 43),
                        out string? beginDiagnostic
                    ),
                    Is.True,
                    beginDiagnostic
                );
                documents.SetInputEnabled(false);
                documents.SetInputEnabled(true);

                Assert.That(
                    documents.FinishSyntheticClick(buttonId, out string? finishDiagnostic),
                    Is.False
                );
                Assert.That(finishDiagnostic, Does.Contain("no pending synthetic click"));
                Assert.That(observed, Is.Empty);
            }
            finally
            {
                documents.Clear();
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void SyntheticPointerClickIsCanceledWhenDocumentsAreCleared()
        {
            ObjectId documentId = Id("cc8b7e8b-7ce1-4a16-a1c5-7ac359c7f053");
            ObjectId rootId = Id("5d627df5-d7ed-4f5e-bc43-992cc21ab6f0");
            ObjectId buttonId = Id("2d1c1284-4c8d-4db8-9594-f1e4bacb1e4b");
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
                UiDocument description = new(
                    documentId,
                    rootId,
                    Children: new[]
                    {
                        new UiNode(buttonId, new UiButton { Events = new[] { UiEventKind.Click } }),
                    }
                );
                documents.Replace(new[] { description }, id => id == documentId ? owned : null);
                Assert.That(
                    documents.BeginSyntheticPointer(
                        buttonId,
                        new Vector2(31, 47),
                        out string? beginDiagnostic
                    ),
                    Is.True,
                    beginDiagnostic
                );
                documents.Clear();
                documents.Replace(new[] { description }, id => id == documentId ? owned : null);
                Assert.That(
                    documents.FinishSyntheticClick(buttonId, out string? finishDiagnostic),
                    Is.False
                );
                Assert.That(finishDiagnostic, Does.Contain("no pending synthetic click"));
                Assert.That(observed, Is.Empty);
            }
            finally
            {
                documents.Clear();
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void SyntheticPointerClickCompletionIsConsumedAfterOneFinish()
        {
            ObjectId documentId = Id("a15b45c7-24af-48fc-a4fc-6c0fe89f3591");
            ObjectId rootId = Id("4012577a-4c2c-49f5-86e6-30eaf6facf31");
            ObjectId buttonId = Id("2e8cc3a9-e8e4-4d25-9b47-bc785293a5a8");
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
                documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            documentId,
                            rootId,
                            Children: new[]
                            {
                                new UiNode(
                                    buttonId,
                                    new UiButton { Events = new[] { UiEventKind.Click } }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Assert.That(
                    documents.BeginSyntheticPointer(
                        buttonId,
                        new Vector2(41, 53),
                        out string? beginDiagnostic
                    ),
                    Is.True,
                    beginDiagnostic
                );
                Assert.That(
                    documents.FinishSyntheticClick(buttonId, out string? firstDiagnostic),
                    Is.True,
                    firstDiagnostic
                );
                Assert.That(
                    documents.FinishSyntheticClick(buttonId, out string? secondDiagnostic),
                    Is.False
                );
                Assert.That(secondDiagnostic, Does.Contain("no pending synthetic click"));
                Assert.That(observed, Has.Count.EqualTo(1));
            }
            finally
            {
                documents.Clear();
                Object.DestroyImmediate(owned);
            }
        }
    }
}
