#nullable enable

using System;
using System.Collections;
using System.Collections.Concurrent;
using System.Linq;
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

    internal abstract record DittoCaptureProbeResult
    {
        internal sealed record Passed(
            string Adapter,
            uint Width,
            uint Height,
            DittoOrientation? Orientation,
            DittoCapturePixelLayout Layout
        ) : DittoCaptureProbeResult;

        internal sealed record Failed(DittoCaptureFailure Failure) : DittoCaptureProbeResult;
    }

    internal abstract record DittoNativeCaptureResult
    {
        internal sealed record Captured(byte[] Png, uint Width, uint Height, ulong Frame)
            : DittoNativeCaptureResult;

        internal sealed record Unavailable(DittoCaptureFailure Failure) : DittoNativeCaptureResult;
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
        private RenderTexture? framebuffer;
        private DittoCapturePixelLayout? layout;
        private System.Action<DittoCaptureProbeResult>? probeCompletion;
        private System.Action<DittoNativeCaptureResult>? captureCompletion;
        private double deadline;
        private int generation;
        private bool configured;
        private bool ready;

        public const string AdapterName = "native-screen-capture";

        public bool IsReady => ready;

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
            adapter.configured = true;
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
            if (Screen.width != width || Screen.height != height)
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
                GraphicsFormat format = SystemInfo.GetCompatibleFormat(
                    GraphicsFormat.R8G8B8A8_UNorm,
                    GraphicsFormatUsage.Render
                );
                probeTexture = Texture(2, 2, format);
                using var source = new TemporaryTexture(DittoCapturePixels.ProbeColors);
                Graphics.Blit(source.Value, probeTexture);
                BeginOperation();
                int requestedGeneration = generation;
                AsyncGPUReadback.Request(
                    probeTexture,
                    0,
                    request =>
                    {
                        bool failed = request.hasError;
                        byte[] pixels = failed
                            ? Array.Empty<byte>()
                            : request.GetData<byte>().ToArray();
                        completions.Enqueue(() =>
                            CompleteProbe(requestedGeneration, pixels, failed)
                        );
                    }
                );
            }
            catch (Exception exception)
            {
                FailProbe(exception.Message);
            }
        }

        public void CaptureCommittedFrame(
            ulong committedFrame,
            System.Action<DittoNativeCaptureResult> completion
        )
        {
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
            if (committedFrame == 0)
            {
                throw new ArgumentOutOfRangeException(nameof(committedFrame));
            }
            captureCompletion = completion ?? throw new ArgumentNullException(nameof(completion));
            BeginOperation();
            StartCoroutine(CaptureAfterEndOfFrame(generation, committedFrame));
        }

        public static DittoNativeCaptureResult ProcessLost() =>
            new DittoNativeCaptureResult.Unavailable(
                Failure("The player exited before a responsive failure frame was captured.")
            );

        private IEnumerator CaptureAfterEndOfFrame(int requestedGeneration, ulong frame)
        {
            yield return new WaitForEndOfFrame();
            if (requestedGeneration != generation || captureCompletion is null)
            {
                yield break;
            }
            if (Screen.width != width || Screen.height != height)
            {
                FailCapture($"Framebuffer dimensions changed to {Screen.width}x{Screen.height}.");
                yield break;
            }
            try
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
                ScreenCapture.CaptureScreenshotIntoRenderTexture(framebuffer);
                AsyncGPUReadback.Request(
                    framebuffer,
                    0,
                    request =>
                    {
                        bool failed = request.hasError;
                        byte[] pixels = failed
                            ? Array.Empty<byte>()
                            : request.GetData<byte>().ToArray();
                        completions.Enqueue(() =>
                            CompleteCapture(requestedGeneration, frame, pixels, failed)
                        );
                    }
                );
            }
            catch (Exception exception)
            {
                FailCapture(exception.Message);
            }
        }

        private void CompleteProbe(int requestedGeneration, byte[] pixels, bool readbackFailed)
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
                    "The startup probe did not preserve dimensions, alpha, rows, and channels."
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
                    detected!
                )
            );
        }

        private void CompleteCapture(
            int requestedGeneration,
            ulong frame,
            byte[] pixels,
            bool readbackFailed
        )
        {
            if (requestedGeneration != generation || captureCompletion is null)
            {
                return;
            }
            if (readbackFailed)
            {
                FailCapture("The framebuffer GPU readback failed.");
                return;
            }
            if (!DittoCapturePixels.TryEncode(pixels, width, height, layout!, out byte[] png))
            {
                FailCapture("The framebuffer could not be encoded as a valid PNG.");
                return;
            }
            System.Action<DittoNativeCaptureResult> completion = captureCompletion;
            captureCompletion = null;
            completion(new DittoNativeCaptureResult.Captured(png, width, height, frame));
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
            else if (captureCompletion is not null)
            {
                FailCapture("Framebuffer capture timed out.");
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

        private void FailCapture(string reason)
        {
            System.Action<DittoNativeCaptureResult>? completion = captureCompletion;
            captureCompletion = null;
            generation++;
            completion?.Invoke(new DittoNativeCaptureResult.Unavailable(Failure(reason)));
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
            if (probeCompletion is not null || captureCompletion is not null)
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
                    _ => null,
                };

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
            FailCapture("The capture adapter was destroyed before capture completed.");
            Release(probeTexture);
            Release(framebuffer);
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
    }

    internal static class DittoCapturePixels
    {
        internal static readonly Color32[] ProbeColors =
        {
            new(11, 23, 47, 61),
            new(11, 23, 47, 61),
            new(131, 149, 167, 181),
            new(131, 149, 167, 181),
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
