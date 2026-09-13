#nullable enable

using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using Battlement.CustomFixtures;
using NUnit.Framework;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.InputSystem.LowLevel;
using UnityEngine.SceneManagement;
using BattlementAction = Battlement.Action;
using FixtureWire = Battlement.FlatBuffers.FixtureGenerated;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    [Parallelizable(ParallelScope.None)]
    public sealed class BattlementReleaseScenarioTests : InputTestFixture
    {
        private static readonly ReleaseScenarioCase[] ScenarioCorpus =
        {
            new("batch-failures", RunBatchFailures),
            new("timing", RunTiming),
            new("snapshot-replacement", RunSnapshotReplacement),
            new("asset-lifetime", RunAssetLifetime),
            new("custom-failure", RunCustomFailure),
            new("pointer-input", RunPointerInput),
            new("fatal-reconnect", RunFatalReconnect),
        };

        private Mouse? mouse;

        public static IEnumerable Cases
        {
            get
            {
                foreach (ReleaseScenarioCase scenario in ScenarioCorpus)
                {
                    yield return new TestCaseData(scenario).SetName($"Release_{scenario.Name}");
                }
            }
        }

        [SetUp]
        public override void Setup()
        {
            base.Setup();
            mouse = InputSystem.AddDevice<Mouse>("Battlement Release Fixture Mouse");
        }

        [TearDown]
        public override void TearDown()
        {
            mouse = null;
            base.TearDown();
        }

        [TestCaseSource(nameof(Cases))]
        public void SharedCorpusRunsThroughNativePlugin(ReleaseScenarioCase scenario)
        {
            using (var host = ReleaseScenarioHost.Create(scenario.Name))
            {
                scenario.Run(host, mouse!);
            }

            Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
        }

        private static void RunBatchFailures(ReleaseScenarioHost host, Mouse mouse)
        {
            host.Connect();
            host.RunFrame();
            host.RunFrame();
            Assert.That(host.Runner.IsInputAvailable, Is.True, Diagnostics(host));

            Assert.That(HasIdentity(11), Is.True, "Earlier commands must remain applied.");
            Assert.That(HasIdentity(13), Is.False, "Commands after a failure must be skipped.");
            Assert.That(HasIdentity(10), Is.False, "Destroyed objects must leave the lookup.");
            Assert.That(
                host.Observer.BatchFailures.Select(failure => failure.ErrorCode),
                Does.Contain(CoreErrorCode.UnknownObject)
            );
            Assert.That(
                host.Logger.Records.Count(record =>
                    record.EventName == "battlement.batch.duplicate"
                ),
                Is.EqualTo(2)
            );
        }

        private static void RunTiming(ReleaseScenarioHost host, Mouse mouse)
        {
            host.Connect();
            host.RunFrame();
            Assert.That(host.Runner.IsInputAvailable, Is.True, Diagnostics(host));
            Assert.That(HasIdentity(20), Is.True);
            Assert.That(HasIdentity(21), Is.False);
            Assert.That(
                NativeFixture.fixture_outstanding_buffers(),
                Is.Not.EqualTo(UIntPtr.Zero),
                "The delayed native batch must retain its response lease across frames."
            );

            host.Clock.Advance(TimeSpan.FromMilliseconds(299));
            host.RunFrame();
            Assert.That(HasIdentity(21), Is.False);

            host.Clock.Advance(TimeSpan.FromMilliseconds(1));
            host.RunFrame();
            Assert.That(HasIdentity(21), Is.True, "Nonblocking work must not delay group three.");
            Assert.That(
                NativeFixture.fixture_outstanding_buffers(),
                Is.EqualTo(UIntPtr.Zero),
                "Completing the delayed batch must release its native response lease."
            );
        }

        private static void RunSnapshotReplacement(ReleaseScenarioHost host, Mouse mouse)
        {
            host.Connect();
            GameObject initial = Identity(30).gameObject;

            host.RunFrame();
            Assert.That(host.Runner.IsInputAvailable, Is.True, Diagnostics(host));
            Assert.That(Identity(30).transform.localPosition.x, Is.EqualTo(5));

            host.RunFrame();
            Assert.That(Identity(30).gameObject, Is.Not.SameAs(initial));
            Assert.That(Identity(30).transform.localPosition.x, Is.EqualTo(2));

            host.RunFrame();
            Assert.That(Identity(30).transform.localPosition.x, Is.EqualTo(3));
        }

        private static void RunAssetLifetime(ReleaseScenarioHost host, Mouse mouse)
        {
            var prefab = new PreparedAsset.Prefab(new PrefabAddress("fixture/release-prefab"));
            host.Connect();
            host.RunFrame();
            host.RunFrame();
            host.RunFrame();
            Assert.That(host.Runner.IsInputAvailable, Is.True, Diagnostics(host));

            Assert.That(host.AssetStorage.PrepareCalls, Does.Contain(prefab));
            Assert.That(HasIdentity(40), Is.True);
            Assert.That(host.Runner.TryGetPreparedAsset(prefab, out _), Is.True);
            Assert.That(
                host.Observer.BatchFailures.Last().ErrorCode,
                Is.EqualTo(CoreErrorCode.AssetInUse)
            );
            FakeAssetHandle handle = host.AssetStorage.Handles.Single(candidate =>
                candidate.Asset == prefab
            );
            Assert.That(handle.IsDisposed, Is.False, "The live prefab must retain its lease.");
        }

        private static void RunCustomFailure(ReleaseScenarioHost host, Mouse mouse)
        {
            ulong submitsBefore = NativeFixture.fixture_submit_calls().ToUInt64();
            var handler = new FixtureHandler(
                FixtureHandlerMode.EmitNestedActionAndReject,
                host.Runner
            );
            host.Runner.RegisterFlatBufferCommand<FixtureWire.FlashPayload, FixtureError>(
                "fixture.character.flash",
                handler
            );
            host.Connect();
            host.RunFrame();

            Assert.That(handler.InvocationCount, Is.EqualTo(1));
            Assert.That(handler.LastFlatBufferPayload.HasValue, Is.True);
            Assert.Throws<ObjectDisposedException>(() =>
                _ = handler.LastFlatBufferPayload!.Value.Scale
            );
            Assert.That(HasIdentity(51), Is.False);
            Assert.That(
                NativeFixture.fixture_submit_calls().ToUInt64(),
                Is.EqualTo(submitsBefore + 2),
                "Nested custom actions and typed custom failures must both reach Rust."
            );
        }

        private static void RunPointerInput(ReleaseScenarioHost host, Mouse mouse)
        {
            host.Connect();
            BattlementIdentity left = Identity(60);
            BattlementIdentity right = Identity(61);
            Object.DestroyImmediate(left.GetComponent<Collider>());
            var child = new GameObject("Release fixture child collider");
            child.transform.SetParent(left.transform, false);
            child.AddComponent<BoxCollider>();
            Physics.SyncTransforms();

            Camera camera = Identity(1).GetComponent<Camera>();
            UnityEngine.Vector2 leftPosition = camera.WorldToScreenPoint(left.transform.position);
            UnityEngine.Vector2 rightPosition = camera.WorldToScreenPoint(right.transform.position);
            Move(host, mouse, leftPosition, false);
            Move(host, mouse, leftPosition, true);
            Move(host, mouse, rightPosition, true);
            Move(host, mouse, leftPosition, true);
            Move(host, mouse, leftPosition, false);

            Assert.That(
                host.Observer.Actions.Select(action => action.Body.GetType().Name),
                Is.EqualTo(
                    new[]
                    {
                        nameof(ActionBody.PointerEnter),
                        nameof(ActionBody.PointerDown),
                        nameof(ActionBody.PointerExit),
                        nameof(ActionBody.PointerEnter),
                        nameof(ActionBody.PointerExit),
                        nameof(ActionBody.PointerEnter),
                        nameof(ActionBody.PointerUp),
                        nameof(ActionBody.PointerClick),
                    }
                )
            );
            var first = (ActionBody.PointerEnter)host.Observer.Actions[0].Body;
            Assert.That(first.ObjectId, Is.EqualTo(Id(60)), "Child hits use the parent identity.");
        }

        private static void RunFatalReconnect(ReleaseScenarioHost host, Mouse mouse)
        {
            host.Connect();
            Assert.That(host.Runner.IsInputAvailable, Is.True);

            host.RunFrame();
            Assert.That(host.Runner.IsInputAvailable, Is.False);
            Assert.That(
                host.Logger.Records.Any(record => record.EventName == "battlement.session.failed"),
                Is.True
            );

            host.Runner.Reconnect();
            Assert.That(host.Runner.IsInputAvailable, Is.True);
        }

        private static void Move(
            ReleaseScenarioHost host,
            Mouse mouse,
            UnityEngine.Vector2 position,
            bool leftButton
        )
        {
            InputSystem.QueueStateEvent(
                mouse,
                new MouseState { position = position }.WithButton(MouseButton.Left, leftButton)
            );
            InputSystem.Update();
            host.RunFrame();
        }

        private static bool HasIdentity(ulong value) =>
            Object
                .FindObjectsByType<BattlementIdentity>()
                .Any(identity => identity.Id == Id(value).Value);

        private static BattlementIdentity Identity(ulong value) =>
            Object
                .FindObjectsByType<BattlementIdentity>()
                .Single(identity => identity.Id == Id(value).Value);

        private static ObjectId Id(ulong value) => new(Guid.Parse(value.ToString("x32")));

        private static string Diagnostics(ReleaseScenarioHost host) =>
            string.Join(
                "\n",
                host.Logger.Records.Select(record => $"{record.EventName}: {record.Message}")
            );

        public sealed class ReleaseScenarioCase
        {
            private readonly System.Action<ReleaseScenarioHost, Mouse> run;

            internal ReleaseScenarioCase(
                string name,
                System.Action<ReleaseScenarioHost, Mouse> run
            ) => (Name, this.run) = (name, run);

            public string Name { get; }

            internal void Run(ReleaseScenarioHost host, Mouse mouse) => run(host, mouse);

            public override string ToString() => Name;
        }

        private static class NativeFixture
        {
            [DllImport("battlement_rules", CallingConvention = CallingConvention.Cdecl)]
            internal static extern UIntPtr fixture_outstanding_buffers();

            [DllImport("battlement_rules", CallingConvention = CallingConvention.Cdecl)]
            internal static extern UIntPtr fixture_submit_calls();
        }
    }

    internal sealed class ReleaseScenarioHost : IDisposable
    {
        private readonly GameObject hostObject;

        private ReleaseScenarioHost(
            GameObject hostObject,
            BattlementRunner runner,
            IBattlementTransport transport,
            string scenario
        )
        {
            this.hostObject = hostObject;
            Runner = runner;
            AssetStorage = new FakeBattlementAssetStorage();
            Clock = new FakeBattlementClock();
            Logger = new FakeBattlementLogger();
            Observer = new RecordingCoreMessageObserver();
            var flatBuffers = new FixtureFlatBufferResponseSchema(
                error => (byte)(FixtureError)error,
                payload =>
                {
                    var value = (FlashPayload)payload;
                    return (value.ObjectId, value.Scale);
                }
            );
            Runner.Configure(
                new BattlementRunnerOptions(
                    transport,
                    AssetStorage,
                    clock: Clock,
                    logger: Logger,
                    useInstantAnimations: true,
                    customCommandTypes: new[] { $"fixture.release.{scenario}" },
                    flatBufferResponseSchema: flatBuffers,
                    flatBufferClientSchema: flatBuffers,
                    coreMessageObserver: Observer
                )
            );
            Runner.RegisterFlatBufferCommand<
                Battlement.FlatBuffers.FixtureGenerated.FlashPayload,
                FixtureError
            >($"fixture.release.{scenario}", new FixtureHandler());
        }

        public BattlementRunner Runner { get; }

        public FakeBattlementAssetStorage AssetStorage { get; }

        public FakeBattlementClock Clock { get; }

        public FakeBattlementLogger Logger { get; }

        public RecordingCoreMessageObserver Observer { get; }

        public static ReleaseScenarioHost Create(string scenario)
        {
            Scene scene = EditorSceneManager.NewScene(
                NewSceneSetup.EmptyScene,
                NewSceneMode.Single
            );
            var hostObject = new GameObject("Battlement release scenario host");
            SceneManager.MoveGameObjectToScene(hostObject, scene);
            BattlementRunner runner = hostObject.AddComponent<BattlementRunner>();
            return new ReleaseScenarioHost(
                hostObject,
                runner,
                new BattlementNativeTransport(),
                scenario
            );
        }

        public void Connect()
        {
            Runner.Connect();
            Physics.SyncTransforms();
        }

        public void RunFrame() => Runner.RunFrame();

        public void Dispose()
        {
            Runner.Stop();
            Runner.Dispose();
            Object.DestroyImmediate(hostObject);
            EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
        }
    }

    internal sealed class RecordingCoreMessageObserver : IBattlementCoreMessageObserver
    {
        public List<BattlementAction> Actions { get; } = new();

        public List<BatchFailed<CoreErrorCode>> BatchFailures { get; } = new();

        public void RecordAction(BattlementAction value) => Actions.Add(value);

        public void RecordBatchFailure(BatchFailed<CoreErrorCode> value) =>
            BatchFailures.Add(value);

        public void RecordOperationFailure(OperationFailed<CoreErrorCode> value) { }
    }
}
