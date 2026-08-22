#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.InputSystem.LowLevel;

namespace Battlement.VisualCapture
{
    internal sealed class BattlementCaptureController : MonoBehaviour
    {
        private static BattlementCaptureController? instance;

        private readonly HashSet<Key> pressedKeys = new();
        private BattlementCaptureScenario scenario = null!;
        private BattlementFrameRecorder? recorder;
        private Keyboard? keyboard;
        private Mouse? mouse;
        private string acknowledgementDirectory = string.Empty;
        private string commandDirectory = string.Empty;
        private bool leftButtonDown;
        private int nextCommandId = 1;

        internal static void Attach(BattlementCaptureScenario selectedScenario)
        {
            if (instance != null)
            {
                throw new InvalidOperationException("Only one capture scenario may be active.");
            }

            var root = new GameObject("Battlement Capture Controller");
            DontDestroyOnLoad(root);
            instance = root.AddComponent<BattlementCaptureController>();
            instance.scenario = selectedScenario;
            instance.Initialize();
        }

        internal static void Dispatch(CaptureRequest request)
        {
            if (instance == null)
            {
                throw new InvalidOperationException("Capture input is unavailable.");
            }
            instance.DispatchInput(request);
        }

        internal static void RequireBalancedInput()
        {
            if (instance != null && (instance.leftButtonDown || instance.pressedKeys.Count > 0))
            {
                throw new InvalidOperationException(
                    "A capture scenario cannot pass with pointer buttons or keys held."
                );
            }
        }

        private void Initialize()
        {
            string? controlDirectory = CaptureArguments.Value("-battlementCaptureControl");
            if (controlDirectory == null)
            {
                throw new InvalidOperationException(
                    "The capture control directory was not supplied."
                );
            }
            commandDirectory = Path.Combine(controlDirectory, "commands");
            acknowledgementDirectory = Path.Combine(controlDirectory, "acks");
            Directory.CreateDirectory(commandDirectory);
            Directory.CreateDirectory(acknowledgementDirectory);

            Application.runInBackground = true;
            if (CaptureArguments.Value("-battlementCaptureInputDriver") != "in-player")
            {
                return;
            }

            InputSystem.settings.backgroundBehavior = InputSettings.BackgroundBehavior.IgnoreFocus;
            foreach (Mouse device in InputSystem.devices.OfType<Mouse>().ToArray())
            {
                InputSystem.DisableDevice(device);
            }
            foreach (Keyboard device in InputSystem.devices.OfType<Keyboard>().ToArray())
            {
                InputSystem.DisableDevice(device);
            }

            mouse = InputSystem.AddDevice<Mouse>("Battlement Capture Mouse");
            keyboard = InputSystem.AddDevice<Keyboard>("Battlement Capture Keyboard");
        }

        private void Update()
        {
            string path = Path.Combine(commandDirectory, $"{nextCommandId:D6}.json");
            if (!File.Exists(path))
            {
                return;
            }

            CaptureCommand? command = JsonUtility.FromJson<CaptureCommand>(File.ReadAllText(path));
            if (command == null || command.CommandId != nextCommandId)
            {
                Acknowledge(
                    nextCommandId,
                    new CaptureAcknowledgement(nextCommandId, false, "Invalid command.")
                );
                nextCommandId++;
                return;
            }

            try
            {
                Execute(command);
            }
            catch (Exception exception)
            {
                Acknowledge(
                    command.CommandId,
                    new CaptureAcknowledgement(command.CommandId, false, exception.Message)
                );
            }
            nextCommandId++;
        }

        private void Execute(CaptureCommand command)
        {
            switch (command.Kind)
            {
                case "dispatch-input":
                    scenario.DispatchRequest(command.RequestId);
                    Acknowledge(
                        command.CommandId,
                        new CaptureAcknowledgement(command.CommandId, true)
                    );
                    break;
                case "capture-png":
                    RequireRecorder()
                        .CapturePng(
                            command.OutputPath,
                            result =>
                                Acknowledge(
                                    command.CommandId,
                                    new CaptureAcknowledgement(
                                        command.CommandId,
                                        result.Success,
                                        result.Error,
                                        outputPath: command.OutputPath
                                    )
                                )
                        );
                    break;
                case "start-video":
                    RequireRecorder()
                        .StartVideo(
                            command,
                            result =>
                                Acknowledge(
                                    command.CommandId,
                                    new CaptureAcknowledgement(
                                        command.CommandId,
                                        result.Success,
                                        result.Error,
                                        result.EncoderPid,
                                        result.Frames,
                                        command.OutputPath
                                    )
                                )
                        );
                    break;
                default:
                    throw new InvalidOperationException($"Unknown capture command: {command.Kind}");
            }
        }

        private BattlementFrameRecorder RequireRecorder()
        {
            if (recorder == null)
            {
                recorder = gameObject.AddComponent<BattlementFrameRecorder>();
            }
            return recorder;
        }

        private void DispatchInput(CaptureRequest request)
        {
            if (mouse == null || keyboard == null)
            {
                throw new InvalidOperationException("In-player input was not selected.");
            }

            if (request.Device == "pointer")
            {
                leftButtonDown = request.Action switch
                {
                    "pointer-left-button-down" when !leftButtonDown => true,
                    "pointer-left-button-up" when leftButtonDown => false,
                    "pointer-move" => leftButtonDown,
                    _ => throw new InvalidOperationException(
                        $"Invalid pointer transition: {request.Action}"
                    ),
                };
                InputSystem.QueueStateEvent(
                    mouse,
                    new MouseState
                    {
                        position = new Vector2(
                            request.Position.x * Screen.width,
                            (1 - request.Position.y) * Screen.height
                        ),
                    }.WithButton(MouseButton.Left, leftButtonDown)
                );
                if (recorder != null)
                {
                    recorder.SetPointer(
                        new Vector2(
                            request.Position.x * Screen.width,
                            (1 - request.Position.y) * Screen.height
                        )
                    );
                }
                return;
            }

            if (!Enum.TryParse(request.Key, out Key key) || key == Key.None)
            {
                throw new InvalidOperationException($"Unsupported capture key: {request.Key}");
            }
            if (request.Action == "key-down" ? !pressedKeys.Add(key) : !pressedKeys.Remove(key))
            {
                throw new InvalidOperationException(
                    $"Invalid key transition: {request.Action} {key}"
                );
            }
            InputSystem.QueueStateEvent(keyboard, new KeyboardState(pressedKeys.ToArray()));
        }

        private void Acknowledge(int commandId, CaptureAcknowledgement acknowledgement) =>
            CaptureFiles.WriteJson(
                Path.Combine(acknowledgementDirectory, $"{commandId:D6}.json"),
                acknowledgement
            );

        private void OnDestroy()
        {
            if (recorder != null)
            {
                recorder.Stop();
            }
            if (mouse != null)
            {
                InputSystem.RemoveDevice(mouse);
            }
            if (keyboard != null)
            {
                InputSystem.RemoveDevice(keyboard);
            }
            instance = null;
        }
    }
}
