#nullable enable

using System;
using System.Collections.Generic;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using NativeMinMaxSlider = UnityEngine.UIElements.MinMaxSlider;
using NativeProgressBar = UnityEngine.UIElements.ProgressBar;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class BattlementUiRangeControlTests
    {
        [Test]
        public void UnboundedRangeAndProgressBarsConstructTheirAuthoredState()
        {
            using var fixture = new RangeFixture();
            Assert.That(fixture.Range.lowLimit, Is.EqualTo(float.MinValue));
            Assert.That(fixture.Range.highLimit, Is.EqualTo(float.MaxValue));
            Assert.That(fixture.Range.value, Is.EqualTo(new Vector2(20, 80)));

            Assert.That(fixture.Progress.lowValue, Is.EqualTo(0));
            Assert.That(fixture.Progress.highValue, Is.EqualTo(100));
            Assert.That(fixture.Progress.value, Is.EqualTo(35));
            Assert.That(fixture.Progress.title, Is.EqualTo("STAGING 35%"));
        }

        [Test]
        public void RangeCaptureStaysLocalThenEmitsOneTypedCommitAndRollsBack()
        {
            using var fixture = new RangeFixture();
            fixture.UpdateRange(
                new UiElement.MinMaxSlider
                {
                    LowLimit = new LowerLimit.Inclusive(0),
                    HighLimit = new UpperLimit.Inclusive(100),
                }
            );
            SendCapture(fixture.Range);
            fixture.Range.value = new Vector2(-10, 95);
            Assert.That(fixture.Range.value, Is.EqualTo(new Vector2(0, 95)));
            Assert.That(fixture.Events, Has.Count.EqualTo(1));
            var live = (UiEventBody.ValueChanging)fixture.Events[0].Body;
            Assert.That(
                live.Value.Proposed,
                Is.EqualTo(new UiValue.F32Range(new FloatRange(0, 95)))
            );

            SendCaptureOut(fixture.Range);
            Assert.That(fixture.Events, Has.Count.EqualTo(2));
            var committed = (UiEventBody.ValueCommitted)fixture.Events[1].Body;
            Assert.That(
                committed.Value.Previous,
                Is.EqualTo(new UiValue.F32Range(new FloatRange(20, 80)))
            );
            Assert.That(
                committed.Value.Proposed,
                Is.EqualTo(new UiValue.F32Range(new FloatRange(0, 95)))
            );
            Assert.That(fixture.Range.value, Is.EqualTo(new Vector2(20, 80)));

            fixture.UpdateRange(new UiElement.MinMaxSlider { MinValue = 0, MaxValue = 95 });
            Assert.That(fixture.Range.value, Is.EqualTo(new Vector2(0, 95)));
            Assert.That(fixture.Events, Has.Count.EqualTo(2));
        }

        [Test]
        public void SparseRangeAndProgressUpdatesValidateAtomicallyAndDoNotEcho()
        {
            using var fixture = new RangeFixture();
            fixture.UpdateRange(
                new UiElement.MinMaxSlider
                {
                    LowLimit = new LowerLimit.Inclusive(-50),
                    HighLimit = new UpperLimit.Inclusive(150),
                    MinValue = -25,
                    MaxValue = 125,
                }
            );
            Assert.That(fixture.Range.value, Is.EqualTo(new Vector2(-25, 125)));
            Assert.Throws<BattlementUiException>(() =>
                fixture.UpdateRange(new UiElement.MinMaxSlider { MinValue = 140 })
            );
            Assert.That(fixture.Range.value, Is.EqualTo(new Vector2(-25, 125)));

            fixture.UpdateProgress(new UiElement.ProgressBar { Value = 82, Title = "READY 82%" });
            Assert.That(fixture.Progress.value, Is.EqualTo(82));
            Assert.That(fixture.Progress.title, Is.EqualTo("READY 82%"));
            Assert.Throws<BattlementUiException>(() =>
                fixture.UpdateProgress(new UiElement.ProgressBar { HighValue = 70 })
            );
            Assert.That(fixture.Progress.highValue, Is.EqualTo(100));
            Assert.That(fixture.Progress.value, Is.EqualTo(82));
            Assert.That(fixture.Events, Is.Empty);
        }

        private static void SendCapture(VisualElement target)
        {
            using PointerCaptureEvent value = PointerCaptureEvent.GetPooled(
                target,
                null,
                PointerId.mousePointerId
            );
            target.SendEvent(value);
        }

        private static void SendCaptureOut(VisualElement target)
        {
            using PointerCaptureOutEvent value = PointerCaptureOutEvent.GetPooled(
                target,
                null,
                PointerId.mousePointerId
            );
            target.SendEvent(value);
        }

        private sealed class RangeFixture : IDisposable
        {
            private readonly GameObject owned;
            private readonly ObjectId documentId = new(Guid.NewGuid());
            private readonly ObjectId rootId = new(Guid.NewGuid());
            private readonly ObjectId rangeId = new(Guid.NewGuid());
            private readonly ObjectId progressId = new(Guid.NewGuid());

            public RangeFixture()
            {
                owned = BattlementUiDocuments.CreateGameObject(
                    new GameObjectKind.UiDocumentState(rootId)
                );
                Documents = new BattlementUiDocuments(value =>
                {
                    Events.Add(value);
                    return true;
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
                                    rangeId,
                                    new UiElement.MinMaxSlider
                                    {
                                        MinValue = 20,
                                        MaxValue = 80,
                                        Events = new[]
                                        {
                                            UiEventKind.ValueChanging,
                                            UiEventKind.ValueCommitted,
                                        },
                                    }
                                ),
                                new(
                                    progressId,
                                    new UiElement.ProgressBar
                                    {
                                        LowValue = 0,
                                        HighValue = 100,
                                        Value = 35,
                                        Title = "STAGING 35%",
                                    }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
            }

            public BattlementUiDocuments Documents { get; }
            public List<UiEvent> Events { get; } = new();
            public NativeMinMaxSlider Range => (NativeMinMaxSlider)Get(rangeId);
            public NativeProgressBar Progress => (NativeProgressBar)Get(progressId);

            public void UpdateRange(UiElement.MinMaxSlider value) => Update(rangeId, value);

            public void UpdateProgress(UiElement.ProgressBar value) => Update(progressId, value);

            private VisualElement Get(ObjectId objectId)
            {
                Assert.That(Documents.TryGet(objectId, out VisualElement? value), Is.True);
                return value!;
            }

            private void Update(ObjectId objectId, UiElement element) =>
                Documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(objectId, element)
                    )
                );

            public void Dispose()
            {
                Documents.Clear();
                Object.DestroyImmediate(owned);
            }
        }
    }
}
