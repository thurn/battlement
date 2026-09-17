#nullable enable

using System;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.SceneManagement;
using UnityEngine.UIElements;

namespace Battlement.Tests
{
    public sealed class SharedMotionDriverTests
    {
        [Test]
        public void UiAndWorldShareSamplingPauseSpeedStopAndRetarget()
        {
            var rendered = new GameObject("Motion world host");
            try
            {
                ObjectId clock = Id();
                ObjectId uiId = Id();
                ObjectId worldId = Id();
                var ui = new VisualElement();
                ui.style.opacity = 0;
                var transform = new BattlementWorldMotionTarget(rendered.transform);
                using var motion = new BattlementMotionWorld(registerPlayerLoop: false);
                motion.Install(ui, uiId, Descriptor(uiId, clock, MotionProperty.Opacity, 1));
                motion
                    .Prepare(
                        transform,
                        worldId,
                        Descriptor(worldId, clock, MotionProperty.LocalPositionX, 1)
                    )!
                    .Commit();
                motion.SetControlledClock(clock, 250_000);
                motion.PostLayout();
                Assert.That(rendered.transform.localPosition.x, Is.EqualTo(0.25f).Within(0.00001));
                Assert.That(
                    ui.style.opacity.value,
                    Is.EqualTo(rendered.transform.localPosition.x).Within(0.00001)
                );
                foreach (ObjectId id in new[] { uiId, worldId })
                    motion.Pause(id, 1, 1);
                motion.SetControlledClock(clock, 750_000);
                motion.PostLayout();
                Assert.That(rendered.transform.localPosition.x, Is.EqualTo(0.25f).Within(0.00001));
                foreach (ObjectId id in new[] { uiId, worldId })
                {
                    motion.SetSpeed(id, 1, 1, 2);
                    motion.Play(id, 1, 1);
                }
                motion.AdvanceControlledClock(clock, 125_000);
                motion.PostLayout();
                Assert.That(ui.style.opacity.value, Is.EqualTo(0.5f).Within(0.00001));
                Assert.That(rendered.transform.localPosition.x, Is.EqualTo(0.5f).Within(0.00001));
                foreach (ObjectId id in new[] { uiId, worldId })
                    motion.Stop(id, 1, 1);
                motion.AdvanceControlledClock(clock, 1_000_000);
                motion.PostLayout();
                Assert.That(rendered.transform.localPosition.x, Is.EqualTo(0.5f).Within(0.00001));
                motion
                    .Prepare(
                        transform,
                        worldId,
                        Descriptor(worldId, clock, MotionProperty.LocalPositionX, 0, 2)
                    )!
                    .Commit();
                motion.AdvanceControlledClock(clock, 500_000);
                motion.PostLayout();
                Assert.That(rendered.transform.localPosition.x, Is.EqualTo(0.25f).Within(0.00001));
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(rendered);
            }
        }

        [Test]
        public void LocalOffsetComposesAfterMovingPlacement()
        {
            var rendered = new GameObject("Motion composition host");
            try
            {
                ObjectId clock = Id();
                ObjectId host = Id();
                using var motion = new BattlementMotionWorld(registerPlayerLoop: false);
                var target = new BattlementWorldMotionTarget(rendered.transform);
                target.WriteScalar(MotionProperty.LocalOffsetY, 0.12);
                motion
                    .Prepare(
                        target,
                        host,
                        Descriptor(host, clock, MotionProperty.LocalPositionX, 4)
                    )!
                    .Commit();
                motion.SetControlledClock(clock, 500_000);
                motion.PostLayout();
                Assert.That(rendered.transform.localPosition.x, Is.EqualTo(2).Within(0.00001));
                Assert.That(rendered.transform.localPosition.y, Is.EqualTo(0.12).Within(0.00001));
                target.WriteScalar(MotionProperty.LocalOffsetY, 0);
                Assert.That(rendered.transform.localPosition.x, Is.EqualTo(2).Within(0.00001));
                Assert.That(rendered.transform.localPosition.y, Is.Zero);
                motion.SetControlledClock(clock, 1_000_000);
                motion.PostLayout();
                Assert.That(rendered.transform.localPosition.x, Is.EqualTo(4).Within(0.00001));
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(rendered);
            }
        }

        [Test]
        public void WorldSnapshotReconnectPreservesPoseClockAndPausedPlayback()
        {
            ObjectId host = Id();
            ObjectId clock = Id();
            using var assets = new BattlementPreparedAssets(new FakeBattlementAssetStorage());
            using var documents = new BattlementUiDocuments();
            using var world = new BattlementWorld(SceneManager.GetActiveScene(), assets);
            world.Motion.Bind(documents);
            BattlementMotionWorld motion = documents.MotionWorld;
            var description = new BattlementGameObject(
                host,
                new GameObjectKind.Empty(),
                new ParentScene.Persistent(),
                null,
                true,
                new LocalTransform(new Vector3(2, 0, 0), Quaternion.Identity, Vector3.One),
                Array.Empty<PointerEvent>(),
                Motion: Descriptor(host, clock, MotionProperty.LocalPositionX, 6)
            );
            world.CreateObject(description);
            motion.SetControlledClock(clock, 250_000);
            motion.PostLayout();
            Assert.That(
                world.RequireObject(host).transform.localPosition.x,
                Is.EqualTo(3).Within(0.00001)
            );
            motion.Pause(host, 1, 1);
            world.PrepareReplacement(new[] { description }, Array.Empty<BattlementScene>());
            world.ReplaceObjects(new[] { description });
            documents.Replace(Array.Empty<UiDocument>(), _ => null, preserveMotion: true);
            Assert.That(
                world.RequireObject(host).transform.localPosition.x,
                Is.EqualTo(3).Within(0.00001)
            );
            motion.SetControlledClock(clock, 750_000);
            motion.PostLayout();
            Assert.That(
                world.RequireObject(host).transform.localPosition.x,
                Is.EqualTo(3).Within(0.00001)
            );
            motion.Play(host, 1, 1);
            motion.AdvanceControlledClock(clock, 250_000);
            motion.PostLayout();
            Assert.That(
                world.RequireObject(host).transform.localPosition.x,
                Is.EqualTo(4).Within(0.00001)
            );
            world.DestroyObject(host);
            Assert.DoesNotThrow(() => motion.PostLayout());
            Assert.Throws<BattlementUiException>(() => motion.Play(host, 1, 1));
        }

        [Test]
        public void ImperativeOffsetKeepsUnrelatedPlacementPlaybackRunning()
        {
            var rendered = new GameObject("Independent Motion properties");
            try
            {
                ObjectId host = Id();
                ObjectId clock = Id();
                ObjectId control = Id();
                using var motion = new BattlementMotionWorld(registerPlayerLoop: false);
                motion
                    .Prepare(
                        new BattlementWorldMotionTarget(rendered.transform),
                        host,
                        Descriptor(host, clock, MotionProperty.LocalPositionX, 4) with
                        {
                            ControlId = control,
                        }
                    )!
                    .Commit();
                motion.SetControlledClock(clock, 250_000);
                motion.PostLayout();
                motion.ApplyControl(
                    control,
                    MotionControlOperationKind.Start,
                    Id(),
                    1,
                    new MotionControlTarget.Target(
                        Descriptor(host, clock, MotionProperty.LocalOffsetY, 1).Slots[0].Target
                    )
                );
                Assert.That(rendered.transform.localPosition.x, Is.EqualTo(1).Within(0.00001));
                motion.SetControlledClock(clock, 500_000);
                motion.PostLayout();
                Assert.That(rendered.transform.localPosition.x, Is.EqualTo(2).Within(0.00001));
                Assert.That(rendered.transform.localPosition.y, Is.EqualTo(0.25).Within(0.00001));
                motion.ApplyControl(control, MotionControlOperationKind.Clear, Id(), 1, null);
                motion.SetControlledClock(clock, 750_000);
                motion.PostLayout();
                Assert.That(rendered.transform.localPosition.x, Is.EqualTo(3).Within(0.00001));
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(rendered);
            }
        }

        [Test]
        public void RunningMotionPollsCompletionCancellationAndFailureWithoutAdvancingTime()
        {
            var rendered = new GameObject("Command Motion host");
            try
            {
                ObjectId host = Id();
                ObjectId clock = Id();
                ObjectId control = Id();
                using var motion = new BattlementMotionWorld(registerPlayerLoop: false);
                motion
                    .Prepare(
                        new BattlementWorldMotionTarget(rendered.transform),
                        host,
                        Descriptor(host, clock, MotionProperty.LocalPositionX, 0) with
                        {
                            ControlId = control,
                        }
                    )!
                    .Commit();
                MotionControlTarget Target(double value) =>
                    new MotionControlTarget.Target(
                        Descriptor(host, clock, MotionProperty.LocalPositionX, value)
                            .Slots[0]
                            .Target
                    );
                ObjectId first = Id();
                IBattlementCommandOperation running = motion.ApplyControl(
                    control,
                    MotionControlOperationKind.Start,
                    first,
                    1,
                    Target(4),
                    blocking: true
                )!;
                Assert.That(running.IsInfinite, Is.False);
                Assert.That(running.IsComplete(TimeSpan.FromHours(1)), Is.False);
                Assert.That(rendered.transform.localPosition.x, Is.Zero);
                motion.SetControlledClock(clock, 500_000);
                motion.PostLayout();
                Assert.That(rendered.transform.localPosition.x, Is.EqualTo(2).Within(0.00001));
                Assert.That(running.IsComplete(TimeSpan.Zero), Is.False);
                motion.SetControlledClock(clock, 1_000_000);
                motion.PostLayout();
                Assert.That(running.IsComplete(TimeSpan.Zero), Is.True);
                Assert.That(
                    motion.DrainEventBatch()!.PlaybackEvents![0].Outcome,
                    Is.EqualTo(MotionPlaybackOutcome.Completed)
                );
                IBattlementCommandOperation cancelled = motion.ApplyControl(
                    control,
                    MotionControlOperationKind.Start,
                    Id(),
                    1,
                    Target(8)
                )!;
                cancelled.Cancel();
                Assert.That(cancelled.IsComplete(TimeSpan.Zero), Is.True);
                Assert.That(
                    motion.DrainEventBatch()!.PlaybackEvents![0].Outcome,
                    Is.EqualTo(MotionPlaybackOutcome.Cancelled)
                );
                IBattlementCommandOperation failed = motion.ApplyControl(
                    control,
                    MotionControlOperationKind.Start,
                    Id(),
                    1,
                    Target(10)
                )!;
                UnityEngine.Object.DestroyImmediate(rendered);
                Assert.DoesNotThrow(() => motion.PostLayout());
                Assert.Throws<InvalidOperationException>(() => failed.IsComplete(TimeSpan.Zero));
                Assert.That(
                    motion.DrainEventBatch()!.PlaybackEvents![0].Outcome,
                    Is.EqualTo(MotionPlaybackOutcome.Failed)
                );
            }
            finally
            {
                if (rendered != null)
                    UnityEngine.Object.DestroyImmediate(rendered);
            }
        }

        [Test]
        public void BlockingInfiniteMotionIsRejectedBeforeReplacingTheRunningWriter()
        {
            var rendered = new GameObject("Finite command Motion host");
            try
            {
                ObjectId host = Id();
                ObjectId clock = Id();
                ObjectId control = Id();
                using var motion = new BattlementMotionWorld(registerPlayerLoop: false);
                MotionDescriptor descriptor = Descriptor(
                    host,
                    clock,
                    MotionProperty.LocalPositionX,
                    0
                ) with
                {
                    ControlId = control,
                };
                motion
                    .Prepare(new BattlementWorldMotionTarget(rendered.transform), host, descriptor)!
                    .Commit();
                MotionTargetDescriptor finite = Descriptor(
                    host,
                    clock,
                    MotionProperty.LocalPositionX,
                    4
                )
                    .Slots[0]
                    .Target;
                IBattlementCommandOperation first = motion.ApplyControl(
                    control,
                    MotionControlOperationKind.Start,
                    Id(),
                    1,
                    new MotionControlTarget.Target(finite)
                )!;
                motion.SetControlledClock(clock, 250_000);
                motion.PostLayout();
                MotionPropertyTrack track = finite.Tracks[0];
                MotionTargetDescriptor infinite = finite with
                {
                    Tracks = new[]
                    {
                        track with
                        {
                            Transition = track.Transition with
                            {
                                Repeat = new MotionRepeat.Forever(),
                            },
                        },
                    },
                };
                Assert.Throws<BattlementUiException>(() =>
                    motion.ApplyControl(
                        control,
                        MotionControlOperationKind.Start,
                        Id(),
                        1,
                        new MotionControlTarget.Target(infinite),
                        blocking: true
                    )
                );
                Assert.That(first.IsComplete(TimeSpan.Zero), Is.False);
                motion.SetControlledClock(clock, 500_000);
                motion.PostLayout();
                Assert.That(rendered.transform.localPosition.x, Is.EqualTo(2).Within(0.00001));
                IBattlementCommandOperation loop = motion.ApplyControl(
                    control,
                    MotionControlOperationKind.Start,
                    Id(),
                    1,
                    new MotionControlTarget.Target(infinite)
                )!;
                Assert.That(first.IsComplete(TimeSpan.Zero), Is.True);
                Assert.That(loop.IsInfinite, Is.True);
                loop.Cancel();
                Assert.That(loop.IsComplete(TimeSpan.Zero), Is.True);
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(rendered);
            }
        }

        [TestCase(false)]
        [TestCase(true)]
        public void RoutedHoverReturnsToMovingUnderlyingOffsetAcrossReconnect(bool repeat)
        {
            ObjectId host = Id();
            ObjectId clock = Id();
            using var assets = new BattlementPreparedAssets(new FakeBattlementAssetStorage());
            using var documents = new BattlementUiDocuments();
            using var world = new BattlementWorld(SceneManager.GetActiveScene(), assets);
            world.Motion.Bind(documents);
            BattlementMotionWorld motion = documents.MotionWorld;
            MotionDescriptor descriptor = Descriptor(host, clock, MotionProperty.LocalPositionX, 4);
            descriptor = descriptor with
            {
                Slots = new[]
                {
                    descriptor.Slots[0],
                    Descriptor(host, clock, MotionProperty.LocalOffsetY, 0.2).Slots[0] with
                    {
                        Slot = 2,
                    },
                    Descriptor(host, clock, MotionProperty.LocalOffsetY, 0.4).Slots[0] with
                    {
                        Slot = 3,
                        Layer = MotionLayer.Hover,
                    },
                },
            };
            if (repeat)
            {
                MotionSlotDescriptor hover = descriptor.Slots[2];
                MotionPropertyTrack track = hover.Target.Tracks[0];
                descriptor = descriptor with
                {
                    Slots = new[]
                    {
                        descriptor.Slots[0],
                        descriptor.Slots[1],
                        hover with
                        {
                            Target = hover.Target with
                            {
                                Tracks = new[]
                                {
                                    track with
                                    {
                                        Transition = track.Transition with
                                        {
                                            Repeat = new MotionRepeat.Forever(),
                                        },
                                    },
                                },
                            },
                        },
                    },
                };
            }
            var description = new BattlementGameObject(
                host,
                new GameObjectKind.Empty(),
                new ParentScene.Persistent(),
                null,
                true,
                LocalTransform.Identity,
                Array.Empty<PointerEvent>(),
                Motion: descriptor
            );
            world.CreateObject(description);
            motion.SetControlledClock(clock, 500_000);
            motion.PostLayout();
            world.Motion.Handle(
                new UiEvent(
                    host,
                    false,
                    false,
                    new UiEventBody.PointerOver(new UiPointerCrossingEvent(new PanelPoint(0, 0)))
                )
            );
            motion.SetControlledClock(clock, 750_000);
            motion.PostLayout();
            Assert.That(
                world.RequireObject(host).transform.localPosition.x,
                Is.EqualTo(3).Within(0.00001)
            );
            Assert.That(
                world.RequireObject(host).transform.localPosition.y,
                Is.EqualTo(0.175).Within(0.00001)
            );
            world.Motion.Handle(
                new UiEvent(
                    host,
                    false,
                    false,
                    new UiEventBody.PointerOut(new UiPointerCrossingEvent(new PanelPoint(0, 0)))
                )
            );
            motion.SetControlledClock(clock, 1_000_000);
            motion.PostLayout();
            Assert.That(
                world.RequireObject(host).transform.localPosition.x,
                Is.EqualTo(4).Within(0.00001)
            );
            Assert.That(
                world.RequireObject(host).transform.localPosition.y,
                Is.EqualTo(0.18125).Within(0.00001)
            );
            world.PrepareReplacement(new[] { description }, Array.Empty<BattlementScene>());
            world.ReplaceObjects(new[] { description });
            documents.Replace(Array.Empty<UiDocument>(), _ => null, preserveMotion: true);
            Assert.That(
                world.RequireObject(host).transform.localPosition.y,
                Is.EqualTo(0.18125).Within(0.00001)
            );
            motion.SetControlledClock(clock, 1_250_000);
            motion.PostLayout();
            Assert.That(
                world.RequireObject(host).transform.localPosition.y,
                Is.EqualTo(0.1875).Within(0.00001)
            );
            motion.SetControlledClock(clock, 1_750_000);
            motion.PostLayout();
            Assert.That(
                world.RequireObject(host).transform.localPosition.y,
                Is.EqualTo(0.2).Within(0.00001)
            );
        }

        private static ObjectId Id() => new(Guid.NewGuid());

        internal static MotionDescriptor Descriptor(
            ObjectId host,
            ObjectId clock,
            MotionProperty property,
            double value,
            uint generation = 1
        ) =>
            new(
                host,
                host,
                generation,
                false,
                new[]
                {
                    new MotionSlotDescriptor(
                        1,
                        generation,
                        MotionLayer.Animate,
                        new MotionTargetDescriptor(
                            new[]
                            {
                                new MotionPropertyTrack(
                                    property,
                                    new MotionValue[] { new MotionValue.Scalar(value) },
                                    new TransitionDefinition(
                                        new TransitionGenerator.Tween(
                                            1_000_000,
                                            new MotionEasing[] { new MotionEasing.Linear() }
                                        ),
                                        0,
                                        new MotionRepeat.None(),
                                        0,
                                        MotionRepeatType.Loop
                                    )
                                ),
                            },
                            Array.Empty<MotionPropertyValue>()
                        ),
                        new MotionCallbackSubscriptions(false, false, false, false, false, false)
                    ),
                },
                new MotionClockSource.Controlled(clock),
                ReducedMotionPolicy.Never
            );
    }
}
