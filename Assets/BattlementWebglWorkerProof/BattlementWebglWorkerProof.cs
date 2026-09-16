#nullable enable

using System;
using System.Runtime.InteropServices;
using UnityEngine;
using Stopwatch = System.Diagnostics.Stopwatch;

namespace Battlement.Integration
{
    /// <summary>Drives the release worker proof inside a browser-hosted Unity player.</summary>
    [DisallowMultipleComponent]
    public sealed class BattlementWebglWorkerProof : MonoBehaviour
    {
        public const string ScenePath =
            "Assets/BattlementWebglWorkerProof/BattlementWebglWorkerProof.unity";
        private const string Scenario = "fixture.release.worker-cancellation";
#if UNITY_WEBGL && !UNITY_EDITOR
        private const string NativeLibrary = "__Internal";
#else
        private const string NativeLibrary = "battlement_rules";
#endif

        [SerializeField]
        private BattlementRunner runner = null!;

        private int phase;
        private bool menuOpen;

        private void Start()
        {
            Type schemaType = Type.GetType(
                "Battlement.CustomFixtures.FixtureFlatBufferResponseSchema, Battlement.Integration",
                true
            )!;
            object schema =
                Activator.CreateInstance(schemaType)
                ?? throw new InvalidOperationException("Fixture schema could not be created.");
            runner.Configure(
                new BattlementRunnerOptions(
                    new BattlementNativeTransport(),
                    new BattlementAddressablesAssetStorage(),
                    customCommandTypes: new[] { Scenario },
                    flatBufferResponseSchema: (IBattlementFlatBufferResponseViewSchema)schema,
                    flatBufferClientSchema: (IBattlementFlatBufferClientSchema)schema
                )
            );
            runner.Connect();
            Debug.Log("BATTLEMENT_WEBGL_WORKER_READY");
        }

        private void Update()
        {
            if (phase == 0 && Observation(6) == 1)
            {
                phase = 1;
                Require(Observation(9) == 1, "rules action ran on the Unity UI thread");
                Debug.Log("BATTLEMENT_WEBGL_WORKER_STARTED_OFF_UI");
            }
            else if (
                phase == 2
                && Observation(1) >= 1
                && Observation(2) >= 1
                && Observation(5) == 2
                && Observation(7) == 1
            )
            {
                phase = 3;
                Debug.Log("BATTLEMENT_WEBGL_WORKER_CANCELLED_CLEANED_STOPPED");
            }
            else if (
                phase == 4
                && Observation(0) == 3
                && Observation(1) == 3
                && Observation(2) == 2
                && Observation(4) == 1
                && Observation(8) == 5
            )
            {
                phase = 5;
                Debug.Log("BATTLEMENT_WEBGL_WORKER_LATEST_REPLACEMENT_ONLY");
            }
            else if (phase == 6 && Observation(3) == 1 && Observation(1) == 4)
            {
                phase = 7;
                Debug.Log("BATTLEMENT_WEBGL_WORKER_REAL_PANIC_FAILED");
            }
            else if (phase == 8 && Observation(4) == 2 && Observation(1) == 5)
            {
                phase = 9;
                Debug.Log("BATTLEMENT_WEBGL_WORKER_RECOVERED");
            }
            else if (phase == 10 && Observation(1) == 6)
            {
                phase = 11;
                Debug.Log("BATTLEMENT_WEBGL_WORKER_NONJOINING_EXIT_CLEANED");
            }
        }

        private void OnGUI()
        {
            GUI.Box(new UnityEngine.Rect(20, 20, 420, 250), "Threaded WebGL cancellation proof");
            GUI.Label(new UnityEngine.Rect(40, 55, 380, 40), Status());
            if (
                phase == 1
                && GUI.Button(new UnityEngine.Rect(40, 100, 180, 42), "Cancel + open menu")
            )
            {
                menuOpen = true;
                runner.Reconnect();
                phase = 2;
                Debug.Log("BATTLEMENT_WEBGL_MENU_OPENED_DURING_CANCEL");
            }
            if (
                phase == 3
                && GUI.Button(new UnityEngine.Rect(40, 100, 180, 42), "Replace held worker")
            )
            {
                runner.Reconnect();
                runner.Reconnect();
                runner.Reconnect();
                Require(Observation(0) == 2, "held computation accumulated rules workers");
                ReleaseComputation();
                phase = 4;
            }
            if (phase == 5 && GUI.Button(new UnityEngine.Rect(40, 100, 180, 42), "Run real panic"))
            {
                runner.Reconnect();
                phase = 6;
            }
            if (phase == 7 && GUI.Button(new UnityEngine.Rect(40, 100, 180, 42), "Run replacement"))
            {
                runner.Reconnect();
                phase = 8;
            }
            if (
                phase == 9
                && GUI.Button(new UnityEngine.Rect(40, 100, 180, 42), "Dispose held worker")
            )
            {
                runner.Reconnect();
                phase = 10;
                StartCoroutine(DisposeWhenComputing());
            }
            if (menuOpen)
            {
                GUI.Box(
                    new UnityEngine.Rect(250, 100, 160, 95),
                    "Local menu\nUI remained responsive"
                );
            }
        }

        private System.Collections.IEnumerator DisposeWhenComputing()
        {
            while (Observation(7) == 0)
                yield return null;
            var stopwatch = Stopwatch.StartNew();
            runner.Stop();
            runner.Dispose();
            stopwatch.Stop();
            Require(stopwatch.ElapsedMilliseconds < 500, "worker disposal joined the rules thread");
            Debug.Log($"BATTLEMENT_WEBGL_WORKER_NONJOINING_EXIT:{stopwatch.ElapsedMilliseconds}");
            ReleaseComputation();
        }

        private string Status() =>
            phase switch
            {
                0 => "Connected — click the left Rust cube to start",
                1 => "Worker started off the UI thread",
                2 => "Cancelling while the local menu opens",
                3 => "Nested destructors ran before the worker stopped",
                4 => "Releasing the finite-pool replacement",
                5 => "Only the latest replacement ran",
                6 => "Running a genuine rules panic",
                7 => "Panic reported as execution failure",
                8 => "Running after the panic",
                9 => "Replacement succeeded; ready to test exit",
                10 => "Disposing with held computation",
                11 => "PASS — threaded unwind proof complete",
                _ => "Waiting",
            };

        private static ulong Observation(uint index) => WorkerObservation(index).ToUInt64();

        private static void Require(bool condition, string message)
        {
            if (!condition)
                throw new InvalidOperationException(message);
        }

        [DllImport(NativeLibrary, EntryPoint = "fixture_worker_observation")]
        private static extern UIntPtr WorkerObservation(uint index);

        [DllImport(NativeLibrary, EntryPoint = "fixture_worker_release_computation")]
        private static extern void ReleaseComputation();
    }
}
