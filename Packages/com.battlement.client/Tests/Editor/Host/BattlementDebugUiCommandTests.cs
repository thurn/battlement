#nullable enable

using System;
using NUnit.Framework;
using UnityEngine;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class BattlementDebugUiCommandTests
    {
        [Test]
        public void OneCommandBodyControlsBothDeveloperSurfaces()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var debugHostObject = new GameObject("Battlement debug UI test");
            BattlementLoggingHost debugHost = debugHostObject.AddComponent<BattlementLoggingHost>();
            debugHost.Initialize();
            try
            {
                SessionId session = new(Guid.NewGuid());
                harness.Transport.EnqueueConnect(FakeBattlementTransport.SnapshotResponse(session));
                harness.Runner.Connect();

                Submit(harness, session, DebugUiSurface.LogViewer, true);
                Submit(harness, session, DebugUiSurface.FpsViewer, true);
                Assert.That(debugHost.IsVisible(DebugUiSurface.LogViewer), Is.True);
                Assert.That(debugHost.IsVisible(DebugUiSurface.FpsViewer), Is.True);

                Submit(harness, session, DebugUiSurface.LogViewer, false);
                Submit(harness, session, DebugUiSurface.FpsViewer, false);
                Assert.That(debugHost.IsVisible(DebugUiSurface.LogViewer), Is.False);
                Assert.That(debugHost.IsVisible(DebugUiSurface.FpsViewer), Is.False);
            }
            finally
            {
                Object.DestroyImmediate(debugHostObject);
            }
        }

        private static void Submit(
            BattlementTestHarness harness,
            SessionId session,
            DebugUiSurface surface,
            bool visible
        )
        {
            var batch = new Batch(
                new BatchId(Guid.NewGuid()),
                session,
                new[]
                {
                    new ParallelCommandGroup<Command>(
                        new[]
                        {
                            new Command(
                                new CommandId(Guid.NewGuid()),
                                new CommandBody.DebugUi(surface, visible)
                            ),
                        }
                    ),
                },
                Start: BatchStart.Now
            );
            harness.Transport.EnqueueSubmit(
                FakeBattlementTransport.ResponseResult(
                    new Response(
                        session,
                        new ResponseMessage<Command>[]
                        {
                            new ResponseMessage<Command>.BatchMessage(batch),
                        }
                    )
                )
            );
            harness.Runner.Submit(new byte[] { 1 });
            harness.Runner.RunFrame();
        }
    }
}
