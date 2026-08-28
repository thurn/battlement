#nullable enable

using System;
using System.Text;
using Newtonsoft.Json;
using NUnit.Framework;

namespace Battlement.Tests
{
    public sealed class BattlementDiagnosticsJsonTests
    {
        [Test]
        public void MetadataSetAndClearRoundTripUsingExternalTags()
        {
            DiagnosticsCommand[] values =
            {
                new DiagnosticsCommand.SetMetadata("battlement.scene", "castle"),
                new DiagnosticsCommand.SetMetadata("battlement.scene"),
            };

            foreach (DiagnosticsCommand value in values)
            {
                Response response = ResponseFor(value);
                string json = Encoding.UTF8.GetString(BattlementJson.SerializeResponse(response));
                StringAssert.Contains("\"Diagnostics\":{\"SetMetadata\":", json);
                Response decoded = BattlementJson.DeserializeResponse(Encoding.UTF8.GetBytes(json));
                Command command = ((ResponseMessage<Command>.BatchMessage)decoded.Messages[0])
                    .Batch
                    .Groups[0]
                    .Commands[0];
                Assert.That(((CommandBody.Diagnostics)command.Body).Command, Is.EqualTo(value));
            }
        }

        [Test]
        public void MetadataClearOmitsTheValue()
        {
            string json = Encoding.UTF8.GetString(
                BattlementJson.SerializeResponse(
                    ResponseFor(new DiagnosticsCommand.SetMetadata("battlement.scene"))
                )
            );

            StringAssert.Contains("\"key\":\"battlement.scene\"", json);
            StringAssert.DoesNotContain("\"value\"", json);
        }

        [Test]
        public void DiagnosticsProtocolRejectsInvalidMetadata()
        {
            Assert.That(
                DiagnosticsProtocol.Validate(new DiagnosticsCommand.SetMetadata(" key", "value")),
                Is.EqualTo(CoreErrorCode.DiagnosticsMetadataInvalid)
            );
            Assert.That(
                DiagnosticsProtocol.Validate(new DiagnosticsCommand.SetMetadata("key", "\0")),
                Is.EqualTo(CoreErrorCode.DiagnosticsMetadataInvalid)
            );
        }

        [Test]
        public void DiagnosticsUnionRejectsUnknownVariant()
        {
            const string json =
                "{\"session_id\":\"aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa\","
                + "\"messages\":[{\"Batch\":{\"batch_id\":\"bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb\","
                + "\"session_id\":\"aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa\","
                + "\"groups\":[{\"commands\":[{"
                + "\"command_id\":\"cccccccc-cccc-cccc-cccc-cccccccccccc\","
                + "\"body\":{\"Diagnostics\":{\"Unknown\":{}}}}]}]}}]}";
            Assert.Throws<JsonSerializationException>(() =>
                BattlementJson.DeserializeResponse(Encoding.UTF8.GetBytes(json))
            );
        }

        private static Response ResponseFor(DiagnosticsCommand command)
        {
            SessionId session = new(Guid.Parse("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa"));
            return new Response(
                session,
                new ResponseMessage<Command>[]
                {
                    new ResponseMessage<Command>.BatchMessage(
                        new Batch(
                            new BatchId(Guid.Parse("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb")),
                            session,
                            new[]
                            {
                                new ParallelCommandGroup<Command>(
                                    new[]
                                    {
                                        new Command(
                                            new CommandId(
                                                Guid.Parse("cccccccc-cccc-cccc-cccc-cccccccccccc")
                                            ),
                                            new CommandBody.Diagnostics(command)
                                        ),
                                    }
                                ),
                            }
                        )
                    ),
                }
            );
        }
    }
}
