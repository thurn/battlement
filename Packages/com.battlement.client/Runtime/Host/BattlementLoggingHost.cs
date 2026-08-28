#nullable enable

using System;
using UnityEngine;
using UnityEngine.InputSystem;

namespace Battlement
{
    internal sealed class BattlementLoggingHost : MonoBehaviour
    {
        private BattlementLogViewer? viewer;
        private BattlementFpsViewer? fpsViewer;

        public void Initialize()
        {
            viewer = new BattlementLogViewer(transform);
            fpsViewer = new BattlementFpsViewer(transform);
            BattlementDebugUi.Register(this);
            Application.logMessageReceivedThreaded += ReceiveUnityLog;
        }

        private void Update()
        {
            BattlementNativeLogging.Drain();
            Keyboard? keyboard = Keyboard.current;
            if (keyboard != null && ShortcutModifiersPressed(keyboard))
            {
                if (keyboard.lKey.wasPressedThisFrame)
                {
                    SetVisible(DebugUiSurface.LogViewer, viewer?.IsVisible != true);
                }
                if (keyboard.fKey.wasPressedThisFrame)
                {
                    SetVisible(DebugUiSurface.FpsViewer, fpsViewer?.IsVisible != true);
                }
            }

            viewer?.Update();
            fpsViewer?.Update();
        }

        private void OnDestroy()
        {
            Application.logMessageReceivedThreaded -= ReceiveUnityLog;
            BattlementDebugUi.Unregister(this);
            viewer?.Dispose();
            fpsViewer?.Dispose();
        }

        private void ReceiveUnityLog(string condition, string stackTrace, LogType type)
        {
            if (BattlementUnityLogging.IsForwarded(condition))
            {
                return;
            }

            BattlementLogSeverity severity = type switch
            {
                LogType.Warning => BattlementLogSeverity.Warning,
                LogType.Error or LogType.Assert or LogType.Exception => BattlementLogSeverity.Error,
                _ => BattlementLogSeverity.Information,
            };
            BattlementLogStore.Add(
                "unity",
                new BattlementLogRecord(
                    severity,
                    type switch
                    {
                        LogType.Exception => "unity.exception",
                        LogType.Assert => "unity.assert",
                        _ => "unity.log",
                    },
                    condition,
                    StackTrace: stackTrace
                )
            );
            if (severity == BattlementLogSeverity.Error)
            {
                viewer?.RequestRefresh();
            }
        }

        internal void SetVisible(DebugUiSurface surface, bool visible)
        {
            switch (surface)
            {
                case DebugUiSurface.LogViewer:
                    viewer?.SetVisible(visible);
                    break;
                case DebugUiSurface.FpsViewer:
                    fpsViewer?.SetVisible(visible);
                    break;
                default:
                    throw new ArgumentOutOfRangeException(nameof(surface));
            }
        }

        internal bool IsVisible(DebugUiSurface surface) =>
            surface switch
            {
                DebugUiSurface.LogViewer => viewer?.IsVisible == true,
                DebugUiSurface.FpsViewer => fpsViewer?.IsVisible == true,
                _ => throw new ArgumentOutOfRangeException(nameof(surface)),
            };

        private static bool ShortcutModifiersPressed(Keyboard keyboard)
        {
            if (!keyboard.shiftKey.isPressed)
            {
                return false;
            }

            bool command = keyboard.leftMetaKey.isPressed || keyboard.rightMetaKey.isPressed;
            bool control = keyboard.leftCtrlKey.isPressed || keyboard.rightCtrlKey.isPressed;
            return command || control;
        }
    }

    internal static class BattlementDebugUi
    {
        private static BattlementLoggingHost? host;

        public static void Register(BattlementLoggingHost value) => host = value;

        public static void Unregister(BattlementLoggingHost value)
        {
            if (ReferenceEquals(host, value))
            {
                host = null;
            }
        }

        public static void SetVisible(CommandBody.DebugUi command)
        {
            if (host != null)
            {
                host.SetVisible(command.Surface, command.Visible);
            }
        }
    }

    internal static class BattlementLoggingBootstrap
    {
        [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.BeforeSplashScreen)]
        private static void Initialize()
        {
            var host = new GameObject("Battlement Logging");
            UnityEngine.Object.DontDestroyOnLoad(host);
            host.AddComponent<BattlementLoggingHost>().Initialize();
        }
    }
}
