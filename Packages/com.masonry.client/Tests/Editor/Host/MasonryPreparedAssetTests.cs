#nullable enable

using System;
using System.Linq;
using NUnit.Framework;
using UnityEngine;

namespace Masonry.Tests
{
    public sealed class MasonryPreparedAssetTests
    {
        [Test]
        public void SnapshotWaitsForEveryPreparedKindBeforePublishingTheSet()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            PreparedAsset[] assets = AllKinds();
            foreach (PreparedAsset _ in assets)
            {
                harness.AssetStorage.EnqueuePending();
            }

            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(preparedAssets: assets)
            );
            harness.Runner.Connect();

            Assert.That(harness.Runner.IsInputAvailable, Is.False);
            Assert.That(harness.AssetStorage.PrepareCalls, Is.EqualTo(assets));
            foreach (PreparedAsset asset in assets)
            {
                Assert.That(harness.Runner.TryGetPreparedAsset(asset, out _), Is.False);
            }

            foreach (FakeAssetHandle handle in harness.AssetStorage.Handles)
            {
                handle.Complete();
            }

            harness.Runner.RunFrame();

            Assert.That(harness.Runner.IsInputAvailable, Is.True);
            foreach (PreparedAsset asset in assets)
            {
                Assert.That(harness.Runner.TryGetPreparedAsset(asset, out object? value), Is.True);
                Assert.That(
                    value,
                    asset is PreparedAsset.Prefab ? Is.TypeOf<GameObject>() : Is.EqualTo(asset)
                );
            }
        }

        [Test]
        public void SnapshotReplacementReusesMatchesAndReleasesRemovedHandles()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            PreparedAsset retained = new PreparedAsset.Prefab(new PrefabAddress("game/knight"));
            PreparedAsset removed = new PreparedAsset.Texture(new TextureAddress("game/board"));
            PreparedAsset added = new PreparedAsset.Material(new MaterialAddress("game/gold"));
            harness.Transport.EnqueueConnect(Response(session, retained, removed));

            harness.Runner.Connect();
            FakeAssetHandle retainedHandle = HandleFor(harness, retained);
            FakeAssetHandle removedHandle = HandleFor(harness, removed);
            harness.Transport.EnqueueSubmit(Response(session, retained, added));

            harness.Runner.Submit(new byte[] { 1 });

            Assert.That(
                harness.AssetStorage.PrepareCalls.Count(asset =>
                    !FakeMasonryTransport.IsFixtureAsset(asset)
                ),
                Is.EqualTo(3)
            );
            Assert.That(HandleFor(harness, retained), Is.SameAs(retainedHandle));
            Assert.That(removedHandle.IsDisposed, Is.True);
            Assert.That(harness.Runner.TryGetPreparedAsset(removed, out _), Is.False);
            Assert.That(harness.Runner.TryGetPreparedAsset(added, out _), Is.True);
        }

        [Test]
        public void FailedReplacementLeavesThePreviousSetAndReleasesTheNewHandle()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            PreparedAsset original = new PreparedAsset.Texture(new TextureAddress("game/old"));
            PreparedAsset failed = new PreparedAsset.AudioClip(new AudioClipAddress("game/bad"));
            harness.Transport.EnqueueConnect(Response(session, original));
            harness.Runner.Connect();
            harness.AssetStorage.EnqueueFailure(
                new MasonryAssetException(CoreErrorCode.AssetTypeMismatch, "wrong type")
            );
            harness.Transport.EnqueueSubmit(Response(session, failed));

            harness.Runner.Submit(new byte[] { 2 });

            Assert.That(harness.Runner.TryGetPreparedAsset(original, out _), Is.True);
            Assert.That(harness.Runner.TryGetPreparedAsset(failed, out _), Is.False);
            Assert.That(LiveNonFixtureHandles(harness), Is.EqualTo(1));
            Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("stop"));
            Assert.That(harness.Logger.Records.Last().Message, Does.Contain("wrong type"));
        }

        [Test]
        public void LaterSnapshotWaitsForPendingPreparation()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            PreparedAsset initial = new PreparedAsset.Texture(new TextureAddress("game/initial"));
            PreparedAsset slow = new PreparedAsset.Texture(new TextureAddress("game/slow"));
            PreparedAsset replacement = new PreparedAsset.Texture(new TextureAddress("game/new"));
            harness.Transport.EnqueueConnect(Response(session, initial));
            harness.Runner.Connect();
            harness.AssetStorage.EnqueuePending();
            harness.Transport.EnqueueSubmit(Response(session, slow));
            harness.Runner.Submit(new byte[] { 3 });
            FakeAssetHandle slowHandle = HandleFor(harness, slow);
            harness.Transport.EnqueuePoll(Response(session, replacement));

            harness.Runner.RunFrame();

            Assert.That(harness.Runner.IsInputAvailable, Is.False);
            Assert.That(slowHandle.IsDisposed, Is.False);
            Assert.That(
                harness.AssetStorage.PrepareCalls.Contains(replacement),
                Is.False,
                "The later snapshot must remain queued behind the pending replacement."
            );

            slowHandle.Complete();
            harness.Runner.RunFrame();

            Assert.That(slowHandle.IsDisposed, Is.True);
            Assert.That(harness.Runner.TryGetPreparedAsset(slow, out _), Is.False);
            Assert.That(harness.Runner.TryGetPreparedAsset(replacement, out _), Is.True);
            Assert.That(harness.Runner.IsInputAvailable, Is.True);
        }

        [Test]
        public void AuthoritativeRemovalRetiresAHandleUntilItsLastLeaseEnds()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            PreparedAsset asset = new PreparedAsset.Prefab(new PrefabAddress("game/leased"));
            harness.Transport.EnqueueConnect(Response(session, asset));
            harness.Runner.Connect();
            FakeAssetHandle handle = HandleFor(harness, asset);
            IMasonryAssetLease lease = harness.Runner.AcquirePreparedAsset(asset);
            harness.Transport.EnqueueSubmit(Response(session));

            harness.Runner.Submit(new byte[] { 4 });

            Assert.That(harness.Runner.TryGetPreparedAsset(asset, out _), Is.False);
            Assert.That(handle.IsDisposed, Is.False);
            Assert.That(LiveNonFixtureHandles(harness), Is.EqualTo(1));

            lease.Dispose();

            Assert.That(handle.IsDisposed, Is.True);
            Assert.That(LiveNonFixtureHandles(harness), Is.Zero);
        }

        [Test]
        public void InvalidSetsFailBeforeStartingAnyLoads()
        {
            using MasonryTestHarness duplicateHarness = MasonryTestHarness.Create();
            PreparedAsset duplicate = new PreparedAsset.Font(new FontAddress("game/font"));
            duplicateHarness.Transport.EnqueueConnect(
                Response(new SessionId(Guid.NewGuid()), duplicate, duplicate)
            );

            duplicateHarness.Runner.Connect();

            Assert.That(duplicateHarness.AssetStorage.PrepareCalls, Is.Empty);
            Assert.That(duplicateHarness.Transport.Calls.Last(), Is.EqualTo("stop"));

            using MasonryTestHarness limitHarness = MasonryTestHarness.Create();
            var tooMany = Enumerable
                .Range(0, 16_385)
                .Select(index =>
                    (PreparedAsset)new PreparedAsset.Texture(new TextureAddress($"game/{index}"))
                )
                .ToArray();
            limitHarness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(preparedAssets: tooMany)
            );

            limitHarness.Runner.Connect();

            Assert.That(limitHarness.AssetStorage.PrepareCalls, Is.Empty);
            Assert.That(limitHarness.Transport.Calls.Last(), Is.EqualTo("stop"));

            using MasonryTestHarness stringHarness = MasonryTestHarness.Create();
            PreparedAsset oversized = new PreparedAsset.Texture(
                new TextureAddress(new string('x', 65_537))
            );
            stringHarness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(preparedAssets: new[] { oversized })
            );

            stringHarness.Runner.Connect();

            Assert.That(stringHarness.AssetStorage.PrepareCalls, Is.Empty);
            Assert.That(stringHarness.Transport.Calls.Last(), Is.EqualTo("stop"));
        }

        private static FakeAssetHandle HandleFor(MasonryTestHarness harness, PreparedAsset asset) =>
            harness.AssetStorage.Handles.Single(handle => handle.Asset == asset);

        private static int LiveNonFixtureHandles(MasonryTestHarness harness) =>
            harness.AssetStorage.Handles.Count(handle =>
                !FakeMasonryTransport.IsFixtureAsset(handle.Asset)
            );

        private static MasonryTransportResult Response(
            SessionId session,
            params PreparedAsset[] assets
        )
        {
            Snapshot snapshot = FakeMasonryTransport.CompleteSnapshot(
                session,
                preparedAssets: assets
            );
            return FakeMasonryTransport.ResponseResult(
                new Response(
                    session,
                    new ResponseMessage<Command>[]
                    {
                        new ResponseMessage<Command>.SnapshotMessage(snapshot),
                    }
                )
            );
        }

        private static PreparedAsset[] AllKinds() =>
            new PreparedAsset[]
            {
                new PreparedAsset.Scene(new SceneAddress("game/scene")),
                new PreparedAsset.Prefab(new PrefabAddress("game/prefab")),
                new PreparedAsset.ParticleEffect(new ParticleEffectAddress("game/effect")),
                new PreparedAsset.Material(new MaterialAddress("game/material")),
                new PreparedAsset.Texture(new TextureAddress("game/texture")),
                new PreparedAsset.AudioClip(new AudioClipAddress("game/audio")),
                new PreparedAsset.Font(new FontAddress("game/font")),
            };
    }
}
