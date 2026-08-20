#nullable enable

using System;
using System.Collections;
using System.Collections.Concurrent;
using System.Diagnostics;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using UnityEngine;
using UnityEngine.Experimental.Rendering;
using UnityEngine.Rendering;

namespace Masonry.VisualCapture
{
    internal readonly struct MediaResult
    {
        internal MediaResult(bool success, string error, int encoderPid = 0, int frames = 0) =>
            (Success, Error, EncoderPid, Frames) = (success, error, encoderPid, frames);

        internal bool Success { get; }
        internal string Error { get; }
        internal int EncoderPid { get; }
        internal int Frames { get; }
    }

    internal sealed class MasonryFrameRecorder : MonoBehaviour
    {
        private readonly ConcurrentQueue<System.Action> completions = new();
        private readonly object frameLock = new();
        private System.Action<MediaResult>? pendingPng;
        private System.Action<MediaResult>? pendingVideo;
        private byte[]? latestFrame;
        private Process? encoder;
        private Task<string>? encoderError;
        private RenderTexture? renderTexture;
        private string pendingPngPath = string.Empty;
        private bool pointerVisible;
        private bool readbackPending;
        private int pointerX;
        private int pointerY;
        private Thread? writerThread;

        internal void CapturePng(string outputPath, System.Action<MediaResult> completion)
        {
            if (pendingPng is not null)
            {
                throw new InvalidOperationException("A PNG capture is already pending.");
            }
            pendingPngPath = outputPath;
            pendingPng = completion;
        }

        internal void StartVideo(CaptureCommand command, System.Action<MediaResult> started)
        {
            if (writerThread is not null)
            {
                throw new InvalidOperationException("Video capture is already active.");
            }
            if (command.Width != Screen.width || command.Height != Screen.height)
            {
                throw new InvalidOperationException(
                    $"Video dimensions {command.Width}x{command.Height} do not match "
                        + $"the player framebuffer {Screen.width}x{Screen.height}."
                );
            }
            pendingVideo = started;
            StartCoroutine(BeginVideoWhenFrameAvailable(command));
        }

        internal void SetPointer(Vector2 position)
        {
            pointerX = Mathf.RoundToInt(position.x);
            pointerY = Mathf.RoundToInt(position.y);
            pointerVisible = true;
        }

        internal void Stop()
        {
            try
            {
                encoder?.StandardInput.Close();
                if (encoder is { HasExited: false })
                {
                    encoder.Kill();
                }
            }
            catch (InvalidOperationException) { }
            writerThread?.Join(TimeSpan.FromSeconds(2));
        }

        private void Start()
        {
            renderTexture = new RenderTexture(
                Screen.width,
                Screen.height,
                0,
                GraphicsFormat.B8G8R8A8_UNorm
            );
            renderTexture.Create();
            StartCoroutine(CaptureFrames());
        }

        private IEnumerator CaptureFrames()
        {
            while (true)
            {
                yield return new WaitForEndOfFrame();
                if (readbackPending || renderTexture == null)
                {
                    continue;
                }

                ScreenCapture.CaptureScreenshotIntoRenderTexture(renderTexture);
                readbackPending = true;
                AsyncGPUReadback.Request(
                    renderTexture,
                    0,
                    TextureFormat.BGRA32,
                    request =>
                    {
                        readbackPending = false;
                        if (request.hasError)
                        {
                            completions.Enqueue(() =>
                                FailPending("GPU framebuffer readback failed.")
                            );
                            return;
                        }

                        byte[] frame = request.GetData<byte>().ToArray();
                        lock (frameLock)
                        {
                            latestFrame = frame;
                        }
                        completions.Enqueue(CompletePng);
                    }
                );
            }
        }

        private IEnumerator BeginVideoWhenFrameAvailable(CaptureCommand command)
        {
            float deadline = Time.realtimeSinceStartup + 10;
            while (LatestFrame() is null && Time.realtimeSinceStartup < deadline)
            {
                yield return null;
            }
            if (LatestFrame() is null)
            {
                MediaResult failure = new(false, "The framebuffer produced no video frame.");
                pendingVideo?.Invoke(failure);
                pendingVideo = null;
                yield break;
            }

            string temporaryPath = command.OutputPath + ".new.mp4";
            encoder = StartEncoder(command, temporaryPath);
            encoderError = encoder.StandardError.ReadToEndAsync();
            pendingVideo?.Invoke(new MediaResult(true, string.Empty, encoder.Id));
            pendingVideo = null;
            writerThread = new Thread(() => WriteVideo(command, temporaryPath))
            {
                IsBackground = true,
                Name = "Masonry visual capture writer",
            };
            writerThread.Start();
        }

        private static Process StartEncoder(CaptureCommand command, string temporaryPath)
        {
            var arguments = new ArgumentListBuilder()
                .Add("-hide_banner", "-loglevel", "error", "-y")
                .Add("-f", "rawvideo", "-pixel_format", "bgra")
                .Add("-video_size", $"{command.Width}x{command.Height}")
                .Add("-framerate", command.FrameRate.ToString(), "-i", "pipe:0")
                .Add("-an", "-c:v", "h264_videotoolbox")
                .Add("-pix_fmt", "yuv420p", "-movflags", "+faststart", temporaryPath);
            var start = new ProcessStartInfo(command.FfmpegPath)
            {
                Arguments = arguments.ToString(),
                CreateNoWindow = true,
                RedirectStandardInput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
            };
            return Process.Start(start)
                ?? throw new InvalidOperationException("FFmpeg did not start.");
        }

        private void WriteVideo(CaptureCommand command, string temporaryPath)
        {
            int totalFrames = checked(command.DurationSeconds * command.FrameRate);
            var clock = Stopwatch.StartNew();
            try
            {
                for (int index = 0; index < totalFrames; index++)
                {
                    TimeSpan target = TimeSpan.FromSeconds((double)(index + 1) / command.FrameRate);
                    TimeSpan remaining = target - clock.Elapsed;
                    if (remaining > TimeSpan.Zero)
                    {
                        Thread.Sleep(remaining);
                    }

                    byte[] frame =
                        LatestFrame()
                        ?? throw new InvalidOperationException("The latest framebuffer was lost.");
                    CompositePointer(frame, command.Width, command.Height);
                    encoder!.StandardInput.BaseStream.Write(frame, 0, frame.Length);
                }
                encoder!.StandardInput.Close();
                encoder.WaitForExit();
                string error = encoderError?.GetAwaiter().GetResult() ?? string.Empty;
                if (encoder.ExitCode != 0)
                {
                    throw new InvalidOperationException(
                        string.IsNullOrWhiteSpace(error) ? "FFmpeg failed." : error.Trim()
                    );
                }
                ReplaceFile(temporaryPath, command.OutputPath);
                CompleteVideo(
                    command.OutputPath,
                    new MediaResult(true, string.Empty, encoder.Id, totalFrames)
                );
            }
            catch (Exception exception)
            {
                if (File.Exists(temporaryPath))
                {
                    File.Delete(temporaryPath);
                }
                CompleteVideo(command.OutputPath, new MediaResult(false, exception.Message));
            }
        }

        private byte[]? LatestFrame()
        {
            lock (frameLock)
            {
                return latestFrame is null ? null : (byte[])latestFrame.Clone();
            }
        }

        private void CompositePointer(byte[] frame, int width, int height)
        {
            if (!pointerVisible)
            {
                return;
            }
            int centerX = Mathf.Clamp(pointerX, 0, width - 1);
            int centerY = Mathf.Clamp(height - 1 - pointerY, 0, height - 1);
            for (int offset = -7; offset <= 7; offset++)
            {
                SetPixel(frame, width, height, centerX + offset, centerY, 255);
                SetPixel(frame, width, height, centerX, centerY + offset, 255);
            }
        }

        private static void SetPixel(byte[] frame, int width, int height, int x, int y, byte value)
        {
            if (x < 0 || x >= width || y < 0 || y >= height)
            {
                return;
            }
            int index = ((y * width) + x) * 4;
            frame[index] = value;
            frame[index + 1] = value;
            frame[index + 2] = value;
            frame[index + 3] = 255;
        }

        private static byte[] FlipRows(byte[] source, int width, int height)
        {
            int stride = checked(width * 4);
            var destination = new byte[source.Length];
            for (int row = 0; row < height; row++)
            {
                Buffer.BlockCopy(
                    source,
                    row * stride,
                    destination,
                    (height - row - 1) * stride,
                    stride
                );
            }
            return destination;
        }

        private void CompletePng()
        {
            if (pendingPng is null || latestFrame is null)
            {
                return;
            }
            try
            {
                var texture = new Texture2D(
                    Screen.width,
                    Screen.height,
                    TextureFormat.BGRA32,
                    false
                );
                texture.LoadRawTextureData(FlipRows(latestFrame, Screen.width, Screen.height));
                texture.Apply();
                string temporaryPath = pendingPngPath + ".new";
                File.WriteAllBytes(temporaryPath, texture.EncodeToPNG());
                Destroy(texture);
                ReplaceFile(temporaryPath, pendingPngPath);
                pendingPng(new MediaResult(true, string.Empty));
            }
            catch (Exception exception)
            {
                pendingPng(new MediaResult(false, exception.Message));
            }
            pendingPng = null;
        }

        private void CompleteVideo(string outputPath, MediaResult result) =>
            completions.Enqueue(() =>
                CaptureFiles.WriteJson(outputPath + ".capture.json", new VideoResult(result))
            );

        private static void ReplaceFile(string source, string destination)
        {
            if (File.Exists(destination))
            {
                File.Replace(source, destination, null);
                return;
            }
            File.Move(source, destination);
        }

        private void FailPending(string error)
        {
            pendingPng?.Invoke(new MediaResult(false, error));
            pendingPng = null;
            pendingVideo?.Invoke(new MediaResult(false, error));
            pendingVideo = null;
        }

        private void Update()
        {
            while (completions.TryDequeue(out System.Action completion))
            {
                completion();
            }
        }

        private void OnDestroy()
        {
            Stop();
            if (renderTexture != null)
            {
                renderTexture.Release();
                Destroy(renderTexture);
            }
        }

        [Serializable]
        private sealed class VideoResult
        {
            internal VideoResult(MediaResult result)
            {
                success = result.Success;
                error = result.Error;
                encoderPid = result.EncoderPid;
                frames = result.Frames;
            }

            [SerializeField]
            private bool success;

            [SerializeField]
            private string error;

            [SerializeField]
            private int encoderPid;

            [SerializeField]
            private int frames;
        }

        private sealed class ArgumentListBuilder
        {
            private readonly System.Collections.Generic.List<string> arguments = new();

            internal ArgumentListBuilder Add(params string[] values)
            {
                foreach (string value in values)
                {
                    arguments.Add("\"" + value.Replace("\\", "\\\\").Replace("\"", "\\\"") + "\"");
                }
                return this;
            }

            public override string ToString() => string.Join(" ", arguments);
        }
    }
}
