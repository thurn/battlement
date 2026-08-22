#nullable enable

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEditor.AddressableAssets.Settings;
using UnityEditor.AssetImporters;
using UnityEngine;

namespace Masonry.Editor
{
    /// <summary>Imports Ogg Opus sources as Unity AudioClips.</summary>
    [ScriptedImporter(1, "opus")]
    public sealed class OpusAudioImporter : ScriptedImporter
    {
        internal const int ChannelCount = 2;
        internal const int SampleRate = 48_000;

        public override void OnImportAsset(AssetImportContext context)
        {
            try
            {
                byte[] pcm = OpusTranscoder.DecodePcm(context.assetPath);
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
    }

    internal sealed class OpusBuildAssets : IDisposable
    {
        private const string GeneratedDirectory = "Assets/Generated/MasonryOpus";

        private readonly Dictionary<string, byte[]> groupFiles = new();
        private readonly AddressableAssetSettings settings;
        private readonly List<EntrySwap> swaps = new();

        private OpusBuildAssets(AddressableAssetSettings settings) => this.settings = settings;

        internal static OpusBuildAssets Prepare(AddressableAssetSettings settings)
        {
            var assets = new OpusBuildAssets(settings);
            try
            {
                assets.PrepareEntries();
                return assets;
            }
            catch
            {
                assets.Dispose();
                throw;
            }
        }

        public void Dispose()
        {
            foreach (EntrySwap swap in swaps)
            {
                settings.RemoveAssetEntry(swap.GeneratedGuid, false);
                AddressableAssetEntry entry = settings.CreateOrMoveEntry(
                    swap.SourceGuid,
                    swap.Group,
                    false,
                    false
                );
                RestoreEntry(entry, swap.Address, swap.Labels);
            }

            AssetDatabase.SaveAssets();
            foreach (EntrySwap swap in swaps)
            {
                AssetDatabase.DeleteAsset(swap.GeneratedPath);
            }
            if (
                Directory.Exists(GeneratedDirectory)
                && !Directory.EnumerateFileSystemEntries(GeneratedDirectory).Any()
            )
            {
                AssetDatabase.DeleteAsset(GeneratedDirectory);
            }
            foreach (KeyValuePair<string, byte[]> groupFile in groupFiles)
            {
                File.WriteAllBytes(groupFile.Key, groupFile.Value);
                AssetDatabase.ImportAsset(groupFile.Key, ImportAssetOptions.ForceSynchronousImport);
            }
        }

        private void PrepareEntries()
        {
            AddressableAssetEntry[] entries = settings
                .groups.Where(group => group != null)
                .SelectMany(group => group.entries)
                .Where(IsOpusEntry)
                .ToArray();
            foreach (
                AddressableAssetGroup group in entries.Select(entry => entry.parentGroup).Distinct()
            )
            {
                string path = AssetDatabase.GetAssetPath(group);
                groupFiles.Add(path, File.ReadAllBytes(path));
            }
            Directory.CreateDirectory(GeneratedDirectory);
            foreach (AddressableAssetEntry entry in entries)
            {
                string sourceGuid = entry.guid;
                string generatedPath = $"{GeneratedDirectory}/{sourceGuid}.wav";
                OpusTranscoder.TranscodeWav(
                    AssetDatabase.GUIDToAssetPath(sourceGuid),
                    generatedPath
                );
                AssetDatabase.ImportAsset(generatedPath, ImportAssetOptions.ForceSynchronousImport);
                string generatedGuid = AssetDatabase.AssetPathToGUID(generatedPath);
                if (string.IsNullOrEmpty(generatedGuid))
                {
                    throw new InvalidOperationException(
                        $"Unity did not import generated audio '{generatedPath}'."
                    );
                }

                var swap = new EntrySwap(
                    sourceGuid,
                    generatedGuid,
                    generatedPath,
                    entry.parentGroup,
                    entry.address,
                    entry.labels.ToArray()
                );
                swaps.Add(swap);
                settings.RemoveAssetEntry(sourceGuid, false);
                AddressableAssetEntry generated = settings.CreateOrMoveEntry(
                    generatedGuid,
                    swap.Group,
                    false,
                    false
                );
                RestoreEntry(generated, swap.Address, swap.Labels);
            }

            AssetDatabase.SaveAssets();
        }

        private static bool IsOpusEntry(AddressableAssetEntry entry) =>
            AssetDatabase
                .GUIDToAssetPath(entry.guid)
                .EndsWith(".opus", StringComparison.OrdinalIgnoreCase);

        private static void RestoreEntry(
            AddressableAssetEntry entry,
            string address,
            IReadOnlyCollection<string> labels
        )
        {
            entry.address = address;
            foreach (string label in labels)
            {
                entry.SetLabel(label, true, false, false);
            }
        }

        private sealed class EntrySwap
        {
            public EntrySwap(
                string sourceGuid,
                string generatedGuid,
                string generatedPath,
                AddressableAssetGroup group,
                string address,
                string[] labels
            )
            {
                SourceGuid = sourceGuid;
                GeneratedGuid = generatedGuid;
                GeneratedPath = generatedPath;
                Group = group;
                Address = address;
                Labels = labels;
            }

            public string SourceGuid { get; }

            public string GeneratedGuid { get; }

            public string GeneratedPath { get; }

            public AddressableAssetGroup Group { get; }

            public string Address { get; }

            public string[] Labels { get; }
        }
    }

    internal static class OpusTranscoder
    {
        internal static byte[] DecodePcm(string assetPath)
        {
            var startInfo = new ProcessStartInfo
            {
                FileName = Executable,
                Arguments =
                    $"-v error -i {Quote(Path.GetFullPath(assetPath))} -f f32le -acodec pcm_f32le "
                    + $"-ar {OpusAudioImporter.SampleRate} "
                    + $"-ac {OpusAudioImporter.ChannelCount} pipe:1",
                CreateNoWindow = true,
                RedirectStandardError = true,
                RedirectStandardOutput = true,
                UseShellExecute = false,
            };
            using Process process =
                Process.Start(startInfo)
                ?? throw new InvalidOperationException($"Could not start '{Executable}'.");
            using var output = new MemoryStream();
            process.StandardOutput.BaseStream.CopyTo(output);
            string error = process.StandardError.ReadToEnd();
            process.WaitForExit();
            if (process.ExitCode != 0)
            {
                throw new InvalidOperationException(
                    $"'{Executable}' exited with code {process.ExitCode}: {error.Trim()}"
                );
            }
            if (
                output.Length == 0
                || output.Length % (sizeof(float) * OpusAudioImporter.ChannelCount) != 0
            )
            {
                throw new InvalidDataException("The decoder returned invalid stereo PCM data.");
            }

            return output.ToArray();
        }

        internal static void TranscodeWav(string sourcePath, string outputPath)
        {
            var startInfo = new ProcessStartInfo
            {
                FileName = Executable,
                Arguments =
                    $"-v error -y -i {Quote(Path.GetFullPath(sourcePath))} -c:a pcm_s16le "
                    + $"-ar {OpusAudioImporter.SampleRate} -ac {OpusAudioImporter.ChannelCount} "
                    + Quote(Path.GetFullPath(outputPath)),
                CreateNoWindow = true,
                RedirectStandardError = true,
                UseShellExecute = false,
            };
            using Process process =
                Process.Start(startInfo)
                ?? throw new InvalidOperationException($"Could not start '{Executable}'.");
            string error = process.StandardError.ReadToEnd();
            process.WaitForExit();
            if (process.ExitCode != 0)
            {
                throw new InvalidOperationException(
                    $"'{Executable}' exited with code {process.ExitCode}: {error.Trim()}"
                );
            }
        }

        private static string Executable =>
            Environment.GetEnvironmentVariable("MASONRY_FFMPEG") ?? "ffmpeg";

        private static string Quote(string value) => $"\"{value.Replace("\"", "\\\"")}\"";
    }
}
