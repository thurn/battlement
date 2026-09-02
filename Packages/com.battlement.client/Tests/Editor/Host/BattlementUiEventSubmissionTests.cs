#nullable enable

using System;
using System.Linq;
using System.Reflection;
using NUnit.Framework;

namespace Battlement.Tests
{
    public sealed class BattlementUiEventSubmissionTests
    {
        private static readonly MethodInfo EmitUiEvent = typeof(BattlementRunner).GetMethod(
            "EmitUiEvent",
            BindingFlags.Instance | BindingFlags.NonPublic
        )!;

        [Test]
        public void ImmediateDispositionPrecedesDeferredResponseApplication()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            harness.Transport.EnqueueConnect(FakeBattlementTransport.SnapshotResponse(session));
            harness.Runner.Connect();
            harness.Transport.EnqueueUiEvent(
                new BattlementUiEventTransportResult(
                    BattlementTransportStatus.Success,
                    UiEventDisposition.PreventDefault,
                    SnapshotPayload(session, inputDisabled: true)
                )
            );

            UiEventDisposition? disposition = Invoke(harness.Runner, Event());

            Assert.That(disposition, Is.EqualTo(UiEventDisposition.PreventDefault));
            Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("submit_ui_event"));
            Assert.That(harness.Runner.IsInputAvailable, Is.True);
            BattlementUiEventInspection inspection = harness.Runner.UiEventInspections.Single();
            Assert.That(inspection.Kind, Is.EqualTo(UiEventKind.Click));
            Assert.That(inspection.AdmissionSequence, Is.EqualTo(1));
            Assert.That(inspection.Disposition, Is.EqualTo(UiEventDisposition.PreventDefault));
            Assert.That(
                inspection.Outcome,
                Is.EqualTo(BattlementUiEventInspectionOutcome.Completed)
            );

            harness.Runner.RunFrame();

            Assert.That(harness.Runner.IsInputAvailable, Is.False);
            Assert.That(inspection.AppliedAt, Is.Not.Null);
        }

        [Test]
        public void UiAndOrdinaryResponsesDrainInAdmissionOrder()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            harness.Transport.EnqueueConnect(FakeBattlementTransport.SnapshotResponse(session));
            harness.Runner.Connect();
            harness.Transport.EnqueueUiEvent(
                new BattlementUiEventTransportResult(
                    BattlementTransportStatus.Success,
                    UiEventDisposition.Continue,
                    SnapshotPayload(session, inputDisabled: true)
                )
            );
            harness.Transport.EnqueueSubmit(
                new BattlementTransportResult(
                    BattlementTransportStatus.Success,
                    SnapshotPayload(session, inputDisabled: false)
                )
            );

            Assert.That(Invoke(harness.Runner, Event()), Is.EqualTo(UiEventDisposition.Continue));
            harness.Runner.Submit(new byte[] { 7 });

            Assert.That(
                harness.Transport.Calls.TakeLast(2),
                Is.EqualTo(new[] { "submit_ui_event", "submit" })
            );
            Assert.That(harness.Runner.IsInputAvailable, Is.True);
        }

        [Test]
        public void FailedUiSubmissionAddsNoPreventionAndStopsAtTheNextBoundary()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            harness.Transport.EnqueueConnect(FakeBattlementTransport.SnapshotResponse(session));
            harness.Runner.Connect();
            harness.Transport.EnqueueUiEvent(
                new BattlementUiEventTransportResult(
                    BattlementTransportStatus.EngineError,
                    UiEventDisposition.PreventDefault,
                    ReadOnlyMemory<byte>.Empty,
                    "fixture failed"
                )
            );

            Assert.That(Invoke(harness.Runner, Event()), Is.Null);
            Assert.That(harness.Runner.IsInputAvailable, Is.False);
            Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("submit_ui_event"));

            harness.Runner.RunFrame();

            Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("stop"));
            Assert.That(harness.Runner.IsInputAvailable, Is.False);
        }

        [TestCase(9u, false)]
        [TestCase(1u, true)]
        public void InvalidSuccessfulResultsAddNoPreventionAndFailAtTheNextBoundary(
            uint disposition,
            bool emptyPayload
        )
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            harness.Transport.EnqueueConnect(FakeBattlementTransport.SnapshotResponse(session));
            harness.Runner.Connect();
            harness.Transport.EnqueueUiEvent(
                new BattlementUiEventTransportResult(
                    BattlementTransportStatus.Success,
                    (UiEventDisposition)disposition,
                    emptyPayload ? ReadOnlyMemory<byte>.Empty : SnapshotPayload(session, false)
                )
            );

            Assert.That(Invoke(harness.Runner, Event()), Is.Null);
            Assert.That(harness.Runner.IsInputAvailable, Is.False);
            Assert.That(
                harness.Runner.UiEventInspections.Single().FailureReason,
                Is.EqualTo(
                    emptyPayload
                        ? BattlementUiEventFailureReason.ResponseSerialization
                        : BattlementUiEventFailureReason.InvalidDisposition
                )
            );

            harness.Runner.RunFrame();

            Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("stop"));
            Assert.That(
                harness.Logger.Records.Last().Message,
                Does.Contain(emptyPayload ? "empty response payload" : "unknown disposition")
            );
        }

        private static UiEvent Event() =>
            new(
                new ObjectId(Guid.NewGuid()),
                true,
                false,
                new UiEventBody.Click(new ClickEvent.NavigationSubmit())
            );

        private static UiEventDisposition? Invoke(BattlementRunner runner, UiEvent value) =>
            (UiEventDisposition?)EmitUiEvent.Invoke(runner, new object[] { value });

        private static byte[] SnapshotPayload(SessionId session, bool inputDisabled) =>
            BattlementJson.SerializeResponse(
                new Response(
                    session,
                    new ResponseMessage<Command>[]
                    {
                        new ResponseMessage<Command>.SnapshotMessage(
                            FakeBattlementTransport.CompleteSnapshot(
                                session,
                                inputDisabled: inputDisabled
                            )
                        ),
                    }
                )
            );
    }
}
