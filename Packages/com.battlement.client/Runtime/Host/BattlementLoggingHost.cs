#nullable enable

using System;
using UnityEngine;
using UnityEngine.InputSystem;

namespace Battlement
{
    internal sealed class BattlementLoggingHost : MonoBehaviour
    {
#if UNITY_EDITOR || BATTLEMENT_DITTO_DIAGNOSTICS
        private BattlementLogViewer? viewer;
#endif
        private BattlementFpsViewer? fpsViewer;

        public void Initialize()
        {
#if UNITY_EDITOR || BATTLEMENT_DITTO_DIAGNOSTICS
            viewer = new BattlementLogViewer(transform);
#endif
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
#if UNITY_EDITOR || BATTLEMENT_DITTO_DIAGNOSTICS
                    SetVisible(DebugUiSurface.LogViewer, viewer?.IsVisible != true);
#endif
                }
                if (keyboard.fKey.wasPressedThisFrame)
                {
                    SetVisible(DebugUiSurface.FpsViewer, fpsViewer?.IsVisible != true);
                }
            }

#if UNITY_EDITOR || BATTLEMENT_DITTO_DIAGNOSTICS
            viewer?.Update();
#endif
            fpsViewer?.Update();
        }

        private void OnDestroy()
        {
            Application.logMessageReceivedThreaded -= ReceiveUnityLog;
            BattlementDebugUi.Unregister(this);
#if UNITY_EDITOR || BATTLEMENT_DITTO_DIAGNOSTICS
            viewer?.Dispose();
#endif
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
#if UNITY_EDITOR || BATTLEMENT_DITTO_DIAGNOSTICS
                viewer?.RequestRefresh();
#endif
            }
        }

        internal void SetVisible(DebugUiSurface surface, bool visible)
        {
            switch (surface)
            {
                case DebugUiSurface.LogViewer:
#if UNITY_EDITOR || BATTLEMENT_DITTO_DIAGNOSTICS
                    viewer?.SetVisible(visible);
#endif
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
                DebugUiSurface.LogViewer => LogViewerVisible(),
                DebugUiSurface.FpsViewer => fpsViewer?.IsVisible == true,
                _ => throw new ArgumentOutOfRangeException(nameof(surface)),
            };

        private bool LogViewerVisible()
        {
#if UNITY_EDITOR || BATTLEMENT_DITTO_DIAGNOSTICS
            return viewer?.IsVisible == true;
#else
            return false;
#endif
        }

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
