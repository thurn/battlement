#nullable enable

using System;
using System.Collections;
using System.IO;
using System.Linq;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.InputSystem.LowLevel;

namespace Masonry.BasicSample
{
    /// <summary>Drives the deterministic packaged-player sample walkthrough.</summary>
    public sealed class BasicSampleCaptureScenario : MonoBehaviour
    {
        private const string ScenarioName = "masonry-basic-sample";
        private BasicSample sample = null!;
        private string statusPath = string.Empty;
        private int requestId;
        private Stage stage;
        private Vector2 cubeA;
        private Vector2 cubeB;
        private Vector2 requestedPointer;
        private string requestedInput = string.Empty;
        private float startedAt;

        private void Start()
        {
            string? selected = Argument("-masonryCaptureScenario");
            Debug.Log($"MASONRY_BASIC_CAPTURE_SELECTED:{selected ?? "none"}");
            if (selected != ScenarioName)
            {
                enabled = false;
                return;
            }
            statusPath = Argument("-masonryCaptureStatus") ?? string.Empty;
            sample = FindAnyObjectByType<BasicSample>();
            BasicSampleCaptureController.Attach(this);
            startedAt = Time.realtimeSinceStartup;
            Publish("starting", null, "capture-driver-started");
            StartCoroutine(WaitForRunning());
        }

        private IEnumerator WaitForRunning()
        {
            float nextDiagnostic = Time.realtimeSinceStartup + 1;
            while (!sample.IsRunning)
            {
                if (Time.realtimeSinceStartup >= nextDiagnostic)
                {
                    nextDiagnostic += 1;
                    string ids = string.Join(
                        ",",
                        FindObjectsByType<MasonryIdentity>().Select(identity => identity.Id)
                    );
                    Debug.Log($"MASONRY_BASIC_CAPTURE_WAITING:{ids}");
                }
                yield return null;
            }
            Debug.Log("MASONRY_BASIC_CAPTURE_RUNNING");
            yield return new WaitForSeconds(0.4f);
            cubeA = ScreenPosition(sample.Cube(0)!);
            cubeB = ScreenPosition(sample.Cube(1)!);
            stage = Stage.MoveToA;
            Request("pointer-move", cubeA, "running");
        }

        private void Update()
        {
            if (!enabled || stage == Stage.Starting || stage == Stage.Passed)
            {
                return;
            }
            if (Time.realtimeSinceStartup - startedAt > 20)
            {
                Publish("failed", "Timed out while driving the basic sample.");
                enabled = false;
                return;
            }

            GameObject cube = sample.Cube(0)!;
            switch (stage)
            {
                case Stage.MoveToA when IsYellow(cube) && IsBlue(sample.Cube(2)!):
                    stage = Stage.PressA;
                    Request(
                        "pointer-left-button-down",
                        cubeA,
                        "hover-yellow",
                        "polled-change-visible"
                    );
                    break;
                case Stage.PressA when Mouse.current.leftButton.wasPressedThisFrame:
                    stage = Stage.ReleaseA;
                    Request("pointer-left-button-up", cubeA, "hover-yellow", "pointer-pressed");
                    break;
                case Stage.ReleaseA when Mouse.current.leftButton.wasReleasedThisFrame:
                    stage = Stage.MoveToB;
                    Request("pointer-move", cubeB, "pointer-clicked", "tween-started");
                    break;
                case Stage.MoveToB
                    when cube.transform.localPosition.z > 1.95f
                        && IsGray(cube)
                        && IsYellow(sample.Cube(1)!)
                        && IsBlue(sample.Cube(2)!):
                    stage = Stage.Passed;
                    Publish(
                        "passed",
                        null,
                        "hover-enter-exit",
                        "click-move-tween",
                        "polled-blue-change",
                        "native-plugin-packaged"
                    );
                    break;
            }
        }

        private void Request(string input, Vector2 pointer, params string[] assertions)
        {
            requestId++;
            requestedInput = input;
            requestedPointer = pointer;
            Write(
                new CaptureStatus
                {
                    phase = "ready",
                    scenario = ScenarioName,
                    assertions = assertions,
                    requestId = requestId,
                    inputDevice = "pointer",
                    input = input,
                    pointerX = pointer.x,
                    pointerY = pointer.y,
                }
            );
        }

        internal void DispatchRequest(int dispatchedRequestId)
        {
            if (dispatchedRequestId != requestId)
            {
                throw new InvalidOperationException(
                    $"Expected request {requestId}, received {dispatchedRequestId}."
                );
            }
            BasicSampleCaptureController.Dispatch(requestedInput, requestedPointer);
        }

        private void Publish(string phase, string? failure, params string[] assertions) =>
            Write(
                new CaptureStatus
                {
                    phase = phase,
                    scenario = ScenarioName,
                    assertions = assertions,
                    requestId = requestId,
                    failure = failure,
                    pointerX = -1,
                    pointerY = -1,
                }
            );

        private void Write(CaptureStatus status)
        {
            if (string.IsNullOrEmpty(statusPath))
            {
                throw new InvalidOperationException("Capture status path was not supplied.");
            }
            string temporary = statusPath + ".new";
            File.WriteAllText(temporary, JsonUtility.ToJson(status, true));
            File.Delete(statusPath);
            File.Move(temporary, statusPath);
        }

        private static Vector2 ScreenPosition(GameObject target)
        {
            Camera camera = Camera.allCameras.Single();
            UnityEngine.Vector3 point = camera.WorldToScreenPoint(target.transform.position);
            return new Vector2(point.x / Screen.width, 1 - (point.y / Screen.height));
        }

        private static bool IsYellow(GameObject target) =>
            target.GetComponent<Renderer>().sharedMaterial.name.Contains("Yellow");

        private static bool IsGray(GameObject target) =>
            target.GetComponent<Renderer>().sharedMaterial.name.Contains("Gray");

        private static bool IsBlue(GameObject target) =>
            target.GetComponent<Renderer>().sharedMaterial.name.Contains("Blue");

        internal static string? Argument(string name)
        {
            string[] arguments = Environment.GetCommandLineArgs();
            int index = Array.IndexOf(arguments, name);
            return index >= 0 && index + 1 < arguments.Length ? arguments[index + 1] : null;
        }

        [Serializable]
        private sealed class CaptureStatus
        {
            public string phase = string.Empty;
            public string scenario = string.Empty;
            public string[] assertions = Array.Empty<string>();
            public int requestId;
            public string inputDevice = string.Empty;
            public string input = string.Empty;
            public float pointerX;
            public float pointerY;
            public string? failure;
        }

        private enum Stage
        {
            Starting,
            MoveToA,
            PressA,
            ReleaseA,
            MoveToB,
            Passed,
        }
    }

    internal sealed class BasicSampleCaptureController : MonoBehaviour
    {
        private static BasicSampleCaptureController? instance;

        private BasicSampleCaptureScenario scenario = null!;
        private Mouse? mouse;
        private string acknowledgementDirectory = string.Empty;
        private string commandDirectory = string.Empty;
        private bool leftButtonDown;
        private int nextCommandId = 1;

        internal static void Attach(BasicSampleCaptureScenario selectedScenario)
        {
            var root = new GameObject("Basic Sample Capture Controller");
            DontDestroyOnLoad(root);
            instance = root.AddComponent<BasicSampleCaptureController>();
            instance.scenario = selectedScenario;
            instance.Initialize();
        }

        internal static void Dispatch(string action, Vector2 pointer)
        {
            if (instance?.mouse == null)
            {
                throw new InvalidOperationException("In-player capture input is unavailable.");
            }

            instance.leftButtonDown = action switch
            {
                "pointer-left-button-down" when !instance.leftButtonDown => true,
                "pointer-left-button-up" when instance.leftButtonDown => false,
                "pointer-move" => instance.leftButtonDown,
                _ => throw new InvalidOperationException($"Invalid pointer transition: {action}"),
            };
            InputSystem.QueueStateEvent(
                instance.mouse,
                new MouseState
                {
                    position = new Vector2(
                        pointer.x * Screen.width,
                        (1 - pointer.y) * Screen.height
                    ),
                }.WithButton(MouseButton.Left, instance.leftButtonDown)
            );
        }

        private void Initialize()
        {
            string controlDirectory =
                BasicSampleCaptureScenario.Argument("-masonryCaptureControl")
                ?? throw new InvalidOperationException(
                    "Capture control directory was not supplied."
                );
            commandDirectory = Path.Combine(controlDirectory, "commands");
            acknowledgementDirectory = Path.Combine(controlDirectory, "acks");
            Directory.CreateDirectory(commandDirectory);
            Directory.CreateDirectory(acknowledgementDirectory);

            Application.runInBackground = true;
            if (BasicSampleCaptureScenario.Argument("-masonryCaptureInputDriver") != "in-player")
            {
                return;
            }
            InputSystem.settings.backgroundBehavior = InputSettings.BackgroundBehavior.IgnoreFocus;
            foreach (Mouse device in InputSystem.devices.OfType<Mouse>().ToArray())
            {
                InputSystem.DisableDevice(device);
            }
            mouse = InputSystem.AddDevice<Mouse>("Basic Sample Capture Mouse");
        }

        private void Update()
        {
            string path = Path.Combine(commandDirectory, $"{nextCommandId:D6}.json");
            if (!File.Exists(path))
            {
                return;
            }

            CaptureCommand? command = JsonUtility.FromJson<CaptureCommand>(File.ReadAllText(path));
            if (command == null || command.commandId != nextCommandId)
            {
                Acknowledge(nextCommandId, false, "Invalid command.");
                nextCommandId++;
                return;
            }

            try
            {
                if (command.kind != "dispatch-input")
                {
                    throw new InvalidOperationException($"Unknown capture command: {command.kind}");
                }
                scenario.DispatchRequest(command.requestId);
                Acknowledge(command.commandId, true, string.Empty);
            }
            catch (Exception exception)
            {
                Acknowledge(command.commandId, false, exception.Message);
            }
            nextCommandId++;
        }

        private void Acknowledge(int commandId, bool success, string error)
        {
            string path = Path.Combine(acknowledgementDirectory, $"{commandId:D6}.json");
            string temporary = path + ".new";
            File.WriteAllText(
                temporary,
                JsonUtility.ToJson(
                    new CaptureAcknowledgement
                    {
                        commandId = commandId,
                        success = success,
                        error = error,
                    },
                    true
                )
            );
            File.Move(temporary, path);
        }

        private void OnDestroy()
        {
            if (mouse != null)
            {
                InputSystem.RemoveDevice(mouse);
            }
            instance = null;
        }

        [Serializable]
        private sealed class CaptureCommand
        {
            public int commandId;
            public string kind = string.Empty;
            public int requestId;
        }

        [Serializable]
        private sealed class CaptureAcknowledgement
        {
            public int commandId;
            public bool success;
            public string error = string.Empty;
        }
    }
}
