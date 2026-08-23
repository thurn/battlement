#nullable enable

using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using Battlement.CustomFixtures;
using Newtonsoft.Json;
using NUnit.Framework;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.InputSystem.LowLevel;
using UnityEngine.SceneManagement;
using BattlementAction = Battlement.Action;
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
                    yield return new TestCaseData(scenario, BattlementTransportKind.Native).SetName(
                        $"Release_{scenario.Name}_Native"
                    );
                    yield return new TestCaseData(scenario, BattlementTransportKind.Http).SetName(
                        $"Release_{scenario.Name}_Http"
                    );
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
        public void SharedCorpusRunsThroughProductionTransport(
            ReleaseScenarioCase scenario,
            BattlementTransportKind transportKind
        )
        {
            using (var host = ReleaseScenarioHost.Create(scenario.Name, transportKind))
            {
                scenario.Run(host, mouse!);
            }

            if (transportKind == BattlementTransportKind.Native)
            {
                Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
            }
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
                host.Codec.BatchFailures.Select(failure => failure.ErrorCode),
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

            host.Clock.Advance(TimeSpan.FromMilliseconds(299));
            host.RunFrame();
            Assert.That(HasIdentity(21), Is.False);

            host.Clock.Advance(TimeSpan.FromMilliseconds(1));
            host.RunFrame();
            Assert.That(HasIdentity(21), Is.True, "Nonblocking work must not delay group three.");
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
                host.Codec.BatchFailures.Last().ErrorCode,
                Is.EqualTo(CoreErrorCode.AssetInUse)
            );
            FakeAssetHandle handle = host.AssetStorage.Handles.Single(candidate =>
                candidate.Asset == prefab
            );
            Assert.That(handle.IsDisposed, Is.False, "The live prefab must retain its lease.");
        }

        private static void RunCustomFailure(ReleaseScenarioHost host, Mouse mouse)
        {
            var handler = new FixtureHandler(FixtureHandlerMode.Throw);
            host.Runner.RegisterCommand(
                "fixture.character.flash",
                handler,
                new FlashPayloadFormatter(),
                new FixtureErrorFormatter()
            );
            host.Connect();
            host.RunFrame();

            Assert.That(handler.InvocationCount, Is.EqualTo(1));
            Assert.That(HasIdentity(51), Is.False);
            Assert.That(
                host.Codec.BatchFailures.Last().ErrorCode,
                Is.EqualTo(CoreErrorCode.HandlerFailed)
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
                host.Codec.Actions.Select(action => action.Body.GetType().Name),
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
            var first = (ActionBody.PointerEnter)host.Codec.Actions[0].Body;
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
            Codec = new RecordingProtocolCodec();
            Runner.Configure(
                new BattlementRunnerOptions(
                    transport,
                    AssetStorage,
                    Codec,
                    Clock,
                    Logger,
                    useInstantAnimations: true,
                    customCommandTypes: new[] { $"fixture.release.{scenario}" }
                )
            );
            Runner.RegisterCommand(
                $"fixture.release.{scenario}",
                new FixtureHandler(),
                new FlashPayloadFormatter(),
                new FixtureErrorFormatter()
            );
        }

        public BattlementRunner Runner { get; }

        public FakeBattlementAssetStorage AssetStorage { get; }

        public FakeBattlementClock Clock { get; }

        public FakeBattlementLogger Logger { get; }

        public RecordingProtocolCodec Codec { get; }

        public static ReleaseScenarioHost Create(
            string scenario,
            BattlementTransportKind transportKind
        )
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
                CreateTransport(transportKind),
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

        private static IBattlementTransport CreateTransport(BattlementTransportKind kind) =>
            kind switch
            {
                BattlementTransportKind.Native => new BattlementNativeTransport(),
                BattlementTransportKind.Http => new BattlementHttpTransport(
                    Environment.GetEnvironmentVariable("BATTLEMENT_RELEASE_FIXTURE_URL")
                        ?? throw new InvalidOperationException(
                            "BATTLEMENT_RELEASE_FIXTURE_URL is required for release scenarios."
                        )
                ),
                _ => throw new ArgumentOutOfRangeException(nameof(kind)),
            };
    }

    internal sealed class RecordingProtocolCodec : IBattlementExtensionProtocolCodec
    {
        private readonly IBattlementExtensionProtocolCodec inner = BattlementJson.Instance;

        public List<BattlementAction> Actions { get; } = new();

        public List<BatchFailed<CoreErrorCode>> BatchFailures { get; } = new();

        public byte[] SerializeConnect(Connect value) => inner.SerializeConnect(value);

        public byte[] SerializeBatchFailure(BatchFailed<CoreErrorCode> value)
        {
            BatchFailures.Add(value);
            return inner.SerializeBatchFailure(value);
        }

        public byte[] SerializeOperationFailure(OperationFailed<CoreErrorCode> value) =>
            inner.SerializeOperationFailure(value);

        public byte[] SerializeAction(BattlementAction value)
        {
            Actions.Add(value);
            return inner.SerializeAction(value);
        }

        public Response DeserializeResponse(ReadOnlyMemory<byte> bytes) =>
            inner.DeserializeResponse(bytes);

        public Response<ICommand> DeserializeResponse(
            ReadOnlyMemory<byte> bytes,
            Func<CommandId, string, bool, ReadOnlyMemory<byte>, ICommand> decodeCustomCommand
        ) => inner.DeserializeResponse(bytes, decodeCustomCommand);

        public byte[] SerializeCustomAction<TPayload>(
            CustomAction<TPayload> value,
            JsonConverter<TPayload>? payloadConverter
        ) => inner.SerializeCustomAction(value, payloadConverter);

        public byte[] SerializeBatchFailure<TError>(
            BatchFailed<TError> value,
            JsonConverter<TError>? errorConverter
        ) => inner.SerializeBatchFailure(value, errorConverter);

        public byte[] SerializeOperationFailure<TError>(
            OperationFailed<TError> value,
            JsonConverter<TError>? errorConverter
        ) => inner.SerializeOperationFailure(value, errorConverter);
    }
}
