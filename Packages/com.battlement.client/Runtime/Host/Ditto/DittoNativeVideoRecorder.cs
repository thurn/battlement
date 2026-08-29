#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using System.Security.Cryptography;

namespace Battlement
{
    internal sealed class DittoNativeVideoRecorder : IDisposable
    {
        private const ulong FramesPerSecond = 30;
        private const ulong MediaReserveBytes = 64 * 1024 * 1024;

        private readonly string directory;
        private readonly string reportedDirectory;
        private readonly uint width;
        private readonly uint height;
        private readonly List<DittoNativeVideoInput> inputs = new();
        private FileStream? stream;
        private string? inputId;
        private string? path;
        private uint startStepIndex;
        private ulong maximumFrames;
        private ulong frameCount;
        private TimeSpan startedAt;
        private bool awaitingStop;

        public DittoNativeVideoRecorder(
            string directory,
            uint width,
            uint height,
            string? reportedDirectory = null
        )
        {
            this.directory = directory ?? throw new ArgumentNullException(nameof(directory));
            this.reportedDirectory = reportedDirectory ?? directory;
            if (width == 0 || height == 0)
            {
                throw new ArgumentOutOfRangeException(nameof(width));
            }
            this.width = width;
            this.height = height;
        }

        public IReadOnlyList<DittoNativeVideoInput> Inputs => inputs;

        public bool IsActive => stream is not null;

        public static ulong RequiredBytes(uint width, uint height, ulong maxDurationMs)
        {
            if (width == 0 || height == 0 || maxDurationMs == 0)
            {
                throw new ArgumentOutOfRangeException(nameof(maxDurationMs));
            }
            ulong frames = checked(checked(maxDurationMs * FramesPerSecond) + 999) / 1_000;
            ulong raw = checked(checked(checked((ulong)width * height) * 4) * frames);
            return checked(raw + MediaReserveBytes);
        }

        public string Begin(uint stepIndex, ulong maxDurationMs, TimeSpan elapsed)
        {
            if (IsActive || awaitingStop)
            {
                throw new InvalidOperationException("A native video is already active.");
            }
            maximumFrames = checked(checked(maxDurationMs * FramesPerSecond) + 999) / 1_000;
            if (maximumFrames == 0)
            {
                throw new ArgumentOutOfRangeException(nameof(maxDurationMs));
            }
            Directory.CreateDirectory(directory);
            inputId = Guid.NewGuid().ToString("D");
            path = Path.Combine(directory, inputId + ".rgba");
            stream = new FileStream(path, FileMode.CreateNew, FileAccess.Write, FileShare.None);
            startStepIndex = stepIndex;
            frameCount = 0;
            startedAt = elapsed;
            awaitingStop = true;
            return inputId;
        }

        public bool AppendFrame(byte[] pixels, DittoCapturePixelLayout layout, TimeSpan elapsed)
        {
            if (stream is null)
            {
                return false;
            }
            byte[] rgba = DittoCapturePixels.TopDownRgba(pixels, width, height, layout);
            long elapsedTicks = checked((elapsed - startedAt).Ticks);
            if (elapsedTicks < 0)
            {
                throw new ArgumentOutOfRangeException(nameof(elapsed));
            }
            ulong dueFrames = Math.Min(
                maximumFrames,
                checked(
                    checked((ulong)elapsedTicks * FramesPerSecond)
                    + (ulong)TimeSpan.TicksPerSecond / 2
                ) / TimeSpan.TicksPerSecond
            );
            while (frameCount < dueFrames)
            {
                stream.Write(rgba, 0, rgba.Length);
                frameCount++;
            }
            if (frameCount < maximumFrames)
            {
                return false;
            }
            FinalizeInput(true);
            return true;
        }

        public void Stop()
        {
            if (!awaitingStop)
            {
                throw new InvalidOperationException("Video stop has no matching start.");
            }
            if (stream is not null)
            {
                FinalizeInput(false);
            }
            awaitingStop = false;
        }

        public bool TruncateForRuntimeFailure()
        {
            if (stream is null)
            {
                return true;
            }
            if (frameCount == 0)
            {
                stream.Dispose();
                stream = null;
                File.Delete(path!);
                return false;
            }
            FinalizeInput(true);
            return true;
        }

        public void Dispose()
        {
            stream?.Dispose();
            stream = null;
        }

        private void FinalizeInput(bool truncated)
        {
            if (frameCount == 0)
            {
                throw new InvalidOperationException("The native video contains no complete frame.");
            }
            stream!.Flush(true);
            stream.Dispose();
            stream = null;
            byte[] bytes = File.ReadAllBytes(path!);
            long expected = checked((long)frameCount * width * height * 4);
            if (bytes.LongLength != expected)
            {
                throw new InvalidDataException(
                    $"The native video contains {bytes.LongLength} bytes; expected {expected}."
                );
            }
            using SHA256 hash = SHA256.Create();
            inputs.Add(
                new DittoNativeVideoInput(
                    inputId!,
                    startStepIndex,
                    Path.Combine(reportedDirectory, Path.GetFileName(path!)),
                    BitConverter
                        .ToString(hash.ComputeHash(bytes))
                        .Replace("-", "")
                        .ToLowerInvariant(),
                    width,
                    height,
                    frameCount,
                    truncated
                )
            );
        }
    }
}
