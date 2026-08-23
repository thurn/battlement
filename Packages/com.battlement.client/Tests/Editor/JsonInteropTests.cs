#nullable enable

using System.Linq;
using System.Text;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using NUnit.Framework;

namespace Battlement.Tests
{
    public sealed class JsonInteropTests
    {
        [Test]
        public void BuiltInCorpusRoundTripsAsStructuralJson()
        {
            Response fixture = JSONFixtureData.ComprehensiveResponse();
            byte[] bytes = BattlementJson.SerializeResponse(fixture);
            Response decoded = BattlementJson.DeserializeResponse(bytes);
            var commands = ((ResponseMessage<Command>.BatchMessage)decoded.Messages[1])
                .Batch.Groups.SelectMany(group => group.Commands)
                .ToArray();

            Assert.That(commands, Has.Length.EqualTo(76));
            Assert.That(
                commands.Select(command => command.Body.GetType()).Distinct().Count(),
                Is.EqualTo(JSONFixtureData.ConcreteCommandTypes().Length)
            );
            Assert.That(JToken.Parse(Encoding.UTF8.GetString(bytes)), Is.Not.Null);
        }

        [Test]
        public void OmitsAndRestoresProtocolDefaults()
        {
            SessionId sessionId = new(JSONFixtureData.SessionGuid);
            Command command = new(
                new CommandId(JSONFixtureData.GuidAt(401)),
                new CommandBody.Particle.Play(new ObjectId(JSONFixtureData.GuidAt(402)))
            );
            Command propertyCommand = new(
                new CommandId(JSONFixtureData.GuidAt(404)),
                new CommandBody.Transform.SetLocalPosition(
                    new ObjectId(JSONFixtureData.GuidAt(405)),
                    new Vector3(1, 2, 3)
                )
            );
            Command reparentCommand = new(
                new CommandId(JSONFixtureData.GuidAt(407)),
                new CommandBody.Object.Reparent(new ObjectId(JSONFixtureData.GuidAt(408)), null)
            );
            Command animatorCommand = new(
                new CommandId(JSONFixtureData.GuidAt(409)),
                new CommandBody.Animator.SetInt(new ObjectId(JSONFixtureData.GuidAt(410)), "score")
            );
            Response response = new(
                sessionId,
                new[]
                {
                    new ResponseMessage<Command>.BatchMessage(
                        new Batch(
                            new BatchId(JSONFixtureData.GuidAt(403)),
                            sessionId,
                            new[]
                            {
                                new ParallelCommandGroup<Command>(
                                    new[]
                                    {
                                        command,
                                        propertyCommand,
                                        reparentCommand,
                                        animatorCommand,
                                    }
                                ),
                            }
                        )
                    ),
                }
            );

            JObject root = JObject.Parse(
                Encoding.UTF8.GetString(BattlementJson.SerializeResponse(response))
            );
            JObject message = (JObject)root["messages"]![0]!;
            JObject batch = (JObject)message["Batch"]!;
            JObject group = (JObject)((JArray)batch["groups"]!)[0]!;
            JObject commandJson = (JObject)((JArray)group["commands"]!)[0]!;
            JObject particle = (JObject)((JObject)commandJson["body"]!)["ParticlePlay"]!;
            JObject propertyCommandJson = (JObject)((JArray)group["commands"]!)[1]!;
            JObject propertyPayload = (JObject)
                ((JObject)propertyCommandJson["body"]!)["TransformSetLocalPosition"]!;
            JObject reparentCommandJson = (JObject)((JArray)group["commands"]!)[2]!;
            JObject reparentPayload = (JObject)
                ((JObject)reparentCommandJson["body"]!)["ObjectReparent"]!;
            JObject animatorCommandJson = (JObject)((JArray)group["commands"]!)[3]!;
            JObject animatorPayload = (JObject)
                ((JObject)animatorCommandJson["body"]!)["AnimatorSetInt"]!;

            Assert.That(commandJson.ContainsKey("blocking"), Is.False);
            Assert.That(particle.ContainsKey("restart"), Is.False);
            Assert.That(particle.ContainsKey("object_id"), Is.True);
            Assert.That(propertyPayload.ContainsKey("on_conflict"), Is.False);
            Assert.That(reparentPayload.ContainsKey("world_position_stays"), Is.False);
            Assert.That(animatorPayload.ContainsKey("value"), Is.False);

            Response decoded = BattlementJson.DeserializeResponse(
                Encoding.UTF8.GetBytes(root.ToString(Formatting.None))
            );
            Command decodedCommand = ((ResponseMessage<Command>.BatchMessage)decoded.Messages[0])
                .Batch
                .Groups[0]
                .Commands[0];
            Command decodedPropertyCommand = (
                (ResponseMessage<Command>.BatchMessage)decoded.Messages[0]
            )
                .Batch
                .Groups[0]
                .Commands[1];
            Assert.That(decodedCommand.IsBlocking, Is.True);
            Assert.That(((CommandBody.Particle.Play)decodedCommand.Body).Restart, Is.False);
            Assert.That(
                ((CommandBody.Transform.SetLocalPosition)decodedPropertyCommand.Body).OnConflict,
                Is.EqualTo(ConflictPolicy.Cancel)
            );
            Assert.That(
                (
                    (CommandBody.Object.Reparent)
                        ((ResponseMessage<Command>.BatchMessage)decoded.Messages[0])
                            .Batch
                            .Groups[0]
                            .Commands[2]
                            .Body
                ).WorldPositionStays,
                Is.False
            );
            Assert.That(
                (
                    (CommandBody.Animator.SetInt)
                        ((ResponseMessage<Command>.BatchMessage)decoded.Messages[0])
                            .Batch
                            .Groups[0]
                            .Commands[3]
                            .Body
                ).Value,
                Is.EqualTo(0)
            );

            Snapshot snapshot = new(
                sessionId,
                new PreparedAsset[0],
                new BattlementScene[0],
                new BattlementGameObject[0]
            );
            Response snapshotResponse = new(
                sessionId,
                new ResponseMessage<Command>[]
                {
                    new ResponseMessage<Command>.SnapshotMessage(snapshot),
                }
            );
            JObject snapshotRoot = JObject.Parse(
                Encoding.UTF8.GetString(BattlementJson.SerializeResponse(snapshotResponse))
            );
            JObject snapshotJson = (JObject)
                ((JObject)((JArray)snapshotRoot["messages"]!)[0]!)!["Snapshot"]!;
            Assert.That(snapshotJson.ContainsKey("input_disabled"), Is.False);
            Response decodedSnapshot = BattlementJson.DeserializeResponse(
                Encoding.UTF8.GetBytes(snapshotRoot.ToString(Formatting.None))
            );
            Assert.That(
                ((ResponseMessage<Command>.SnapshotMessage)decodedSnapshot.Messages[0])
                    .Snapshot
                    .IsInputDisabled,
                Is.False
            );
        }

        [Test]
        public void RestoresOmittedSingleFieldAndValueTypeDefaults()
        {
            SessionId sessionId = new(JSONFixtureData.SessionGuid);
            Command input = new(
                new CommandId(JSONFixtureData.GuidAt(420)),
                new CommandBody.Input.SetEnabled(false)
            );
            Command play = new(
                new CommandId(JSONFixtureData.GuidAt(421)),
                new CommandBody.Audio.Play(new AudioClipAddress("game/audio/defaults"))
            );
            Command stop = new(
                new CommandId(JSONFixtureData.GuidAt(422)),
                new CommandBody.Audio.Stop(play.Id)
            );
            Response response = new(
                sessionId,
                new[]
                {
                    new ResponseMessage<Command>.BatchMessage(
                        new Batch(
                            new BatchId(JSONFixtureData.GuidAt(423)),
                            sessionId,
                            new[] { new ParallelCommandGroup<Command>(new[] { input, play, stop }) }
                        )
                    ),
                }
            );

            byte[] bytes = BattlementJson.SerializeResponse(response);
            JObject root = JObject.Parse(Encoding.UTF8.GetString(bytes));
            JArray commands = (JArray)
                ((JObject)((JObject)((JArray)root["messages"]!)[0]!)["Batch"]!)["groups"]![0]![
                    "commands"
                ]!;
            JObject inputPayload = (JObject)((JObject)commands[0]!["body"]!)["InputSetEnabled"]!;
            JObject playPayload = (JObject)((JObject)commands[1]!["body"]!)["AudioPlay"]!;
            JObject stopPayload = (JObject)((JObject)commands[2]!["body"]!)["AudioStop"]!;

            Assert.That(inputPayload.ContainsKey("enabled"), Is.False);
            Assert.That(playPayload.ContainsKey("fade_in_ms"), Is.False);
            Assert.That(stopPayload.ContainsKey("fade_out_ms"), Is.False);

            Response decoded = BattlementJson.DeserializeResponse(bytes);
            var decodedCommands = ((ResponseMessage<Command>.BatchMessage)decoded.Messages[0])
                .Batch
                .Groups[0]
                .Commands;
            Assert.That(
                ((CommandBody.Input.SetEnabled)decodedCommands[0].Body).IsEnabled,
                Is.False
            );
            Assert.That(
                ((CommandBody.Audio.Play)decodedCommands[1].Body).FadeIn,
                Is.EqualTo(System.TimeSpan.Zero)
            );
            Assert.That(
                ((CommandBody.Audio.Stop)decodedCommands[2].Body).FadeOut,
                Is.EqualTo(System.TimeSpan.Zero)
            );
        }

        [Test]
        public void GameOwnedPayloadUsesTheRegisteredDecodeEscapeHatch()
        {
            Response<ICommand> fixture = JSONFixtureData.CustomResponse();
            byte[] bytes = BattlementJson.SerializeResponse<JSONFixtureData.SamplePayload>(fixture);
            Response<ICommand> decoded = BattlementJson.DeserializeResponse(
                bytes,
                (id, type, blocking, payload) =>
                    new CustomCommand<JSONFixtureData.SamplePayload>(
                        id,
                        type,
                        BattlementJson.Deserialize<JSONFixtureData.SamplePayload>(payload),
                        blocking
                    )
            );
            var command =
                (CustomCommand<JSONFixtureData.SamplePayload>)
                    ((ResponseMessage<ICommand>.BatchMessage)decoded.Messages[0])
                        .Batch
                        .Groups[0]
                        .Commands[0];

            Assert.That(command.Type, Is.EqualTo("cards.reveal"));
            Assert.That(command.Payload, Is.EqualTo(new JSONFixtureData.SamplePayload("queen", 2)));
        }

        [Test]
        public void ClientMessagesAndTypedScalarsRoundTrip()
        {
            foreach (byte[] bytes in JSONFixtureData.ClientMessages().Values)
            {
                ClientMessage<JSONFixtureData.SampleError, JSONFixtureData.SamplePayload> decoded =
                    BattlementJson.DeserializeClientMessage<
                        JSONFixtureData.SampleError,
                        JSONFixtureData.SamplePayload
                    >(bytes);
                Assert.That(decoded, Is.Not.Null);
            }
        }

        [Test]
        public void MalformedInputIsRejected()
        {
            byte[] connect = BattlementJson.SerializeConnect(JSONFixtureData.Connect());
            Assert.Throws<JsonSerializationException>(() =>
                BattlementJson.DeserializeConnect(
                    connect.Concat(Encoding.UTF8.GetBytes("null")).ToArray()
                )
            );
            Assert.Throws<JsonSerializationException>(() =>
                BattlementJson.DeserializeConnect(connect.Take(connect.Length - 1).ToArray())
            );
            Assert.Throws<JsonSerializationException>(() =>
                BattlementJson.DeserializeConnect(
                    Encoding.UTF8.GetBytes(
                        "{\"unity_version\":\"x\",\"screen\":{\"width\":1,\"height\":1},"
                            + "\"custom_command_types\":[],\"persistent_data_path\":null,"
                            + "\"streaming_assets_path\":null}"
                    )
                )
            );
            Assert.Throws<JsonSerializationException>(() =>
                BattlementJson.DeserializeConnect(
                    Encoding.UTF8.GetBytes(
                        "{\"platform\":\"macOS\",\"unity_version\":\"x\","
                            + "\"screen\":{\"width\":1,\"height\":1},"
                            + "\"custom_command_types\":[],"
                            + "\"persistent_data_path\":null,\"streaming_assets_path\":null,"
                            + "\"extra\":1} trailing"
                    )
                )
            );
            Assert.Throws<JsonSerializationException>(() =>
                BattlementJson.DeserializeResponse(
                    Encoding.UTF8.GetBytes(
                        "{\"session_id\":\"00000000-0000-0000-0000-000000000000\",\"messages\":[]}"
                    )
                )
            );
            Assert.Throws<JsonSerializationException>(() =>
                BattlementJson.DeserializeResponse(
                    Encoding.UTF8.GetBytes(
                        "{\"session_id\":\"00112233-4455-6677-8899-aabbccddeeff\","
                            + "\"messages\":[{\"Unknown\":null}]}"
                    )
                )
            );
            Assert.Throws<JsonSerializationException>(() =>
                BattlementJson.DeserializeConnect(
                    Encoding.UTF8.GetBytes("[" + new string('[', 128))
                )
            );
        }
    }
}
