#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using NUnit.Framework;

namespace Battlement.Tests
{
    public sealed class DittoAccessibilityAssertionTests
    {
        [TestCase("selected")]
        [TestCase("checked")]
        [TestCase("disabled")]
        [TestCase("current_page")]
        [TestCase("parent")]
        public void DecodedAssertionsRejectIncorrectStateAndRelationships(string field)
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            ObjectId document = new(Guid.NewGuid());
            ObjectId root = new(Guid.NewGuid());
            ObjectId navigation = new(Guid.NewGuid());
            ObjectId page = new(Guid.NewGuid());
            Snapshot snapshot = FakeBattlementTransport.CompleteSnapshot(
                session,
                objects: new[]
                {
                    new BattlementGameObject(
                        document,
                        new GameObjectKind.UiDocumentState(root),
                        new ParentScene.Persistent(),
                        null,
                        true,
                        LocalTransform.Identity,
                        Array.Empty<PointerEvent>()
                    ),
                }
            ) with
            {
                Ui = new[]
                {
                    new UiDocument(
                        document,
                        root,
                        Children: new[]
                        {
                            new UiNode(
                                navigation,
                                new UiElement.Box(),
                                new[] { new UiNode(page, new UiElement.Button()) }
                            ),
                        }
                    ),
                },
            };
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.ResponseResult(
                    new Response(
                        session,
                        new ResponseMessage<Command>[]
                        {
                            new ResponseMessage<Command>.SnapshotMessage(snapshot),
                        }
                    )
                )
            );
            harness.Runner.Connect();
            var update = new AccessibilityUpdatePayload(
                new AccessibilitySnapshot(
                    1,
                    new[] { navigation },
                    new[]
                    {
                        new AccessibilityNodeSnapshot(
                            navigation,
                            null,
                            new[] { page },
                            SemanticRole.Navigation,
                            "Pages",
                            null,
                            new SemanticState(),
                            null,
                            new AccessibilityActionSet()
                        ),
                        new AccessibilityNodeSnapshot(
                            page,
                            navigation,
                            Array.Empty<ObjectId>(),
                            SemanticRole.Button,
                            "Controls",
                            null,
                            new SemanticState(
                                Selected: false,
                                Checked: CheckedState.False,
                                Current: CurrentPage.Page
                            ),
                            null,
                            new AccessibilityActionSet(Activate: true)
                        ),
                    }
                ),
                Array.Empty<string>()
            );
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
                                new CommandBody.AccessibilityUpdate(update)
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
            var targets = new DittoInputTargets(
                harness.Runner,
                new Dictionary<string, ObjectId>(),
                1280,
                720
            );
            JObject assertion = JObject.Parse(
                @"{
                'target': {'role': 'button', 'name': 'Controls'},
                'role': 'button', 'name': 'Controls',
                'selected': false, 'checked': false, 'disabled': false, 'current_page': true,
                'parent': {'role': 'navigation', 'name': 'Pages'}
            }"
            );
            Assert.That(targets.Evaluate(Decode(assertion)).Matches, Is.True);

            assertion[field] =
                field == "parent"
                    ? JObject.Parse("{'role':'navigation','name':'Other pages'}")
                    : new JValue(!(bool)assertion[field]!);
            Assert.That(targets.Evaluate(Decode(assertion)).Matches, Is.False, field);

            foreach (
                string optional in new[]
                {
                    "selected",
                    "checked",
                    "disabled",
                    "current_page",
                    "parent",
                }
            )
                assertion[optional] = JValue.CreateNull();
            Assert.That(targets.Evaluate(Decode(assertion)).Matches, Is.True);
        }

        private static DittoAccessibilityAssertion Decode(JObject assertion)
        {
            JObject fixture = JObject.Parse(
                File.ReadAllText(
                    "Packages/com.battlement.client/Tests/Fixtures/Ditto/job-contract.json"
                )
            );
            JObject job = (JObject)fixture["valid"]!;
            job["scenarios"]![0]!["steps"] = new JArray(
                new JObject
                {
                    ["index"] = 0,
                    ["name"] = "Check semantic state",
                    ["timeout_ms"] = 1000,
                    ["action"] = new JObject { ["accessibility-assert"] = assertion },
                }
            );
            DittoJob decoded = DittoJobCodec.Decode(
                Encoding.UTF8.GetBytes(job.ToString(Formatting.None))
            );
            return (
                (DittoStepAction.AccessibilityAssert)decoded.Scenarios[0].Steps[0].Action
            ).Value;
        }
    }
}
