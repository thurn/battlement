#nullable enable

using System;
using System.Linq;
using Masonry;
using UnityEngine;

namespace Masonry.BasicSample
{
    /// <summary>Bootstraps the standalone sample and renders its diagnostic status.</summary>
    [DisallowMultipleComponent]
    public sealed class BasicSample : MonoBehaviour
    {
        private static readonly Guid[] CubeIds =
        {
            Guid.Parse("00000000-0000-0000-0000-000000000064"),
            Guid.Parse("00000000-0000-0000-0000-000000000065"),
            Guid.Parse("00000000-0000-0000-0000-000000000066"),
        };

        private BasicSampleTransport transport = null!;
        private MasonryRunner runner = null!;
        private int observedSubmissions;
        private string lastAction = "none";
        private string lastCommand = "initial snapshot";

        /// <summary>Whether all three Rust-authored cubes are visible.</summary>
        public bool IsRunning => Cubes.All(cube => cube != null);

        /// <summary>Returns a cube by its stable sample index.</summary>
        public GameObject? Cube(int index) =>
            FindObjectsByType<MasonryIdentity>()
                .SingleOrDefault(identity => identity.Id == CubeIds[index])
                ?.gameObject;

        private GameObject?[] Cubes => Enumerable.Range(0, 3).Select(Cube).ToArray();

        private void Start()
        {
            transport = new BasicSampleTransport();
            runner = gameObject.AddComponent<MasonryRunner>();
            runner.Configure(
                new MasonryRunnerOptions(
                    transport,
                    new MasonryAddressablesAssetStorage(),
                    MasonryMessagePack.Instance
                )
            );
            runner.Connect();
        }

        private void Update()
        {
            if (transport.SubmissionCount != observedSubmissions)
            {
                observedSubmissions = transport.SubmissionCount;
                UpdateObservedProtocolState();
            }
            if (transport.LastResponseSource == "polled")
            {
                lastCommand = "cube C → blue";
            }
        }

        private void OnGUI()
        {
            float scale = Math.Max(1, Math.Min(Screen.width / 1280f, Screen.height / 720f));
            var title = new GUIStyle(GUI.skin.label)
            {
                fontSize = Mathf.RoundToInt(36 * scale),
                fontStyle = FontStyle.Bold,
                normal = { textColor = UnityEngine.Color.white },
            };
            var status = new GUIStyle(GUI.skin.label)
            {
                fontSize = Mathf.RoundToInt(22 * scale),
                normal = { textColor = new UnityEngine.Color(0.82f, 0.88f, 0.96f) },
            };
            GUI.Label(
                new Rect(28 * scale, 20 * scale, 900 * scale, 54 * scale),
                "Masonry — Basic Native Sample",
                title
            );
            DrawCubeLabels(status, scale);
            string connection = IsRunning ? "Running" : "Connecting";
            GUI.Label(
                new Rect(28 * scale, Screen.height - 138 * scale, 1120 * scale, 120 * scale),
                $"{connection}  •  native masonry_rules\n"
                    + $"last action: {lastAction}  •  last command: {lastCommand}  •  "
                    + $"response: {transport?.LastResponseSource ?? "none"}",
                status
            );
        }

        private void DrawCubeLabels(GUIStyle style, float scale)
        {
            style.fontStyle = FontStyle.Bold;
            style.alignment = TextAnchor.MiddleCenter;
            Camera? camera = Camera.allCameras.FirstOrDefault();
            if (camera == null)
            {
                return;
            }
            for (int index = 0; index < CubeIds.Length; index++)
            {
                GameObject? cube = Cube(index);
                if (cube == null)
                {
                    continue;
                }
                UnityEngine.Vector3 point = camera.WorldToScreenPoint(cube.transform.position);
                GUI.Label(
                    new Rect(
                        point.x - 40 * scale,
                        Screen.height - point.y - 96 * scale,
                        80 * scale,
                        40 * scale
                    ),
                    ((char)('A' + index)).ToString(),
                    style
                );
            }
            style.alignment = TextAnchor.UpperLeft;
        }

        private void UpdateObservedProtocolState()
        {
            GameObject? moved = Cubes.FirstOrDefault(cube =>
                cube != null && Math.Abs(cube.transform.localPosition.z) > 1.5f
            );
            if (moved != null)
            {
                lastAction = "pointer click";
                lastCommand = "500 ms move tween";
                return;
            }

            bool yellow = Cubes.Any(cube =>
                cube != null && cube.GetComponent<Renderer>().sharedMaterial.name.Contains("Yellow")
            );
            lastAction = yellow ? "pointer enter" : "pointer exit";
            lastCommand = yellow ? "target → yellow" : "target → gray";
        }
    }
}
