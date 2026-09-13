#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using NUnit.Framework;
using UnityEngine.TestTools;

namespace Battlement.Tests
{
    public sealed class BattlementApplicationTests
    {
        [Test]
        public void LifecycleObservationsSurviveInputDisablementAndResume()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(session, inputDisabled: true)
            );
            harness.Transport.DefaultSubmitResult = () =>
                FakeBattlementTransport.ResponseResult(
                    new Response(session, Array.Empty<ResponseMessage<Command>>())
                );
            harness.Runner.Connect();
            Connect connect = harness.Transport.ConnectValues.Single();
            Assert.That(connect.ApplicationState, Is.EqualTo(new ApplicationState()));

            LogAssert.ignoreFailingMessages = true;
            try
            {
                harness.Runner.SendMessage("OnApplicationFocus", false);
                harness.Runner.SendMessage("OnApplicationPause", true);
                harness.Runner.SendMessage("OnApplicationPause", false);
                harness.Runner.SendMessage("OnApplicationFocus", true);
                harness.Runner.SendMessage("OnApplicationFocus", true);
            }
            finally
            {
                LogAssert.ignoreFailingMessages = false;
            }
            harness.Runner.RunFrame();

            ApplicationState[] observed = harness
                .Transport.Actions.Select(action =>
                    ((ActionBody.ApplicationStateChanged)action.Body).Value
                )
                .ToArray();
            Assert.That(
                observed,
                Is.EqualTo(
                    new[]
                    {
                        new ApplicationState(false, false),
                        new ApplicationState(false, true),
                        new ApplicationState(false, false),
                        new ApplicationState(true, false),
                    }
                )
            );
            Assert.That(harness.Transport.Calls, Does.Not.Contain("stop"));
            Assert.That(harness.Runner.IsInputAvailable, Is.False);
        }

        [Test]
        public void ExternalUrlCommandUsesTheConfiguredPlatformHandlerOnce()
        {
            var opened = new List<string>();
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                openExternalUrl: opened.Add
            );
            SessionId session = new(Guid.NewGuid());
            harness.Transport.EnqueueConnect(FakeBattlementTransport.SnapshotResponse(session));
            harness.Runner.Connect();
            const string url = "https://example.com/privacy?source=settings#policy";
            var command = new Command(
                new CommandId(Guid.NewGuid()),
                new CommandBody.ApplicationOpenUrl(url)
            );
            var batch = new Batch(
                new BatchId(Guid.NewGuid()),
                session,
                new[] { new ParallelCommandGroup<Command>(new[] { command }) },
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
            Assert.That(opened, Is.EqualTo(new[] { url }));
        }
    }
}
