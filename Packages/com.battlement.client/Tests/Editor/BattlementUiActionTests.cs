#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class BattlementUiActionTests
    {
        [Test]
        public void ParticleBurstsRestartExpireAndReleaseTheirNativeObjects()
        {
            using var fixture = new Fixture();
            var streak = new UiParticleStreak(
                new[] { 0.5f, 0.5f },
                new[] { 32f, -20f },
                new[] { 18f, 4f },
                22,
                new Color(0, 1, 1, 1),
                500,
                0
            );
            var burst = new VisualElementAction.ParticleStreaks(new[] { streak });
            fixture.Perform(fixture.FocusId, burst);
            ParticleSystem native = Resources
                .FindObjectsOfTypeAll<ParticleSystem>()
                .Single(value => value.name == "Battlement UI particles");
            VisualElement first = fixture.Focus.Q("native-particle-streaks");
            Assert.That(first, Is.Not.Null);
            Assert.That(first.pickingMode, Is.EqualTo(PickingMode.Ignore));
            Assert.That(fixture.Documents.DittoActiveFiniteTimelineCount, Is.EqualTo(1));
            fixture.Perform(fixture.FocusId, burst);
            Assert.That(native == null, Is.True);
            Assert.That(first.parent, Is.Null);
            Assert.That(fixture.Documents.DittoActiveFiniteTimelineCount, Is.EqualTo(1));

            fixture.Time = TimeSpan.FromMilliseconds(501);
            fixture.Documents.Advance();
            Assert.That(fixture.Focus.Q("native-particle-streaks"), Is.Null);
            Assert.That(fixture.Documents.DittoActiveFiniteTimelineCount, Is.Zero);

            fixture.Perform(fixture.FocusId, burst);
            fixture.Perform(
                fixture.FocusId,
                new VisualElementAction.ParticleStreaks(Array.Empty<UiParticleStreak>())
            );
            Assert.That(fixture.Focus.Q("native-particle-streaks"), Is.Null);
            fixture.Instant = true;
            fixture.Perform(fixture.FocusId, burst);
            Assert.That(fixture.Documents.DittoActiveFiniteTimelineCount, Is.Zero);
            fixture.Instant = false;
            fixture.Perform(fixture.FocusId, burst);
            fixture.Documents.Destroy(new CommandBody.VisualElement.Destroy(fixture.FocusId));
            Assert.That(fixture.Documents.DittoActiveFiniteTimelineCount, Is.Zero);
            Assert.That(fixture.Focus.Q("native-particle-streaks"), Is.Null);

            fixture.Perform(fixture.ScrollId, burst);
            fixture.Documents.Create(
                new CommandBody.VisualElement.Create(
                    fixture.ScrollId,
                    new UiNode(Id("25110000-0000-4000-8000-00000000000a"), new UiElement.Box()),
                    0
                )
            );
            Assert.That(fixture.Scroll.Q("native-particle-streaks"), Is.Null);
            Assert.That(
                Resources
                    .FindObjectsOfTypeAll<ParticleSystem>()
                    .Any(value => value.name == "Battlement UI particles"),
                Is.False
            );

            Assert.Throws<BattlementUiException>(() =>
                fixture.Perform(
                    fixture.OutsideId,
                    new VisualElementAction.ParticleStreaks(
                        new[] { streak with { LifetimeMs = 0 } }
                    )
                )
            );
        }

        [Test]
        public void EveryActionUsesPublicStateAndValidatesItsPreconditions()
        {
            using var fixture = new Fixture();

            fixture.Perform(fixture.TextId, new VisualElementAction.Focus());
            Assert.That(
                fixture.Focus.panel!.focusController.focusedElement,
                Is.SameAs(fixture.Text)
            );
            fixture.Perform(
                fixture.TextId,
                new VisualElementAction.CapturePointer(Fixture.PointerId)
            );
            Assert.That(fixture.Text.HasPointerCapture(Fixture.PointerId), Is.True);
            fixture.Perform(
                fixture.TextId,
                new VisualElementAction.ReleasePointer(Fixture.PointerId)
            );
            Assert.That(fixture.Text.HasPointerCapture(Fixture.PointerId), Is.False);
            fixture.Perform(
                fixture.ScrollId,
                new VisualElementAction.ScrollTo(fixture.ScrollChildId)
            );
            fixture.Perform(fixture.TextId, new VisualElementAction.SelectText(3, 1));
            var selection = (ITextSelection)fixture.Text;
            Assert.That(selection.cursorIndex, Is.EqualTo(3));
            Assert.That(selection.selectIndex, Is.EqualTo(1));
            fixture.Perform(fixture.TextId, new VisualElementAction.Blur());
            Assert.That(
                fixture.Focus.panel.focusController.focusedElement,
                Is.Not.SameAs(fixture.Text)
            );
            fixture.Perform(fixture.FieldId, new VisualElementAction.Focus());
            ((INotifyValueChanged<string>)fixture.FieldInput).value = "Uncommitted local draft";
            fixture.Perform(fixture.FieldId, new VisualElementAction.SelectText(20, 4));
            Assert.That(fixture.Field.cursorIndex, Is.EqualTo(20));
            Assert.That(fixture.Field.selectIndex, Is.EqualTo(4));
            fixture.Perform(fixture.FieldId, new VisualElementAction.Blur());

            Assert.Throws<BattlementUiException>(() =>
                fixture.Perform(fixture.FocusId, new VisualElementAction.Blur())
            );
            Assert.Throws<BattlementUiException>(() =>
                fixture.Perform(fixture.TextId, new VisualElementAction.SelectText(5, 0))
            );
            Assert.Throws<BattlementUiException>(() =>
                fixture.Perform(
                    fixture.ScrollId,
                    new VisualElementAction.ScrollTo(fixture.OutsideId)
                )
            );
        }

        [Test]
        public void InputDisableSilentlyRestoresDraftDragFocusAndCapture()
        {
            using var fixture = new Fixture();
            fixture.Perform(fixture.FieldId, new VisualElementAction.Focus());
            fixture.Perform(
                fixture.SliderId,
                new VisualElementAction.CapturePointer(Fixture.PointerId)
            );
            ((INotifyValueChanged<string>)fixture.FieldInput).value = "Uncommitted draft";
            using (
                PointerCaptureEvent capture = PointerCaptureEvent.GetPooled(
                    fixture.Slider,
                    null,
                    Fixture.PointerId
                )
            )
            {
                fixture.Slider.SendEvent(capture);
            }
            fixture.Slider.value = 82;
            fixture.Events.Clear();

            fixture.Documents.SetInputEnabled(false);

            Assert.That(fixture.FieldInput.text, Is.EqualTo("Committed: North Gate"));
            Assert.That(fixture.Slider.value, Is.EqualTo(38));
            Assert.That(fixture.Slider.HasPointerCapture(Fixture.PointerId), Is.False);
            Assert.That(
                fixture.Field.panel!.focusController.focusedElement,
                Is.Not.SameAs(fixture.Field)
            );
            Assert.That(fixture.Events, Is.Empty);
        }

        private sealed class Fixture : IDisposable
        {
            public static readonly int PointerId = UnityEngine.UIElements.PointerId.mousePointerId;

            private readonly GameObject owned;

            public Fixture()
            {
                ObjectId documentId = Id("25110000-0000-4000-8000-000000000001");
                ObjectId rootId = Id("25110000-0000-4000-8000-000000000002");
                FocusId = Id("25110000-0000-4000-8000-000000000003");
                ScrollId = Id("25110000-0000-4000-8000-000000000004");
                ScrollChildId = Id("25110000-0000-4000-8000-000000000005");
                TextId = Id("25110000-0000-4000-8000-000000000006");
                OutsideId = Id("25110000-0000-4000-8000-000000000007");
                FieldId = Id("25110000-0000-4000-8000-000000000008");
                SliderId = Id("25110000-0000-4000-8000-000000000009");
                Events = new List<UiEvent>();
                owned = BattlementUiDocuments.CreateGameObject(
                    new GameObjectKind.UiDocumentState(rootId)
                );
                Documents = new BattlementUiDocuments(
                    value =>
                    {
                        Events.Add(value);
                        return UiEventDisposition.Continue;
                    },
                    now: () => Time,
                    instantMotion: () => Instant
                );
                Documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            documentId,
                            rootId,
                            Children: new UiNode[]
                            {
                                new(FocusId, new UiElement.Box { Focusable = true }),
                                new(
                                    ScrollId,
                                    new UiElement.ScrollView(),
                                    new UiNode[] { new(ScrollChildId, new UiElement.Box()) }
                                ),
                                new(
                                    TextId,
                                    new UiElement.TextElement
                                    {
                                        Text = "A🚀B",
                                        Selectable = true,
                                        Focusable = true,
                                    }
                                ),
                                new(OutsideId, new UiElement.Box()),
                                new(
                                    FieldId,
                                    new UiElement.TextField { Value = "Committed: North Gate" }
                                ),
                                new(
                                    SliderId,
                                    new UiElement.Slider
                                    {
                                        LowValue = 0,
                                        HighValue = 100,
                                        Value = 38,
                                        Events = new[]
                                        {
                                            UiEventKind.ValueChanging,
                                            UiEventKind.ValueCommitted,
                                        },
                                    }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Focus = Get<VisualElement>(FocusId);
                Scroll = Get<ScrollView>(ScrollId);
                Text = Get<TextElement>(TextId);
                Field = Get<TextField>(FieldId);
                FieldInput = Field.Q<VisualElement>(TextField.textInputUssName).Q<TextElement>();
                Slider = Get<UnityEngine.UIElements.Slider>(SliderId);
            }

            public ObjectId FocusId { get; }
            public TimeSpan Time { get; set; }
            public bool Instant { get; set; }
            public ObjectId ScrollId { get; }
            public ObjectId ScrollChildId { get; }
            public ObjectId TextId { get; }
            public ObjectId OutsideId { get; }
            public ObjectId FieldId { get; }
            public ObjectId SliderId { get; }
            public BattlementUiDocuments Documents { get; }
            public List<UiEvent> Events { get; }
            public VisualElement Focus { get; }
            public ScrollView Scroll { get; }
            public TextElement Text { get; }
            public TextField Field { get; }
            public TextElement FieldInput { get; }
            public UnityEngine.UIElements.Slider Slider { get; }

            public void Perform(ObjectId objectId, VisualElementAction action) =>
                Documents.PerformAction(
                    new CommandBody.VisualElement.PerformAction(objectId, action)
                );

            public void Dispose()
            {
                Documents.Clear();
                Object.DestroyImmediate(owned);
            }

            private T Get<T>(ObjectId id)
                where T : VisualElement
            {
                Assert.That(Documents.TryGet(id, out VisualElement? value), Is.True);
                return (T)value!;
            }
        }

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
