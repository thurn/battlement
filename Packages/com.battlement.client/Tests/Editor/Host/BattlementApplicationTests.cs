#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using Newtonsoft.Json.Linq;
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
            harness.Transport.DefaultSubmitResult = FakeBattlementTransport.ResponseResult(
                new Response(session, Array.Empty<ResponseMessage<Command>>())
            );
            harness.Runner.Connect();
            Connect connect = BattlementJson.Deserialize<Connect>(
                harness.Transport.ConnectMessages.Single()
            );
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
                .Transport.SubmitMessages.Select(bytes =>
                {
                    JToken state = JObject.Parse(Encoding.UTF8.GetString(bytes))["Action"]![
                        "body"
                    ]!["ApplicationStateChanged"]!;
                    Assert.That(state["focused"], Is.Not.Null);
                    Assert.That(state["paused"], Is.Not.Null);
                    var action = (ClientMessage<CoreErrorCode, byte>.ActionMessage)
                        BattlementJson.DeserializeClientMessage<CoreErrorCode, byte>(bytes);
                    return ((ActionBody.ApplicationStateChanged)action.Action.Body).Value;
                })
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
