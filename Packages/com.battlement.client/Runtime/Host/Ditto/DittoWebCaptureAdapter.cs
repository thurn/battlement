#nullable enable

using System;
using Newtonsoft.Json;
using UnityEngine;

namespace Battlement
{
    internal static class DittoWebSessionRoute
    {
        public static bool TryResolve(string launcherUrl, out string sessionUrl)
        {
            sessionUrl = string.Empty;
            if (!Uri.TryCreate(launcherUrl, UriKind.Absolute, out Uri? uri))
            {
                return false;
            }
            const string suffix = "/launcher";
            if (
                uri.Scheme != Uri.UriSchemeHttp
                || uri.Host != "127.0.0.1"
                || !uri.AbsolutePath.EndsWith(suffix, StringComparison.Ordinal)
            )
            {
                return false;
            }
            string route = uri.AbsolutePath.Substring(0, uri.AbsolutePath.Length - suffix.Length);
            sessionUrl = uri.GetLeftPart(UriPartial.Authority) + route;
            return true;
        }
    }

    internal interface IDittoWebBrowserBridge
    {
        void Install(string owner);

        void Probe(string owner, uint width, uint height);

        void Capture(
            string owner,
            string url,
            string artifactId,
            uint width,
            uint height,
            ulong frame
        );
    }

    internal abstract record DittoWebProbeResult
    {
        internal sealed record Passed(string Adapter, uint Width, uint Height)
            : DittoWebProbeResult;

        internal sealed record Failed(DittoCaptureFailure Failure) : DittoWebProbeResult;
    }

    internal abstract record DittoWebCaptureResult
    {
        internal sealed record Uploaded(
            string ArtifactId,
            string Sha256,
            uint Width,
            uint Height,
            ulong Frame
        ) : DittoWebCaptureResult;

        internal sealed record Unavailable(DittoCaptureFailure Failure) : DittoWebCaptureResult;
    }

    internal sealed class DittoWebCaptureAdapter : MonoBehaviour
    {
        private const double OperationTimeoutSeconds = 10;

        private IDittoWebBrowserBridge bridge = new DittoWebBrowserBridge();
        private Action<DittoWebProbeResult>? probeCompletion;
        private Action<DittoWebCaptureResult>? captureCompletion;
        private string expectedArtifactId = string.Empty;
        private uint width;
        private uint height;
        private double deadline;
        private bool configured;
        private bool ready;

        public const string AdapterName = "webgl-canvas-png";

        public bool IsReady => ready;

        public static DittoWebCaptureAdapter Attach(
            GameObject owner,
            uint width,
            uint height,
            IDittoWebBrowserBridge? bridge = null
        )
        {
            if (owner == null)
            {
                throw new ArgumentNullException(nameof(owner));
            }
            if (width == 0 || height == 0)
            {
                throw new ArgumentOutOfRangeException(nameof(width));
            }
            var adapter = owner.AddComponent<DittoWebCaptureAdapter>();
            adapter.width = width;
            adapter.height = height;
            adapter.bridge = bridge ?? new DittoWebBrowserBridge();
            adapter.configured = true;
            adapter.bridge.Install(owner.name);
            return adapter;
        }

        public void Probe(Action<DittoWebProbeResult> completion)
        {
            RequireConfigured();
            RequireIdle();
            probeCompletion = completion ?? throw new ArgumentNullException(nameof(completion));
            deadline = Time.realtimeSinceStartupAsDouble + OperationTimeoutSeconds;
            bridge.Probe(gameObject.name, width, height);
        }

        public void UploadCommittedFrame(
            string artifactUrl,
            string artifactId,
            ulong committedFrame,
            Action<DittoWebCaptureResult> completion
        )
        {
            RequireConfigured();
            RequireIdle();
            if (!ready)
            {
                completion(
                    new DittoWebCaptureResult.Unavailable(
                        Failure("The WebGL capture adapter has not passed its startup probe.")
                    )
                );
                return;
            }
            if (
                !Uri.TryCreate(artifactUrl, UriKind.Absolute, out Uri? url)
                || url.Scheme != Uri.UriSchemeHttp
                || url.Host != "127.0.0.1"
            )
            {
                throw new ArgumentException(
                    "Artifact URL must use IPv4 loopback HTTP.",
                    nameof(artifactUrl)
                );
            }
            DittoLifecycleValidation.Identifier("artifact_id", artifactId);
            if (committedFrame == 0)
            {
                throw new ArgumentOutOfRangeException(nameof(committedFrame));
            }
            expectedArtifactId = artifactId;
            captureCompletion = completion ?? throw new ArgumentNullException(nameof(completion));
            deadline = Time.realtimeSinceStartupAsDouble + OperationTimeoutSeconds;
            bridge.Capture(gameObject.name, artifactUrl, artifactId, width, height, committedFrame);
        }

        public void CompleteWebProbe(string json)
        {
            if (probeCompletion is null)
            {
                return;
            }
            BridgeResult result = Decode(json);
            Action<DittoWebProbeResult> completion = probeCompletion;
            probeCompletion = null;
            ready = result.Ok && result.Width == width && result.Height == height;
            completion(
                ready
                    ? new DittoWebProbeResult.Passed(AdapterName, width, height)
                    : new DittoWebProbeResult.Failed(
                        Failure(
                            result.Ok
                                ? "The WebGL probe reported unexpected dimensions."
                                : result.Reason ?? "The WebGL startup probe failed."
                        )
                    )
            );
        }

        public void CompleteWebCapture(string json)
        {
            if (captureCompletion is null)
            {
                return;
            }
            BridgeResult result = Decode(json);
            Action<DittoWebCaptureResult> completion = captureCompletion;
            captureCompletion = null;
            bool valid =
                result.Ok
                && result.ArtifactId == expectedArtifactId
                && result.Width == width
                && result.Height == height
                && ValidSha256(result.Sha256)
                && result.Frame > 0;
            expectedArtifactId = string.Empty;
            completion(
                valid
                    ? new DittoWebCaptureResult.Uploaded(
                        result.ArtifactId!,
                        result.Sha256!,
                        result.Width,
                        result.Height,
                        result.Frame
                    )
                    : new DittoWebCaptureResult.Unavailable(
                        Failure(
                            result.Ok
                                ? "The WebGL PNG upload acknowledgement is invalid."
                                : result.Reason ?? "The WebGL PNG upload failed."
                        )
                    )
            );
        }

        private void Update()
        {
            if (Time.realtimeSinceStartupAsDouble < deadline)
            {
                return;
            }
            if (probeCompletion is not null)
            {
                FailProbe("The WebGL startup probe timed out.");
            }
            else if (captureCompletion is not null)
            {
                FailCapture("The WebGL PNG upload timed out.");
            }
        }

        private void FailProbe(string reason)
        {
            Action<DittoWebProbeResult>? completion = probeCompletion;
            probeCompletion = null;
            ready = false;
            completion?.Invoke(new DittoWebProbeResult.Failed(Failure(reason)));
        }

        private void FailCapture(string reason)
        {
            Action<DittoWebCaptureResult>? completion = captureCompletion;
            captureCompletion = null;
            expectedArtifactId = string.Empty;
            completion?.Invoke(new DittoWebCaptureResult.Unavailable(Failure(reason)));
        }

        private void RequireConfigured()
        {
            if (!configured)
            {
                throw new InvalidOperationException("The WebGL capture adapter is not configured.");
            }
        }

        private void RequireIdle()
        {
            if (probeCompletion is not null || captureCompletion is not null)
            {
                throw new InvalidOperationException(
                    "A WebGL capture operation is already pending."
                );
            }
        }

        private static BridgeResult Decode(string json) =>
            JsonConvert.DeserializeObject<BridgeResult>(json)
            ?? throw new InvalidOperationException("The WebGL bridge returned no result.");

        private static bool ValidSha256(string? value) =>
            value is not null
            && value.Length == 64
            && Array.TrueForAll(
                value.ToCharArray(),
                character => character is >= '0' and <= '9' or >= 'a' and <= 'f'
            );

        private static DittoCaptureFailure Failure(string reason) =>
            new(DittoErrorCode.ImageCaptureFailed, reason);

        private void OnDestroy()
        {
            FailProbe("The WebGL adapter was destroyed during its startup probe.");
            FailCapture("The WebGL adapter was destroyed before upload completed.");
        }

        private sealed record BridgeResult(
            [property: JsonProperty("ok")] bool Ok,
            [property: JsonProperty("artifactId")] string? ArtifactId,
            [property: JsonProperty("sha256")] string? Sha256,
            [property: JsonProperty("width")] uint Width,
            [property: JsonProperty("height")] uint Height,
            [property: JsonProperty("frame")] ulong Frame,
            [property: JsonProperty("reason")] string? Reason
        );
    }

    internal sealed class DittoWebBrowserBridge : IDittoWebBrowserBridge
    {
        public void Install(string owner)
        {
#if UNITY_WEBGL && !UNITY_EDITOR
            BattlementDittoWebInstall(owner);
#else
            throw new PlatformNotSupportedException("The browser bridge requires WebGL.");
#endif
        }

        public void Probe(string owner, uint width, uint height)
        {
#if UNITY_WEBGL && !UNITY_EDITOR
            BattlementDittoWebProbe(owner, width, height);
#else
            throw new PlatformNotSupportedException("The browser bridge requires WebGL.");
#endif
        }

        public void Capture(
            string owner,
            string url,
            string artifactId,
            uint width,
            uint height,
            ulong frame
        )
        {
#if UNITY_WEBGL && !UNITY_EDITOR
            BattlementDittoWebCapture(owner, url, artifactId, width, height, frame);
#else
            throw new PlatformNotSupportedException("The browser bridge requires WebGL.");
#endif
        }

#if UNITY_WEBGL && !UNITY_EDITOR
        [System.Runtime.InteropServices.DllImport("__Internal")]
        private static extern void BattlementDittoWebInstall(string owner);

        [System.Runtime.InteropServices.DllImport("__Internal")]
        private static extern void BattlementDittoWebProbe(string owner, uint width, uint height);

        [System.Runtime.InteropServices.DllImport("__Internal")]
        private static extern void BattlementDittoWebCapture(
            string owner,
            string url,
            string artifactId,
            uint width,
            uint height,
            ulong frame
        );
#endif
    }
}
