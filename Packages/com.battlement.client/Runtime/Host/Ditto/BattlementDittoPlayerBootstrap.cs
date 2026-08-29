#nullable enable

#if BATTLEMENT_DITTO_DIAGNOSTICS
using System;
using System.Collections;
using Newtonsoft.Json.Linq;
using UnityEngine;
using UnityEngine.Networking;

namespace Battlement
{
    /// <summary>
    /// Activates the build-only Ditto player runtime when a session route exists.
    /// </summary>
    public sealed class BattlementDittoPlayerBootstrap : MonoBehaviour
    {
        private const string SessionArgument = "--battlement-ditto-url";

        private static BattlementDittoPlayerBootstrap? instance;

        private string sessionUrl = string.Empty;

        private string playerSessionId = string.Empty;

        private bool polling;

        internal static DittoJob? CurrentJob { get; private set; }

        /// <summary>Whether this process was launched for a Ditto session.</summary>
        public static bool IsActive => instance is not null;

        internal static string SessionUrl =>
            instance?.sessionUrl
            ?? throw new InvalidOperationException("The Ditto player route is unavailable.");

        internal static string PlayerSessionId =>
            instance?.playerSessionId.Length > 0
                ? instance.playerSessionId
                : throw new InvalidOperationException("The Ditto player session is unavailable.");

        internal static BattlementLogObserver? BootstrapLogs { get; private set; }

        internal static event Action<DittoJob>? JobAvailable;

        [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.BeforeSplashScreen)]
        private static void Initialize()
        {
            bool found = TrySessionUrl(Environment.GetCommandLineArgs(), out string sessionUrl);
#if UNITY_WEBGL && !UNITY_EDITOR
            found =
                found || DittoWebSessionRoute.TryResolve(Application.absoluteURL, out sessionUrl);
#endif
            if (!found)
            {
                return;
            }
            BootstrapLogs = BattlementLogStore.Observe();
            var host = new GameObject("Battlement Ditto");
            DontDestroyOnLoad(host);
            instance = host.AddComponent<BattlementDittoPlayerBootstrap>();
            instance.sessionUrl = sessionUrl;
            host.AddComponent<BattlementDittoPlayerCoordinator>();
#if UNITY_WEBGL && !UNITY_EDITOR
            new DittoWebBrowserBridge().Install(host.name);
#endif
            instance.StartCoroutine(instance.Fetch());
        }

        internal static void WaitForNextJob()
        {
            if (instance is null || CurrentJob is null || instance.polling)
            {
                return;
            }
            instance.StartCoroutine(instance.Poll(CurrentJob.JobId));
        }

        internal static bool TrySessionUrl(string[] arguments, out string sessionUrl)
        {
            sessionUrl = string.Empty;
            for (var index = 0; index < arguments.Length; index++)
            {
                if (arguments[index] != SessionArgument)
                {
                    continue;
                }
                if (index + 1 >= arguments.Length)
                {
                    return false;
                }
                sessionUrl = arguments[index + 1].TrimEnd('/');
                return Uri.TryCreate(sessionUrl, UriKind.Absolute, out _);
            }
            return false;
        }

        public void ReportBrowserFailure(string json)
        {
            try
            {
                JObject value = JObject.Parse(json);
                string kind = value.Value<string>("kind") switch
                {
                    "console" => "console",
                    "exception" => "exception",
                    "promise" => "promise",
                    _ => "failure",
                };
                string message = value.Value<string>("message") ?? "Unknown browser failure.";
                BattlementUnityLogging.Log(
                    "ditto-player",
                    new BattlementLogRecord(
                        BattlementLogSeverity.Error,
                        $"ditto.browser-{kind}",
                        message.Substring(0, Math.Min(message.Length, 4096))
                    )
                );
            }
            catch (Exception exception)
            {
                LogFailure("ditto.browser-bridge-invalid", exception.Message);
            }
        }

        private IEnumerator Fetch()
        {
            using UnityWebRequest request = UnityWebRequest.Get(sessionUrl + "/job");
            request.downloadHandler = new DownloadHandlerBuffer();
            yield return request.SendWebRequest();
            if (request.result != UnityWebRequest.Result.Success)
            {
                BattlementUnityLogging.Log(
                    "ditto-player",
                    new BattlementLogRecord(
                        BattlementLogSeverity.Error,
                        "ditto.job-fetch-failed",
                        request.error ?? "The Ditto job request failed."
                    )
                );
                yield break;
            }
            try
            {
                playerSessionId =
                    request.GetResponseHeader("X-Ditto-Player-Session-Id") ?? string.Empty;
                DittoLifecycleValidation.Identifier("player_session_id", playerSessionId);
                Publish(
                    DittoJobCodec.Decode(new ReadOnlyMemory<byte>(request.downloadHandler.data))
                );
            }
            catch (Exception exception)
            {
                BattlementUnityLogging.Log(
                    "ditto-player",
                    new BattlementLogRecord(
                        BattlementLogSeverity.Error,
                        "ditto.job-invalid",
                        exception.Message
                    )
                );
            }
        }

        private IEnumerator Poll(string completedJobId)
        {
            polling = true;
            try
            {
                while (true)
                {
                    string path = $"/next-job?after={UnityWebRequest.EscapeURL(completedJobId)}";
                    using UnityWebRequest request = UnityWebRequest.Get(sessionUrl + path);
                    request.downloadHandler = new DownloadHandlerBuffer();
                    yield return request.SendWebRequest();
                    if (request.responseCode == 204)
                    {
                        continue;
                    }
                    if (request.responseCode == 410)
                    {
                        CurrentJob = null;
                        yield break;
                    }
                    if (request.result != UnityWebRequest.Result.Success)
                    {
                        LogFailure("ditto.next-job-failed", request.error);
                        yield break;
                    }
                    try
                    {
                        Publish(
                            DittoJobCodec.Decode(
                                new ReadOnlyMemory<byte>(request.downloadHandler.data)
                            )
                        );
                    }
                    catch (Exception exception)
                    {
                        LogFailure("ditto.job-invalid", exception.Message);
                    }
                    yield break;
                }
            }
            finally
            {
                polling = false;
            }
        }

        private static void Publish(DittoJob job)
        {
            CurrentJob = job;
            JobAvailable?.Invoke(job);
        }

        private static void LogFailure(string eventName, string? message)
        {
            BattlementUnityLogging.Log(
                "ditto-player",
                new BattlementLogRecord(
                    BattlementLogSeverity.Error,
                    eventName,
                    message ?? "The Ditto job request failed."
                )
            );
        }
    }
}
#endif
