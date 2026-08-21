#nullable enable

using System;
using System.Collections;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using Unity.Profiling;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.InputSystem.LowLevel;

namespace Masonry.Performance
{
    /// <summary>Runs Task 38's diagnostic pointer-response-tween scenario.</summary>
    [DisallowMultipleComponent]
    public sealed class MasonryPerformanceSmoke : MonoBehaviour
    {
        private static readonly string[] MarkerNames =
        {
            "Masonry.Frame",
            "Masonry.Poll",
            "Masonry.Serialization",
            "Masonry.Transport",
            "Masonry.Response.Parse",
            "Masonry.Response.Apply",
        };

        [SerializeField]
        private MasonryRunner runner = null!;

        private readonly List<(string Name, ProfilerRecorder Recorder)> markers = new();
        private PerformanceSmokeTransport transport = null!;
        private ProfilerRecorder allocations;
        private bool finished;
        private bool recordersStarted;
        private float startedAt;

        private IEnumerator Start()
        {
            startedAt = Time.realtimeSinceStartup;
            transport = new PerformanceSmokeTransport();
            runner.Configure(
                new MasonryRunnerOptions(
                    transport,
                    new MasonryAddressablesAssetStorage(),
                    MasonryMessagePack.Instance
                )
            );
            runner.Connect();

            yield return WaitForTarget();
            if (finished)
            {
                yield break;
            }
            StartRecorders();
            yield return ClickTarget();
            if (finished)
            {
                yield break;
            }
            yield return WaitForTween();
            if (finished)
            {
                yield break;
            }
            yield return null;
            Finish(0, null);
        }

        private IEnumerator WaitForTarget()
        {
            while (!runner.IsInputAvailable || Target() == null)
            {
                if (!RequireWithinTimeout("snapshot preparation"))
                {
                    yield break;
                }
                yield return null;
            }
        }

        private IEnumerator ClickTarget()
        {
            Mouse mouse = Mouse.current ?? InputSystem.AddDevice<Mouse>();
            Camera camera = Camera.main;
            if (!camera)
            {
                camera = FindAnyObjectByType<Camera>();
            }
            Vector2 position = camera.WorldToScreenPoint(Target()!.transform.position);

            InputSystem.QueueStateEvent(mouse, new MouseState { position = position });
            yield return null;
            InputSystem.QueueStateEvent(
                mouse,
                new MouseState { position = position }.WithButton(MouseButton.Left)
            );
            yield return null;
            InputSystem.QueueStateEvent(mouse, new MouseState { position = position });

            while (transport.ClickCount == 0)
            {
                if (!RequireWithinTimeout("pointer click response"))
                {
                    yield break;
                }
                yield return null;
            }
        }

        private IEnumerator WaitForTween()
        {
            while (Target()!.transform.localPosition.x < 1.99f)
            {
                if (!RequireWithinTimeout("500 ms movement tween"))
                {
                    yield break;
                }
                yield return null;
            }
        }

        private void StartRecorders()
        {
            allocations = ProfilerRecorder.StartNew(
                ProfilerCategory.Memory,
                "GC Allocated In Frame",
                256
            );
            markers.AddRange(
                MarkerNames.Select(name =>
                    (name, ProfilerRecorder.StartNew(ProfilerCategory.Scripts, name, 256))
                )
            );
            recordersStarted = true;
        }

        private void Finish(int exitCode, string? failure)
        {
            if (finished)
            {
                return;
            }
            finished = true;
            try
            {
                string reportPath = ReportPath();
                File.WriteAllText(reportPath, Report(failure));
                Debug.Log($"MASONRY_PERFORMANCE_REPORT:{reportPath}");
            }
            catch (Exception exception)
            {
                Debug.LogError($"MASONRY_PERFORMANCE_FAILED:{exception}");
                exitCode = 1;
            }
            finally
            {
                if (recordersStarted)
                {
                    allocations.Dispose();
                    foreach ((string _, ProfilerRecorder recorder) in markers)
                    {
                        recorder.Dispose();
                    }
                }
            }

            Application.Quit(exitCode);
        }

        private string Report(string? failure)
        {
            var lines = new List<string>
            {
                "Masonry performance smoke (diagnostic; compare trends, not hardware)",
                "scenario: pointer click -> immediate response -> 500 ms tween",
            };
            if (recordersStarted)
            {
                lines.Add($"frames: {allocations.Count}");
                lines.Add(SampleLine("GC Allocated In Frame", allocations, "bytes", 1));
                lines.AddRange(
                    markers.Select(marker => SampleLine(marker.Name, marker.Recorder, "us", 0.001))
                );
            }
            if (failure != null)
            {
                lines.Add($"failure: {failure}");
            }
            return string.Join(Environment.NewLine, lines) + Environment.NewLine;
        }

        private static string SampleLine(
            string name,
            ProfilerRecorder recorder,
            string unit,
            double scale
        )
        {
            ProfilerRecorderSample[] samples = recorder.ToArray();
            long total = samples.Sum(sample => sample.Value);
            long maximum = samples.Length == 0 ? 0 : samples.Max(sample => sample.Value);
            return string.Format(
                CultureInfo.InvariantCulture,
                "{0}: samples={1} total={2:0.###}{4} max={3:0.###}{4}",
                name,
                samples.Length,
                total * scale,
                maximum * scale,
                unit
            );
        }

        private static string ReportPath()
        {
            string[] arguments = Environment.GetCommandLineArgs();
            int option = Array.IndexOf(arguments, "--masonry-performance-report");
            if (option < 0 || option + 1 >= arguments.Length)
            {
                throw new InvalidOperationException(
                    "--masonry-performance-report PATH is required."
                );
            }
            return arguments[option + 1];
        }

        private GameObject? Target()
        {
            MasonryIdentity? identity = FindObjectsByType<MasonryIdentity>()
                .SingleOrDefault(candidate =>
                    candidate.Id == PerformanceSmokeTransport.TargetId.Value
                );
            return identity ? identity.gameObject : null;
        }

        private bool RequireWithinTimeout(string phase)
        {
            if (Time.realtimeSinceStartup - startedAt <= 20)
            {
                return true;
            }
            Finish(1, $"Timed out during {phase}.");
            return false;
        }
    }
}
