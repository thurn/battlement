#nullable enable

using System;
using System.Linq;
using UnityEngine;
using UnityEngine.EventSystems;
using Object = UnityEngine.Object;
using UnityPanelInputConfiguration = UnityEngine.UIElements.PanelInputConfiguration;

namespace Battlement
{
    internal sealed class BattlementPanelInputCoordinator : IDisposable
    {
        private UnityPanelInputConfiguration? ownedConfiguration;
        private BattlementWorldDocumentRaycaster? ownedWorldRaycaster;

        public UnityPanelInputConfiguration? OwnedConfiguration => ownedConfiguration;

        public void ValidateBeforeReplacement(Snapshot snapshot)
        {
            PanelInputConfigurationValue value =
                snapshot.PanelInputConfiguration ?? new PanelInputConfigurationValue();
            ValidateValue(value);
            if (!RequiresWorldInput(snapshot))
                return;
            RequireEventSystem();
            RejectAuthoredConfiguration();
        }

        public void Apply(Snapshot snapshot, Camera? inputCamera)
        {
            if (!RequiresWorldInput(snapshot))
            {
                Clear();
                return;
            }
            if (inputCamera == null)
            {
                throw Invalid(
                    "World-space UI input requires an enabled active main or explicit camera."
                );
            }
            EventSystem eventSystem = RequireEventSystem();
            RejectAuthoredConfiguration();
            if (ownedConfiguration == null)
            {
                ownedConfiguration =
                    eventSystem.gameObject.AddComponent<UnityPanelInputConfiguration>();
            }
            if (ownedWorldRaycaster == null)
            {
                ownedWorldRaycaster =
                    eventSystem.gameObject.AddComponent<BattlementWorldDocumentRaycaster>();
            }
            ownedWorldRaycaster.camera = snapshot.InputCameraId is null ? null : inputCamera;
            ApplyValue(
                ownedConfiguration,
                snapshot.PanelInputConfiguration ?? new PanelInputConfigurationValue(),
                snapshot.InputCameraId is null,
                inputCamera
            );
        }

        public void Clear()
        {
            if (ownedWorldRaycaster != null)
            {
                ownedWorldRaycaster.enabled = false;
                Destroy(ownedWorldRaycaster);
            }
            if (ownedConfiguration != null)
            {
                ownedConfiguration.enabled = false;
                Destroy(ownedConfiguration);
            }
            ownedWorldRaycaster = null;
            ownedConfiguration = null;
        }

        public void Dispose() => Clear();

        public static void ValidateValue(PanelInputConfigurationValue value)
        {
            if (!Enum.IsDefined(typeof(PanelInputRedirection), value.InputRedirection))
                throw Invalid("Unknown panel input redirection value.");
            InteractionDistance distance =
                value.MaximumInteractionDistance ?? new InteractionDistance.Unbounded();
            switch (distance)
            {
                case InteractionDistance.Unbounded:
                    break;
                case InteractionDistance.Inclusive finite
                    when float.IsFinite(finite.Value) && finite.Value >= 0:
                    break;
                default:
                    throw Invalid(
                        "Panel maximum interaction distance must be unbounded or finite "
                            + "and nonnegative."
                    );
            }
        }

        private static bool RequiresWorldInput(Snapshot snapshot) =>
            snapshot.Objects.Any(value =>
                value.Kind is GameObjectKind.UiDocumentState state
                && state.PanelSettings?.RenderMode == PanelRenderMode.WorldSpace
            );

        private static EventSystem RequireEventSystem()
        {
            EventSystem? eventSystem = Object.FindAnyObjectByType<EventSystem>(
                FindObjectsInactive.Exclude
            );
            if (eventSystem == null || !eventSystem.isActiveAndEnabled)
                throw Invalid("World-space UI input requires an active EventSystem.");
            return eventSystem;
        }

        private void RejectAuthoredConfiguration()
        {
            UnityPanelInputConfiguration? conflict = Object
                .FindObjectsByType<UnityPanelInputConfiguration>(FindObjectsInactive.Exclude)
                .FirstOrDefault(value =>
                    value.isActiveAndEnabled && !ReferenceEquals(value, ownedConfiguration)
                );
            if (conflict != null)
            {
                throw Invalid(
                    "An active project-authored PanelInputConfiguration conflicts with "
                        + "Battlement world-space UI input."
                );
            }
        }

        private static void ApplyValue(
            UnityPanelInputConfiguration target,
            PanelInputConfigurationValue value,
            bool usesMainCamera,
            Camera camera
        )
        {
            target.processWorldSpaceInput = true;
            target.interactionLayers = new LayerMask
            {
                value = unchecked((int)value.InteractionLayers.Value),
            };
            target.maxInteractionDistance = value.MaximumInteractionDistance switch
            {
                InteractionDistance.Inclusive finite => finite.Value,
                _ => float.PositiveInfinity,
            };
            target.defaultEventCameraIsMainCamera = usesMainCamera;
            target.eventCameras = usesMainCamera ? Array.Empty<Camera>() : new[] { camera };
            target.panelInputRedirection = value.InputRedirection switch
            {
                PanelInputRedirection.Never => UnityPanelInputConfiguration
                    .PanelInputRedirection
                    .Never,
                PanelInputRedirection.Always => UnityPanelInputConfiguration
                    .PanelInputRedirection
                    .Always,
                _ => UnityPanelInputConfiguration.PanelInputRedirection.AutoSwitch,
            };
            target.autoCreatePanelComponents = true;
        }

        private static BattlementWorldException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);

        private static void Destroy(Object value)
        {
            if (Application.isPlaying)
                Object.Destroy(value);
            else
                Object.DestroyImmediate(value);
        }
    }
}
