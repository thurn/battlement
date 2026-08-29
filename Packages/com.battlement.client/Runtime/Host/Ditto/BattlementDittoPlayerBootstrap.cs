#nullable enable

#if BATTLEMENT_DITTO_DIAGNOSTICS
using System;
using System.Collections;
using UnityEngine;
using UnityEngine.Networking;

namespace Battlement
{
    internal sealed class BattlementDittoPlayerBootstrap : MonoBehaviour
    {
        private const string SessionArgument = "--battlement-ditto-url";

        public static DittoJob? CurrentJob { get; private set; }

        public static BattlementLogObserver? BootstrapLogs { get; private set; }

        [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.BeforeSplashScreen)]
        private static void Initialize()
        {
            if (!TrySessionUrl(Environment.GetCommandLineArgs(), out string sessionUrl))
            {
                return;
            }
            BootstrapLogs = BattlementLogStore.Observe();
            var host = new GameObject("Battlement Ditto");
            DontDestroyOnLoad(host);
            host.AddComponent<BattlementDittoPlayerBootstrap>().StartCoroutine(Fetch(sessionUrl));
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

        private static IEnumerator Fetch(string sessionUrl)
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
                CurrentJob = DittoJobCodec.Decode(
                    new ReadOnlyMemory<byte>(request.downloadHandler.data)
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
    }
}
#endif
