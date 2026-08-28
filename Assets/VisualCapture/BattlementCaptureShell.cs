#nullable enable

using System;
using UnityEngine;
using UnityColor = UnityEngine.Color;
using UnityLightType = UnityEngine.LightType;

namespace Battlement.VisualCapture
{
    /// <summary>Reusable camera, lighting, color, and labeling for capture scenes.</summary>
    public sealed class BattlementCaptureShell : MonoBehaviour
    {
        [SerializeField]
        private Camera captureCamera = null!;

        [SerializeField]
        private Light keyLight = null!;

        [SerializeField]
        private Material primaryMaterial = null!;

        [SerializeField]
        private Material accentMaterial = null!;

        [SerializeField]
        private Material successMaterial = null!;

        [SerializeField]
        private string title = "Battlement visual evidence";

        [SerializeField]
        private string phase = "Ready";

        [SerializeField]
        private string[] legend = Array.Empty<string>();

        [SerializeField]
        private bool persistentBootstrap;

        /// <summary>Updates the concise phase label shown in capture media.</summary>
        public void SetPhase(string value) => phase = value;

        /// <summary>Updates the title shown in capture media.</summary>
        public void SetTitle(string value) => title = value;

        /// <summary>Updates the explanatory legend shown in capture media.</summary>
        public void SetLegend(params string[] values) => legend = values;

        /// <summary>Gets the authored primary fixture material.</summary>
        public Material PrimaryMaterial => primaryMaterial;

        /// <summary>Gets the authored accent fixture material.</summary>
        public Material AccentMaterial => accentMaterial;

        private void Awake()
        {
            if (
                !captureCamera
                || !keyLight
                || !primaryMaterial
                || !accentMaterial
                || !successMaterial
            )
            {
                throw new InvalidOperationException(
                    "BattlementCaptureShell is missing an authored camera, light, or material."
                );
            }

            captureCamera.clearFlags = CameraClearFlags.SolidColor;
            captureCamera.backgroundColor = new UnityColor(0.035f, 0.047f, 0.075f);
            captureCamera.fieldOfView = 45;
            keyLight.type = UnityLightType.Directional;
            keyLight.intensity = 1.25f;
            if (persistentBootstrap)
            {
                DontDestroyOnLoad(gameObject);
            }
        }

        private void OnGUI()
        {
            GUIStyle titleStyle = new(GUI.skin.label)
            {
                fontSize = 28,
                fontStyle = FontStyle.Bold,
                normal = { textColor = UnityColor.white },
            };
            GUIStyle phaseStyle = new(GUI.skin.label)
            {
                fontSize = 18,
                normal = { textColor = new UnityColor(0.55f, 0.82f, 1) },
            };
            GUI.Label(new UnityEngine.Rect(32, 24, Screen.width - 64, 40), title, titleStyle);
            GUI.Label(new UnityEngine.Rect(34, 64, Screen.width - 68, 30), phase, phaseStyle);
            for (int index = 0; index < legend.Length; index++)
            {
                GUI.Label(
                    new UnityEngine.Rect(34, 102 + (index * 24), Screen.width - 68, 24),
                    $"• {legend[index]}"
                );
            }
        }
    }
}
