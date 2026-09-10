#nullable enable

using System;
using System.Collections.Concurrent;
using System.Linq;
using Newtonsoft.Json;
using UnityEngine;
using UnityEngine.Experimental.Rendering;
using UnityEngine.Rendering;

namespace Battlement
{
    internal enum DittoCaptureRowOrder
    {
        BottomUp,
        TopDown,
    }

    internal enum DittoCaptureChannelOrder
    {
        Rgba,
        Bgra,
    }

    internal sealed record DittoCapturePixelLayout(
        DittoCaptureRowOrder Rows,
        DittoCaptureChannelOrder Channels
    );

    internal sealed record DittoCaptureFailure(DittoErrorCode Code, string Reason);

    internal sealed record DittoObservationRegion(uint X, uint Y, uint Width, uint Height);

    internal enum DittoFingerprintScope
    {
        None,
        FullFrame,
        TargetRegion,
    }

    internal sealed record DittoObserverFrameTiming(
        ulong MotionPrepareNs,
        ulong RunnerFrameNs,
        ulong NativeFrameCompleteNs,
        ulong InputReleaseAndSyncTransportNs,
        ulong ResponseDecodeNs,
        ulong ResponseApplyNs,
        ulong EndOfFrameWaitNs,
        ulong TextureSetupNs,
        ulong CaptureRequestCpuNs,
        ulong SynchronousReadbackNs,
        ulong CpuHashNs,
        uint ObservedPixels,
        ulong LayoutObservationNs,
        ulong RecorderBookkeepingNs
    );

    internal sealed record DittoVisualObservation(
        ulong Fingerprint,
        ulong? FullFrameFingerprint,
        DittoFingerprintScope Scope,
        DittoObserverFrameTiming Timing
    );

    internal sealed record DittoObserverBaseline(
        uint ObservedPixels,
        ulong TextureSetupNs,
        ulong RequestCpuNs,
        ulong RequestToCallbackNs,
        ulong CallbackToMainThreadNs
    );

    internal sealed record DittoRenderCommit(
        ulong Frame,
        ulong RenderGeneration,
        ulong PixelFingerprint,
        [property: JsonIgnore] DittoFingerprintScope FingerprintScope = DittoFingerprintScope.None,
        [property: JsonIgnore] DittoObserverFrameTiming? ObserverTiming = null,
        [property: JsonIgnore] ulong? FullFrameFingerprint = null,
        [property: JsonIgnore] long EndOfFrameTick = 0
    )
    {
        public bool IdentifiesSamePresentation(DittoRenderCommit other) =>
            Frame == other.Frame
            && RenderGeneration == other.RenderGeneration
            && PixelFingerprint == other.PixelFingerprint
            && FingerprintScope == other.FingerprintScope
            && FullFrameFingerprint == other.FullFrameFingerprint;
    }

    internal abstract record DittoCaptureProbeResult
    {
        internal sealed record Passed(
            string Adapter,
            uint Width,
            uint Height,
            DittoOrientation? Orientation,
            DittoCapturePixelLayout Layout,
            DittoObserverBaseline? ObserverBaseline = null
        ) : DittoCaptureProbeResult;

        internal sealed record Failed(DittoCaptureFailure Failure) : DittoCaptureProbeResult;
    }

    internal abstract record DittoNativeCaptureResult
    {
        internal sealed record Captured(
            byte[] Png,
            uint Width,
            uint Height,
            DittoRenderCommit Commit
        ) : DittoNativeCaptureResult;

        internal sealed record Unavailable(DittoCaptureFailure Failure) : DittoNativeCaptureResult;
    }

    internal interface IDittoNativeCommittedFrameSource
    {
        DittoVisualObservation CommitPresentedFrame(
            uint width,
            uint height,
            DittoObservationRegion? region,
            bool fullFrameDiagnostic
        );

        void ReadCommittedFrame(Action<byte[], bool> completion);

        byte[] ReadCommittedFrame();

        void Dispose();
    }

    internal sealed class DittoNativeCaptureAdapter : MonoBehaviour
    {
        private const double OperationTimeoutSeconds = 10;

        private readonly ConcurrentQueue<System.Action> completions = new();
        private DittoPlatform platform;
        private DittoOrientation? expectedOrientation;
        private uint width;
        private uint height;
        private RenderTexture? probeTexture;
        private IDittoNativeCommittedFrameSource? committedFrameSource;
        private DittoCapturePixelLayout? layout;
        private System.Action<DittoCaptureProbeResult>? probeCompletion;
        private CaptureOperation? captureOperation;
        private double deadline;
        private int generation;
        private long probeRequestedAt;
        private ulong probeRequestCpuNs;
        private ulong probeTextureSetupNs;
        private ulong renderGeneration;
        private DittoRenderCommit? latestCommit;
        private bool configured;
        private bool ready;
        private bool validateDisplay;

        public const string AdapterName = "native-screen-capture";

        public bool IsReady => ready;

        public DittoCapturePixelLayout VideoLayout =>
            ready
                ? new DittoCapturePixelLayout(
                    DittoCaptureRowOrder.BottomUp,
                    DittoCaptureChannelOrder.Rgba
                )
                : throw new InvalidOperationException(
                    "The native capture adapter has not passed its startup probe."
                );

        public static DittoNativeCaptureAdapter Attach(
            GameObject owner,
            DittoPlatform platform,
            uint width,
            uint height,
            DittoOrientation? orientation
        )
        {
            if (owner == null)
            {
                throw new ArgumentNullException(nameof(owner));
            }
            if (platform is not DittoPlatform.Macos and not DittoPlatform.IosSimulator)
            {
                throw new ArgumentOutOfRangeException(
                    nameof(platform),
                    "The native capture adapter supports macOS and iOS Simulator."
                );
            }
            if (width == 0 || height == 0)
            {
                throw new ArgumentOutOfRangeException(nameof(width));
            }
            var adapter = owner.AddComponent<DittoNativeCaptureAdapter>();
            adapter.platform = platform;
            adapter.width = width;
            adapter.height = height;
            adapter.expectedOrientation = orientation;
            adapter.committedFrameSource = new UnityCommittedFrameSource();
            adapter.configured = true;
            adapter.validateDisplay = true;
            return adapter;
        }

        internal static DittoNativeCaptureAdapter AttachForTesting(
            GameObject owner,
            uint width,
            uint height,
            DittoCapturePixelLayout layout,
            IDittoNativeCommittedFrameSource source
        )
        {
            var adapter = owner.AddComponent<DittoNativeCaptureAdapter>();
            adapter.platform = DittoPlatform.Macos;
            adapter.width = width;
            adapter.height = height;
            adapter.layout = layout;
            adapter.committedFrameSource = source;
            adapter.configured = true;
            adapter.ready = true;
            return adapter;
        }

        public void Probe(System.Action<DittoCaptureProbeResult> completion)
        {
            RequireConfigured();
            RequireIdle();
            probeCompletion = completion ?? throw new ArgumentNullException(nameof(completion));
            if (!SystemInfo.supportsAsyncGPUReadback)
            {
                FailProbe("Asynchronous GPU readback is unsupported.");
                return;
            }
            if (validateDisplay && (Screen.width != width || Screen.height != height))
            {
                FailProbe(
                    $"Framebuffer dimensions {Screen.width}x{Screen.height} do not match "
                        + $"the configured surface {width}x{height}."
                );
                return;
            }
            DittoOrientation? orientation = CurrentOrientation();
            if (platform == DittoPlatform.IosSimulator && orientation != expectedOrientation)
            {
                FailProbe(
                    $"Framebuffer orientation {orientation?.ToString() ?? "unknown"} does not "
                        + $"match {expectedOrientation?.ToString() ?? "unknown"}."
                );
                return;
            }

            try
            {
                long setupStarted = System.Diagnostics.Stopwatch.GetTimestamp();
                GraphicsFormat format = SystemInfo.GetCompatibleFormat(
                    GraphicsFormat.R8G8B8A8_UNorm,
                    GraphicsFormatUsage.Render
                );
                probeTexture = Texture(2, 2, format);
                using var source = new TemporaryTexture(DittoCapturePixels.ProbeColors);
                probeTextureSetupNs = ElapsedNanoseconds(setupStarted);
                long requestStarted = System.Diagnostics.Stopwatch.GetTimestamp();
                Graphics.Blit(source.Value, probeTexture);
                BeginOperation();
                int requestedGeneration = generation;
                probeRequestedAt = System.Diagnostics.Stopwatch.GetTimestamp();
                AsyncGPUReadback.Request(
                    probeTexture,
                    0,
                    request =>
                    {
                        long callbackAt = System.Diagnostics.Stopwatch.GetTimestamp();
                        bool failed = request.hasError;
                        byte[] pixels = failed
                            ? Array.Empty<byte>()
                            : request.GetData<byte>().ToArray();
                        completions.Enqueue(() =>
                            CompleteProbe(requestedGeneration, pixels, failed, callbackAt)
                        );
                    }
                );
                probeRequestCpuNs = ElapsedNanoseconds(requestStarted);
            }
            catch (Exception exception)
            {
                FailProbe(exception.Message);
            }
        }

        public DittoRenderCommit CommitPresentedFrame(
            ulong committedFrame,
            DittoObservationRegion? region = null,
            bool fullFrameDiagnostic = false
        )
        {
            RequireConfigured();
            RequireReady();
            RequireIdle();
            if (committedFrame == 0)
            {
                throw new ArgumentOutOfRangeException(nameof(committedFrame));
            }
            if (validateDisplay && (Screen.width != width || Screen.height != height))
            {
                throw new InvalidOperationException(
                    $"Framebuffer dimensions changed to {Screen.width}x{Screen.height}."
                );
            }
            DittoVisualObservation observation = committedFrameSource!.CommitPresentedFrame(
                width,
                height,
                region,
                fullFrameDiagnostic
            );
            latestCommit = new DittoRenderCommit(
                committedFrame,
                checked(++renderGeneration),
                observation.Fingerprint,
                observation.Scope,
                observation.Timing,
                observation.FullFrameFingerprint
            );
            return latestCommit;
        }

        public DittoRenderCommit ObservePresentedFrame(ulong committedFrame)
        {
            RequireConfigured();
            RequireReady();
            RequireIdle();
            if (committedFrame == 0)
            {
                throw new ArgumentOutOfRangeException(nameof(committedFrame));
            }
            return new DittoRenderCommit(committedFrame, checked(++renderGeneration), 0);
        }

        public void CaptureCommittedFrame(
            DittoRenderCommit commit,
            System.Action<DittoNativeCaptureResult> completion
        )
        {
            Debug.Log(
                $"[Battlement/Ditto-trace] capture-request frame={commit.Frame} "
                    + $"render-generation={commit.RenderGeneration} "
                    + $"pid={System.Diagnostics.Process.GetCurrentProcess().Id} "
                    + $"source=internal-framebuffer window=player-main-framebuffer "
                    + $"display={Screen.width}x{Screen.height}"
            );
            RequireConfigured();
            RequireIdle();
            if (!ready || layout is null)
            {
                completion(
                    new DittoNativeCaptureResult.Unavailable(
                        Failure("The native capture adapter has not passed its startup probe.")
                    )
                );
                return;
            }
            if (commit.Frame == 0 || commit.RenderGeneration == 0)
            {
                throw new ArgumentOutOfRangeException(nameof(commit));
            }
            if (latestCommit?.IdentifiesSamePresentation(commit) != true)
            {
                completion(
                    new DittoNativeCaptureResult.Unavailable(
                        Failure(
                            $"Render commit {commit.RenderGeneration} for frame {commit.Frame} "
                                + "does not match the retained framebuffer snapshot."
                        )
                    )
                );
                return;
            }
            var operation = new CaptureOperation(
                commit,
                completion ?? throw new ArgumentNullException(nameof(completion)),
                Time.realtimeSinceStartupAsDouble + OperationTimeoutSeconds
            );
            captureOperation = operation;
            BeginOperation();
            try
            {
                committedFrameSource!.ReadCommittedFrame(
                    (pixels, failed) =>
                        completions.Enqueue(() => CompleteCapture(operation, pixels, failed))
                );
            }
            catch (Exception exception)
            {
                FailCapture(operation, exception.Message);
            }
        }

        public byte[] CaptureVideoFrame(DittoRenderCommit commit)
        {
            RequireConfigured();
            if (!ready || layout is null)
            {
                throw new InvalidOperationException(
                    "The native capture adapter has not passed its startup probe."
                );
            }
            if (commit.Frame == 0 || latestCommit?.IdentifiesSamePresentation(commit) != true)
            {
                throw new InvalidOperationException(
                    "The requested video frame is not the retained render commit."
                );
            }
            if (Screen.width != width || Screen.height != height)
            {
                throw new InvalidOperationException(
                    $"Framebuffer dimensions changed to {Screen.width}x{Screen.height}."
                );
            }
            return OrientedPixels(committedFrameSource!.ReadCommittedFrame());
        }

        public static DittoNativeCaptureResult ProcessLost() =>
            new DittoNativeCaptureResult.Unavailable(
                Failure("The player exited before a responsive failure frame was captured.")
            );

        private void CompleteProbe(
            int requestedGeneration,
            byte[] pixels,
            bool readbackFailed,
            long callbackAt
        )
        {
            if (requestedGeneration != generation || probeCompletion is null)
            {
                return;
            }
            if (readbackFailed)
            {
                FailProbe("The startup probe GPU readback failed.");
                return;
            }
            if (!DittoCapturePixels.TryProbe(pixels, out DittoCapturePixelLayout? detected))
            {
                FailProbe(
                    "The startup probe did not preserve dimensions, alpha, rows, and channels. "
                        + $"Received {pixels.Length} bytes: {BitConverter.ToString(pixels)}."
                );
                return;
            }
            layout = detected;
            ready = true;
            System.Action<DittoCaptureProbeResult> completion = probeCompletion;
            probeCompletion = null;
            completion(
                new DittoCaptureProbeResult.Passed(
                    AdapterName,
                    width,
                    height,
                    CurrentOrientation(),
                    detected!,
                    new DittoObserverBaseline(
                        4,
                        probeTextureSetupNs,
                        probeRequestCpuNs,
                        Nanoseconds(callbackAt - probeRequestedAt),
                        ElapsedNanoseconds(callbackAt)
                    )
                )
            );
        }

        private void CompleteCapture(CaptureOperation operation, byte[] pixels, bool readbackFailed)
        {
            if (!ReferenceEquals(captureOperation, operation))
            {
                return;
            }
            DittoRenderCommit commit = operation.Commit;
            if (latestCommit?.IdentifiesSamePresentation(commit) != true)
            {
                FailCapture(
                    operation,
                    "The retained framebuffer changed before GPU readback completed."
                );
                return;
            }
            if (readbackFailed)
            {
                FailCapture(operation, "The framebuffer GPU readback failed.");
                return;
            }
            DittoNativeCaptureResult result = BindCapturedPixels(
                latestCommit,
                commit,
                OrientedPixels(pixels),
                width,
                height,
                layout!
            );
            if (result is DittoNativeCaptureResult.Unavailable unavailable)
            {
                FailCapture(operation, unavailable.Failure.Reason);
                return;
            }
            captureOperation = null;
            var captured = (DittoNativeCaptureResult.Captured)result;
            Debug.Log(
                $"[Battlement/Ditto-trace] capture-ack frame={commit.Frame} "
                    + $"render-generation={commit.RenderGeneration} "
                    + $"pid={System.Diagnostics.Process.GetCurrentProcess().Id} "
                    + $"bytes={captured.Png.Length}"
            );
            operation.Completion(captured);
        }

        internal static DittoNativeCaptureResult BindCapturedPixels(
            DittoRenderCommit? retained,
            DittoRenderCommit requested,
            byte[] pixels,
            uint width,
            uint height,
            DittoCapturePixelLayout layout
        )
        {
            if (retained?.IdentifiesSamePresentation(requested) != true)
            {
                return new DittoNativeCaptureResult.Unavailable(
                    Failure(
                        $"Render commit {requested.RenderGeneration} for frame {requested.Frame} "
                            + "does not match the retained framebuffer snapshot."
                    )
                );
            }
            if (!DittoCapturePixels.TryEncode(pixels, width, height, layout, out byte[] png))
            {
                return new DittoNativeCaptureResult.Unavailable(
                    Failure("The framebuffer could not be encoded as a valid PNG.")
                );
            }
            return new DittoNativeCaptureResult.Captured(png, width, height, requested);
        }

        private void RequireReady()
        {
            if (!ready || layout is null)
            {
                throw new InvalidOperationException(
                    "The native capture adapter has not passed its startup probe."
                );
            }
        }

        private void BeginOperation()
        {
            generation++;
            deadline = Time.realtimeSinceStartupAsDouble + OperationTimeoutSeconds;
        }

        private void Update()
        {
            while (completions.TryDequeue(out System.Action completion))
            {
                completion();
            }
            if (Time.realtimeSinceStartupAsDouble < deadline)
            {
                return;
            }
            if (probeCompletion is not null)
            {
                FailProbe("The startup capture probe timed out.");
            }
            else if (
                captureOperation is CaptureOperation operation
                && Time.realtimeSinceStartupAsDouble >= operation.Deadline
            )
            {
                FailCapture(operation, "Framebuffer capture timed out.");
            }
        }

        private void FailProbe(string reason)
        {
            System.Action<DittoCaptureProbeResult>? completion = probeCompletion;
            probeCompletion = null;
            ready = false;
            generation++;
            completion?.Invoke(new DittoCaptureProbeResult.Failed(Failure(reason)));
        }

        internal void ExpirePendingCaptureForTesting()
        {
            if (captureOperation is CaptureOperation operation)
            {
                FailCapture(operation, "Framebuffer capture timed out.");
            }
        }

        internal void CompletePendingCallbacksForTesting()
        {
            while (completions.TryDequeue(out System.Action completion))
            {
                completion();
            }
        }

        private void FailCapture(CaptureOperation operation, string reason)
        {
            if (!ReferenceEquals(captureOperation, operation))
            {
                return;
            }
            captureOperation = null;
            generation++;
            operation.Completion(new DittoNativeCaptureResult.Unavailable(Failure(reason)));
        }

        private void RequireConfigured()
        {
            if (!configured)
            {
                throw new InvalidOperationException(
                    "The native capture adapter is not configured."
                );
            }
        }

        private void RequireIdle()
        {
            if (probeCompletion is not null || captureOperation is not null)
            {
                throw new InvalidOperationException(
                    "A native capture operation is already pending."
                );
            }
        }

        private DittoOrientation? CurrentOrientation() =>
            platform == DittoPlatform.Macos
                ? null
                : Screen.orientation switch
                {
                    ScreenOrientation.Portrait => DittoOrientation.Portrait,
                    ScreenOrientation.PortraitUpsideDown => DittoOrientation.PortraitUpsideDown,
                    ScreenOrientation.LandscapeLeft => DittoOrientation.LandscapeLeft,
                    ScreenOrientation.LandscapeRight => DittoOrientation.LandscapeRight,
                    _ => expectedOrientation,
                };

        private static ulong ElapsedNanoseconds(long started) =>
            Nanoseconds(System.Diagnostics.Stopwatch.GetTimestamp() - started);

        private static ulong Nanoseconds(long ticks) =>
            checked(
                (ulong)Math.Max(0, ticks)
                * 1_000_000_000UL
                / (ulong)System.Diagnostics.Stopwatch.Frequency
            );

        private byte[] OrientedPixels(byte[] pixels) =>
            DittoCapturePixels.FlipRows(pixels, width, height);

        private static RenderTexture Texture(int width, int height, GraphicsFormat format)
        {
            var texture = new RenderTexture(width, height, 0) { graphicsFormat = format };
            if (!texture.Create())
            {
                Destroy(texture);
                throw new InvalidOperationException(
                    "The capture render texture could not be created."
                );
            }
            return texture;
        }

        private static DittoCaptureFailure Failure(string reason) =>
            new(DittoErrorCode.ImageCaptureFailed, reason);

        private void OnDestroy()
        {
            FailProbe("The capture adapter was destroyed during its startup probe.");
            if (captureOperation is CaptureOperation operation)
            {
                FailCapture(
                    operation,
                    "The capture adapter was destroyed before capture completed."
                );
            }
            Release(probeTexture);
            committedFrameSource?.Dispose();
        }

        private static void Release(RenderTexture? texture)
        {
            if (texture == null)
            {
                return;
            }
            texture.Release();
            Destroy(texture);
        }

        private sealed class TemporaryTexture : IDisposable
        {
            public TemporaryTexture(Color32[] pixels)
            {
                Value = new Texture2D(2, 2, TextureFormat.RGBA32, false, true);
                Value.SetPixels32(pixels);
                Value.Apply();
            }

            public Texture2D Value { get; }

            public void Dispose() => Destroy(Value);
        }

        private sealed record CaptureOperation(
            DittoRenderCommit Commit,
            System.Action<DittoNativeCaptureResult> Completion,
            double Deadline
        );

        private sealed class UnityCommittedFrameSource : IDittoNativeCommittedFrameSource
        {
            private RenderTexture? framebuffer;
            private Texture2D? fingerprint;

            public DittoVisualObservation CommitPresentedFrame(
                uint width,
                uint height,
                DittoObservationRegion? region,
                bool fullFrameDiagnostic
            )
            {
                long setupStarted = System.Diagnostics.Stopwatch.GetTimestamp();
                DittoObservationRegion observed = region ?? new(0, 0, width, height);
                ValidateRegion(width, height, observed);
                DittoObservationRegion readback = fullFrameDiagnostic
                    ? new DittoObservationRegion(0, 0, width, height)
                    : observed;
                EnsureTextures(width, height, readback.Width, readback.Height);
                ulong setupNs = ElapsedNanoseconds(setupStarted);
                long captureStarted = System.Diagnostics.Stopwatch.GetTimestamp();
                ScreenCapture.CaptureScreenshotIntoRenderTexture(framebuffer);
                ulong captureNs = ElapsedNanoseconds(captureStarted);
                RenderTexture? previous = RenderTexture.active;
                try
                {
                    RenderTexture.active = framebuffer;
                    long readbackStarted = System.Diagnostics.Stopwatch.GetTimestamp();
                    fingerprint!.ReadPixels(
                        new UnityEngine.Rect(
                            readback.X,
                            readback.Y,
                            readback.Width,
                            readback.Height
                        ),
                        0,
                        0,
                        false
                    );
                    ulong readbackNs = ElapsedNanoseconds(readbackStarted);
                    long hashStarted = System.Diagnostics.Stopwatch.GetTimestamp();
                    const ulong offset = 14_695_981_039_346_656_037;
                    const ulong prime = 1_099_511_628_211;
                    Unity.Collections.NativeArray<byte> pixels =
                        fingerprint.GetRawTextureData<byte>();
                    ulong fullHash = offset;
                    foreach (byte value in pixels)
                    {
                        fullHash = (fullHash ^ value) * prime;
                    }
                    ulong hash =
                        fullFrameDiagnostic && region is not null
                            ? HashRegion(pixels, width, observed, offset, prime)
                            : fullHash;
                    return new DittoVisualObservation(
                        hash,
                        fullFrameDiagnostic ? fullHash : null,
                        region is null
                            ? DittoFingerprintScope.FullFrame
                            : DittoFingerprintScope.TargetRegion,
                        new DittoObserverFrameTiming(
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                            setupNs,
                            captureNs,
                            readbackNs,
                            ElapsedNanoseconds(hashStarted),
                            checked(readback.Width * readback.Height),
                            0,
                            0
                        )
                    );
                }
                finally
                {
                    RenderTexture.active = previous;
                }
            }

            public void ReadCommittedFrame(Action<byte[], bool> completion) =>
                AsyncGPUReadback.Request(
                    framebuffer,
                    0,
                    request =>
                        completion(
                            request.hasError
                                ? Array.Empty<byte>()
                                : request.GetData<byte>().ToArray(),
                            request.hasError
                        )
                );

            public byte[] ReadCommittedFrame()
            {
                EnsureTextures(
                    checked((uint)framebuffer!.width),
                    checked((uint)framebuffer.height),
                    checked((uint)framebuffer.width),
                    checked((uint)framebuffer.height)
                );
                RenderTexture? previous = RenderTexture.active;
                try
                {
                    RenderTexture.active = framebuffer;
                    fingerprint!.ReadPixels(
                        new UnityEngine.Rect(0, 0, framebuffer!.width, framebuffer.height),
                        0,
                        0,
                        false
                    );
                    return fingerprint.GetRawTextureData<byte>().ToArray();
                }
                finally
                {
                    RenderTexture.active = previous;
                }
            }

            public void Dispose()
            {
                Release(framebuffer);
                if (fingerprint != null)
                {
                    Destroy(fingerprint);
                }
            }

            private void EnsureTextures(
                uint width,
                uint height,
                uint fingerprintWidth,
                uint fingerprintHeight
            )
            {
                if (framebuffer == null)
                {
                    framebuffer = Texture(
                        checked((int)width),
                        checked((int)height),
                        SystemInfo.GetCompatibleFormat(
                            GraphicsFormat.R8G8B8A8_UNorm,
                            GraphicsFormatUsage.Render
                        )
                    );
                }
                if (
                    fingerprint != null
                    && fingerprint.width == checked((int)fingerprintWidth)
                    && fingerprint.height == checked((int)fingerprintHeight)
                )
                    return;
                if (fingerprint != null)
                    Destroy(fingerprint);
                fingerprint = new Texture2D(
                    checked((int)fingerprintWidth),
                    checked((int)fingerprintHeight),
                    TextureFormat.RGBA32,
                    false,
                    true
                );
            }

            private static void ValidateRegion(
                uint width,
                uint height,
                DittoObservationRegion region
            )
            {
                if (
                    region.Width == 0
                    || region.Height == 0
                    || checked(region.X + region.Width) > width
                    || checked(region.Y + region.Height) > height
                )
                    throw new ArgumentOutOfRangeException(nameof(region));
            }

            private static ulong HashRegion(
                Unity.Collections.NativeArray<byte> pixels,
                uint stridePixels,
                DittoObservationRegion region,
                ulong offset,
                ulong prime
            )
            {
                ulong hash = offset;
                for (uint y = region.Y; y < checked(region.Y + region.Height); y++)
                {
                    int start = checked((int)((y * stridePixels + region.X) * 4));
                    int end = checked(start + (int)(region.Width * 4));
                    for (int index = start; index < end; index++)
                        hash = (hash ^ pixels[index]) * prime;
                }
                return hash;
            }

            private static ulong ElapsedNanoseconds(long started) =>
                Nanoseconds(System.Diagnostics.Stopwatch.GetTimestamp() - started);
        }
    }

    internal static class DittoCapturePixels
    {
        internal static readonly Color32[] ProbeColors =
        {
            new(11, 23, 47, 61),
            new(67, 79, 97, 109),
            new(131, 149, 167, 181),
            new(191, 199, 211, 223),
        };

        public static bool TryProbe(byte[] pixels, out DittoCapturePixelLayout? layout)
        {
            foreach (DittoCaptureRowOrder rows in Enum.GetValues(typeof(DittoCaptureRowOrder)))
            {
                foreach (
                    DittoCaptureChannelOrder channels in Enum.GetValues(
                        typeof(DittoCaptureChannelOrder)
                    )
                )
                {
                    var candidate = new DittoCapturePixelLayout(rows, channels);
                    if (pixels.SequenceEqual(Bytes(2, 2, ProbeColors, candidate)))
                    {
                        layout = candidate;
                        return true;
                    }
                }
            }
            layout = null;
            return false;
        }

        public static bool TryEncode(
            byte[] pixels,
            uint width,
            uint height,
            DittoCapturePixelLayout layout,
            out byte[] png
        )
        {
            png = Array.Empty<byte>();
            if (pixels.Length != checked((long)width * height * 4))
            {
                return false;
            }
            Texture2D? texture = null;
            try
            {
                texture = new Texture2D(
                    checked((int)width),
                    checked((int)height),
                    TextureFormat.RGBA32,
                    false,
                    true
                );
                texture.LoadRawTextureData(
                    Bytes(checked((int)width), checked((int)height), pixels, layout)
                );
                texture.Apply();
                png = texture.EncodeToPNG();
                return png.Length >= 8
                    && png[0] == 0x89
                    && png[1] == 0x50
                    && png[2] == 0x4e
                    && png[3] == 0x47;
            }
            catch (Exception)
            {
                png = Array.Empty<byte>();
                return false;
            }
            finally
            {
                if (texture != null)
                {
                    if (Application.isPlaying)
                    {
                        UnityEngine.Object.Destroy(texture);
                    }
                    else
                    {
                        UnityEngine.Object.DestroyImmediate(texture);
                    }
                }
            }
        }

        internal static byte[] Bytes(
            int width,
            int height,
            Color32[] colors,
            DittoCapturePixelLayout layout
        )
        {
            var rgba = new byte[checked(width * height * 4)];
            for (var index = 0; index < colors.Length; index++)
            {
                Color32 color = colors[index];
                int offset = index * 4;
                rgba[offset] = color.r;
                rgba[offset + 1] = color.g;
                rgba[offset + 2] = color.b;
                rgba[offset + 3] = color.a;
            }
            return Bytes(width, height, rgba, layout);
        }

        internal static byte[] TopDownRgba(
            byte[] source,
            uint width,
            uint height,
            DittoCapturePixelLayout layout
        )
        {
            if (source.LongLength != checked((long)width * height * 4))
            {
                throw new ArgumentException(
                    "A video frame has the wrong byte size.",
                    nameof(source)
                );
            }
            int columns = checked((int)width);
            int rows = checked((int)height);
            int stride = checked(columns * 4);
            var output = new byte[source.Length];
            for (var row = 0; row < rows; row++)
            {
                int sourceRow = layout.Rows == DittoCaptureRowOrder.TopDown ? row : rows - row - 1;
                for (var column = 0; column < columns; column++)
                {
                    int sourceOffset = sourceRow * stride + column * 4;
                    int outputOffset = row * stride + column * 4;
                    bool bgra = layout.Channels == DittoCaptureChannelOrder.Bgra;
                    output[outputOffset] = source[sourceOffset + (bgra ? 2 : 0)];
                    output[outputOffset + 1] = source[sourceOffset + 1];
                    output[outputOffset + 2] = source[sourceOffset + (bgra ? 0 : 2)];
                    output[outputOffset + 3] = source[sourceOffset + 3];
                }
            }
            return output;
        }

        internal static byte[] FlipRows(byte[] source, uint width, uint height)
        {
            if (source.LongLength != checked((long)width * height * 4))
            {
                throw new ArgumentException(
                    "A framebuffer has the wrong byte size.",
                    nameof(source)
                );
            }
            var output = new byte[source.Length];
            int stride = checked((int)width * 4);
            for (var row = 0; row < height; row++)
            {
                int sourceOffset = checked(row * stride);
                int outputOffset = checked((int)(height - row - 1) * stride);
                Buffer.BlockCopy(source, sourceOffset, output, outputOffset, stride);
            }
            return output;
        }

        private static byte[] Bytes(
            int width,
            int height,
            byte[] source,
            DittoCapturePixelLayout layout
        )
        {
            int stride = checked(width * 4);
            var output = new byte[source.Length];
            for (var row = 0; row < height; row++)
            {
                int sourceRow =
                    layout.Rows == DittoCaptureRowOrder.BottomUp ? row : height - row - 1;
                for (var column = 0; column < width; column++)
                {
                    int sourceOffset = sourceRow * stride + column * 4;
                    int outputOffset = row * stride + column * 4;
                    bool bgra = layout.Channels == DittoCaptureChannelOrder.Bgra;
                    output[outputOffset] = source[sourceOffset + (bgra ? 2 : 0)];
                    output[outputOffset + 1] = source[sourceOffset + 1];
                    output[outputOffset + 2] = source[sourceOffset + (bgra ? 0 : 2)];
                    output[outputOffset + 3] = source[sourceOffset + 3];
                }
            }
            return output;
        }
    }
}
