#nullable enable

using System;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;
using UiView = Battlement.UiElement.VisualElement;

namespace Battlement.Tests
{
    public sealed class AuthoredStyleMotionTests
    {
        [TestCase(false)]
        [TestCase(true)]
        public void StyleOnlyUpdatesPreserveFocusAndReplaceItsUnderlyingValue(bool focused)
        {
            ObjectId document = new(Guid.NewGuid());
            ObjectId root = new(Guid.NewGuid());
            ObjectId host = new(Guid.NewGuid());
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(root)
            );
            using var documents = new BattlementUiDocuments();
            try
            {
                documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            document,
                            root,
                            Children: new[]
                            {
                                new UiNode(
                                    host,
                                    new UiView
                                    {
                                        Focusable = true,
                                        Style = new UiStyle(Opacity: UiStyle.Set(0.4f)),
                                        Motion = Descriptor(host),
                                    }
                                ),
                            }
                        ),
                    },
                    id => id == document ? owned : null
                );
                Assert.That(documents.TryGet(host, out VisualElement? target), Is.True);
                BattlementMotionWorld world = documents.MotionWorldForTests;
                if (focused)
                    Focus(documents, host, true);
                Sample(world, host);
                Assert.That(
                    target!.style.opacity.value,
                    Is.EqualTo(focused ? 0.9f : 0.4f).Within(0.001)
                );

                Assert.That(target!.style.translate.value.x.value, Is.EqualTo(10).Within(0.01));
                Patch(documents, host, new UiStyle(Opacity: UiStyle.Set(0.6f)));
                Sample(world, host);
                Assert.That(
                    target.style.opacity.value,
                    Is.EqualTo(focused ? 0.9f : 0.6f).Within(0.001)
                );
                Assert.That(target.style.translate.value.x.value, Is.EqualTo(20).Within(0.01));
                Focus(documents, host, true);
                Sample(world, host);
                Patch(
                    documents,
                    host,
                    new UiStyle(PaddingLeft: UiStyle.Set<UiLength>(new UiLength.Px(12)))
                );
                Sample(world, host);
                Focus(documents, host, false);
                Sample(world, host);
                Assert.That(target.style.opacity.value, Is.EqualTo(0.6f).Within(0.001));

                Focus(documents, host, true);
                Sample(world, host);
                Patch(documents, host, new UiStyle(Opacity: UiStyle.Set(0.9f)));
                Sample(world, host);
                Focus(documents, host, false);
                Sample(world, host);
                Assert.That(target.style.opacity.value, Is.EqualTo(0.9f).Within(0.001));
                Patch(documents, host, new UiStyle(Opacity: UiStyle.Set(0.7f)));
                Sample(world, host);
                Assert.That(target.style.opacity.value, Is.EqualTo(0.7f).Within(0.001));
                Patch(documents, host, new UiStyle(Opacity: UiStyle.Reset<float>()));
                Sample(world, host);
                Assert.That(target.style.opacity.keyword, Is.EqualTo(StyleKeyword.Null));
            }
            finally
            {
                documents.Clear();
                Object.DestroyImmediate(owned);
            }
        }

        private static void Focus(BattlementUiDocuments documents, ObjectId host, bool focused)
        {
            documents.PerformAction(
                new CommandBody.VisualElement.PerformAction(
                    host,
                    focused ? new VisualElementAction.Focus() : new VisualElementAction.Blur()
                )
            );
            if (!focused)
                return;
            Assert.That(documents.TryGet(host, out VisualElement? target), Is.True);
            using KeyDownEvent key = KeyDownEvent.GetPooled(
                '\0',
                KeyCode.LeftArrow,
                EventModifiers.None
            );
            key.target = target;
            target!.SendEvent(key);
        }

        private static void Patch(BattlementUiDocuments documents, ObjectId host, UiStyle style) =>
            documents.Update(
                new CommandBody.VisualElement.Update(
                    new VisualElementUpdate.Properties(host, new UiView { Style = style })
                )
            );

        private static void Sample(BattlementMotionWorld world, ObjectId clock)
        {
            world.AdvanceControlledClock(clock, 100_000);
            world.PreLayout();
            world.PostLayout();
        }

        private static MotionDescriptor Descriptor(ObjectId host) =>
            new(
                host,
                host,
                1,
                false,
                new[]
                {
                    new MotionSlotDescriptor(
                        2,
                        1,
                        MotionLayer.Animate,
                        new MotionTargetDescriptor(
                            new[]
                            {
                                new MotionPropertyTrack(
                                    MotionProperty.X,
                                    new MotionValue[]
                                    {
                                        new MotionValue.Length(UiLength.FromComponents(100, 0)),
                                    },
                                    new TransitionDefinition(
                                        new TransitionGenerator.Tween(
                                            1_000_000,
                                            new MotionEasing[] { new MotionEasing.Linear() },
                                            null
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
                    new MotionSlotDescriptor(
                        1,
                        1,
                        MotionLayer.FocusVisible,
                        new MotionTargetDescriptor(
                            new[]
                            {
                                new MotionPropertyTrack(
                                    MotionProperty.Opacity,
                                    new MotionValue[] { new MotionValue.Scalar(0.9) },
                                    new TransitionDefinition(
                                        new TransitionGenerator.Tween(
                                            1,
                                            new MotionEasing[] { new MotionEasing.Linear() },
                                            null
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
                new MotionClockSource.Controlled(host),
                ReducedMotionPolicy.Never
            );
    }
}
