#nullable enable

using System;
using System.Threading;
using UnityEngine;
using UnityEngine.InputSystem;

namespace Battlement
{
    internal sealed class BattlementLoggingHost : MonoBehaviour
    {
#if UNITY_EDITOR || DEVELOPMENT_BUILD || BATTLEMENT_DITTO_DIAGNOSTICS
        private BattlementLogViewer? viewer;
#endif
#if !UNITY_EDITOR && DEVELOPMENT_BUILD
        private BattlementDevelopmentConsole? developmentConsole;
#endif
        private BattlementFpsViewer? fpsViewer;

        public void Initialize()
        {
#if UNITY_EDITOR || DEVELOPMENT_BUILD || BATTLEMENT_DITTO_DIAGNOSTICS
            viewer = new BattlementLogViewer(transform);
#endif
#if !UNITY_EDITOR && DEVELOPMENT_BUILD
            developmentConsole = new BattlementDevelopmentConsole(() => viewer!.ShowErrors());
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
#if UNITY_EDITOR || DEVELOPMENT_BUILD || BATTLEMENT_DITTO_DIAGNOSTICS
                    ToggleLogViewer();
#endif
                }
                if (keyboard.fKey.wasPressedThisFrame)
                {
                    SetVisible(DebugUiSurface.FpsViewer, fpsViewer?.IsVisible != true);
                }
            }

#if !UNITY_EDITOR && DEVELOPMENT_BUILD
            developmentConsole?.Update();
#endif
#if UNITY_EDITOR || DEVELOPMENT_BUILD || BATTLEMENT_DITTO_DIAGNOSTICS
            viewer?.Update();
#endif
            fpsViewer?.Update();
        }

        public void ToggleLogViewer() =>
            SetVisible(DebugUiSurface.LogViewer, !IsVisible(DebugUiSurface.LogViewer));

        private void OnDestroy()
        {
            Application.logMessageReceivedThreaded -= ReceiveUnityLog;
            BattlementDebugUi.Unregister(this);
#if !UNITY_EDITOR && DEVELOPMENT_BUILD
            developmentConsole?.Dispose();
#endif
#if UNITY_EDITOR || DEVELOPMENT_BUILD || BATTLEMENT_DITTO_DIAGNOSTICS
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
#if UNITY_EDITOR || DEVELOPMENT_BUILD || BATTLEMENT_DITTO_DIAGNOSTICS
                viewer?.RequestRefresh();
#endif
            }
        }

        internal void SetVisible(DebugUiSurface surface, bool visible)
        {
            switch (surface)
            {
                case DebugUiSurface.LogViewer:
#if UNITY_EDITOR || DEVELOPMENT_BUILD || BATTLEMENT_DITTO_DIAGNOSTICS
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
#if UNITY_EDITOR || DEVELOPMENT_BUILD || BATTLEMENT_DITTO_DIAGNOSTICS
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

    internal sealed class BattlementDevelopmentConsole : IDisposable
    {
        private readonly System.Action showErrors;
        private readonly bool developerConsoleWasEnabled;
        private int showRequested;

        public BattlementDevelopmentConsole(System.Action showErrors)
        {
            this.showErrors = showErrors;
            developerConsoleWasEnabled = Debug.developerConsoleEnabled;
            Debug.developerConsoleEnabled = false;
            Application.logMessageReceivedThreaded += Receive;
        }

        public void Update()
        {
            if (Interlocked.Exchange(ref showRequested, 0) != 0)
            {
                showErrors();
            }
        }

        public void Dispose()
        {
            Application.logMessageReceivedThreaded -= Receive;
            Debug.developerConsoleEnabled = developerConsoleWasEnabled;
        }

        private void Receive(string condition, string stackTrace, LogType type)
        {
            if (type is LogType.Error or LogType.Assert or LogType.Exception)
            {
                Interlocked.Exchange(ref showRequested, 1);
            }
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
