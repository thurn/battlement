#nullable enable

using System;
using System.Linq;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text;
using Battlement.CustomFixtures;
using Google.FlatBuffers;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;
using FixtureWire = Battlement.FlatBuffers.FixtureGenerated;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement.Tests
{
    public sealed class BattlementNativeTransportTests
    {
        [TestCase(Wire.GameObjectKind.Cube, PrimitiveType.Cube)]
        [TestCase(Wire.GameObjectKind.Sphere, PrimitiveType.Sphere)]
        [TestCase(Wire.GameObjectKind.Capsule, PrimitiveType.Capsule)]
        [TestCase(Wire.GameObjectKind.Cylinder, PrimitiveType.Cylinder)]
        [TestCase(Wire.GameObjectKind.Plane, PrimitiveType.Plane)]
        [TestCase(Wire.GameObjectKind.Quad, PrimitiveType.Quad)]
        public void DirectPrimitiveKindsPreserveSchemaIdentity(
            Wire.GameObjectKind kind,
            PrimitiveType expected
        ) => Assert.That(BattlementObjectFactory.DirectPrimitiveType(kind), Is.EqualTo(expected));

        [Test]
        public void VerifierAcceptsATableBeyondTheSignedSixteenBitAddressRange()
        {
            var builder = new FlatBufferBuilder(64);
            StringOffset value = builder.CreateString("value");
            Offset<Wire.TextPropertyValue> root = Wire.TextPropertyValue.CreateTextPropertyValue(
                builder,
                value
            );
            _ = builder.CreateString(new string('x', 40 * 1024));
            builder.Finish(root.Value);

            var bytes = new ByteBuffer(builder.SizedByteArray());

            Assert.That(
                new Verifier(bytes).VerifyBuffer(
                    string.Empty,
                    false,
                    Wire.TextPropertyValueVerify.Verify
                ),
                Is.True
            );
        }

        [Test]
        public void PaintMaterializerPreservesAbsentOptionalVectors()
        {
            var builder = new FlatBufferBuilder(128);
            VectorOffset layers = Wire.PaintStylePropertyValue.CreateLayersVector(
                builder,
                Array.Empty<Offset<Wire.PaintLayerValue>>()
            );
            Offset<Wire.PaintStylePropertyValue> paint =
                Wire.PaintStylePropertyValue.CreatePaintStylePropertyValue(
                    builder,
                    has_blend_mode: true,
                    layersOffset: layers
                );
            Offset<Wire.UiProperty> property = Wire.UiProperty.CreateUiProperty(
                builder,
                Wire.UiPropertyKey.Paint,
                Wire.PropState.Set,
                value_type: Wire.UiPropertyValue.PaintStylePropertyValue,
                valueOffset: paint.Value
            );
            VectorOffset properties = Wire.UiElement.CreatePropertiesVector(
                builder,
                new[] { property }
            );
            VectorOffset eventSubscriptions = Wire.UiElement.CreateEventSubscriptionsVector(
                builder,
                Array.Empty<Offset<Wire.UiEventSubscriptionValue>>()
            );
            VectorOffset partStyles = Wire.UiElement.CreatePartStylesVector(
                builder,
                Array.Empty<Offset<Wire.PartStyle>>()
            );
            Offset<Wire.UiElement> encoded = Wire.UiElement.CreateUiElement(
                builder,
                propertiesOffset: properties,
                event_subscriptionsOffset: eventSubscriptions,
                part_stylesOffset: partStyles
            );
            builder.Finish(encoded.Value);

            UiElement decoded = BattlementFlatBufferMaterializer.UiElement(
                Wire.UiElement.GetRootAsUiElement(new ByteBuffer(builder.SizedByteArray()))
            );

            Assert.That(decoded.Paint.IsSet, Is.True);
            Assert.That(decoded.Paint.Value.Background, Is.Null);
            Assert.That(decoded.Paint.Value.PaintFilter, Is.Null);
            Assert.That(decoded.Paint.Value.ClipPolygon, Is.Null);
            Assert.That(decoded.Paint.Value.BoxShadow, Is.Null);
        }

        [Test]
        public void ComposedFixtureClientWriterProducesVerifiedTypedMessages()
        {
            var schema = FixtureSchema();
            var actionId = new ActionId(new Guid("00112233-4455-6677-8899-aabbccddeeff"));
            var sessionId = new SessionId(new Guid("10213243-5465-7687-98a9-bacbdcedfe0f"));
            var objectId = new ObjectId(new Guid("ffeeddcc-bbaa-9988-7766-554433221100"));
            ReadOnlyMemory<byte> encoded = schema.SerializeCustomAction(
                new CustomAction<FlashPayload>(
                    actionId,
                    sessionId,
                    "fixture.flash.completed",
                    new FlashPayload(objectId, 1.5f)
                )
            );

            var bytes = new ByteBuffer(encoded.ToArray());
            Assert.That(
                new Verifier(bytes).VerifyBuffer(
                    "BTCM",
                    true,
                    FixtureWire.FixtureClientMessageVerify.Verify
                ),
                Is.True
            );
            bytes.Position = FlatBufferConstants.SizePrefixLength;
            FixtureWire.FixtureClientMessage root =
                FixtureWire.FixtureClientMessage.GetRootAsFixtureClientMessage(bytes);
            Assert.That(root.BodyType, Is.EqualTo(FixtureWire.FixtureClientBody.FixtureAction));
            FixtureWire.FixtureAction action = root.BodyAsFixtureAction();
            Assert.That(action.ActionType, Is.EqualTo("fixture.flash.completed"));
            Assert.That(action.Payload!.Value.Scale, Is.EqualTo(1.5f));
            Assert.That(
                BattlementFlatBufferCore.ReadUuid(action.SessionId, "session"),
                Is.EqualTo(sessionId.Value)
            );
        }

        [Test]
        public void ContractMismatchIsRejectedBeforeEngineCreation()
        {
            BattlementNativeLogging.Drain();
            BattlementLogStore.Clear();
            using var transport = new BattlementNativeTransport();

            using BattlementTransportResult result = transport.Connect(ConnectBytes("normal"));

            Assert.That(result.Status, Is.EqualTo(BattlementTransportStatus.AbiError));
            Assert.That(result.Diagnostic, Does.Contain("wire contract mismatch"));
            Assert.That(transport.HasEngine, Is.False);
            Assert.That(EngineJournal(), Is.Empty);
        }

        [Test]
        public void RunnerReusesOneEngineAcrossConnectsAndDestroysItAtShutdown()
        {
            ulong callsBefore = NativeFixture.fixture_connect_calls().ToUInt64();
            var transport = Transport("normal");
            using (transport)
            {
                using BattlementTransportResult first = transport.Connect(ConnectBytes("normal"));
                Assert.That(first.Status, Is.EqualTo(Success));
                Assert.That(transport.LastConnectResult!.Status, Is.EqualTo(Success));
                transport.Stop();
                using BattlementTransportResult second = transport.Connect(ConnectBytes("normal"));
                Assert.That(second.Status, Is.EqualTo(Success));
                Assert.That(transport.LastConnectResult!.Status, Is.EqualTo(Success));
                Assert.That(
                    NativeFixture.fixture_connect_calls().ToUInt64(),
                    Is.EqualTo(callsBefore + 2)
                );
                second.Dispose();
                Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
            }

            Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
        }

        [Test]
        public void AbandonedResponseIsReleasedOnTheOwningThreadAfterFinalization()
        {
            using BattlementNativeTransport transport = Transport("poll-response");
            using BattlementTransportResult connected = transport.Connect(
                ConnectBytes("poll-response")
            );
            Assert.That(connected.Status, Is.EqualTo(Success));
            connected.Dispose();

            AbandonPollResponse(transport);
            Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo((UIntPtr)1));
            GC.Collect();
            GC.WaitForPendingFinalizers();
            GC.Collect();

            using BattlementTransportResult drained = transport.Poll();
            Assert.That(drained.Status, Is.EqualTo(Success));
            Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo((UIntPtr)1));
            drained.Dispose();
            Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
        }

        [Test]
        public void DiagnosticsExposeLiveAllocationAndReleaseAccounting()
        {
            using BattlementNativeTransport transport = Transport("normal");
            BattlementNativeTransportDiagnostics initial = transport.Diagnostics;
            Assert.That(initial.LiveBufferCount, Is.Zero);
            Assert.That(initial.LiveAllocationBytes, Is.Zero);
            Assert.That(initial.PendingFinalizerReleases, Is.Zero);
            Assert.That(initial.HandoffPayloadCopies, Is.Zero);

            using BattlementTransportResult connected = transport.Connect(ConnectBytes("normal"));
            BattlementNativeTransportDiagnostics live = transport.Diagnostics;
            Assert.That(live.Generation, Is.GreaterThan(initial.Generation));
            Assert.That(live.LiveBufferCount, Is.EqualTo(1));
            Assert.That(
                live.LiveAllocationBytes,
                Is.GreaterThanOrEqualTo(connected.Payload.Length)
            );
            Assert.That(live.PendingFinalizerReleases, Is.Zero);

            connected.Dispose();
            BattlementNativeTransportDiagnostics released = transport.Diagnostics;
            Assert.That(released.LiveBufferCount, Is.Zero);
            Assert.That(released.LiveAllocationBytes, Is.Zero);
            Assert.That(
                released.IdleBuilderBytes,
                Is.GreaterThanOrEqualTo(initial.IdleBuilderBytes)
            );
            Assert.That(released.HandoffPayloadCopies, Is.Zero);
        }

        [Test]
        public void DittoScenarioSessionsCreateDistinctEnginesOnlyWhenReached()
        {
            BattlementNativeLogging.Drain();
            BattlementLogStore.Clear();
            using var transport = Transport("ditto-one");
            Assert.That(transport.HasEngine, Is.False);
            Assert.That(EngineJournal(), Is.Empty);

            DittoNativeEngineSession first = CreateSession(transport);
            Assert.That(Guid.TryParse(first.Id, out _), Is.True);
            Assert.That(transport.HasEngine, Is.True);
            Assert.That(first.Connect(ConnectBytes("ditto-one")).Status, Is.EqualTo(Success));
            Assert.Throws<InvalidOperationException>(() => first.Connect(ConnectBytes("normal")));
            Assert.That(first.Destroy().Status, Is.EqualTo(Success));
            Assert.That(first.Destroy().Status, Is.EqualTo(Success));
            Assert.That(transport.HasEngine, Is.False);

            DittoNativeEngineSession second = CreateSession(transport);
            Assert.That(second.Id, Is.Not.EqualTo(first.Id));
            Assert.That(second.Connect(ConnectBytes("ditto-two")).Status, Is.EqualTo(Success));
            Assert.That(second.Destroy().Status, Is.EqualTo(Success));
            Assert.That(transport.HasEngine, Is.False);

            BattlementLogEntry[] journal = EngineJournal();
            Assert.That(
                journal.Select(entry => entry.Record.EventName),
                Is.EqualTo(
                    new[]
                    {
                        "fixture.engine.created",
                        "fixture.engine.connected",
                        "fixture.engine.destroyed",
                        "fixture.engine.created",
                        "fixture.engine.connected",
                        "fixture.engine.destroyed",
                    }
                )
            );
            string firstEngine = journal[0].Record.Fields!["engine_id"];
            string secondEngine = journal[3].Record.Fields!["engine_id"];
            Assert.That(secondEngine, Is.Not.EqualTo(firstEngine));
            Assert.That(journal[1].Record.Fields!["platform"], Is.EqualTo("ditto-one"));
            Assert.That(journal[4].Record.Fields!["platform"], Is.EqualTo("ditto-two"));
        }

        [Test]
        public void DittoScenarioCreationRejectsAnUnfinishedEngineWithoutDestroyingIt()
        {
            using var transport = Transport("normal");
            DittoNativeEngineSession first = CreateSession(transport);

            DittoNativeEngineSession? duplicate = DittoNativeEngineSession.Create(
                transport,
                out BattlementTransportResult result
            );

            Assert.That(duplicate, Is.Null);
            Assert.That(result.Status, Is.EqualTo(BattlementTransportStatus.AbiError));
            Assert.That(result.Diagnostic, Does.Contain("already active"));
            Assert.That(transport.HasEngine, Is.True);
            Assert.That(first.Destroy().Status, Is.EqualTo(Success));
        }

        [Test]
        public void DittoScenarioCreationScopesItsSemanticFixtureToEngineConstruction()
        {
            const string original = "outer-fixture";
            string? previous = Environment.GetEnvironmentVariable(
                DittoNativeEngineSession.SemanticFixtureEnvironment
            );
            string? previousRuntime = Environment.GetEnvironmentVariable(
                DittoNativeEngineSession.DeterministicRuntimeEnvironment
            );
            Environment.SetEnvironmentVariable(
                DittoNativeEngineSession.SemanticFixtureEnvironment,
                original
            );
            try
            {
                (string? Fixture, string? Runtime) observed =
                    DittoNativeEngineSession.WithSemanticFixture(
                        "castling",
                        () =>
                            (
                                Environment.GetEnvironmentVariable(
                                    DittoNativeEngineSession.SemanticFixtureEnvironment
                                ),
                                Environment.GetEnvironmentVariable(
                                    DittoNativeEngineSession.DeterministicRuntimeEnvironment
                                )
                            )
                    );

                Assert.That(observed.Fixture, Is.EqualTo("castling"));
                Assert.That(observed.Runtime, Is.EqualTo("1"));
                Assert.That(
                    Environment.GetEnvironmentVariable(
                        DittoNativeEngineSession.SemanticFixtureEnvironment
                    ),
                    Is.EqualTo(original)
                );
                Assert.That(
                    Environment.GetEnvironmentVariable(
                        DittoNativeEngineSession.DeterministicRuntimeEnvironment
                    ),
                    Is.EqualTo(previousRuntime)
                );
            }
            finally
            {
                Environment.SetEnvironmentVariable(
                    DittoNativeEngineSession.SemanticFixtureEnvironment,
                    previous
                );
            }
        }

        [Test]
        public void SubmitAndPollExposeOwnedResponsesAndNoMessageDistinctly()
        {
            using (BattlementNativeTransport transport = Transport("normal"))
            {
                using BattlementTransportResult connected = transport.Connect(
                    ConnectBytes("normal")
                );
                Assert.That(connected.Status, Is.EqualTo(Success));

                using BattlementTransportResult submit = transport.Submit(ClientMessage());
                Assert.That(submit.Status, Is.EqualTo(Success), submit.Diagnostic);
                Assert.That(submit.Payload.IsEmpty, Is.False);
                Assert.That(transport.Poll().Status, Is.EqualTo(NoMessage));
                Assert.That(
                    NativeFixture.fixture_outstanding_buffers(),
                    Is.Not.EqualTo(UIntPtr.Zero)
                );
            }

            using BattlementNativeTransport polling = Transport("poll-response");
            using BattlementTransportResult pollConnected = polling.Connect(
                ConnectBytes("poll-response")
            );
            Assert.That(pollConnected.Status, Is.EqualTo(Success));
            using BattlementTransportResult poll = polling.Poll();
            Assert.That(poll.Status, Is.EqualTo(Success));
            Assert.That(poll.Payload.IsEmpty, Is.False);
            poll.Dispose();
            pollConnected.Dispose();
            Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
        }

        [Test]
        public void FixedErrorStatusesPreserveDiagnosticsAndReleaseNativeBuffers()
        {
            using (BattlementNativeTransport malformed = Transport("malformed"))
            {
                BattlementTransportResult result = malformed.Connect(new byte[] { 0xc1 });
                Assert.That(result.Status, Is.EqualTo(BattlementTransportStatus.InvalidArgument));
                Assert.That(result.Diagnostic, Does.Contain("invalid connect"));
            }

            using (BattlementNativeTransport failed = Transport("engine-error"))
            {
                BattlementTransportResult result = failed.Connect(ConnectBytes("engine-error"));
                Assert.That(result.Status, Is.EqualTo(BattlementTransportStatus.EngineError));
                Assert.That(result.Diagnostic, Is.EqualTo("fixture engine error"));
            }

            Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
        }

        [Test]
        public void CaughtPanicPreservesLeadingRustTracingAndReleasesNativeBuffers()
        {
            BattlementNativeLogging.Drain();
            BattlementLogStore.Clear();
            LogAssert.Expect(
                LogType.Log,
                new System.Text.RegularExpressions.Regex(
                    @"^\[Battlement/Rust\]\[[^\]]+\] Preparing fixture connect panic"
                )
            );
            LogAssert.Expect(
                LogType.Log,
                new System.Text.RegularExpressions.Regex(
                    @"^\[Battlement/Rust\]\[[^\]]+\] Triggering fixture connect panic"
                )
            );
            using BattlementNativeTransport panicked = Transport("panic-connect");

            BattlementTransportResult result = panicked.Connect(ConnectBytes("panic-connect"));

            Assert.That(result.Status, Is.EqualTo(BattlementTransportStatus.Panic));
            Assert.That(result.Diagnostic, Does.Contain("fixture connect panic"));
            string[] leadingMessages = BattlementLogStore
                .Snapshot(out _)
                .Where(entry => entry.Source == "rust")
                .Select(entry => entry.Record.Message)
                .Where(message => message.Contains("fixture connect panic"))
                .ToArray();
            Assert.That(
                leadingMessages,
                Is.EqualTo(
                    new[] { "Preparing fixture connect panic", "Triggering fixture connect panic" }
                )
            );
            using BattlementTransportResult recovered = panicked.Connect(ConnectBytes("normal"));
            Assert.That(
                recovered.Status,
                Is.EqualTo(BattlementTransportStatus.Success),
                "A panic must destroy the poisoned engine before a new game starts."
            );
            recovered.Dispose();
            Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
        }

        [Test]
        public void RustTracingIsForwardedToUnityAndTheLogViewerStore()
        {
            BattlementNativeLogging.Drain();
            BattlementLogStore.Clear();
            LogAssert.Expect(
                LogType.Log,
                new System.Text.RegularExpressions.Regex(
                    @"^\[Battlement/Rust\]\[fixture\.rust_event\] native trace"
                )
            );

            NativeFixture.fixture_trace();
            BattlementNativeLogging.Drain();

            BattlementLogEntry record = BattlementLogStore
                .Snapshot(out _)
                .Single(entry => entry.Record.EventName == "fixture.rust_event");
            Assert.That(record.Source, Is.EqualTo("rust"));
            Assert.That(record.Record.Message, Is.EqualTo("native trace"));
            Assert.That(record.Record.Fields!["mode"], Is.EqualTo("test"));
        }

        [Test]
        public void CaughtDestroyPanicIsForwardedToUnityAndTheLogViewerStore()
        {
            BattlementLogStore.Clear();
            using var transport = Transport("panic-destroy");
            DittoNativeEngineSession session = CreateSession(transport);
            Assert.That(session.Connect(ConnectBytes("panic-destroy")).Status, Is.EqualTo(Success));
            LogAssert.Expect(
                LogType.Error,
                new System.Text.RegularExpressions.Regex(
                    @"^\[Battlement/Rust\]\[battlement\.rust\.destroy_panic\] "
                        + @"Rust engine panicked during destruction\."
                )
            );

            BattlementTransportResult result = session.Destroy();

            Assert.That(result.Status, Is.EqualTo(BattlementTransportStatus.Panic));
            Assert.That(result.Diagnostic, Does.Contain("fixture destroy panic"));
            Assert.That(transport.HasEngine, Is.False);
            BattlementLogEntry record = BattlementLogStore
                .Snapshot(out _)
                .Single(entry => entry.Record.EventName == "battlement.rust.destroy_panic");
            Assert.That(record.Record.StackTrace, Does.Contain("fixture destroy panic"));
        }

        [Test]
        public void DestroyStatusesRequireBoundedDiagnosticsAndPreserveClassification()
        {
            byte[] diagnostic = Encoding.UTF8.GetBytes("fixture destroy error");
            IntPtr data = Marshal.AllocHGlobal(diagnostic.Length);
            try
            {
                Marshal.Copy(diagnostic, 0, data, diagnostic.Length);
                BattlementTransportResult result = BattlementNativeTransport.TranslateDestroy(
                    3,
                    new BattlementNativeBuffer(
                        1,
                        data,
                        (ulong)diagnostic.Length,
                        (ulong)diagnostic.Length
                    )
                );
                Assert.That(result.Status, Is.EqualTo(BattlementTransportStatus.EngineError));
                Assert.That(result.Diagnostic, Is.EqualTo("fixture destroy error"));

                result = BattlementNativeTransport.TranslateDestroy(3, default);
                Assert.That(result.Status, Is.EqualTo(BattlementTransportStatus.AbiError));
                Assert.That(result.Diagnostic, Does.Contain("without a diagnostic"));

                result = BattlementNativeTransport.TranslateDestroy(
                    3,
                    new BattlementNativeBuffer(
                        1,
                        data,
                        BattlementNativeTransport.MaximumPayloadBytes + 1u,
                        BattlementNativeTransport.MaximumPayloadBytes + 1u
                    )
                );
                Assert.That(result.Status, Is.EqualTo(BattlementTransportStatus.AbiError));
                Assert.That(result.Diagnostic, Does.Contain("exceeded"));
            }
            finally
            {
                Marshal.FreeHGlobal(data);
            }
        }

        [Test]
        public void PayloadLimitAcceptsSixteenMiBAndFreesRejectedOutput()
        {
            using (BattlementNativeTransport boundary = Transport("maximum-response"))
            {
                using BattlementTransportResult result = boundary.Connect(
                    ConnectBytes("maximum-response")
                );
                Assert.That(result.Status, Is.EqualTo(Success));
                Assert.That(result.Payload.Length, Is.EqualTo(16 * 1024 * 1024));
                Assert.That(
                    NativeFixture.fixture_outstanding_buffers(),
                    Is.Not.EqualTo(UIntPtr.Zero)
                );
                result.Dispose();
                Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
            }

            using (BattlementNativeTransport oversized = Transport("oversized-response"))
            {
                BattlementTransportResult result = oversized.Connect(
                    ConnectBytes("oversized-response")
                );
                Assert.That(result.Status, Is.EqualTo(BattlementTransportStatus.EngineError));
                Assert.That(result.Diagnostic, Does.Contain("exceed"));
                Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
            }
        }

        [Test]
        public void NativeResponseMemoryIsUncopiedThreadBoundAndDisposedWithItsLease()
        {
            using BattlementNativeTransport transport = Transport("normal");
            using BattlementTransportResult result = transport.Connect(ConnectBytes("normal"));
            Assert.That(result.Status, Is.EqualTo(Success));
            ReadOnlyMemory<byte> payload = result.BorrowedPayload;
            Assert.That(MemoryMarshal.TryGetArray(payload, out _), Is.False);
            Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.Not.EqualTo(UIntPtr.Zero));
            var allocator = new BattlementNativeByteBufferAllocator(
                (BattlementNativeBufferMemory)result.PayloadOwner!
            );
            var buffer = new ByteBuffer(allocator, 0);
            Assert.That(buffer.Get(0), Is.EqualTo(payload.Span[0]));
            Assert.Throws<InvalidOperationException>(() => _ = allocator.Span);
            Assert.Throws<InvalidOperationException>(() => _ = allocator.Memory);
            Assert.Throws<InvalidOperationException>(() => _ = allocator.ReadOnlyMemory);
            Assert.Throws<InvalidOperationException>(() => allocator.GrowFront(payload.Length + 1));

            Exception? wrongThread = null;
            var thread = new System.Threading.Thread(() =>
            {
                try
                {
                    _ = payload.Span[0];
                }
                catch (Exception exception)
                {
                    wrongThread = exception;
                }
            });
            thread.Start();
            thread.Join();
            Assert.That(wrongThread, Is.TypeOf<InvalidOperationException>());

            result.Dispose();
            Assert.Throws<ObjectDisposedException>(() => _ = payload.Span[0]);
            Assert.Throws<ObjectDisposedException>(() => _ = buffer.Get(0));
            Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
        }

        [Test]
        public void PublicNativeResponsePayloadCannotMutatePublishedBytes()
        {
            using BattlementNativeTransport transport = Transport("normal");
            using BattlementTransportResult result = transport.Connect(ConnectBytes("normal"));
            byte expected = result.BorrowedPayload.Span[0];

            ReadOnlyMemory<byte> exposed = result.Payload;
            Assert.That(
                MemoryMarshal.TryGetArray(exposed, out ArraySegment<byte> segment),
                Is.True
            );
            segment.Array![segment.Offset] ^= 0xff;

            Assert.That(result.BorrowedPayload.Span[0], Is.EqualTo(expected));
            Assert.That(result.Payload.Span[0], Is.EqualTo(expected));
        }

        [Test]
        public void FlatBufferBatchLeaseOutlivesItsResponseAndReleasesExactlyOnce()
        {
            using BattlementNativeTransport transport = Transport("maximum-response");
            using BattlementTransportResult result = transport.Connect(
                ConnectBytes("maximum-response")
            );
            var response = new BattlementFlatBufferResponse(
                result.Payload,
                result.DetachPayloadOwner()
            );
            IBattlementBatchView batch = response.ReadBatch(0);

            response.Dispose();
            Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.Not.EqualTo(UIntPtr.Zero));
            Assert.That(batch.GroupCount, Is.EqualTo(1));
            Assert.That(batch.CommandCount(0), Is.EqualTo(1));

            Exception? wrongThread = null;
            var thread = new System.Threading.Thread(() =>
            {
                try
                {
                    _ = batch.Id;
                }
                catch (Exception exception)
                {
                    wrongThread = exception;
                }
            });
            thread.Start();
            thread.Join();
            Assert.That(wrongThread, Is.TypeOf<InvalidOperationException>());

            batch.Dispose();
            batch.Dispose();
            Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
            Assert.Throws<ObjectDisposedException>(() => _ = batch.GroupCount);
        }

        [Test]
        public void ResponseReaderRejectsEveryTruncatedRangeAndHostileRootOffset()
        {
            using BattlementNativeTransport transport = Transport("normal");
            using BattlementTransportResult result = transport.Connect(ConnectBytes("normal"));
            byte[] bytes = result.Payload.ToArray();
            result.Dispose();

            for (int length = 0; length < bytes.Length; length++)
            {
                Assert.Throws<System.IO.InvalidDataException>(() =>
                    new BattlementFlatBufferResponse(
                        new ReadOnlyMemory<byte>(bytes, 0, length),
                        null
                    )
                );
            }
            byte[] hostile = (byte[])bytes.Clone();
            hostile[4] = 0xff;
            hostile[5] = 0xff;
            hostile[6] = 0xff;
            hostile[7] = 0xff;
            Assert.Throws<System.IO.InvalidDataException>(() =>
                new BattlementFlatBufferResponse(hostile, null)
            );
        }

        [Test]
        public void CSharpUiEventWriterRoundTripsThroughRustVerifierAndBorrowedEngineCall()
        {
            using BattlementNativeTransport transport = Transport("normal");
            using BattlementTransportResult connected = transport.Connect(ConnectBytes("normal"));
            SessionId session;
            using (var response = new BattlementFlatBufferResponse(connected.Payload, null))
                session = response.SessionId;
            var action = new UiEventAction(
                new ActionId(Guid.Parse("11111111-1111-4111-8111-111111111111")),
                session,
                new UiEvent(
                    new ObjectId(Guid.Parse("33333333-3333-4333-8333-333333333333")),
                    true,
                    true,
                    new UiEventBody.Click(new ClickEvent.NavigationSubmit())
                )
            );

            using BattlementUiEventTransportResult result = transport.SubmitUiEvent(action);

            Assert.That(result.Status, Is.EqualTo(Success));
            Assert.That(result.Disposition, Is.EqualTo(UiEventDisposition.PreventDefault));
            using (var response = new BattlementFlatBufferResponse(result.ResponsePayload, null))
                Assert.That(response.SessionId, Is.EqualTo(session));
            ReadOnlyMemory<byte> exposed = result.ResponsePayload;
            Assert.That(
                MemoryMarshal.TryGetArray(exposed, out ArraySegment<byte> segment),
                Is.True
            );
            byte expected = result.BorrowedResponsePayload.Span[0];
            segment.Array![segment.Offset] ^= 0xff;
            Assert.That(result.BorrowedResponsePayload.Span[0], Is.EqualTo(expected));
            Assert.That(result.ResponsePayload.Span[0], Is.EqualTo(expected));
        }

        [Test]
        public void ClientBuilderGrowthAndRetainedCapacityAreObservable()
        {
            using BattlementNativeTransport transport = Transport("normal");
            using BattlementTransportResult connected = transport.Connect(ConnectBytes("normal"));
            SessionId session;
            using (var response = new BattlementFlatBufferResponse(connected.Payload, null))
                session = response.SessionId;
            BattlementNativeTransportDiagnostics before = transport.Diagnostics;
            var action = new UiEventAction(
                new ActionId(Guid.Parse("11111111-1111-4111-8111-111111111112")),
                session,
                new UiEvent(
                    new ObjectId(Guid.Parse("33333333-3333-4333-8333-333333333334")),
                    true,
                    false,
                    new UiEventBody.Input(new TextInputEvent(new string('x', 8 * 1024)))
                )
            );

            using BattlementUiEventTransportResult result = transport.SubmitUiEvent(action);
            BattlementNativeTransportDiagnostics after = transport.Diagnostics;

            Assert.That(result.Status, Is.EqualTo(Success));
            Assert.That(after.ClientBuilderGrowths, Is.GreaterThan(before.ClientBuilderGrowths));
            Assert.That(
                after.ClientBuilderCopiedBytes,
                Is.GreaterThan(before.ClientBuilderCopiedBytes)
            );
            Assert.That(
                after.ClientBuilderRetainedBytes,
                Is.GreaterThan(before.ClientBuilderRetainedBytes)
            );
            Assert.That(after.HandoffPayloadCopies, Is.Zero);

            UiEventAction oversized = action with
            {
                Id = new ActionId(Guid.Parse("11111111-1111-4111-8111-111111111113")),
                Event = action.Event with
                {
                    Body = new UiEventBody.Input(
                        new TextInputEvent(new string('y', 2 * 1024 * 1024))
                    ),
                },
            };
            using BattlementUiEventTransportResult large = transport.SubmitUiEvent(oversized);
            BattlementNativeTransportDiagnostics trimmed = transport.Diagnostics;
            Assert.That(large.Status, Is.EqualTo(Success));
            Assert.That(trimmed.ClientBuilderRetainedBytes, Is.EqualTo(5 * 1024));
            Assert.That(trimmed.HandoffPayloadCopies, Is.Zero);
        }

        [Test]
        public void DirectNativeLabelUpdateStaysTypedThroughTheCSharpBatchView()
        {
            FixtureFlatBufferResponseSchema schema = FixtureSchema();
            using BattlementNativeTransport transport = Transport("direct-native-label");
            using BattlementTransportResult connected = transport.Connect(
                ConnectBytes("direct-native-label")
            );
            SessionId session;
            using (
                var response = new BattlementCustomFlatBufferResponse(
                    connected.Payload,
                    connected.DetachPayloadOwner(),
                    schema
                )
            )
                session = response.SessionId;

            using BattlementTransportResult result = transport.Submit(
                new Action(
                    new ActionId(new Guid("00000002-1234-5678-90ab-000000000003")),
                    session,
                    new ActionBody.KeyDown(PhysicalKey.KeyA)
                )
            );
            using var returned = new BattlementCustomFlatBufferResponse(
                result.Payload,
                result.DetachPayloadOwner(),
                schema
            );
            using IBattlementBatchView batch = returned.ReadBatch(0);

            BattlementCommandExecution command = batch.ReadCommand(0, 0);

            Assert.That(command.CoreBody, Is.Null);
            Assert.That(schema.OwnedCoreCommandMaterializations, Is.Zero);
            Assert.That(command.DirectLabelUpdate.HasValue, Is.True);
            Assert.That(
                command.DirectLabelUpdate!.Value.ReadText(),
                Is.EqualTo("direct native label")
            );
            Assert.That(
                command.DirectLabelUpdate.Value.ObjectId.Value,
                Is.EqualTo(new Guid(Enumerable.Repeat((byte)0x42, 16).ToArray()))
            );
        }

        [Test]
        public void DirectNativeUiScalarUpdatesAvoidOwnedElementMaterialization()
        {
            FixtureFlatBufferResponseSchema schema = FixtureSchema();
            using BattlementNativeTransport transport = Transport("direct-native-ui-scalars");
            using BattlementTransportResult connected = transport.Connect(
                ConnectBytes("direct-native-ui-scalars")
            );
            SessionId session;
            using (
                var response = new BattlementCustomFlatBufferResponse(
                    connected.Payload,
                    connected.DetachPayloadOwner(),
                    schema
                )
            )
                session = response.SessionId;

            using BattlementTransportResult result = transport.Submit(
                new Action(
                    new ActionId(new Guid("00000002-1234-5678-90ab-000000000006")),
                    session,
                    new ActionBody.KeyDown(PhysicalKey.KeyA)
                )
            );
            using var returned = new BattlementCustomFlatBufferResponse(
                result.Payload,
                result.DetachPayloadOwner(),
                schema
            );
            using IBattlementBatchView batch = returned.ReadBatch(0);

            BattlementDirectUiScalar[] updates = Enumerable
                .Range(0, 14)
                .Select(index => batch.ReadCommand(0, index).DirectUiScalar!.Value)
                .ToArray();

            Assert.That(schema.OwnedCoreCommandMaterializations, Is.Zero);
            Assert.That(updates[0].ReadText(), Is.EqualTo("borrowed text"));
            Assert.That(updates[1].Boolean, Is.True);
            Assert.That(updates[2].Unsigned, Is.EqualTo(3));
            Assert.That(
                Enumerable.Range(0, updates[3].IndexCount).Select(updates[3].ReadIndex),
                Is.EqualTo(new uint[] { 1, 4, 7 })
            );
            Assert.That(updates[4].Boolean, Is.True);
            Assert.That(updates[4].Unsigned, Is.EqualTo(2));
            Assert.That(updates[4].ReadText(), Is.EqualTo("third"));
            Assert.That(updates[5].First, Is.EqualTo(4.5f));
            Assert.That(updates[6].First, Is.EqualTo(6.25f));
            Assert.That(updates[7].Integer, Is.EqualTo(-12));
            Assert.That(updates[8].First, Is.EqualTo(2.0f));
            Assert.That(updates[8].Second, Is.EqualTo(8.0f));
            Assert.That(updates[9].Unsigned, Is.EqualTo(5));
            Assert.That(updates[10].ReadText(), Is.EqualTo("text only"));
            Assert.That(updates[11].Boolean, Is.True);
            Assert.That(updates[12].ReadText(), Is.EqualTo("ready"));
            Assert.That(updates[12].Boolean, Is.False);
            Assert.That(updates[13].Unsigned, Is.EqualTo(200));
            Assert.That(updates[13].UnsignedSecond, Is.EqualTo(100));
        }

        [Test]
        public void DirectNativeImageCreateStaysFlattenedThroughTheCSharpBatchView()
        {
            FixtureFlatBufferResponseSchema schema = FixtureSchema();
            using BattlementNativeTransport transport = Transport("direct-native-image");
            using BattlementTransportResult connected = transport.Connect(
                ConnectBytes("direct-native-image")
            );
            SessionId session;
            using (
                var response = new BattlementCustomFlatBufferResponse(
                    connected.Payload,
                    connected.DetachPayloadOwner(),
                    schema
                )
            )
                session = response.SessionId;

            using BattlementTransportResult result = transport.Submit(
                new Action(
                    new ActionId(new Guid("00000002-1234-5678-90ab-000000000004")),
                    session,
                    new ActionBody.KeyDown(PhysicalKey.KeyA)
                )
            );
            using var returned = new BattlementCustomFlatBufferResponse(
                result.Payload,
                result.DetachPayloadOwner(),
                schema
            );
            using IBattlementBatchView batch = returned.ReadBatch(0);

            BattlementCommandExecution command = batch.ReadCommand(0, 0);

            Assert.That(command.CoreBody, Is.Null);
            Assert.That(schema.OwnedCoreCommandMaterializations, Is.Zero);
            Assert.That(command.DirectImageObjectCreate.HasValue, Is.True);
            BattlementDirectImageObjectCreate image = command.DirectImageObjectCreate!.Value;
            Assert.That(image.Texture, Is.EqualTo("fixture-texture"));
            Assert.That(image.Width, Is.EqualTo(12));
            Assert.That(image.Height, Is.EqualTo(8));
            Assert.That(image.Fit, Is.EqualTo(ImageFit.Contain));
            Assert.That(image.Placement.PositionY, Is.EqualTo(2));
            Assert.That(image.Placement.ScaleZ, Is.EqualTo(4));
            Assert.That(image.Placement.PointerEvents, Is.EqualTo(new[] { PointerEvent.Click }));
            Assert.That(image.Placement.DragMode, Is.EqualTo(DragMode.PreserveOffset));
        }

        [Test]
        public void DirectNativeTransformAndActiveCommandsStayFlattenedThroughTheBatchView()
        {
            FixtureFlatBufferResponseSchema schema = FixtureSchema();
            using BattlementNativeTransport transport = Transport("direct-native-transforms");
            using BattlementTransportResult connected = transport.Connect(
                ConnectBytes("direct-native-transforms")
            );
            SessionId session;
            using (
                var response = new BattlementCustomFlatBufferResponse(
                    connected.Payload,
                    connected.DetachPayloadOwner(),
                    schema
                )
            )
                session = response.SessionId;

            using BattlementTransportResult result = transport.Submit(
                new Action(
                    new ActionId(new Guid("00000002-1234-5678-90ab-000000000005")),
                    session,
                    new ActionBody.KeyDown(PhysicalKey.KeyA)
                )
            );
            using var returned = new BattlementCustomFlatBufferResponse(
                result.Payload,
                result.DetachPayloadOwner(),
                schema
            );
            using IBattlementBatchView batch = returned.ReadBatch(0);

            BattlementCommandExecution localRotation = batch.ReadCommand(0, 0);
            BattlementCommandExecution worldRotation = batch.ReadCommand(0, 1);
            BattlementCommandExecution scale = batch.ReadCommand(0, 2);
            BattlementCommandExecution active = batch.ReadCommand(0, 3);
            BattlementCommandExecution positionTween = batch.ReadCommand(0, 4);
            BattlementCommandExecution localRotationTween = batch.ReadCommand(0, 5);
            BattlementCommandExecution worldRotationTween = batch.ReadCommand(0, 6);
            BattlementCommandExecution scaleTween = batch.ReadCommand(0, 7);
            BattlementCommandExecution primitive = batch.ReadCommand(0, 8);
            BattlementCommandExecution prefab = batch.ReadCommand(0, 9);
            BattlementCommandExecution empty = batch.ReadCommand(0, 10);
            BattlementCommandExecution text = batch.ReadCommand(0, 11);
            BattlementCommandExecution camera = batch.ReadCommand(0, 12);
            BattlementCommandExecution light = batch.ReadCommand(0, 13);
            BattlementCommandExecution reparent = batch.ReadCommand(0, 14);
            BattlementCommandExecution particle = batch.ReadCommand(0, 15);
            BattlementCommandExecution audio = batch.ReadCommand(0, 16);
            BattlementCommandExecution particlePlay = batch.ReadCommand(0, 17);
            BattlementCommandExecution audioStop = batch.ReadCommand(0, 18);
            BattlementCommandExecution audioVolume = batch.ReadCommand(0, 19);
            BattlementCommandExecution wait = batch.ReadCommand(0, 20);
            BattlementCommandExecution vibration = batch.ReadCommand(0, 21);
            BattlementCommandExecution debugUi = batch.ReadCommand(0, 22);
            BattlementCommandExecution audioPause = batch.ReadCommand(0, 23);
            BattlementCommandExecution audioResume = batch.ReadCommand(0, 24);
            BattlementCommandExecution audioSeek = batch.ReadCommand(0, 25);
            BattlementCommandExecution audioBuffering = batch.ReadCommand(0, 26);
            BattlementCommandExecution audioReplace = batch.ReadCommand(0, 27);
            BattlementCommandExecution audioTween = batch.ReadCommand(0, 28);
            BattlementCommandExecution openUrl = batch.ReadCommand(0, 29);
            BattlementCommandExecution sceneLoad = batch.ReadCommand(0, 30);
            BattlementCommandExecution sceneUnload = batch.ReadCommand(0, 31);
            BattlementCommandExecution scenePrimary = batch.ReadCommand(0, 32);
            BattlementCommandExecution particleStop = batch.ReadCommand(0, 33);
            BattlementCommandExecution cancel = batch.ReadCommand(0, 34);

            Assert.That(schema.OwnedCoreCommandMaterializations, Is.Zero);
            Assert.That(localRotation.CoreBody, Is.Null);
            Assert.That(localRotation.DirectRotation!.Value.World, Is.False);
            Assert.That(localRotation.DirectRotation.Value.Z, Is.EqualTo(0.5));
            Assert.That(localRotation.DirectConflictPolicy, Is.EqualTo(ConflictPolicy.Cancel));
            Assert.That(worldRotation.CoreBody, Is.Null);
            Assert.That(worldRotation.DirectRotation!.Value.World, Is.True);
            Assert.That(worldRotation.DirectRotation.Value.Y, Is.EqualTo(0.25));
            Assert.That(scale.CoreBody, Is.Null);
            Assert.That(scale.DirectScale!.Value.Z, Is.EqualTo(4));
            Assert.That(active.CoreBody, Is.Null);
            Assert.That(active.DirectObjectActive!.Value.Active, Is.False);
            Assert.That(positionTween.CoreBody, Is.Null);
            Assert.That(positionTween.DirectTweenLocalPosition!.Value.World, Is.True);
            Assert.That(positionTween.DirectTweenLocalPosition.Value.Z, Is.EqualTo(7));
            Assert.That(
                positionTween.DirectTweenLocalPosition.Value.Tween.DurationMilliseconds,
                Is.EqualTo(250)
            );
            Assert.That(localRotationTween.CoreBody, Is.Null);
            Assert.That(localRotationTween.DirectTweenRotation!.Value.World, Is.False);
            Assert.That(localRotationTween.DirectTweenRotation.Value.W, Is.EqualTo(0.75));
            Assert.That(worldRotationTween.CoreBody, Is.Null);
            Assert.That(worldRotationTween.DirectTweenRotation!.Value.World, Is.True);
            Assert.That(
                worldRotationTween.DirectTweenRotation.Value.Tween.DurationMilliseconds,
                Is.EqualTo(350)
            );
            Assert.That(scaleTween.CoreBody, Is.Null);
            Assert.That(scaleTween.DirectTweenScale!.Value.Y, Is.EqualTo(7));
            Assert.That(primitive.CoreBody, Is.Null);
            Assert.That(
                primitive.DirectPrimitiveObjectCreate!.Value.Kind,
                Is.EqualTo(Wire.GameObjectKind.Plane)
            );
            Assert.That(
                primitive.DirectPrimitiveObjectCreate.Value.Materials[0].Address,
                Is.EqualTo("fixture-material")
            );
            Assert.That(prefab.CoreBody, Is.Null);
            Assert.That(
                prefab.DirectPrefabObjectCreate!.Value.Address,
                Is.EqualTo("fixture-prefab")
            );
            Assert.That(prefab.DirectPrefabObjectCreate.Value.Materials[0].Slot, Is.EqualTo(1));
            Assert.That(
                prefab.DirectPrefabObjectCreate.Value.Animator!.Value.State,
                Is.EqualTo("Idle")
            );
            Assert.That(empty.CoreBody, Is.Null);
            Assert.That(empty.DirectEmptyObjectCreate.HasValue, Is.True);
            Assert.That(text.CoreBody, Is.Null);
            Assert.That(text.DirectTextObjectCreate!.Value.Text, Is.EqualTo("fixture text"));
            Assert.That(text.DirectTextObjectCreate.Value.WrapWidth, Is.EqualTo(9));
            Assert.That(text.DirectTextObjectCreate.Value.Horizontal, Is.EqualTo(1));
            Assert.That(text.DirectTextObjectCreate.Value.Vertical, Is.EqualTo(1));
            Assert.That(camera.CoreBody, Is.Null);
            Assert.That(camera.DirectCameraObjectCreate!.Value.FieldOfView, Is.EqualTo(70));
            Assert.That(light.CoreBody, Is.Null);
            Assert.That(light.DirectLightObjectCreate!.Value.Range, Is.EqualTo(12));
            Assert.That(reparent.CoreBody, Is.Null);
            Assert.That(reparent.DirectObjectReparent!.Value.WorldPositionStays, Is.True);
            Assert.That(particle.CoreBody, Is.Null);
            Assert.That(particle.DirectParticleSpawn!.Value.Address, Is.EqualTo("fixture-effect"));
            Assert.That(particle.DirectParticleSpawn.Value.Z, Is.EqualTo(11));
            Assert.That(particle.DirectParticleSpawn.Value.LifetimeMilliseconds, Is.EqualTo(750));
            Assert.That(audio.CoreBody, Is.Null);
            Assert.That(audio.DirectAudioPlay!.Value.Address, Is.EqualTo("fixture-audio"));
            Assert.That(audio.DirectAudioPlay.Value.Volume, Is.EqualTo(0.75));
            Assert.That(audio.DirectAudioPlay.Value.Loop, Is.True);
            Assert.That(particlePlay.CoreBody, Is.Null);
            Assert.That(particlePlay.DirectParticlePlay!.Value.Restart, Is.True);
            Assert.That(audioStop.CoreBody, Is.Null);
            Assert.That(audioStop.DirectAudioStop!.Value.FadeOutMilliseconds, Is.EqualTo(200));
            Assert.That(audioVolume.CoreBody, Is.Null);
            Assert.That(audioVolume.DirectAudioVolume!.Value.Volume, Is.EqualTo(0.5));
            Assert.That(audioVolume.DirectConflictPolicy, Is.EqualTo(ConflictPolicy.Cancel));
            Assert.That(wait.CoreBody, Is.Null);
            Assert.That(wait.DirectWait!.Value.DurationMilliseconds, Is.EqualTo(300));
            Assert.That(vibration.CoreBody, Is.Null);
            Assert.That(vibration.DirectVibration!.Value.HighFrequency, Is.EqualTo(0.75));
            Assert.That(debugUi.CoreBody, Is.Null);
            Assert.That(debugUi.DirectDebugUi!.Value.Surface, Is.EqualTo(1));
            Assert.That(
                audioPause.DirectAudioControl!.Value.Kind,
                Is.EqualTo(BattlementDirectAudioControlKind.Pause)
            );
            Assert.That(
                audioResume.DirectAudioControl!.Value.Kind,
                Is.EqualTo(BattlementDirectAudioControlKind.Resume)
            );
            Assert.That(audioSeek.DirectAudioControl!.Value.PositionMilliseconds, Is.EqualTo(500));
            Assert.That(audioBuffering.DirectAudioControl!.Value.Buffering, Is.True);
            Assert.That(
                audioReplace.DirectAudioControl!.Value.Address,
                Is.EqualTo("replacement-audio")
            );
            Assert.That(audioTween.CoreBody, Is.Null);
            Assert.That(audioTween.DirectTweenAudioVolume!.Value.Volume, Is.EqualTo(0.25));
            Assert.That(
                audioTween.DirectTweenAudioVolume.Value.Tween.DurationMilliseconds,
                Is.EqualTo(600)
            );
            Assert.That(openUrl.DirectOpenUrl!.Value.Url, Does.StartWith("https://"));
            Assert.That(
                sceneLoad.DirectSceneCommand!.Value.Kind,
                Is.EqualTo(BattlementDirectSceneCommandKind.Load)
            );
            Assert.That(sceneLoad.DirectSceneCommand.Value.MakePrimary, Is.True);
            Assert.That(
                sceneUnload.DirectSceneCommand!.Value.Kind,
                Is.EqualTo(BattlementDirectSceneCommandKind.Unload)
            );
            Assert.That(
                scenePrimary.DirectSceneCommand!.Value.Kind,
                Is.EqualTo(BattlementDirectSceneCommandKind.SetPrimary)
            );
            Assert.That(particleStop.DirectParticleStop!.Value.Clear, Is.True);
            Assert.That(
                cancel.DirectCancel!.Value.CommandId.Value,
                Is.EqualTo(new Guid("97979797-9797-9797-9797-979797979797"))
            );
        }

        [Test]
        public void WrongThreadCallsDoNotEnterTheNativeEngine()
        {
            using BattlementNativeTransport transport = Transport("normal");
            BattlementTransportResult? result = null;
            var thread = new System.Threading.Thread(() =>
                result = transport.Connect(ConnectBytes("normal"))
            );
            thread.Start();
            thread.Join();
            Assert.That(result!.Status, Is.EqualTo(BattlementTransportStatus.AbiError));
            Assert.That(NativeFixture.fixture_outstanding_buffers(), Is.EqualTo(UIntPtr.Zero));
        }

        [Test]
        public void PlatformMappingNamesTheRequiredPluginArtifact()
        {
#if UNITY_EDITOR_OSX
            Assert.That(
                BattlementNativeTransport.RequiredPluginName,
                Is.EqualTo("libbattlement_rules.dylib")
            );
#elif UNITY_EDITOR_WIN
            Assert.That(
                BattlementNativeTransport.RequiredPluginName,
                Is.EqualTo("battlement_rules.dll")
            );
#endif
        }

        [Test]
        public void NativeBufferShapeValidationRejectsMalformedAndOversizedOutputs()
        {
            Assert.That(
                new BattlementNativeBuffer(1, IntPtr.Zero, 1, 1).ValidateShape(16),
                Does.Contain("nonempty finished range")
            );
            Assert.That(
                new BattlementNativeBuffer(1, new IntPtr(1), 0, 1).ValidateShape(16),
                Does.Contain("nonempty finished range")
            );
            Assert.That(
                new BattlementNativeBuffer(1, new IntPtr(1), 17, 17).ValidateShape(16),
                Does.Contain("16-byte limit")
            );
            Assert.That(
                new BattlementNativeBuffer(1, new IntPtr(1), 16, 16).ValidateShape(16),
                Is.Null
            );
            Assert.That(
                new BattlementNativeBuffer(1, new IntPtr(1), 2, 1).ValidateShape(16),
                Does.Contain("allocation size")
            );
        }

        private static BattlementNativeTransport Transport(string platform)
        {
            var transport = new BattlementNativeTransport();
            transport.SetExpectedWireContractDigest(FixtureFlatBufferResponseSchema.ContractDigest);
            return transport;
        }

        private static FixtureFlatBufferResponseSchema FixtureSchema() =>
            new(
                error => (byte)(FixtureError)error,
                payload =>
                {
                    var value = (FlashPayload)payload;
                    return (value.ObjectId, value.Scale);
                }
            );

        [MethodImpl(MethodImplOptions.NoInlining)]
        private static void AbandonPollResponse(BattlementNativeTransport transport)
        {
            BattlementTransportResult result = transport.Poll();
            Assert.That(result.Status, Is.EqualTo(Success));
            Assert.That(result.Payload.IsEmpty, Is.False);
        }

        private static DittoNativeEngineSession CreateSession(BattlementNativeTransport transport)
        {
            DittoNativeEngineSession? session = DittoNativeEngineSession.Create(
                transport,
                out BattlementTransportResult result
            );
            Assert.That(result.Status, Is.EqualTo(Success));
            return session!;
        }

        private static BattlementLogEntry[] EngineJournal() =>
            BattlementLogStore
                .Snapshot(out _)
                .Where(entry => entry.Record.EventName.StartsWith("fixture.engine."))
                .ToArray();

        private static Connect ConnectBytes(string platform) =>
            new(
                platform,
                Application.unityVersion,
                new ScreenSize((uint)Screen.width, (uint)Screen.height)
            );

        private static Action ClientMessage() =>
            new(
                new ActionId(new Guid("00000002-1234-5678-90ab-000000000002")),
                new SessionId(new Guid("00112233-4455-6677-8899-aabbccddeeff")),
                new ActionBody.KeyDown(PhysicalKey.KeyA)
            );

        private static BattlementTransportStatus Success => BattlementTransportStatus.Success;

        private static BattlementTransportStatus NoMessage => BattlementTransportStatus.NoMessage;

        private static class NativeFixture
        {
            [DllImport("battlement_rules", CallingConvention = CallingConvention.Cdecl)]
            internal static extern UIntPtr fixture_outstanding_buffers();

            [DllImport("battlement_rules", CallingConvention = CallingConvention.Cdecl)]
            internal static extern UIntPtr fixture_connect_calls();

            [DllImport("battlement_rules", CallingConvention = CallingConvention.Cdecl)]
            internal static extern void fixture_trace();
        }
    }
}
