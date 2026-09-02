#nullable enable

using System;
using System.Collections.Generic;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class BattlementAccessibilityManagerTests
    {
        [Test]
        public void UnnamedRowsRemainInTheirNamedTable()
        {
            using var fixture = new Fixture();
            fixture.ApplySnapshot(SemanticRole.Table, SemanticRole.Row);
            Assert.That(fixture.Manager.Mirror, Has.Count.EqualTo(2));
            Assert.That(fixture.Manager.Active, Has.Count.EqualTo(2));
        }

        [Test]
        public void CompleteMirrorFiltersInertHostsAndRejectsStaleActions()
        {
            using var fixture = new Fixture();
            fixture.ApplySnapshot();

            Assert.That(fixture.Manager.Mirror, Has.Count.EqualTo(2));
            Assert.That(fixture.Manager.Active, Has.Count.EqualTo(2));
            Assert.That(fixture.Activate(fixture.Manager.Generation), Is.True);
            Assert.That(fixture.Events, Has.Count.EqualTo(1));
            Assert.That(fixture.Activate(fixture.Manager.Generation - 1), Is.False);

            fixture.SetContainerInert();

            Assert.That(fixture.Manager.Active, Is.Empty);
            Assert.That(fixture.Activate(fixture.Manager.Generation), Is.False);
            Assert.That(fixture.Events, Has.Count.EqualTo(1));
        }

        [Test]
        public void BackendReactivationRebuildsFromMirrorWithANewGeneration()
        {
            using var fixture = new Fixture();
            fixture.ApplySnapshot();
            ulong first = fixture.Manager.Generation;

            fixture.Manager.SetBackendAvailable(false);
            Assert.That(fixture.Manager.Active, Is.Empty);
            fixture.Manager.SetBackendAvailable(true);

            Assert.That(fixture.Manager.Generation, Is.EqualTo(first + 2));
            Assert.That(fixture.Manager.Active, Has.Count.EqualTo(2));
            Assert.That(fixture.Activate(first), Is.False);
        }

        private sealed class Fixture : IDisposable
        {
            private readonly ObjectId documentId = Id("25130000-0000-4000-8000-000000000001");
            private readonly ObjectId rootId = Id("25130000-0000-4000-8000-000000000002");
            private readonly ObjectId containerId = Id("25130000-0000-4000-8000-000000000003");
            private readonly ObjectId buttonId = Id("25130000-0000-4000-8000-000000000004");
            private readonly GameObject owned;

            public Fixture()
            {
                Events = new List<UiEvent>();
                owned = BattlementUiDocuments.CreateGameObject(
                    new GameObjectKind.UiDocumentState(rootId)
                );
                Documents = new BattlementUiDocuments(value =>
                {
                    Events.Add(value);
                    return UiEventDisposition.PreventDefault;
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
                                    new UiElement.Box(),
                                    new UiNode[]
                                    {
                                        new(buttonId, new UiElement.Button { Text = "Save" }),
                                    }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
            }

            public BattlementUiDocuments Documents { get; }
            public List<UiEvent> Events { get; }
            public BattlementAccessibilityManager Manager => Documents.AccessibilityForTests;

            public void ApplySnapshot(
                SemanticRole containerRole = SemanticRole.Group,
                SemanticRole childRole = SemanticRole.Button
            ) =>
                Documents.Apply(
                    new AccessibilityUpdatePayload(
                        new AccessibilitySnapshot(
                            1,
                            new[] { containerId },
                            new AccessibilityNodeSnapshot[]
                            {
                                Node(
                                    containerId,
                                    null,
                                    new[] { buttonId },
                                    containerRole,
                                    containerRole == SemanticRole.Table ? "Bindings" : null,
                                    new AccessibilityActionSet()
                                ),
                                Node(
                                    buttonId,
                                    containerId,
                                    Array.Empty<ObjectId>(),
                                    childRole,
                                    childRole == SemanticRole.Row ? null : "Save changes",
                                    new AccessibilityActionSet(Activate: true)
                                ),
                            }
                        ),
                        new[] { "Saved" }
                    )
                );

            public bool Activate(ulong generation) =>
                Manager.Dispatch(
                    new AccessibilityEvent(generation, buttonId, new AccessibilityAction.Activate())
                );

            public void SetContainerInert()
            {
                Documents.BeginCommit();
                Documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            containerId,
                            new UiElement.Box { Inert = true }
                        )
                    )
                );
                Documents.EndCommit();
            }

            public void Dispose()
            {
                Documents.Clear();
                Object.DestroyImmediate(owned);
            }

            private static AccessibilityNodeSnapshot Node(
                ObjectId id,
                ObjectId? parent,
                IReadOnlyList<ObjectId> children,
                SemanticRole role,
                string? label,
                AccessibilityActionSet actions
            ) => new(id, parent, children, role, label, null, new SemanticState(), null, actions);
        }

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
