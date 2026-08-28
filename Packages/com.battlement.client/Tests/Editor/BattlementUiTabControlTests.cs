#nullable enable

using System;
using System.Collections.Generic;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;
using UiTab = Battlement.UiElement.Tab;
using UiTabView = Battlement.UiElement.TabView;

namespace Battlement.Tests
{
    public sealed class BattlementUiTabControlTests
    {
        [Test]
        public void TabViewVetoesNativeProposalsAndAppliesAuthoredResponsesSilently()
        {
            ObjectId documentId = Id("738aa862-48e3-4d08-92bc-018cb4da75c0");
            ObjectId rootId = Id("0b702aa8-c0d5-42f4-96ca-4caa61b0ce81");
            ObjectId viewId = Id("6d568cfa-cada-4d1e-a2a0-4f4540ba84d0");
            ObjectId firstId = Id("cb8204e7-80b8-4dab-8bd4-3d15bf9ef295");
            ObjectId secondId = Id("ce9bbdce-8469-4e12-a81b-f39ef79fdfc2");
            ObjectId thirdId = Id("e0c1787f-4cd2-4ae8-8035-31a263344f8e");
            ObjectId addedId = Id("bb6df82c-f307-451a-8417-af0ad0daed21");
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
                                    viewId,
                                    new UiTabView
                                    {
                                        SelectedTabIndex = 1,
                                        Reorderable = true,
                                        Events = new[]
                                        {
                                            UiEventKind.TabSelectionRequested,
                                            UiEventKind.TabCloseRequested,
                                            UiEventKind.TabReorderRequested,
                                        },
                                    },
                                    new UiNode[]
                                    {
                                        Tab(firstId, "BOARD", closeable: false),
                                        Tab(secondId, "NOTES", closeable: true),
                                        Tab(thirdId, "LOG", closeable: true),
                                    }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Assert.That(documents.TryGet(viewId, out VisualElement? value), Is.True);
                var view = (TabView)value!;
                Assert.That(view.selectedTabIndex, Is.EqualTo(1));
                Assert.That(view.reorderable, Is.True);

                view.selectedTabIndex = 2;
                Assert.That(view.selectedTabIndex, Is.EqualTo(1), "Selection is controlled.");
                Assert.That(events, Has.Count.EqualTo(1));
                Assert.That(events[0].Body, Is.TypeOf<UiEventBody.TabSelectionRequested>());
                var selection = (UiEventBody.TabSelectionRequested)events[0].Body;
                Assert.That(selection.Value.PreviousIndex, Is.EqualTo(1));
                Assert.That(selection.Value.ProposedIndex, Is.EqualTo(2));
                Assert.That(selection.Value.ProposedTabId, Is.EqualTo(thirdId));

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            viewId,
                            new UiTabView { SelectedTabIndex = 2 }
                        )
                    )
                );
                Assert.That(view.selectedTabIndex, Is.EqualTo(2));
                Assert.That(events, Has.Count.EqualTo(1), "Authored selection must not echo.");

                view.ReorderTab(2, 0);
                Assert.That(view.GetTab(0).label, Is.EqualTo("BOARD"), "Reorder is controlled.");
                Assert.That(events, Has.Count.EqualTo(2));
                var reorder = (UiEventBody.TabReorderRequested)events[1].Body;
                Assert.That(reorder.Value.TabId, Is.EqualTo(thirdId));
                Assert.That(reorder.Value.PreviousIndex, Is.EqualTo(2));
                Assert.That(reorder.Value.ProposedIndex, Is.EqualTo(0));

                documents.Update(
                    new CommandBody.VisualElement.Update(new VisualElementUpdate.Index(thirdId, 0))
                );
                Assert.That(view.GetTab(0).label, Is.EqualTo("LOG"));
                Assert.That(view.selectedTabIndex, Is.EqualTo(2));
                Assert.That(events, Has.Count.EqualTo(2), "Authored reorder must not echo.");

                documents.Create(
                    new CommandBody.VisualElement.Create(
                        viewId,
                        Tab(addedId, "METRICS", closeable: true),
                        1
                    )
                );
                Assert.That(view.GetTab(1).label, Is.EqualTo("METRICS"));
                Assert.That(view.selectedTabIndex, Is.EqualTo(2));
                Assert.That(events, Has.Count.EqualTo(2), "Authored insertion must not echo.");

                Assert.That(documents.TryGet(secondId, out VisualElement? second), Is.True);
                VisualElement? close = ((Tab)second!).tabHeader.Q(
                    className: UnityEngine.UIElements.Tab.closeButtonUssClassName
                );
                Assert.That(close, Is.Not.Null);
                Click(close!);
                Assert.That(view.childCount, Is.EqualTo(4), "Native close must be vetoed.");
                Assert.That(events, Has.Count.EqualTo(3));
                var closeRequest = (UiEventBody.TabCloseRequested)events[2].Body;
                Assert.That(closeRequest.Value.TabId, Is.EqualTo(secondId));

                documents.Destroy(new CommandBody.VisualElement.Destroy(secondId));
                Assert.That(view.childCount, Is.EqualTo(3));
                Assert.That(view.selectedTabIndex, Is.EqualTo(2));
                Assert.That(events, Has.Count.EqualTo(3), "Authored removal must not echo.");

                Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Properties(
                                viewId,
                                new UiTabView { SelectedTabIndex = 9, Reorderable = false }
                            )
                        )
                    )
                );
                Assert.That(view.selectedTabIndex, Is.EqualTo(2));
                Assert.That(view.reorderable, Is.True);

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            viewId,
                            new UiTabView
                            {
                                SelectedTabIndex = Prop<uint>.Reset(),
                                Reorderable = Prop<bool>.Reset(),
                            }
                        )
                    )
                );
                Assert.That(documents.TryGet(viewId, out VisualElement? resetView), Is.True);
                Assert.That(resetView, Is.SameAs(view));
                Assert.That(view.selectedTabIndex, Is.Zero);
                Assert.That(view.reorderable, Is.EqualTo(new TabView().reorderable));
                Assert.That(view.childCount, Is.EqualTo(3));
            }
            finally
            {
                documents.Clear();
                Object.DestroyImmediate(owned);
            }
        }

        private static UiNode Tab(ObjectId objectId, string text, bool closeable) =>
            new(
                objectId,
                new UiTab { Text = text, Closeable = closeable },
                new UiNode[]
                {
                    new(
                        new ObjectId(Guid.NewGuid()),
                        new Battlement.UiElement.Label { Text = $"{text} content" }
                    ),
                }
            );

        private static void Click(VisualElement target)
        {
            using PointerDownEvent down = PointerDownEvent.GetPooled(
                new Event { type = EventType.MouseDown, button = 0 }
            );
            down.target = target;
            target.SendEvent(down);
            using PointerUpEvent up = PointerUpEvent.GetPooled(
                new Event { type = EventType.MouseUp, button = 0 }
            );
            up.target = target;
            target.SendEvent(up);
        }

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
