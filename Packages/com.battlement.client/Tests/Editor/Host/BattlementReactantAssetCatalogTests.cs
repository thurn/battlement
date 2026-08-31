#nullable enable

using System;
using System.IO;
using System.Linq;
using NUnit.Framework;
using UnityEditor;

namespace Battlement.Tests
{
    [Parallelizable(ParallelScope.None)]
    public sealed class BattlementReactantAssetCatalogTests
    {
        private const string Address =
            "battlement-reactant/generated/"
            + "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.png";
        private const string ExtraAddress =
            "battlement-reactant/generated/"
            + "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.png";
        private const string Root = "Assets/BattlementReactantAssetCatalogTests";
        private const string ResourceDirectory = Root + "/Resources";
        private const string ResourcePath =
            ResourceDirectory + "/BattlementReactantAssetCatalog.json";

        [SetUp]
        public void SetUp()
        {
            AssetDatabase.DeleteAsset(Root);
            Directory.CreateDirectory(ResourceDirectory);
        }

        [TearDown]
        public void TearDown()
        {
            AssetDatabase.DeleteAsset(Root);
            AssetDatabase.Refresh(ImportAssetOptions.ForceSynchronousImport);
        }

        [Test]
        public void InitialCatalogMismatchFailsBeforeStartingAnyAssetLoad()
        {
            WriteCatalog(Address);
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            harness.Transport.EnqueueConnect(FakeBattlementTransport.SnapshotResponse());

            harness.Runner.Connect();

            Assert.That(harness.AssetStorage.PrepareCalls, Is.Empty);
            Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("stop"));
            Assert.That(harness.Logger.Records.Last().Message, Does.Contain(Address));
            Assert.That(harness.Logger.Records.Last().Message, Does.Contain("missing"));
        }

        [Test]
        public void OpaqueExtraRegistrationFailsBeforeStartingAnyAssetLoad()
        {
            WriteCatalog();
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    preparedAssets: new[] { GeneratedTexture(Address) }
                )
            );

            harness.Runner.Connect();

            Assert.That(harness.AssetStorage.PrepareCalls, Is.Empty);
            Assert.That(harness.Logger.Records.Last().Message, Does.Contain(Address));
            Assert.That(harness.Logger.Records.Last().Message, Does.Contain("module scope"));
        }

        [Test]
        public void ReplacementMismatchIsRejectedBeforePreparingItsAddition()
        {
            WriteCatalog(Address);
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            harness.Transport.EnqueueConnect(Response(session, GeneratedTexture(Address)));
            harness.Runner.Connect();
            int initialLoads = harness.AssetStorage.PrepareCalls.Count;
            harness.Transport.EnqueueSubmit(
                Response(session, GeneratedTexture(Address), GeneratedTexture(ExtraAddress))
            );

            harness.Runner.Submit(new byte[] { 1 });

            Assert.That(harness.AssetStorage.PrepareCalls, Has.Count.EqualTo(initialLoads));
            Assert.That(
                harness.AssetStorage.PrepareCalls,
                Has.None.EqualTo(GeneratedTexture(ExtraAddress))
            );
            Assert.That(harness.Logger.Records.Last().Message, Does.Contain(ExtraAddress));
        }

        [Test]
        public void SidecarShapeAndOrderingAreStrict()
        {
            string hash = new('0', 64);
            string reversed =
                $"{{\"addresses\":[\"{ExtraAddress}\",\"{Address}\"],"
                + $"\"manifestSha256\":\"{hash}\"}}";
            Assert.That(
                () => BattlementReactantAssetCatalog.Parse(reversed),
                Throws.InvalidOperationException.With.Message.Contains("sorted and unique")
            );
            Assert.That(
                () =>
                    BattlementReactantAssetCatalog.Parse(
                        $"{{\"addresses\":[],\"manifestSha256\":\"{hash}\",\"extra\":true}}"
                    ),
                Throws.InvalidOperationException.With.Message.Contains("exactly")
            );
        }

        private static PreparedAsset GeneratedTexture(string address) =>
            new PreparedAsset.Texture(new TextureAddress(address));

        private static BattlementTransportResult Response(
            SessionId session,
            params PreparedAsset[] assets
        )
        {
            Snapshot snapshot = FakeBattlementTransport.CompleteSnapshot(
                session,
                preparedAssets: assets
            );
            return FakeBattlementTransport.ResponseResult(
                new Response(
                    session,
                    new ResponseMessage<Command>[]
                    {
                        new ResponseMessage<Command>.SnapshotMessage(snapshot),
                    }
                )
            );
        }

        private static void WriteCatalog(params string[] addresses)
        {
            File.WriteAllText(
                ResourcePath,
                "{\"addresses\":["
                    + string.Join(",", addresses.Select(address => $"\"{address}\""))
                    + $"],\"manifestSha256\":\"{new string('0', 64)}\"}}"
            );
            AssetDatabase.ImportAsset(ResourcePath, ImportAssetOptions.ForceSynchronousImport);
        }
    }
}
