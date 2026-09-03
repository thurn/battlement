#nullable enable

using System;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;

namespace Battlement.Tests
{
    public sealed class LayoutIntegrationTests
    {
        [Test]
        public void PresentationLayoutRunsAfterTheCurrentMotionSample()
        {
            ObjectId host = Id("8ce3a260-fef0-47d9-92a7-4108a3b23590");
            ObjectId clock = Id("f7475e2d-c918-43e0-867f-75e43ef47a75");
            var target = new VisualElement();
            float presentedX = -1;
            using var world = new BattlementMotionWorld(
                registerPlayerLoop: false,
                presentationChanged: () => presentedX = ReadX(target)
            );
            world.Install(target, host, Descriptor(host, clock));

            world.SetControlledClock(clock, 500_000);
            world.PostLayout();

            Assert.That(presentedX, Is.EqualTo(50).Within(0.00001));
        }

        [Test]
        public void UnresolvableReconstructionPreservesTheCommittedNativeTree()
        {
            ObjectId documentId = Id("6778a7ea-cb72-4ae6-89af-a2e51031fb3f");
            ObjectId rootId = Id("d567537c-4214-4250-b1d5-858c2809f77a");
            ObjectId childId = Id("4e4dc70b-910f-4369-904c-b1d1439b9704");
            GameObject owner = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            using var documents = new BattlementUiDocuments();
            try
            {
                UiDocument snapshot = Snapshot(documentId, rootId, childId);
                documents.Replace(new[] { snapshot }, id => id == documentId ? owner : null);
                Assert.That(documents.TryGet(childId, out VisualElement? committed), Is.True);

                Assert.Throws<InvalidOperationException>(() =>
                    documents.Replace(new[] { snapshot }, _ => null, preserveMotion: true)
                );

                Assert.That(documents.TryGet(childId, out VisualElement? retained), Is.True);
                Assert.That(retained, Is.SameAs(committed));
                Assert.That(retained!.panel, Is.SameAs(committed!.panel));
                Assert.That(
                    owner.GetComponent<UIDocument>().rootVisualElement.Contains(retained),
                    Is.True
                );
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(owner);
            }
        }

        [Test]
        public void EmptyAuthoritativeSnapshotRetiresPreviousNativeContent()
        {
            ObjectId documentId = Id("f330c463-a1ed-468a-b31d-b2fe3ed678ad");
            ObjectId rootId = Id("60f13044-8f70-4104-837f-10d6879447a6");
            ObjectId childId = Id("21fe7d41-8045-4d15-a985-9c1e39d3565d");
            GameObject owner = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            using var documents = new BattlementUiDocuments();
            try
            {
                documents.Replace(
                    new[] { Snapshot(documentId, rootId, childId) },
                    id => id == documentId ? owner : null
                );

                documents.Replace(Array.Empty<UiDocument>(), _ => null, preserveMotion: true);

                Assert.That(documents.TryGet(childId, out _), Is.False);
                Assert.That(owner.GetComponent<UIDocument>().rootVisualElement.childCount, Is.Zero);
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(owner);
            }
        }

        private static UiDocument Snapshot(
            ObjectId documentId,
            ObjectId rootId,
            ObjectId childId
        ) =>
            new(
                documentId,
                rootId,
                Children: new[]
                {
                    new UiNode(childId, new UiElement.VisualElement { Name = "committed" }),
                }
            );

        private static MotionDescriptor Descriptor(ObjectId host, ObjectId clock) =>
            new(
                host,
                host,
                1,
                false,
                new[]
                {
                    new MotionSlotDescriptor(
                        1,
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
                },
                new MotionClockSource.Controlled(clock),
                ReducedMotionPolicy.Never,
                null
            );

        private static float ReadX(VisualElement target) =>
            BattlementMotionPropertyWriter.Read(target, MotionProperty.X)
                is MotionValue.Length value
                ? (float)value.Value.Pixels
                : 0;

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
