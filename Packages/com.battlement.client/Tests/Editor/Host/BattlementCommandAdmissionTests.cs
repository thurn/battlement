#nullable enable

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Reflection;
using Google.FlatBuffers;
using NUnit.Framework;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement.Tests
{
    public sealed class BattlementCommandAdmissionTests
    {
        [Test]
        public void EveryDeclaredCommandPassesSerializedAdmissionAndRejectsAnEmptyIdentity()
        {
            var admitted = new HashSet<Wire.CoreCommandKind>();
            var elapsed = Stopwatch.StartNew();
            Type[] bodies = typeof(CommandBody)
                .Assembly.GetTypes()
                .Where(type => !type.IsAbstract && typeof(CommandBody).IsAssignableFrom(type))
                .OrderBy(type => type.FullName, StringComparer.Ordinal)
                .ToArray();
            Assert.That(bodies, Is.Not.Empty);
            var errors = new List<string>();
            foreach (Type type in bodies)
            {
                try
                {
                    var body = (CommandBody)AdmissionSample.Create(type)!;
                    var command = new Command(
                        new CommandId(Guid.NewGuid()),
                        body,
                        body is not CommandBody.Particle.Play
                    );
                    ReadOnlyMemory<byte> bytes = Serialize(command);
                    using var response = new BattlementFlatBufferResponse(bytes, null);
                    Assert.That(response.ReadBatch(0).ReadCommand(0, 0).Id, Is.EqualTo(command.Id));
                    var buffer = new ByteBuffer(bytes.ToArray()) { Position = 4 };
                    Wire.CoreCommand wire = Wire
                        .Response.GetRootAsResponse(buffer)
                        .Messages(0)!
                        .Value.MessageAsBatch()
                        .Groups(0)!
                        .Value.Commands(0)!
                        .Value.CommandAsCoreCommand();
                    admitted.Add(wire.Kind);
                    Assert.Throws<InvalidDataException>(
                        () =>
                        {
                            using var invalid = new BattlementFlatBufferResponse(
                                Serialize(command with { Id = new CommandId(Guid.Empty) }),
                                null
                            );
                        },
                        type.FullName
                    );
                }
                catch (Exception error)
                {
                    errors.Add($"{type.FullName}: {error}");
                }
            }
            // This wire-only command has no CommandBody record in the host model.
            using (var pointer = new BattlementFlatBufferResponse(WorldPointer(false), null))
                admitted.Add(Wire.CoreCommandKind.InputSetWorldPointer);
            Assert.Throws<InvalidDataException>(() =>
            {
                using var invalid = new BattlementFlatBufferResponse(WorldPointer(true), null);
            });
            TestContext.WriteLine(string.Join("\n", errors));
            Assert.That(
                admitted,
                Is.EquivalentTo(Enum.GetValues(typeof(Wire.CoreCommandKind))),
                "Every wire command must have a successfully admitted domain sample."
            );
            Assert.That(errors, Is.Empty, string.Join("\n", errors));
            TestContext.WriteLine(
                $"Admitted {admitted.Count} kinds in {elapsed.Elapsed.TotalMilliseconds:F1} ms."
            );
        }

        [Test]
        public void InvalidPayloadValuesFailSerializedAdmission()
        {
            var id = new ObjectId(Guid.NewGuid());
            foreach (
                CommandBody body in new CommandBody[]
                {
                    new CommandBody.Audio.SetVolume(new CommandId(Guid.NewGuid()), double.NaN),
                    new CommandBody.SetBoxHitRegion(
                        id,
                        new BoxHitRegionState(new Vector3(-1, 1, 1), Vector3.Zero)
                    ),
                }
            )
            {
                Assert.Throws<InvalidDataException>(
                    () =>
                    {
                        using var response = new BattlementFlatBufferResponse(
                            Serialize(new Command(new CommandId(Guid.NewGuid()), body)),
                            null
                        );
                    },
                    body.GetType().FullName
                );
            }
        }

        private static Offset<Wire.Uuid> Uuid(FlatBufferBuilder builder, byte value)
        {
            var bytes = new byte[16];
            bytes[0] = value;
            return Wire.Uuid.CreateUuid(builder, bytes);
        }

        private static ReadOnlyMemory<byte> WorldPointer(bool invalid)
        {
            var builder = new FlatBufferBuilder(256);
            Wire.WorldPointerPayload.StartWorldPointerPayload(builder);
            Wire.WorldPointerPayload.AddObjectId(builder, Uuid(builder, 1));
            var payload = Wire.WorldPointerPayload.EndWorldPointerPayload(builder);
            Wire.CoreCommand.StartCoreCommand(builder);
            Wire.CoreCommand.AddKind(builder, Wire.CoreCommandKind.InputSetWorldPointer);
            Wire.CoreCommand.AddPayloadType(builder, Wire.CoreCommandPayload.WorldPointerPayload);
            Wire.CoreCommand.AddPayload(builder, payload.Value);
            Wire.CoreCommand.AddCommandId(builder, Uuid(builder, invalid ? (byte)0 : (byte)2));
            var command = Wire.CoreCommand.EndCoreCommand(builder);
            var entry = Wire.CommandEntry.CreateCommandEntry(
                builder,
                Wire.CommandEntryPayload.CoreCommand,
                command.Value
            );
            var commands = Wire.ParallelCommandGroup.CreateCommandsVector(builder, new[] { entry });
            var group = Wire.ParallelCommandGroup.CreateParallelCommandGroup(builder, commands);
            var groups = Wire.Batch.CreateGroupsVector(builder, new[] { group });
            Wire.Batch.StartBatch(builder);
            Wire.Batch.AddGroups(builder, groups);
            Wire.Batch.AddBatchId(builder, Uuid(builder, 3));
            Wire.Batch.AddSessionId(builder, Uuid(builder, 4));
            var batch = Wire.Batch.EndBatch(builder);
            var message = Wire.ResponseMessageEntry.CreateResponseMessageEntry(
                builder,
                Wire.ResponseMessage.Batch,
                batch.Value
            );
            var messages = Wire.Response.CreateMessagesVector(builder, new[] { message });
            Wire.Response.StartResponse(builder);
            Wire.Response.AddMessages(builder, messages);
            Wire.Response.AddSessionId(builder, Uuid(builder, 4));
            var response = Wire.Response.EndResponse(builder);
            Wire.Response.FinishSizePrefixedResponseBuffer(builder, response);
            return builder.SizedByteArray();
        }

        private static ReadOnlyMemory<byte> Serialize(Command command)
        {
            var session = new SessionId(Guid.NewGuid());
            return BattlementFlatBufferResponseFixtures.Write(
                new Response(
                    session,
                    new ResponseMessage<Command>[]
                    {
                        new ResponseMessage<Command>.BatchMessage(
                            new Batch(
                                new BatchId(Guid.NewGuid()),
                                session,
                                new[] { new ParallelCommandGroup<Command>(new[] { command }) }
                            )
                        ),
                    }
                )
            );
        }
    }

    // Domain constructors supply defaults; special values express semantic constraints,
    // not a second inventory of command kinds or their wire payload mapping.
    internal static class AdmissionSample
    {
        internal static object? Create(Type type)
        {
            if (Nullable.GetUnderlyingType(type) is not null)
                return null;
            if (type == typeof(Guid))
                return Guid.NewGuid();
            if (type == typeof(string))
                return "https://example.com/admission";
            if (type == typeof(TimeSpan))
                return TimeSpan.FromMilliseconds(1);
            if (type == typeof(Quaternion))
                return Quaternion.Identity;
            if (type == typeof(CommandBody.Camera.SetClipping))
                return new CommandBody.Camera.SetClipping(new ObjectId(Guid.NewGuid()), 1, 2);
            if (type == typeof(DiagnosticsCommand))
                return new DiagnosticsCommand.SetReporting(true);
            if (type == typeof(MotionDescriptor))
                return null;
            if (type == typeof(MotionControlCommand))
                return new MotionControlCommand.Start(
                    new ObjectId(Guid.NewGuid()),
                    1,
                    new MotionControlTarget.Variant("visible")
                );
            if (type == typeof(MotionScopeCommand))
                return new MotionScopeCommand.Start(
                    new ObjectId(Guid.NewGuid()),
                    1,
                    Array.Empty<MotionSequenceEntry>()
                );
            if (type.IsEnum)
                return Enum.GetValues(type).GetValue(0);
            if (type.IsPrimitive)
                return Convert.ChangeType(1, type);
            if (type.IsGenericType && type.GetGenericTypeDefinition() == typeof(IReadOnlyList<>))
                return Array.CreateInstance(type.GetGenericArguments()[0], 0);
            if (type.IsAbstract)
            {
                type = type
                    .Assembly.GetTypes()
                    .Where(candidate => !candidate.IsAbstract && type.IsAssignableFrom(candidate))
                    .OrderBy(candidate =>
                        Constructor(candidate)
                            .GetParameters()
                            .Count(parameter => !parameter.HasDefaultValue)
                    )
                    .ThenBy(candidate => candidate.FullName, StringComparer.Ordinal)
                    .First();
            }
            if (type.IsValueType && type.GetConstructors().Length == 0)
                return Activator.CreateInstance(type);
            ConstructorInfo constructor = Constructor(type);
            return constructor.Invoke(
                constructor
                    .GetParameters()
                    .Select(parameter =>
                        parameter.HasDefaultValue
                            ? parameter.DefaultValue
                            : Create(parameter.ParameterType)
                    )
                    .ToArray()
            );
        }

        private static ConstructorInfo Constructor(Type type) =>
            type.GetConstructors()
                .OrderBy(value =>
                    value.GetParameters().Count(parameter => !parameter.HasDefaultValue)
                )
                .ThenBy(value => value.GetParameters().Length)
                .First();
    }
}
