#nullable enable

using System;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.EventSystems;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;
using UnityPanelInputConfiguration = UnityEngine.UIElements.PanelInputConfiguration;

namespace Battlement.Tests
{
    public sealed class BattlementPanelInputTests
    {
        [Test]
        public void WorldDocumentFactoryAppliesGeometryAndLeavesUnityColliderPolicy()
        {
            var state = new GameObjectKind.UiDocumentState(
                new ObjectId(Guid.NewGuid()),
                new PanelSettingsValue(RenderMode: PanelRenderMode.WorldSpace),
                DocumentPosition.Absolute,
                WorldSpaceSizeMode.Fixed,
                new ScreenSize(420, 240),
                PivotReferenceSize.Layout,
                DocumentPivot.BottomRight,
                17
            );
            GameObject documentObject = Battlement.UI.BattlementUiDocuments.CreateGameObject(state);
            try
            {
                UIDocument document = documentObject.GetComponent<UIDocument>();
                Assert.That(
                    document.panelSettings.renderMode,
                    Is.EqualTo(UnityEngine.UIElements.PanelRenderMode.WorldSpace)
                );
                Assert.That(document.position, Is.EqualTo(Position.Absolute));
                Assert.That(
                    document.worldSpaceSizeMode,
                    Is.EqualTo(UnityEngine.UIElements.WorldSpaceSizeMode.Fixed)
                );
                Assert.That(document.worldSpaceSize, Is.EqualTo(new Vector2(420, 240)));
                Assert.That(
                    document.pivotReferenceSize,
                    Is.EqualTo(UnityEngine.UIElements.PivotReferenceSize.Layout)
                );
                Assert.That(document.pivot, Is.EqualTo(Pivot.BottomRight));
                Assert.That(document.sortingOrder, Is.EqualTo(17));
                Assert.That(documentObject.GetComponent<Collider>(), Is.Null);
            }
            finally
            {
                Object.DestroyImmediate(documentObject);
            }
        }

        [Test]
        public void CoordinatorMapsFiniteExplicitCameraConfigurationExactly()
        {
            var eventObject = new GameObject("Event System");
            var cameraObject = new GameObject("Input Camera");
            eventObject.AddComponent<EventSystem>();
            Camera camera = cameraObject.AddComponent<Camera>();
            var coordinator = new BattlementPanelInputCoordinator();
            try
            {
                Snapshot snapshot = WorldSnapshot(
                    new PanelInputConfigurationValue(
                        new InteractionLayerMask(0x8000_0005),
                        new InteractionDistance.Inclusive(12.5f),
                        PanelInputRedirection.Always
                    ),
                    new ObjectId(Guid.NewGuid())
                );

                coordinator.ValidateBeforeReplacement(snapshot);
                coordinator.Apply(snapshot, camera);

                UnityPanelInputConfiguration value = coordinator.OwnedConfiguration!;
                Assert.That(value.processWorldSpaceInput, Is.True);
                Assert.That(
                    unchecked((uint)value.interactionLayers.value),
                    Is.EqualTo(0x8000_0005)
                );
                Assert.That(value.maxInteractionDistance, Is.EqualTo(12.5f));
                Assert.That(value.defaultEventCameraIsMainCamera, Is.False);
                Assert.That(value.eventCameras, Is.EqualTo(new[] { camera }));
                Assert.That(
                    value.panelInputRedirection,
                    Is.EqualTo(UnityPanelInputConfiguration.PanelInputRedirection.Always)
                );
                Assert.That(value.autoCreatePanelComponents, Is.True);
            }
            finally
            {
                coordinator.Dispose();
                Object.DestroyImmediate(cameraObject);
                Object.DestroyImmediate(eventObject);
            }
        }

        [Test]
        public void CoordinatorMapsUnboundedMainCameraAndCleansUpWithFinalDocument()
        {
            var eventObject = new GameObject("Event System");
            var cameraObject = new GameObject("Main Camera") { tag = "MainCamera" };
            eventObject.AddComponent<EventSystem>();
            Camera camera = cameraObject.AddComponent<Camera>();
            var coordinator = new BattlementPanelInputCoordinator();
            try
            {
                Snapshot world = WorldSnapshot(new PanelInputConfigurationValue(), null);
                coordinator.Apply(world, camera);
                UnityPanelInputConfiguration value = coordinator.OwnedConfiguration!;
                Assert.That(value.maxInteractionDistance, Is.EqualTo(float.PositiveInfinity));
                Assert.That(value.defaultEventCameraIsMainCamera, Is.True);
                Assert.That(value.eventCameras, Is.Empty);

                coordinator.Apply(ScreenSnapshot(), camera);

                Assert.That(coordinator.OwnedConfiguration, Is.Null);
                Assert.That(eventObject.GetComponent<UnityPanelInputConfiguration>(), Is.Null);
            }
            finally
            {
                coordinator.Dispose();
                Object.DestroyImmediate(cameraObject);
                Object.DestroyImmediate(eventObject);
            }
        }

        [Test]
        public void AuthoredConfigurationIsRejectedBeforeOwnershipAndRemainsUntouched()
        {
            var eventObject = new GameObject("Event System");
            eventObject.AddComponent<EventSystem>();
            UnityPanelInputConfiguration authored =
                eventObject.AddComponent<UnityPanelInputConfiguration>();
            authored.maxInteractionDistance = 91;
            var coordinator = new BattlementPanelInputCoordinator();
            try
            {
                BattlementWorldException error = Assert.Throws<BattlementWorldException>(() =>
                    coordinator.ValidateBeforeReplacement(
                        WorldSnapshot(new PanelInputConfigurationValue(), null)
                    )
                )!;

                Assert.That(error.Message, Does.Contain("project-authored"));
                Assert.That(coordinator.OwnedConfiguration, Is.Null);
                Assert.That(authored.enabled, Is.True);
                Assert.That(authored.maxInteractionDistance, Is.EqualTo(91));
            }
            finally
            {
                coordinator.Dispose();
                Object.DestroyImmediate(eventObject);
            }
        }

        [Test]
        public void AuthoredPhysicsRaycasterRemainsUntouchedBesidePriorityWorldRaycaster()
        {
            var eventObject = new GameObject("Event System");
            var cameraObject = new GameObject("Input Camera");
            eventObject.AddComponent<EventSystem>();
            Camera camera = cameraObject.AddComponent<Camera>();
            PhysicsRaycaster authored = cameraObject.AddComponent<PhysicsRaycaster>();
            authored.eventMask = 0x1234;
            var coordinator = new BattlementPanelInputCoordinator();
            try
            {
                coordinator.Apply(
                    WorldSnapshot(new PanelInputConfigurationValue(), new ObjectId(Guid.NewGuid())),
                    camera
                );

                Assert.That(authored.enabled, Is.True);
                Assert.That(authored.eventMask.value, Is.EqualTo(0x1234));
                BattlementWorldDocumentRaycaster priority =
                    eventObject.GetComponent<BattlementWorldDocumentRaycaster>();
                Assert.That(priority, Is.Not.Null);
                Assert.That(priority.camera, Is.SameAs(camera));

                coordinator.Apply(ScreenSnapshot(), camera);

                Assert.That(authored.enabled, Is.True);
                Assert.That(authored.eventMask.value, Is.EqualTo(0x1234));
            }
            finally
            {
                coordinator.Dispose();
                Object.DestroyImmediate(cameraObject);
                Object.DestroyImmediate(eventObject);
            }
        }

        [Test]
        public void WorldInputRequiresActiveEventSystemAndUsableCamera()
        {
            var coordinator = new BattlementPanelInputCoordinator();
            Snapshot snapshot = WorldSnapshot(new PanelInputConfigurationValue(), null);
            try
            {
                Assert.That(
                    Assert
                        .Throws<BattlementWorldException>(() =>
                            coordinator.ValidateBeforeReplacement(snapshot)
                        )!
                        .Message,
                    Does.Contain("active EventSystem")
                );

                var eventObject = new GameObject("Event System");
                eventObject.AddComponent<EventSystem>();
                try
                {
                    Assert.That(
                        Assert
                            .Throws<BattlementWorldException>(() =>
                                coordinator.Apply(snapshot, null)
                            )!
                            .Message,
                        Does.Contain("camera")
                    );
                }
                finally
                {
                    Object.DestroyImmediate(eventObject);
                }
            }
            finally
            {
                coordinator.Dispose();
            }
        }

        private static Snapshot WorldSnapshot(
            PanelInputConfigurationValue configuration,
            ObjectId? cameraId
        )
        {
            Snapshot snapshot = new(
                new SessionId(Guid.NewGuid()),
                Array.Empty<PreparedAsset>(),
                Array.Empty<BattlementScene>(),
                new[]
                {
                    new BattlementGameObject(
                        new ObjectId(Guid.NewGuid()),
                        new GameObjectKind.UiDocumentState(
                            new ObjectId(Guid.NewGuid()),
                            new PanelSettingsValue(RenderMode: PanelRenderMode.WorldSpace)
                        )
                    ),
                }
            );
            return snapshot with
            {
                InputCameraId = cameraId,
                PanelInputConfiguration = configuration,
            };
        }

        private static Snapshot ScreenSnapshot() =>
            new(
                new SessionId(Guid.NewGuid()),
                Array.Empty<PreparedAsset>(),
                Array.Empty<BattlementScene>(),
                Array.Empty<BattlementGameObject>()
            );
    }
}
