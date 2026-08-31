#nullable enable

using System.Text;
using Newtonsoft.Json.Linq;
using NUnit.Framework;

namespace Battlement.Tests
{
    public sealed class MotionProtocolTests
    {
        [Test]
        public void CompleteDescriptorRoundTripsWithoutLosingUnionPayloads()
        {
            ObjectId hostId = Id("e6c173ab-fecb-461c-b215-8972cf45ad7a");
            MotionDescriptor descriptor = Descriptor(hostId);
            Response response = ResponseWith(hostId, descriptor);

            byte[] json = BattlementJson.SerializeResponse(response);
            Response decoded = BattlementJson.DeserializeResponse(json);
            var batch = (ResponseMessage<Command>.BatchMessage)decoded.Messages[0];
            var update = (CommandBody.VisualElement.Update)batch.Batch.Groups[0].Commands[0].Body;
            var properties = (VisualElementUpdate.Properties)update.Value;
            MotionDescriptor actual = properties.Element.Motion.Value;

            Assert.That(BattlementJson.SerializeResponse(decoded), Is.EqualTo(json));
            Assert.That(actual.DescriptorId, Is.EqualTo(descriptor.DescriptorId));
            Assert.That(actual.Slots[0].Target.Tracks[0].Times, Is.EqualTo(new[] { 0d, 0.4, 1 }));
            Assert.That(actual.Variants!.Names, Is.EqualTo(new[] { "west", "selected" }));
            Assert.That(actual.Variants.CustomSnapshot, Is.EqualTo(91));
            Assert.That(actual.Variants.When, Is.EqualTo(VariantWhen.AfterChildren));
            Assert.That(actual.Variants.StaggerDirection, Is.EqualTo(StaggerDirection.Reverse));
            string text = Encoding.UTF8.GetString(json);
            StringAssert.Contains("\"Controlled\"", text);
            StringAssert.Contains("\"Mirror\"", text);
            StringAssert.Contains("\"CubicBezier\"", text);
            StringAssert.Contains("\"Discrete\":\"hidden\"", text);
        }

        private static MotionDescriptor Descriptor(ObjectId hostId) =>
            new(
                Id("9b23ca44-42f2-498d-8037-0e4158765c23"),
                hostId,
                9,
                new[]
                {
                    new MotionPropertyValue(MotionProperty.Opacity, new MotionValue.Scalar(0.2)),
                },
                false,
                new[]
                {
                    new MotionSlotDescriptor(
                        42,
                        3,
                        MotionLayer.Hover,
                        new MotionTargetDescriptor(
                            new[]
                            {
                                new MotionPropertyTrack(
                                    MotionProperty.Opacity,
                                    new MotionValue[]
                                    {
                                        new MotionValue.Scalar(0.2),
                                        new MotionValue.Scalar(0.7),
                                        new MotionValue.Scalar(1),
                                    },
                                    new TransitionDefinition(
                                        new TransitionGenerator.Tween(
                                            750_000,
                                            new MotionEasing[]
                                            {
                                                new MotionEasing.CubicBezier(
                                                    new[] { 0.42, 0, 1, 1 }
                                                ),
                                                new MotionEasing.EaseOut(),
                                            },
                                            new[] { 0d, 0.4, 1 }
                                        ),
                                        -50_000,
                                        new MotionRepeat.Count(2),
                                        25_000,
                                        MotionRepeatType.Mirror
                                    ),
                                    new[] { 0d, 0.4, 1 }
                                ),
                            },
                            new[]
                            {
                                new MotionPropertyValue(
                                    MotionProperty.Visibility,
                                    new MotionValue.Discrete(JToken.FromObject("hidden"))
                                ),
                            }
                        ),
                        new MotionCallbackSubscriptions(true, true, true, true, true, true)
                    ),
                },
                new MotionClockSource.Controlled(Id("030b27a6-8f4a-4d4b-9b11-67d84fd7c920")),
                ReducedMotionPolicy.Always,
                new MotionTargetDescriptor(
                    new[]
                    {
                        new MotionPropertyTrack(
                            MotionProperty.X,
                            new MotionValue[] { new MotionValue.Length(new MotionLength(-12, 0)) },
                            Immediate(),
                            null
                        ),
                    },
                    System.Array.Empty<MotionPropertyValue>()
                ),
                null,
                null,
                null,
                null,
                new MotionVariantResolution(
                    new[] { "west", "selected" },
                    true,
                    91,
                    3,
                    470_000,
                    VariantWhen.AfterChildren,
                    StaggerDirection.Reverse
                )
            );

        private static TransitionDefinition Immediate() =>
            new(
                new TransitionGenerator.Immediate(),
                0,
                new MotionRepeat.None(),
                0,
                MotionRepeatType.Loop
            );

        private static Response ResponseWith(ObjectId hostId, MotionDescriptor descriptor)
        {
            SessionId session = new(Id("5accf140-b8a6-4542-aedf-98a96ad94d1d").Value);
            return new Response(
                session,
                new ResponseMessage<Command>[]
                {
                    new ResponseMessage<Command>.BatchMessage(
                        new Batch(
                            new BatchId(Id("56567f2b-c40b-4e54-adc7-20522a42b14c").Value),
                            session,
                            new[]
                            {
                                new ParallelCommandGroup<Command>(
                                    new[]
                                    {
                                        new Command(
                                            new CommandId(
                                                Id("ef0dfe82-9dbc-4f4f-832e-a0b361668a87").Value
                                            ),
                                            new CommandBody.VisualElement.Update(
                                                new VisualElementUpdate.Properties(
                                                    hostId,
                                                    new UiElement.Box { Motion = descriptor }
                                                )
                                            )
                                        ),
                                    }
                                ),
                            }
                        )
                    ),
                }
            );
        }

        private static ObjectId Id(string value) => new(System.Guid.Parse(value));
    }
}
