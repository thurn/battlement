#nullable enable

using System;
using System.Collections.Generic;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using NativeSlider = UnityEngine.UIElements.Slider;
using NativeSliderInt = UnityEngine.UIElements.SliderInt;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class BattlementUiSliderControlTests
    {
        [Test]
        public void SlidersConstructAuthoredStateAndConditionalPublicParts()
        {
            using var fixture = new SliderFixture();
            NativeSlider slider = fixture.Float;
            Assert.That(slider.label, Is.EqualTo("THRUST"));
            Assert.That(slider.lowValue, Is.EqualTo(0));
            Assert.That(slider.highValue, Is.EqualTo(10));
            Assert.That(slider.value, Is.EqualTo(2.5f));
            Assert.That(slider.fill, Is.True);
            Assert.That(slider.pageSize, Is.EqualTo(0.5f));
            Assert.That(slider.showInputField, Is.True);
            Assert.That(slider.direction, Is.EqualTo(SliderDirection.Horizontal));
            Assert.That(slider.inverted, Is.False);
            RequireParts(slider, hasTextInput: true);

            NativeSliderInt integer = fixture.Integer;
            Assert.That(integer.label, Is.EqualTo("GEAR"));
            Assert.That(integer.lowValue, Is.EqualTo(1));
            Assert.That(integer.highValue, Is.EqualTo(8));
            Assert.That(integer.value, Is.EqualTo(3));
            Assert.That(integer.direction, Is.EqualTo(SliderDirection.Vertical));
            Assert.That(integer.inverted, Is.True);
            RequireParts(integer, hasTextInput: false);
        }

        [Test]
        public void DragProposalsRemainLocalUntilOneReleaseCommitAndRollback()
        {
            using var fixture = new SliderFixture();
            SendCapture(fixture.Float);
            fixture.Float.value = 7.5f;
            Assert.That(fixture.Float.value, Is.EqualTo(7.5f));
            Assert.That(fixture.Events, Has.Count.EqualTo(1));
            var changing = (UiEventBody.ValueChanging)fixture.Events[0].Body;
            Assert.That(changing.Value.Proposed, Is.EqualTo(new UiValue.F32(7.5f)));

            SendCaptureOut(fixture.Float);
            Assert.That(fixture.Events, Has.Count.EqualTo(2));
            var committed = (UiEventBody.ValueCommitted)fixture.Events[1].Body;
            Assert.That(committed.Value.Previous, Is.EqualTo(new UiValue.F32(2.5f)));
            Assert.That(committed.Value.Proposed, Is.EqualTo(new UiValue.F32(7.5f)));
            Assert.That(fixture.Float.value, Is.EqualTo(2.5f));
            SendCaptureOut(fixture.Float);
            Assert.That(fixture.Events, Has.Count.EqualTo(2));

            fixture.UpdateFloat(7.5f);
            Assert.That(fixture.Float.value, Is.EqualTo(7.5f));
            Assert.That(fixture.Events, Has.Count.EqualTo(2), "Authored writes are silent.");
        }

        [Test]
        public void IntegerSliderEmitsTypedValuesAndMergedRangeValidationIsAtomic()
        {
            using var fixture = new SliderFixture();
            SendCapture(fixture.Integer);
            fixture.Integer.value = 7;
            SendCaptureOut(fixture.Integer);
            Assert.That(fixture.Events, Has.Count.EqualTo(2));
            var changing = (UiEventBody.ValueChanging)fixture.Events[0].Body;
            Assert.That(changing.Value.Proposed, Is.EqualTo(new UiValue.I32(7)));
            var committed = (UiEventBody.ValueCommitted)fixture.Events[1].Body;
            Assert.That(committed.Value.Previous, Is.EqualTo(new UiValue.I32(3)));
            Assert.That(committed.Value.Proposed, Is.EqualTo(new UiValue.I32(7)));
            Assert.That(fixture.Integer.value, Is.EqualTo(3));

            Assert.Throws<BattlementUiException>(() =>
                fixture.UpdateInteger(new UiElement.SliderInt { HighValue = 2 })
            );
            Assert.That(fixture.Integer.highValue, Is.EqualTo(8));
            Assert.Throws<BattlementUiException>(() =>
                fixture.UpdateFloat(new UiElement.Slider { PageSize = -1 })
            );
            Assert.That(fixture.Float.pageSize, Is.EqualTo(0.5f));
        }

        [Test]
        public void SparseValueUpdatesValidateAgainstTheAuthoredRange()
        {
            using var fixture = new SliderFixture();
            fixture.UpdateFloat(new UiElement.Slider { HighValue = 100 });
            fixture.UpdateFloat(75);

            Assert.That(fixture.Float.highValue, Is.EqualTo(100));
            Assert.That(fixture.Float.value, Is.EqualTo(75));
        }

        [Test]
        public void EverySliderPropertyResetsSilentlyToItsNativeDefault()
        {
            using var fixture = new SliderFixture();
            fixture.UpdateFloat(
                new UiElement.Slider
                {
                    Label = Prop<string>.Reset(),
                    LowValue = Prop<float>.Reset(),
                    HighValue = Prop<float>.Reset(),
                    Value = Prop<float>.Reset(),
                    Fill = Prop<bool>.Reset(),
                    PageSize = Prop<float>.Reset(),
                    ShowInputField = Prop<bool>.Reset(),
                    Direction = Prop<UiSliderDirection>.Reset(),
                    Inverted = Prop<bool>.Reset(),
                }
            );
            fixture.UpdateInteger(
                new UiElement.SliderInt
                {
                    Label = Prop<string>.Reset(),
                    LowValue = Prop<int>.Reset(),
                    HighValue = Prop<int>.Reset(),
                    Value = Prop<int>.Reset(),
                    Fill = Prop<bool>.Reset(),
                    PageSize = Prop<float>.Reset(),
                    ShowInputField = Prop<bool>.Reset(),
                    Direction = Prop<UiSliderDirection>.Reset(),
                    Inverted = Prop<bool>.Reset(),
                }
            );

            var floatDefaults = new NativeSlider();
            Assert.That(fixture.Float.label, Is.Empty);
            Assert.That(fixture.Float.lowValue, Is.EqualTo(floatDefaults.lowValue));
            Assert.That(fixture.Float.highValue, Is.EqualTo(floatDefaults.highValue));
            Assert.That(fixture.Float.value, Is.EqualTo(floatDefaults.value));
            Assert.That(fixture.Float.fill, Is.EqualTo(floatDefaults.fill));
            Assert.That(fixture.Float.pageSize, Is.EqualTo(floatDefaults.pageSize));
            Assert.That(fixture.Float.showInputField, Is.EqualTo(floatDefaults.showInputField));
            Assert.That(fixture.Float.direction, Is.EqualTo(floatDefaults.direction));
            Assert.That(fixture.Float.inverted, Is.EqualTo(floatDefaults.inverted));
            var intDefaults = new NativeSliderInt();
            Assert.That(fixture.Integer.label, Is.Empty);
            Assert.That(fixture.Integer.lowValue, Is.EqualTo(intDefaults.lowValue));
            Assert.That(fixture.Integer.highValue, Is.EqualTo(intDefaults.highValue));
            Assert.That(fixture.Integer.value, Is.EqualTo(intDefaults.value));
            Assert.That(fixture.Integer.fill, Is.EqualTo(intDefaults.fill));
            Assert.That(fixture.Integer.pageSize, Is.EqualTo(intDefaults.pageSize));
            Assert.That(fixture.Integer.showInputField, Is.EqualTo(intDefaults.showInputField));
            Assert.That(fixture.Integer.direction, Is.EqualTo(intDefaults.direction));
            Assert.That(fixture.Integer.inverted, Is.EqualTo(intDefaults.inverted));
            Assert.That(fixture.Events, Is.Empty);
        }

        [Test]
        public void FinalProposalRetriesAfterLiveTrafficTemporarilyOccupiesTheTransport()
        {
            var accepted = new List<UiEvent>();
            bool rejectFirstCommit = true;
            using var fixture = new SliderFixture(value =>
            {
                if (value.Body is UiEventBody.ValueCommitted && rejectFirstCommit)
                {
                    rejectFirstCommit = false;
                    return false;
                }
                accepted.Add(value);
                return true;
            });
            SendCapture(fixture.Float);
            fixture.Float.value = 8;
            SendCaptureOut(fixture.Float);
            Assert.That(accepted, Has.Count.EqualTo(1));
            Assert.That(accepted[0].Body, Is.TypeOf<UiEventBody.ValueChanging>());

            fixture.Documents.Advance();
            Assert.That(accepted, Has.Count.EqualTo(2));
            Assert.That(accepted[1].Body, Is.TypeOf<UiEventBody.ValueCommitted>());
            fixture.Documents.Advance();
            Assert.That(
                accepted,
                Has.Count.EqualTo(2),
                "Accepted final proposals are not retried."
            );
        }

        [Test]
        public void RejectedFinalProposalsRemainOrderedAcrossLaterReleasesAndAuthoredUpdates()
        {
            var accepted = new List<UiEventBody.ValueCommitted>();
            bool rejectCommits = true;
            using var fixture = new SliderFixture(value =>
            {
                if (value.Body is not UiEventBody.ValueCommitted committed)
                    return true;
                if (rejectCommits)
                    return false;
                accepted.Add(committed);
                return true;
            });

            SendCapture(fixture.Float);
            fixture.Float.value = 8;
            SendCaptureOut(fixture.Float);
            SendCapture(fixture.Float);
            fixture.Float.value = 9;
            SendCaptureOut(fixture.Float);
            fixture.UpdateFloat(6);

            rejectCommits = false;
            fixture.Documents.Advance();
            Assert.That(accepted, Has.Count.EqualTo(2));
            Assert.That(accepted[0].Value.Previous, Is.EqualTo(new UiValue.F32(2.5f)));
            Assert.That(accepted[0].Value.Proposed, Is.EqualTo(new UiValue.F32(8)));
            Assert.That(accepted[1].Value.Previous, Is.EqualTo(new UiValue.F32(2.5f)));
            Assert.That(accepted[1].Value.Proposed, Is.EqualTo(new UiValue.F32(9)));
            Assert.That(fixture.Float.value, Is.EqualTo(6));
        }

        [Test]
        public void UnsubscribedFinalProposalsAreNotRetainedForLaterDelivery()
        {
            using var fixture = new SliderFixture();
            fixture.UpdateFloat(new UiElement.Slider { Events = Array.Empty<UiEventKind>() });
            SendCapture(fixture.Float);
            fixture.Float.value = 8;
            SendCaptureOut(fixture.Float);
            fixture.UpdateFloat(
                new UiElement.Slider { Events = new[] { UiEventKind.ValueCommitted } }
            );

            fixture.Documents.Advance();
            Assert.That(
                fixture.Events.FindAll(value => value.Body is UiEventBody.ValueCommitted),
                Is.Empty
            );
        }

        private static void RequireParts(VisualElement target, bool hasTextInput)
        {
            Assert.That(
                target.Q<VisualElement>(className: BaseSlider<float>.labelUssClassName),
                Is.Not.Null
            );
            Assert.That(
                target.Q<VisualElement>(className: BaseSlider<float>.inputUssClassName),
                Is.Not.Null
            );
            Assert.That(
                target.Q<VisualElement>(className: BaseSlider<float>.trackerUssClassName),
                Is.Not.Null
            );
            Assert.That(
                target.Q<VisualElement>(className: BaseSlider<float>.draggerUssClassName),
                Is.Not.Null
            );
            Assert.That(
                target.Q<TextField>(className: TextField.ussClassName),
                hasTextInput ? Is.Not.Null : Is.Null
            );
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

        private sealed class SliderFixture : IDisposable
        {
            private readonly GameObject owned;
            private readonly ObjectId documentId = new(Guid.NewGuid());
            private readonly ObjectId rootId = new(Guid.NewGuid());
            private readonly ObjectId floatId = new(Guid.NewGuid());
            private readonly ObjectId integerId = new(Guid.NewGuid());

            public SliderFixture(Func<UiEvent, bool>? emit = null)
            {
                owned = BattlementUiDocuments.CreateGameObject(
                    new GameObjectKind.UiDocumentState(rootId)
                );
                Documents = new BattlementUiDocuments(value =>
                {
                    Events.Add(value);
                    return emit is null || emit(value) ? UiEventDisposition.Continue : null;
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
                                    floatId,
                                    new UiElement.Slider
                                    {
                                        Label = "THRUST",
                                        LowValue = 0,
                                        HighValue = 10,
                                        Value = 2.5f,
                                        Fill = true,
                                        PageSize = 0.5f,
                                        ShowInputField = true,
                                        Direction = UiSliderDirection.Horizontal,
                                        Events = new[]
                                        {
                                            UiEventKind.ValueChanging,
                                            UiEventKind.ValueCommitted,
                                        },
                                    }
                                ),
                                new(
                                    integerId,
                                    new UiElement.SliderInt
                                    {
                                        Label = "GEAR",
                                        LowValue = 1,
                                        HighValue = 8,
                                        Value = 3,
                                        Fill = true,
                                        Direction = UiSliderDirection.Vertical,
                                        Inverted = true,
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
            }

            public BattlementUiDocuments Documents { get; }
            public List<UiEvent> Events { get; } = new();
            public NativeSlider Float => (NativeSlider)Get(floatId);
            public NativeSliderInt Integer => (NativeSliderInt)Get(integerId);

            public void UpdateFloat(float value) =>
                UpdateFloat(new UiElement.Slider { Value = value });

            public void UpdateFloat(UiElement.Slider value) => Update(floatId, value);

            public void UpdateInteger(UiElement.SliderInt value) => Update(integerId, value);

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
