#nullable enable

using System;
using System.Collections.Generic;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class BattlementFocusCoordinatorTests
    {
        [Test]
        public void InertSubtreeSuppressesInteractionAndRestoresLatestAuthoredState()
        {
            using var fixture = new Fixture();

            Assert.That(fixture.Target.focusable, Is.False);
            Assert.That(fixture.Target.tabIndex, Is.EqualTo(-1));
            Assert.That(fixture.Target.pickingMode, Is.EqualTo(PickingMode.Ignore));
            Assert.Throws<BattlementUiException>(() => fixture.FocusTarget());
            fixture.SendPointerDown();
            Assert.That(fixture.Events, Is.Empty);

            fixture.UpdateTarget(tabIndex: 7);
            Assert.That(fixture.Target.focusable, Is.False);
            Assert.That(fixture.Target.tabIndex, Is.EqualTo(-1));
            Assert.That(fixture.Target.pickingMode, Is.EqualTo(PickingMode.Ignore));

            fixture.SetInert(false);
            Assert.That(fixture.Target.focusable, Is.True);
            Assert.That(fixture.Target.tabIndex, Is.EqualTo(7));
            Assert.That(fixture.Target.pickingMode, Is.EqualTo(PickingMode.Position));
            fixture.SendPointerDown();
            Assert.That(fixture.Events, Has.Count.EqualTo(1));
        }

        [Test]
        public void AutoFocusIsOneShotAndFocusVisibleTracksInputModality()
        {
            using var fixture = new Fixture();

            Assert.That(
                fixture.AutoFocus.panel!.focusController.focusedElement,
                Is.SameAs(fixture.AutoFocus)
            );
            fixture.SetInert(false);
            fixture.FocusTarget();
            fixture.UpdateAutoFocusName();
            Assert.That(
                fixture.Target.panel!.focusController.focusedElement,
                Is.SameAs(fixture.Target)
            );

            fixture.SendNavigationKey();
            Assert.That(
                fixture.Target.ClassListContains(BattlementFocusCoordinator.FocusVisibleClass),
                Is.True
            );
            fixture.BlurTarget();
            Assert.That(
                fixture.Target.ClassListContains(BattlementFocusCoordinator.FocusVisibleClass),
                Is.False
            );
            fixture.FocusTarget();
            fixture.SendNavigationKey();
            fixture.ReparentTarget();
            Assert.That(
                fixture.Target.panel!.focusController.focusedElement,
                Is.SameAs(fixture.Target)
            );
            fixture.SendPointerDown();
            Assert.That(
                fixture.Target.ClassListContains(BattlementFocusCoordinator.FocusVisibleClass),
                Is.False
            );
        }

        [Test]
        public void AutoFocusWaitsUntilTheCompleteCommitIsInstalled()
        {
            ObjectId documentId = Id("25120000-0000-4000-8000-000000000011");
            ObjectId rootId = Id("25120000-0000-4000-8000-000000000012");
            ObjectId targetId = Id("25120000-0000-4000-8000-000000000013");
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            var documents = new BattlementUiDocuments();
            try
            {
                documents.BeginCommit();
                documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            documentId,
                            rootId,
                            Children: new UiNode[]
                            {
                                new(
                                    targetId,
                                    new UiElement.Box { Focusable = true, AutoFocus = true }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Assert.That(documents.TryGet(targetId, out VisualElement? target), Is.True);
                Assert.That(target!.panel!.focusController.focusedElement, Is.Not.SameAs(target));

                documents.EndCommit();

                Assert.That(target.panel.focusController.focusedElement, Is.SameAs(target));
            }
            finally
            {
                documents.Clear();
                Object.DestroyImmediate(owned);
            }
        }

        private sealed class Fixture : IDisposable
        {
            private readonly ObjectId containerId = Id("25120000-0000-4000-8000-000000000003");
            private readonly ObjectId targetId = Id("25120000-0000-4000-8000-000000000004");
            private readonly ObjectId autoFocusId = Id("25120000-0000-4000-8000-000000000005");
            private readonly ObjectId secondContainerId = Id(
                "25120000-0000-4000-8000-000000000006"
            );
            private readonly GameObject owned;

            public Fixture()
            {
                ObjectId documentId = Id("25120000-0000-4000-8000-000000000001");
                ObjectId rootId = Id("25120000-0000-4000-8000-000000000002");
                Events = new List<UiEvent>();
                owned = BattlementUiDocuments.CreateGameObject(
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
                            Children: new UiNode[]
                            {
                                new(
                                    containerId,
                                    new UiElement.Box { Inert = true },
                                    new UiNode[]
                                    {
                                        new(
                                            targetId,
                                            new UiElement.Box
                                            {
                                                Focusable = true,
                                                TabIndex = 3,
                                                PickingMode = UiPickingMode.Position,
                                                Events = new[] { UiEventKind.PointerDown },
                                            }
                                        ),
                                    }
                                ),
                                new(
                                    autoFocusId,
                                    new UiElement.Box { Focusable = true, AutoFocus = true }
                                ),
                                new(secondContainerId, new UiElement.Box()),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Target = Get(targetId);
                AutoFocus = Get(autoFocusId);
            }

            public BattlementUiDocuments Documents { get; }
            public List<UiEvent> Events { get; }
            public VisualElement Target { get; }
            public VisualElement AutoFocus { get; }

            public void Dispose()
            {
                Documents.Clear();
                Object.DestroyImmediate(owned);
            }

            public void FocusTarget() =>
                Documents.PerformAction(
                    new CommandBody.VisualElement.PerformAction(
                        targetId,
                        new VisualElementAction.Focus()
                    )
                );

            public void BlurTarget() =>
                Documents.PerformAction(
                    new CommandBody.VisualElement.PerformAction(
                        targetId,
                        new VisualElementAction.Blur()
                    )
                );

            public void ReparentTarget() =>
                Documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Parent(targetId, secondContainerId)
                    )
                );

            public void SendNavigationKey()
            {
                using KeyDownEvent value = KeyDownEvent.GetPooled(
                    '\0',
                    KeyCode.LeftArrow,
                    EventModifiers.None
                );
                value.target = Target;
                Target.SendEvent(value);
            }

            public void SendPointerDown()
            {
                using PointerDownEvent value = PointerDownEvent.GetPooled(
                    new Event { type = EventType.MouseDown, button = 0 }
                );
                value.target = Target;
                Target.SendEvent(value);
            }

            public void SetInert(bool value) =>
                Update(containerId, new UiElement.Box { Inert = value });

            public void UpdateAutoFocusName() =>
                Update(autoFocusId, new UiElement.Box { Name = "updated" });

            public void UpdateTarget(int tabIndex) =>
                Update(
                    targetId,
                    new UiElement.Box
                    {
                        Focusable = true,
                        TabIndex = tabIndex,
                        PickingMode = UiPickingMode.Position,
                    }
                );

            private VisualElement Get(ObjectId id)
            {
                Assert.That(Documents.TryGet(id, out VisualElement? value), Is.True);
                return value!;
            }

            private void Update(ObjectId id, UiElement value) =>
                Documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(id, value)
                    )
                );
        }

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
