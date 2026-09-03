#nullable enable

using System;
using System.Collections.Generic;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine.UIElements;

namespace Battlement.Tests
{
    public sealed class CssMotionTests
    {
        [Test]
        public void PseudoStatePriorityTransitionsFromRenderedPresentation()
        {
            ObjectId id = Id("e83c1cb4-6785-4f11-9704-0397276910ce");
            var target = new VisualElement { focusable = true };
            target.style.opacity = 0.2f;
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(target, id, Descriptor(id, pseudo: true));

            world.SetPseudoState(id, MotionPseudoState.Hover, true);
            world.AdvanceControlledClock(id, 100_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(0.5f).Within(0.00001));

            world.SetPseudoState(id, MotionPseudoState.Active, true);
            world.AdvanceControlledClock(id, 200_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(1f).Within(0.00001));
        }

        [Test]
        public void CssAnimationUsesExactIterationsPauseDirectionAndFill()
        {
            ObjectId id = Id("5c4a141a-fb84-462f-ac42-ab5d676330ae");
            var target = new VisualElement();
            target.style.opacity = 0;
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(target, id, Descriptor(id, animation: true));
            world.AdvanceControlledClock(id, 500_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(0.5f).Within(0.00001));

            world.AdvanceControlledClock(id, 500_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(1f).Within(0.00001));
            world.AdvanceControlledClock(id, 500_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(0.5f).Within(0.00001));
            world.AdvanceControlledClock(id, 500_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(0f).Within(0.00001));
        }

        [Test]
        public void DecorationsAreNonInteractiveAndRemovedWithDescriptor()
        {
            ObjectId id = Id("bd92beaa-c9e8-4083-bf24-b6a4913b14a9");
            var target = new VisualElement();
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(target, id, Descriptor(id, decoration: true));
            VisualElement decoration = target.Q("battlement-decoration-77");
            Assert.That(decoration, Is.Not.Null);
            Assert.That(decoration.pickingMode, Is.EqualTo(PickingMode.Ignore));
            world.RemoveHost(id);
            Assert.That(target.Q("battlement-decoration-77"), Is.Null);
        }

        [Test]
        public void PlayStateUpdatesPreserveElapsedTimeAndRestartKeysResetIt()
        {
            ObjectId id = Id("6c980bd8-8e02-4275-9cb5-9798210842fc");
            var target = new VisualElement();
            target.style.opacity = 0;
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(
                target,
                id,
                Descriptor(
                    id,
                    animation: true,
                    direction: AnimationDirection.Normal,
                    repeat: new MotionRepeat.None()
                )
            );
            world.AdvanceControlledClock(id, 500_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(0.5f).Within(0.00001));

            world.Install(
                target,
                id,
                Descriptor(
                    id,
                    animation: true,
                    generation: 2,
                    playState: AnimationPlayState.Paused,
                    direction: AnimationDirection.Normal,
                    repeat: new MotionRepeat.None()
                )
            );
            world.AdvanceControlledClock(id, 250_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(0.5f).Within(0.00001));

            world.Install(
                target,
                id,
                Descriptor(
                    id,
                    animation: true,
                    generation: 3,
                    direction: AnimationDirection.Normal,
                    repeat: new MotionRepeat.None()
                )
            );
            world.AdvanceControlledClock(id, 250_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(0.75f).Within(0.00001));

            world.Install(
                target,
                id,
                Descriptor(
                    id,
                    animation: true,
                    generation: 4,
                    restartKey: 2002,
                    direction: AnimationDirection.Normal,
                    repeat: new MotionRepeat.None()
                )
            );
            world.AdvanceControlledClock(id, 250_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(0.25f).Within(0.00001));
        }

        [Test]
        public void AdditiveCompositionUsesTheStableLowerOwnerWithoutFrameDrift()
        {
            ObjectId id = Id("77d5bd88-8740-41f6-ae58-4ae3fcb88131");
            var target = new VisualElement();
            target.style.opacity = 0.25f;
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(
                target,
                id,
                Descriptor(
                    id,
                    animation: true,
                    composition: AnimationComposition.Add,
                    direction: AnimationDirection.Normal,
                    repeat: new MotionRepeat.None()
                )
            );
            world.AdvanceControlledClock(id, 500_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(0.75f).Within(0.00001));
            world.AdvanceControlledClock(id, 250_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(1f).Within(0.00001));
        }

        [Test]
        public void AccumulationAddsEachCompletedIterationExactlyOnce()
        {
            ObjectId id = Id("c82d70d7-f456-40da-af86-ae80a6abaf94");
            var target = new VisualElement();
            target.style.opacity = 0.25f;
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(
                target,
                id,
                Descriptor(
                    id,
                    animation: true,
                    composition: AnimationComposition.Accumulate,
                    direction: AnimationDirection.Normal,
                    repeat: new MotionRepeat.Count(1)
                )
            );
            world.AdvanceControlledClock(id, 1_500_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(1.75f).Within(0.00001));
        }

        [Test]
        public void DisplayDisappearsAtTheDiscreteTransitionEndpoint()
        {
            ObjectId id = Id("0c288453-7373-4c69-94a5-e1f1d0f8405d");
            var target = new VisualElement();
            target.style.display = DisplayStyle.Flex;
            TransitionDefinition transition = new(
                new TransitionGenerator.Tween(
                    200_000,
                    new MotionEasing[] { new MotionEasing.Linear() }
                ),
                0,
                new MotionRepeat.None(),
                0,
                MotionRepeatType.Loop
            );
            Prop<MotionDescriptor> descriptor = Prop<MotionDescriptor>.Set(
                new MotionDescriptor(
                    id,
                    id,
                    1,
                    false,
                    Array.Empty<MotionSlotDescriptor>(),
                    new MotionClockSource.Controlled(id),
                    ReducedMotionPolicy.Never,
                    null,
                    new[]
                    {
                        new MotionPseudoStyle(
                            MotionPseudoState.Active,
                            new[]
                            {
                                new MotionPropertyValue(
                                    MotionProperty.Display,
                                    new MotionValue.Discrete("none")
                                ),
                            }
                        ),
                    },
                    new StyleTransitionDescriptor(
                        new[] { new StylePropertyTransition(MotionProperty.Display, transition) },
                        null,
                        true
                    ),
                    Array.Empty<CssAnimationDescriptor>(),
                    Array.Empty<MotionDecorationDescriptor>()
                )
            );
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(target, id, descriptor);
            world.SetPseudoState(id, MotionPseudoState.Active, true);
            world.AdvanceControlledClock(id, 100_000);
            world.PostLayout();
            Assert.That(target.style.display.value, Is.EqualTo(DisplayStyle.Flex));
            world.AdvanceControlledClock(id, 100_000);
            world.PostLayout();
            Assert.That(target.style.display.value, Is.EqualTo(DisplayStyle.None));
        }

        [Test]
        public void RendererCapabilityRejectionPrecedesDescriptorActivation()
        {
            ObjectId id = Id("6a27d761-7e66-4a26-9071-01e3645b5c38");
            CssAnimationDescriptor unsupported = new(
                101,
                1,
                7007,
                new[]
                {
                    new CssPropertyTrack(
                        MotionProperty.AlignContent,
                        new MotionValue[]
                        {
                            new MotionValue.Discrete("stretch"),
                            new MotionValue.Discrete("center"),
                        },
                        new double[] { 0, 1 },
                        new TransitionDefinition(
                            new TransitionGenerator.Tween(
                                100_000,
                                new MotionEasing[] { new MotionEasing.Linear() }
                            ),
                            0,
                            new MotionRepeat.None(),
                            0,
                            MotionRepeatType.Loop
                        )
                    ),
                },
                AnimationDirection.Normal,
                AnimationFill.Both,
                AnimationPlayState.Running,
                AnimationComposition.Replace,
                "unsupported"
            );
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            Assert.Throws<BattlementUiException>(() =>
                world.Install(new VisualElement(), id, Motion(id, 1, new[] { unsupported }))
            );
            Assert.That(world.DescriptorCount, Is.Zero);
        }

        [Test]
        public void KeyframesInsertTheUnderlyingValueAtMissingEndpoints()
        {
            ObjectId id = Id("bf0474b3-c290-4a7b-bdb3-a63485092bbc");
            var target = new VisualElement();
            target.style.opacity = 0.2f;
            CssAnimationDescriptor animation = Animation(
                generation: 1,
                restartKey: 3003,
                values: new MotionValue[] { new MotionValue.Scalar(1) },
                times: new double[] { 0.5 },
                repeat: new MotionRepeat.None()
            );
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(target, id, Motion(id, 1, new[] { animation }));
            world.AdvanceControlledClock(id, 250_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(0.6f).Within(0.00001));
            world.AdvanceControlledClock(id, 500_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(0.6f).Within(0.00001));
        }

        [Test]
        public void DecorationIdentityAndPlaybackSurviveHostRefresh()
        {
            ObjectId id = Id("47550152-e378-4be4-93ce-4576f1477ec7");
            var target = new VisualElement();
            CssAnimationDescriptor animation = Animation(
                generation: 1,
                restartKey: 4004,
                repeat: new MotionRepeat.None()
            );
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(target, id, Motion(id, 1, decorations: Decorations(animation)));
            VisualElement first = target.Q("battlement-decoration-77");
            world.AdvanceControlledClock(id, 500_000);
            world.PostLayout();
            Assert.That(first.style.opacity.value, Is.EqualTo(0.5f).Within(0.00001));

            CssAnimationDescriptor refreshed = animation with { Generation = 2 };
            world.Install(target, id, Motion(id, 2, decorations: Decorations(refreshed)));
            VisualElement second = target.Q("battlement-decoration-77");
            Assert.That(second, Is.SameAs(first));
            world.AdvanceControlledClock(id, 250_000);
            world.PostLayout();
            Assert.That(second.style.opacity.value, Is.EqualTo(0.75f).Within(0.00001));
        }

        [Test]
        public void StaticStyleChangesTransitionFromTheRenderedPresentation()
        {
            ObjectId id = Id("db564bd4-16d2-4757-9c64-d6885c17dafd");
            var target = new VisualElement();
            target.style.opacity = 0.2f;
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(target, id, Descriptor(id, pseudo: true));
            BattlementPreparedMotionAdmission prepared = world.Prepare(
                target,
                id,
                Descriptor(id, pseudo: true, generation: 2)
            )!;
            target.style.opacity = 0.8f;
            prepared.Commit();
            world.AdvanceControlledClock(id, 100_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(0.5f).Within(0.00001));
        }

        private static Prop<MotionDescriptor> Descriptor(
            ObjectId id,
            bool pseudo = false,
            bool animation = false,
            bool decoration = false,
            uint generation = 1,
            ulong restartKey = 1001,
            AnimationPlayState playState = AnimationPlayState.Running,
            AnimationComposition composition = AnimationComposition.Replace,
            AnimationDirection direction = AnimationDirection.Alternate,
            MotionRepeat? repeat = null
        )
        {
            TransitionDefinition tween = new(
                new TransitionGenerator.Tween(
                    200_000,
                    new MotionEasing[] { new MotionEasing.Linear() }
                ),
                0,
                new MotionRepeat.None(),
                0,
                MotionRepeatType.Loop
            );
            MotionPseudoStyle[] pseudoStyles = pseudo
                ? new[]
                {
                    new MotionPseudoStyle(MotionPseudoState.Hover, new[] { Value(0.8) }),
                    new MotionPseudoStyle(MotionPseudoState.Active, new[] { Value(1) }),
                }
                : Array.Empty<MotionPseudoStyle>();
            CssAnimationDescriptor[] animations = animation
                ? new[]
                {
                    Animation(
                        generation,
                        restartKey,
                        playState: playState,
                        composition: composition,
                        direction: direction,
                        repeat: repeat ?? new MotionRepeat.Count(1)
                    ),
                }
                : Array.Empty<CssAnimationDescriptor>();
            MotionDecorationDescriptor[] decorations = decoration
                ? new[]
                {
                    new MotionDecorationDescriptor(
                        77,
                        DecorationPlacement.After,
                        DecorationPosition.Fill,
                        DecorationOverflow.Hidden,
                        new UiStyle(),
                        Array.Empty<CssAnimationDescriptor>()
                    ),
                }
                : Array.Empty<MotionDecorationDescriptor>();
            return Prop<MotionDescriptor>.Set(
                new MotionDescriptor(
                    id,
                    id,
                    generation,
                    false,
                    Array.Empty<MotionSlotDescriptor>(),
                    new MotionClockSource.Controlled(id),
                    ReducedMotionPolicy.Never,
                    null,
                    pseudoStyles,
                    new StyleTransitionDescriptor(
                        pseudo
                            ? new[] { new StylePropertyTransition(MotionProperty.Opacity, tween) }
                            : Array.Empty<StylePropertyTransition>(),
                        null,
                        false
                    ),
                    animations,
                    decorations
                )
            );
        }

        private static CssAnimationDescriptor Animation(
            uint generation,
            ulong restartKey,
            IReadOnlyList<MotionValue>? values = null,
            IReadOnlyList<double>? times = null,
            AnimationPlayState playState = AnimationPlayState.Running,
            AnimationComposition composition = AnimationComposition.Replace,
            AnimationDirection direction = AnimationDirection.Normal,
            AnimationFill fill = AnimationFill.Both,
            MotionRepeat? repeat = null
        ) =>
            new(
                100,
                generation,
                restartKey,
                new[]
                {
                    new CssPropertyTrack(
                        MotionProperty.Opacity,
                        values
                            ?? new MotionValue[]
                            {
                                new MotionValue.Scalar(0),
                                new MotionValue.Scalar(1),
                            },
                        times ?? new double[] { 0, 1 },
                        new TransitionDefinition(
                            new TransitionGenerator.Tween(
                                1_000_000,
                                new MotionEasing[] { new MotionEasing.Linear() }
                            ),
                            0,
                            repeat ?? new MotionRepeat.None(),
                            0,
                            direction
                                is AnimationDirection.Alternate
                                    or AnimationDirection.AlternateReverse
                                ? MotionRepeatType.Reverse
                                : MotionRepeatType.Loop
                        )
                    ),
                },
                direction,
                fill,
                playState,
                composition,
                "test"
            );

        private static MotionDecorationDescriptor[] Decorations(CssAnimationDescriptor animation) =>
            new[]
            {
                new MotionDecorationDescriptor(
                    77,
                    DecorationPlacement.After,
                    DecorationPosition.Fill,
                    DecorationOverflow.Hidden,
                    new UiStyle(),
                    new[] { animation }
                ),
            };

        private static Prop<MotionDescriptor> Motion(
            ObjectId id,
            uint generation,
            IReadOnlyList<CssAnimationDescriptor>? animations = null,
            IReadOnlyList<MotionDecorationDescriptor>? decorations = null
        ) =>
            Prop<MotionDescriptor>.Set(
                new MotionDescriptor(
                    id,
                    id,
                    generation,
                    false,
                    Array.Empty<MotionSlotDescriptor>(),
                    new MotionClockSource.Controlled(id),
                    ReducedMotionPolicy.Never,
                    null,
                    Array.Empty<MotionPseudoStyle>(),
                    new StyleTransitionDescriptor(
                        Array.Empty<StylePropertyTransition>(),
                        null,
                        false
                    ),
                    animations ?? Array.Empty<CssAnimationDescriptor>(),
                    decorations ?? Array.Empty<MotionDecorationDescriptor>()
                )
            );

        private static MotionPropertyValue Value(double value) =>
            new(MotionProperty.Opacity, new MotionValue.Scalar(value));

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
