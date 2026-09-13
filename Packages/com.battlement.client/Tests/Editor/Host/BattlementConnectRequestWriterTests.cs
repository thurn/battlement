#nullable enable

using System;
using Google.FlatBuffers;
using NUnit.Framework;
using WireConnectRequest = Battlement.FlatBuffers.Generated.ConnectRequest;
using WireConnectRequestVerify = Battlement.FlatBuffers.Generated.ConnectRequestVerify;

namespace Battlement.Tests
{
    public sealed class BattlementConnectRequestWriterTests
    {
        [Test]
        public void WritesVerifiedSizePrefixedConnectWithoutPayloadCopy()
        {
            var writer = new BattlementConnectRequestWriter();
            ReadOnlyMemory<byte> memory = writer.Write(
                new Connect(
                    "macOS 日本語 🚀",
                    "6000.5.8f1",
                    new ScreenSize(2560, 1440),
                    new[] { "cards.draw", "cards.shuffle" },
                    "/tmp/保存",
                    null,
                    new[] { "core", "ui" }
                )
                {
                    ApplicationState = new ApplicationState(true, false),
                    ReducedMotionPreference = ReducedMotionPreference.Reduce,
                }
            );

            byte[] bytes = memory.ToArray();
            Assert.That(BitConverter.ToUInt32(bytes, 0), Is.EqualTo(bytes.Length - 4));
            var buffer = new ByteBuffer(bytes);
            var verifier = new Verifier(buffer, new Options(64, 1_000_000, true, true));
            Assert.That(
                verifier.VerifyBuffer("BTCO", true, WireConnectRequestVerify.Verify),
                Is.True
            );
            buffer.Position = 4;
            WireConnectRequest request = WireConnectRequest.GetRootAsConnectRequest(buffer);
            Assert.That(request.Platform, Is.EqualTo("macOS 日本語 🚀"));
            Assert.That(request.Screen!.Value.Width, Is.EqualTo(2560));
            Assert.That(request.CustomCommandTypesLength, Is.EqualTo(2));
            Assert.That(request.CustomCommandTypes(1), Is.EqualTo("cards.shuffle"));
            Assert.That(request.PersistentDataPath, Is.EqualTo("/tmp/保存"));
            Assert.That(request.StreamingAssetsPath, Is.Null);
        }

        [Test]
        public void ReusesExclusiveBuilderStorageAndRejectsNoncanonicalTypes()
        {
            var writer = new BattlementConnectRequestWriter();
            Connect valid = new(
                "macOS",
                "6000.5.8f1",
                new ScreenSize(1, 1),
                new[] { "cards.draw", "cards.shuffle" }
            );
            _ = writer.Write(valid);
            int allocation = writer.AllocationBytes;
            _ = writer.Write(valid);
            Assert.That(writer.AllocationBytes, Is.EqualTo(allocation));

            Connect duplicate = valid with
            {
                CustomCommandTypes = new[] { "cards.draw", "cards.draw" },
            };
            Assert.Throws<System.IO.InvalidDataException>(() => writer.Write(duplicate));
        }

        [Test]
        public void VerifierEnforcesTraversalBudgetAndResetsCountersBetweenCalls()
        {
            byte[] bytes = WriteMinimalConnect();
            var options = new Options(64, 1_000_000, 8, true, true);
            var verifier = new Verifier(new ByteBuffer(bytes), options);

            Assert.That(
                verifier.VerifyBuffer("BTCO", true, WireConnectRequestVerify.Verify),
                Is.False
            );

            options.maxApparentSize = Options.DEFAULT_MAX_APPARENT_SIZE;
            Assert.That(
                verifier.VerifyBuffer("BTCO", true, WireConnectRequestVerify.Verify),
                Is.True
            );
            ulong successfulTraversalSize = verifier.apparentSize;

            options.maxApparentSize = successfulTraversalSize;
            Assert.That(
                verifier.VerifyBuffer("BTCO", true, WireConnectRequestVerify.Verify),
                Is.True
            );
            Assert.That(verifier.apparentSize, Is.EqualTo(successfulTraversalSize));
        }

        [Test]
        public void VerifierRejectsBadPrefixIdentifierAndHostileOffsetsWithoutThrowing()
        {
            byte[] badPrefix = WriteMinimalConnect();
            badPrefix[0] ^= 1;
            Assert.That(Verify(badPrefix, "BTCO"), Is.False);

            byte[] wrongIdentifier = WriteMinimalConnect();
            Assert.That(Verify(wrongIdentifier, "NOPE"), Is.False);

            byte[] hostileVtableOffset = WriteMinimalConnect();
            int rootTable = 4 + BitConverter.ToInt32(hostileVtableOffset, 4);
            Array.Copy(BitConverter.GetBytes(int.MinValue), 0, hostileVtableOffset, rootTable, 4);
            Assert.DoesNotThrow(() => Assert.That(Verify(hostileVtableOffset, "BTCO"), Is.False));
        }

        private static byte[] WriteMinimalConnect()
        {
            return new BattlementConnectRequestWriter()
                .Write(new Connect("test", "test", new ScreenSize(1, 1), Array.Empty<string>()))
                .ToArray();
        }

        private static bool Verify(byte[] bytes, string identifier)
        {
            return new Verifier(new ByteBuffer(bytes), new Options()).VerifyBuffer(
                identifier,
                true,
                WireConnectRequestVerify.Verify
            );
        }
    }
}
