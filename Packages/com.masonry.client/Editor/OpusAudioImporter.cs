#nullable enable

using System;
using System.Diagnostics;
using System.IO;
using UnityEditor.AssetImporters;
using UnityEngine;

namespace Masonry.Editor
{
    /// <summary>Imports Ogg Opus sources as Unity AudioClips.</summary>
    [ScriptedImporter(1, "opus")]
    public sealed class OpusAudioImporter : ScriptedImporter
    {
        private const int ChannelCount = 2;
        private const int SampleRate = 48_000;

        public override void OnImportAsset(AssetImportContext context)
        {
            try
            {
                byte[] pcm = Decode(context.assetPath);
                int sampleFrames = pcm.Length / sizeof(float) / ChannelCount;
                var samples = new float[sampleFrames * ChannelCount];
                Buffer.BlockCopy(pcm, 0, samples, 0, pcm.Length);
                AudioClip clip = AudioClip.Create(
                    Path.GetFileNameWithoutExtension(context.assetPath),
                    sampleFrames,
                    ChannelCount,
                    SampleRate,
                    false
                );
                if (!clip.SetData(samples, 0))
                {
                    throw new InvalidOperationException("Unity rejected the decoded Opus samples.");
                }

                context.AddObjectToAsset("AudioClip", clip);
                context.SetMainObject(clip);
            }
            catch (Exception exception)
            {
                context.LogImportError($"Could not import Opus audio: {exception.Message}");
            }
        }

        private static byte[] Decode(string assetPath)
        {
            string executable = Environment.GetEnvironmentVariable("MASONRY_FFMPEG") ?? "ffmpeg";
            var startInfo = new ProcessStartInfo
            {
                FileName = executable,
                Arguments =
                    $"-v error -i {Quote(Path.GetFullPath(assetPath))} -f f32le -acodec pcm_f32le "
                    + $"-ar {SampleRate} -ac {ChannelCount} pipe:1",
                CreateNoWindow = true,
                RedirectStandardError = true,
                RedirectStandardOutput = true,
                UseShellExecute = false,
            };
            using Process process =
                Process.Start(startInfo)
                ?? throw new InvalidOperationException($"Could not start '{executable}'.");
            using var output = new MemoryStream();
            process.StandardOutput.BaseStream.CopyTo(output);
            string error = process.StandardError.ReadToEnd();
            process.WaitForExit();
            if (process.ExitCode != 0)
            {
                throw new InvalidOperationException(
                    $"'{executable}' exited with code {process.ExitCode}: {error.Trim()}"
                );
            }
            if (output.Length == 0 || output.Length % (sizeof(float) * ChannelCount) != 0)
            {
                throw new InvalidDataException("The decoder returned invalid stereo PCM data.");
            }

            return output.ToArray();
        }

        private static string Quote(string value) => $"\"{value.Replace("\"", "\\\"")}\"";
    }
}
