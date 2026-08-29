#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class DittoPlayerStateResetTests
    {
        [Test]
        public void BoundaryJournalsDirtyResetAndCleanPublicUnityStateExactlyOnce()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            using var engineTransport = new BattlementNativeTransport();
            var authored = new GameObject("Project-authored sentinel");
            var documentId = new ObjectId(Guid.NewGuid());
            var rootId = new ObjectId(Guid.NewGuid());
            var documentObject = new BattlementGameObject(
                documentId,
                new GameObjectKind.UiDocumentState(rootId),
                new ParentScene.Persistent(),
                null,
                true,
                LocalTransform.Identity,
                Array.Empty<PointerEvent>()
            );
            SessionId sessionId = new(Guid.NewGuid());
            Snapshot snapshot = FakeBattlementTransport.CompleteSnapshot(
                sessionId,
                objects: new[] { documentObject },
                globalKeys: new[] { PhysicalKey.KeyA }
            ) with
            {
                Ui = new[] { new UiDocument(documentId, rootId) },
            };
            harness.AssetStorage.EnqueueSceneUnloadPending();
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.ResponseResult(
                    new Response(
                        sessionId,
                        new ResponseMessage<Command>[]
                        {
                            new ResponseMessage<Command>.SnapshotMessage(snapshot),
                        }
                    )
                )
            );
            harness.Runner.Connect();
            FakeSceneHandle sceneHandle = harness.AssetStorage.SceneHandles.Single();
            PreparedAsset preparedScene = sceneHandle.Asset;
            var cameraId = new ObjectId(
                Object
                    .FindObjectsByType<BattlementIdentity>()
                    .Single(identity => identity.Id != documentId.Value)
                    .Id
            );
            DittoNativeEngineSession engine = CreateEngine(engineTransport, "ditto-reset");
            var journal = new List<UnityState>();
            journal.Add(
                State("before", harness, engineTransport, authored, cameraId, preparedScene)
            );

            var boundary = new DittoPlayerStateReset(
                harness.Runner,
                engine,
                () => harness.Clock.Elapsed
            );
            boundary.Begin();
            journal.Add(
                State("reset", harness, engineTransport, authored, cameraId, preparedScene)
            );
            Assert.That(boundary.IsComplete, Is.False);

            sceneHandle.CompleteUnload();
            Assert.That(boundary.Advance(), Is.True);
            journal.Add(
                State("after", harness, engineTransport, authored, cameraId, preparedScene)
            );

            Assert.That(
                journal,
                Is.EqualTo(
                    new[]
                    {
                        new UnityState("before", true, true, true, true, 2, 1, 2, true),
                        new UnityState("reset", false, false, false, false, 0, 0, 2, true),
                        new UnityState("after", false, false, false, false, 0, 0, 1, true),
                    }
                )
            );
            Assert.That(harness.AssetStorage.LiveHandleCount, Is.Zero);
            Assert.That(harness.AssetStorage.SceneHandles, Is.Empty);
            Assert.That(boundary.IsReusable, Is.True);

            DittoNativeEngineSession next = CreateEngine(engineTransport, "ditto-next");
            boundary.Begin();
            Assert.That(boundary.Advance(), Is.True);
            Assert.That(engineTransport.HasEngine, Is.True);
            Assert.That(next.Destroy().Status, Is.EqualTo(BattlementTransportStatus.Success));
        }

        [Test]
        public void DestroyFailureCompletesSafeResetAndMarksPlayerNonReusable()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            using var engineTransport = new BattlementNativeTransport();
            harness.Runner.Connect();
            DittoNativeEngineSession engine = CreateEngine(engineTransport, "panic-destroy");
            LogAssert.Expect(
                LogType.Error,
                new System.Text.RegularExpressions.Regex(
                    @"^\[Battlement/Rust\]\[battlement\.rust\.destroy_panic\]"
                )
            );
            var boundary = new DittoPlayerStateReset(
                harness.Runner,
                engine,
                () => harness.Clock.Elapsed
            );

            boundary.Begin();

            Assert.That(boundary.IsComplete, Is.True);
            Assert.That(boundary.IsReusable, Is.False);
            Assert.That(boundary.Failure!.Stage, Is.EqualTo(DittoBoundaryStage.Destroy));
            Assert.That(boundary.Failure.Diagnostic, Does.Contain("fixture destroy panic"));
            Assert.That(engineTransport.HasEngine, Is.False);
            Assert.That(Object.FindObjectsByType<BattlementIdentity>(), Is.Empty);
        }

        [Test]
        public void ResetFailureAndTimeoutIdentifyTheResetStage()
        {
            using (BattlementTestHarness failed = BattlementTestHarness.Create())
            {
                failed.Runner.Connect();
                failed
                    .AssetStorage.SceneHandles.Single()
                    .SetFailure(new InvalidOperationException("fixture unload failure"));
                var boundary = new DittoPlayerStateReset(
                    failed.Runner,
                    null,
                    () => failed.Clock.Elapsed
                );

                boundary.Begin();

                Assert.That(boundary.IsComplete, Is.True);
                Assert.That(boundary.IsReusable, Is.False);
                Assert.That(boundary.Failure!.Stage, Is.EqualTo(DittoBoundaryStage.Reset));
                Assert.That(boundary.Failure.Diagnostic, Does.Contain("fixture unload failure"));
            }

            using BattlementTestHarness timedOut = BattlementTestHarness.Create();
            timedOut.AssetStorage.EnqueueSceneUnloadPending();
            timedOut.Runner.Connect();
            var timeout = new DittoPlayerStateReset(
                timedOut.Runner,
                null,
                () => timedOut.Clock.Elapsed
            );
            timeout.Begin();
            timedOut.Clock.Advance(TimeSpan.FromSeconds(10));

            Assert.That(timeout.Advance(), Is.True);
            Assert.That(timeout.IsReusable, Is.False);
            Assert.That(timeout.Failure!.Stage, Is.EqualTo(DittoBoundaryStage.Reset));
            Assert.That(timeout.Failure.Diagnostic, Does.Contain("exceeded 10 seconds"));
        }

        private static UnityState State(
            string stage,
            BattlementTestHarness harness,
            BattlementNativeTransport transport,
            GameObject authored,
            ObjectId objectId,
            PreparedAsset asset
        ) =>
            new(
                stage,
                transport.HasEngine,
                harness.Runner.IsInputAvailable,
                harness.Runner.IsGlobalKeyEnabled(PhysicalKey.KeyA),
                harness.Runner.TryGetObject(objectId, out _)
                    && harness.Runner.TryGetPreparedAsset(asset, out _),
                Object.FindObjectsByType<BattlementIdentity>().Length,
                Object
                    .FindObjectsByType<UIDocument>()
                    .Count(value => value.gameObject.name == "Battlement UI Document"),
                UnityEngine.SceneManagement.SceneManager.sceneCount,
                authored != null
            );

        private static DittoNativeEngineSession CreateEngine(
            BattlementNativeTransport transport,
            string platform
        )
        {
            DittoNativeEngineSession? engine = DittoNativeEngineSession.Create(
                transport,
                out BattlementTransportResult created
            );
            Assert.That(created.Status, Is.EqualTo(BattlementTransportStatus.Success));
            Assert.That(
                engine!
                    .Connect(
                        BattlementJson.SerializeConnect(
                            new Connect(
                                platform,
                                Application.unityVersion,
                                new ScreenSize((uint)Screen.width, (uint)Screen.height)
                            )
                        )
                    )
                    .Status,
                Is.EqualTo(BattlementTransportStatus.Success)
            );
            return engine;
        }

        private sealed record UnityState(
            string Stage,
            bool HasEngine,
            bool HasInput,
            bool HasKey,
            bool HasObjectAndAsset,
            int IdentityCount,
            int UiDocumentCount,
            int SceneCount,
            bool AuthoredObjectSurvives
        );
    }
}
