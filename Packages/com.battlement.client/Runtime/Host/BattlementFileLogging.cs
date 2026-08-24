#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using Newtonsoft.Json;
using UnityEngine;
using UnityEngine.InputSystem;

namespace Battlement
{
    /// <summary>Writes structured Battlement records to the native JSON Lines stream.</summary>
    public sealed class BattlementFileLogger : IBattlementLogger
    {
        public void Log(BattlementLogRecord record)
        {
            Preconditions.CheckNotNull(record, nameof(record));
            BattlementFileLogging.Write(
                record.Severity,
                record.EventName,
                record.Message,
                record.Fields,
                record.Exception?.ToString(),
                record.StackTrace
            );
        }
    }

    internal static class BattlementFileLogging
    {
        private const int Ok = 0;
        private const int MaximumReadBytes = 4 * 1024 * 1024;
        private static readonly object Gate = new();
        private static readonly UTF8Encoding Utf8 = new(false, true);
        private static bool attempted;
        private static bool active;
        private static string? failure;

        public static bool IsActive
        {
            get
            {
                lock (Gate)
                {
                    return active;
                }
            }
        }

        public static string? Failure
        {
            get
            {
                lock (Gate)
                {
                    return failure;
                }
            }
        }

        public static string LogPath =>
            Path.Combine(Application.persistentDataPath, "Battlement", "Logs", "battlement.jsonl");

        public static void Initialize()
        {
            lock (Gate)
            {
                if (attempted && active)
                {
                    return;
                }

                attempted = true;
                try
                {
                    string directory = Path.GetDirectoryName(LogPath)!;
                    byte[] path = Utf8.GetBytes(directory);
                    Invoke(
                        BattlementNativeMethods.battlement_log_initialize(
                            path,
                            checked((ulong)path.LongLength),
                            out BattlementNativeBuffer error
                        ),
                        error,
                        "initialize"
                    );
                    active = true;
                    failure = null;
                }
                catch (Exception exception)
                {
                    Disable(exception);
                }
            }
        }

        public static void Write(
            BattlementLogSeverity severity,
            string eventName,
            string message,
            IReadOnlyDictionary<string, string>? fields = null,
            string? exception = null,
            string? stackTrace = null
        )
        {
            lock (Gate)
            {
                if (!active)
                {
                    return;
                }

                try
                {
                    byte[] record = Utf8.GetBytes(
                        JsonConvert.SerializeObject(
                            new
                            {
                                severity = SeverityName(severity),
                                event_name = eventName,
                                message,
                                fields = fields ?? new Dictionary<string, string>(),
                                exception,
                                stack_trace = stackTrace,
                            }
                        )
                    );
                    Invoke(
                        BattlementNativeMethods.battlement_log_write(
                            record,
                            checked((ulong)record.LongLength),
                            out BattlementNativeBuffer error
                        ),
                        error,
                        "write"
                    );
                }
                catch (Exception writeFailure)
                {
                    Disable(writeFailure);
                }
            }
        }

        public static string Read(ref ulong offset)
        {
            lock (Gate)
            {
                if (!active)
                {
                    return string.Empty;
                }

                BattlementNativeBuffer output = default;
                try
                {
                    int status = BattlementNativeMethods.battlement_log_read(
                        offset,
                        MaximumReadBytes,
                        out output,
                        out ulong nextOffset
                    );
                    string text = Decode(output);
                    if (status != Ok)
                    {
                        throw new IOException($"Native log read failed: {text}");
                    }

                    offset = nextOffset;
                    return text;
                }
                catch (Exception readFailure)
                {
                    Disable(readFailure);
                    return string.Empty;
                }
                finally
                {
                    Free(output);
                }
            }
        }

        public static void Sync()
        {
            lock (Gate)
            {
                if (!active)
                {
                    return;
                }

                try
                {
                    Invoke(
                        BattlementNativeMethods.battlement_log_sync(
                            out BattlementNativeBuffer error
                        ),
                        error,
                        "sync"
                    );
#if UNITY_WEBGL && !UNITY_EDITOR
                    BattlementWebLogSync();
#endif
                }
                catch (Exception syncFailure)
                {
                    Disable(syncFailure);
                }
            }
        }

        public static void Close()
        {
            lock (Gate)
            {
                if (!active)
                {
                    return;
                }

                try
                {
                    Invoke(
                        BattlementNativeMethods.battlement_log_close(
                            out BattlementNativeBuffer error
                        ),
                        error,
                        "close"
                    );
                }
                catch (Exception closeFailure)
                {
                    WriteFallback(closeFailure);
                }
                finally
                {
                    active = false;
                }
            }
        }

        private static void Invoke(int status, BattlementNativeBuffer output, string operation)
        {
            try
            {
                if (status != Ok)
                {
                    throw new IOException($"Native log {operation} failed: {Decode(output)}");
                }
            }
            finally
            {
                Free(output);
            }
        }

        private static string Decode(BattlementNativeBuffer buffer)
        {
            if (buffer.Data == IntPtr.Zero || buffer.Length == 0)
            {
                return string.Empty;
            }
            if (buffer.Length > int.MaxValue)
            {
                throw new IOException("Native log output exceeds the managed buffer limit.");
            }

            byte[] bytes = new byte[checked((int)buffer.Length)];
            Marshal.Copy(buffer.Data, bytes, 0, bytes.Length);
            return Utf8.GetString(bytes);
        }

        private static void Free(BattlementNativeBuffer buffer)
        {
            if (buffer.Data != IntPtr.Zero && buffer.Length != 0)
            {
                BattlementNativeMethods.battlement_buffer_free(buffer);
            }
        }

        private static string SeverityName(BattlementLogSeverity severity) =>
            severity switch
            {
                BattlementLogSeverity.Trace => "trace",
                BattlementLogSeverity.Information => "information",
                BattlementLogSeverity.Warning => "warning",
                BattlementLogSeverity.Error => "error",
                _ => throw new ArgumentOutOfRangeException(nameof(severity)),
            };

        private static void Disable(Exception exception)
        {
            active = false;
            failure = exception.Message;
            WriteFallback(exception);
        }

        private static void WriteFallback(Exception exception)
        {
            try
            {
                string directory = Path.Combine(
                    Application.persistentDataPath,
                    "Battlement",
                    "Errors"
                );
                Directory.CreateDirectory(directory);
                File.AppendAllText(
                    Path.Combine(directory, "file-logging-initialization.txt"),
                    $"{DateTimeOffset.UtcNow:O} {exception}\n"
                );
            }
            catch
            {
                Console.Error.WriteLine($"Battlement file logging failed: {exception}");
            }
        }

#if UNITY_WEBGL && !UNITY_EDITOR
        [DllImport("__Internal")]
        private static extern void BattlementWebLogSync();
#endif
    }

    internal sealed class BattlementLoggingHost : MonoBehaviour
    {
        private const float SyncIntervalSeconds = 5f;
        private BattlementLogViewer? viewer;
        private float nextSync;
        private bool quitting;

        public void Initialize()
        {
            viewer = new BattlementLogViewer(transform);
            Application.logMessageReceivedThreaded += ReceiveUnityLog;
            AppDomain.CurrentDomain.UnhandledException += ReceiveUnhandledException;
            nextSync = Time.realtimeSinceStartup + SyncIntervalSeconds;
        }

        private void Update()
        {
            Keyboard? keyboard = Keyboard.current;
            bool modifier =
                keyboard != null
                && (
                    keyboard.leftMetaKey.isPressed
                    || keyboard.rightMetaKey.isPressed
                    || keyboard.leftCtrlKey.isPressed
                    || keyboard.rightCtrlKey.isPressed
                );
            bool shortcut =
                modifier && keyboard!.shiftKey.isPressed && keyboard.lKey.wasPressedThisFrame;
            if (shortcut)
            {
                viewer?.Toggle();
            }

            viewer?.Update();
            if (Time.realtimeSinceStartup >= nextSync)
            {
                BattlementFileLogging.Sync();
                nextSync = Time.realtimeSinceStartup + SyncIntervalSeconds;
            }
        }

        private void OnApplicationPause(bool paused)
        {
            if (paused)
            {
                BattlementFileLogging.Sync();
            }
        }

        private void OnApplicationFocus(bool focused)
        {
            if (!focused)
            {
                BattlementFileLogging.Sync();
            }
        }

        private void OnApplicationQuit()
        {
            quitting = true;
            BattlementFileLogging.Sync();
            BattlementFileLogging.Close();
        }

        private void OnDestroy()
        {
            Application.logMessageReceivedThreaded -= ReceiveUnityLog;
            AppDomain.CurrentDomain.UnhandledException -= ReceiveUnhandledException;
            viewer?.Dispose();
            if (!quitting)
            {
                BattlementFileLogging.Sync();
            }
        }

        private void ReceiveUnityLog(string condition, string stackTrace, LogType type)
        {
            BattlementLogSeverity severity = type switch
            {
                LogType.Warning => BattlementLogSeverity.Warning,
                LogType.Error or LogType.Assert or LogType.Exception => BattlementLogSeverity.Error,
                _ => BattlementLogSeverity.Information,
            };
            BattlementFileLogging.Write(
                severity,
                type switch
                {
                    LogType.Exception => "unity.exception",
                    LogType.Assert => "unity.assert",
                    _ => "unity.log",
                },
                condition,
                stackTrace: stackTrace
            );
            if (severity == BattlementLogSeverity.Error)
            {
                BattlementFileLogging.Sync();
                viewer?.RequestRefresh();
            }
        }

        private void ReceiveUnhandledException(object sender, UnhandledExceptionEventArgs args)
        {
            Exception? exception = args.ExceptionObject as Exception;
            BattlementFileLogging.Write(
                BattlementLogSeverity.Error,
                "unity.unhandled_exception",
                exception?.Message ?? "Unhandled managed exception",
                exception: exception?.ToString(),
                stackTrace: exception?.StackTrace
            );
            BattlementFileLogging.Sync();
            viewer?.RequestRefresh();
        }
    }

    internal static class BattlementLoggingBootstrap
    {
        [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.BeforeSplashScreen)]
        private static void Initialize()
        {
            BattlementFileLogging.Initialize();
            var host = new GameObject("Battlement Logging");
            UnityEngine.Object.DontDestroyOnLoad(host);
            host.AddComponent<BattlementLoggingHost>().Initialize();
        }
    }
}
