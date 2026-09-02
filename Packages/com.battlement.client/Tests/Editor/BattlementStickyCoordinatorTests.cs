#nullable enable

using System;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class BattlementStickyCoordinatorTests
    {
        [TestCase(-20, 0, 200, 40, 10, 10)]
        [TestCase(-20, 0, 30, 40, 10, -10)]
        [TestCase(-20, 0, 200, 40, -5, -5)]
        public void LeadingEdgeFormulaHonorsViewportThenContainingBlock(
            float normalStart,
            float viewportStart,
            float containingEnd,
            float size,
            float inset,
            float expected
        ) =>
            Assert.That(
                BattlementStickyCoordinator.ResolveLeading(
                    normalStart,
                    viewportStart,
                    containingEnd,
                    size,
                    inset
                ),
                Is.EqualTo(expected)
            );

        [TestCase(220, 200, 0, 40, 10, 150)]
        [TestCase(220, 200, 180, 40, 10, 180)]
        [TestCase(220, 200, 0, 240, -5, 0)]
        public void TrailingEdgeFormulaHonorsViewportThenContainingBlock(
            float normalEnd,
            float viewportEnd,
            float containingStart,
            float size,
            float inset,
            float expected
        ) =>
            Assert.That(
                BattlementStickyCoordinator.ResolveTrailing(
                    normalEnd,
                    viewportEnd,
                    containingStart,
                    size,
                    inset
                ),
                Is.EqualTo(expected)
            );

        [Test]
        public void OrdinaryParentUsesAnEmptyFlowSlotAndResetRestoresTheSameHost()
        {
            var coordinator = new BattlementStickyCoordinator();
            var scroll = new ScrollView();
            var parent = new VisualElement();
            var before = new VisualElement();
            var host = new TextField();
            var after = new VisualElement();
            scroll.Add(parent);
            parent.Add(before);
            parent.Add(host);
            parent.Add(after);
            host.SetValueWithoutNotify("draft");

            coordinator.Apply(host, Sticky(top: 0), 1);

            Assert.That(
                coordinator.TryGetPlaceholder(host, out BattlementLayoutSlot? slot),
                Is.True
            );
            Assert.That(slot!.parent, Is.SameAs(parent));
            Assert.That(slot.childCount, Is.EqualTo(1));
            Assert.That(parent.IndexOf(slot), Is.EqualTo(1));
            Assert.That(coordinator.TryGetPresentation(host, out VisualElement? entry), Is.True);
            Assert.That(host.parent, Is.SameAs(slot));
            Assert.That(entry!.childCount, Is.Zero);

            coordinator.Apply(host, Prop<Sticky>.Reset(), 1);

            Assert.That(host.parent, Is.SameAs(parent));
            Assert.That(parent.IndexOf(host), Is.EqualTo(1));
            Assert.That(host.value, Is.EqualTo("draft"));
            Assert.That(coordinator.TryGetPlaceholder(host, out _), Is.False);
        }

        [Test]
        public void LayoutContainerRetainsItsStableSlotWhileStickyIsPresented()
        {
            var coordinator = new BattlementStickyCoordinator();
            var scroll = new ScrollView();
            var grid = new BattlementLayoutContainer(BattlementLayoutContainerKind.Grid);
            var host = new VisualElement();
            scroll.Add(grid);
            grid.Adapter.Insert(host, 0);
            BattlementLayoutSlot slot = grid.Adapter.SlotFor(host);

            coordinator.Apply(host, Sticky(left: 3), 0);

            Assert.That(grid.Adapter.SlotFor(host), Is.SameAs(slot));
            Assert.That(slot.childCount, Is.EqualTo(1));

            coordinator.Apply(host, Prop<Sticky>.Reset(), 0);

            Assert.That(grid.Adapter.SlotFor(host), Is.SameAs(slot));
            Assert.That(host.parent, Is.SameAs(slot));
        }

        [Test]
        public void SurfaceSortsEveryContainingBlockByOrderThenSourceOrdinal()
        {
            var coordinator = new BattlementStickyCoordinator();
            var scroll = new ScrollView();
            var firstParent = new VisualElement();
            var secondParent = new VisualElement();
            var later = new VisualElement();
            var earlier = new VisualElement();
            var highest = new VisualElement();
            scroll.Add(firstParent);
            scroll.Add(secondParent);
            firstParent.Add(later);
            secondParent.Add(earlier);
            secondParent.Add(highest);
            coordinator.Apply(later, Sticky(top: 0, order: 0), 4);
            coordinator.Apply(earlier, Sticky(top: 0, order: 0), 2);
            coordinator.Apply(highest, Sticky(bottom: 0, order: 1), 1);
            coordinator.TryGetPresentation(later, out VisualElement? laterEntry);
            coordinator.TryGetPresentation(earlier, out VisualElement? earlierEntry);
            coordinator.TryGetPresentation(highest, out VisualElement? highestEntry);

            Assert.That(earlierEntry!.parent, Is.SameAs(laterEntry!.parent));
            Assert.That(laterEntry.parent, Is.SameAs(highestEntry!.parent));
            Assert.That(earlierEntry.parent.IndexOf(earlierEntry), Is.EqualTo(0));
            Assert.That(laterEntry.parent.IndexOf(laterEntry), Is.EqualTo(1));
            Assert.That(highestEntry.parent.IndexOf(highestEntry), Is.EqualTo(2));
        }

        [Test]
        public void MovingBetweenScrollViewsPreservesHostIdentity()
        {
            var coordinator = new BattlementStickyCoordinator();
            var first = new ScrollView();
            var second = new ScrollView();
            var host = new VisualElement();
            first.Add(host);
            coordinator.Apply(host, Sticky(right: 0), 0);

            coordinator.PrepareHierarchyChange(host);
            host.RemoveFromHierarchy();
            second.Add(host);
            coordinator.Apply(host, Sticky(right: 0), 0);

            Assert.That(coordinator.TryGetPresentation(host, out VisualElement? entry), Is.True);
            Assert.That(host.parent, Is.TypeOf<BattlementLayoutSlot>());
            Assert.That(entry!.childCount, Is.Zero);
            Assert.That(entry!.parent!.parent, Is.SameAs(second.contentViewport));
        }

        [Test]
        public void MissingScrollAncestryDoesNotChangeTheTree()
        {
            var coordinator = new BattlementStickyCoordinator();
            var parent = new VisualElement();
            var host = new VisualElement();
            parent.Add(host);

            Assert.Throws<System.InvalidOperationException>(() =>
                coordinator.Apply(host, Sticky(top: 0), 0)
            );
            Assert.That(host.parent, Is.SameAs(parent));
            Assert.That(parent.childCount, Is.EqualTo(1));
        }

        [Test]
        public void SnapshotAttachesNestedStickyAfterScrollHierarchyIsComplete()
        {
            ObjectId documentId = Id("7ad4602c-0ef5-49fc-949d-57bc5eb5b599");
            ObjectId rootId = Id("25ddc31a-3629-43c5-be86-d2c6afb9087e");
            ObjectId scrollId = Id("2edeb715-504d-4397-a46e-7039ea15354d");
            ObjectId gridId = Id("4986c21d-aa36-4240-8896-fd7184a075e2");
            ObjectId stickyId = Id("69ae8a2f-99a6-4cf5-81d3-0469d08c6d89");
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
                                    scrollId,
                                    new UiElement.ScrollView(),
                                    new UiNode[]
                                    {
                                        new(
                                            gridId,
                                            new UiElement.Grid(),
                                            new UiNode[]
                                            {
                                                new(
                                                    stickyId,
                                                    new UiElement.VisualElement
                                                    {
                                                        Sticky = new Sticky(0, null, null, null, 1),
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

                Assert.That(documents.TryGet(stickyId, out VisualElement? host), Is.True);
                Assert.That(documents.TryGet(scrollId, out VisualElement? scroll), Is.True);
                Assert.That(host!.GetFirstAncestorOfType<ScrollView>(), Is.SameAs(scroll));
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void DocumentLifecyclePreservesIdentityAndRejectsInvalidUpdatesBeforeMutation()
        {
            ObjectId documentId = Id("63e00b00-e10f-4bad-a1a7-f8a9df26f1ca");
            ObjectId rootId = Id("240f8b56-ce10-462a-b5ef-73346ea27566");
            ObjectId scrollId = Id("15cc0298-d152-44ee-90b6-07124f67fe48");
            ObjectId stickyId = Id("1c1bdc72-ea45-49eb-8e14-c70793d83a55");
            ObjectId invalidId = Id("7b130f76-1838-49e9-bbb5-53ff7f08abc5");
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
                                    scrollId,
                                    new UiElement.ScrollView(),
                                    new UiNode[]
                                    {
                                        new(
                                            stickyId,
                                            new UiElement.VisualElement
                                            {
                                                Name = "stable",
                                                Sticky = new Sticky(0, null, null, null, 1),
                                            }
                                        ),
                                    }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Assert.That(documents.TryGet(stickyId, out VisualElement? host), Is.True);
                VisualElement identity = host!;
                Assert.That(documents.TryGet(scrollId, out VisualElement? scrollValue), Is.True);
                var scroll = (ScrollView)scrollValue!;
                Assert.That(identity.parent, Is.Not.SameAs(scroll.contentContainer));

                BattlementUiException? styleFailure = Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Properties(
                                stickyId,
                                new UiElement.VisualElement
                                {
                                    Name = "not-applied",
                                    Style = new UiStyle(Position: UiStyle.Set(UiPosition.Absolute)),
                                }
                            )
                        )
                    )
                );
                Assert.That(styleFailure!.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
                Assert.That(identity.name, Is.EqualTo("stable"));

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            stickyId,
                            new UiElement.VisualElement { Sticky = Prop<Sticky>.Reset() }
                        )
                    )
                );
                Assert.That(documents.TryGet(stickyId, out host), Is.True);
                Assert.That(host, Is.SameAs(identity));
                Assert.That(host!.parent, Is.SameAs(scroll.contentContainer));

                BattlementUiException? createFailure = Assert.Throws<BattlementUiException>(() =>
                    documents.Create(
                        new CommandBody.VisualElement.Create(
                            rootId,
                            new UiNode(
                                invalidId,
                                new UiElement.VisualElement
                                {
                                    Sticky = new Sticky(null, null, 0, null, 0),
                                }
                            )
                        )
                    )
                );
                Assert.That(createFailure!.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
                Assert.That(documents.TryGet(invalidId, out _), Is.False);
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        private static Prop<Sticky> Sticky(
            float? top = null,
            float? right = null,
            float? bottom = null,
            float? left = null,
            int order = 0
        ) => Prop<Sticky>.Set(new Sticky(top, right, bottom, left, order));

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
