#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using MessagePack;
using MessagePack.Formatters;

namespace Battlement.Tests
{
    internal static class MessagePackFixtureData
    {
        internal static readonly Guid SessionGuid = new("00112233-4455-6677-8899-aabbccddeeff");

        internal static Connect Connect() =>
            new(
                "macOS",
                "6000.5.8f1",
                new ScreenSize(2560, 1440),
                new[] { "cards.draw", "cards.shuffle" },
                "/var/game/data",
                "/var/game/assets"
            );

        internal static IReadOnlyDictionary<string, byte[]> ClientMessages()
        {
            SessionId sessionId = new(SessionGuid);
            ActionId actionId = new(GuidAt(2));
            BatchId batchId = new(GuidAt(3));
            CommandId commandId = new(GuidAt(4));
            var payloadFormatter = new SamplePayloadFormatter();
            var errorFormatter = new SampleErrorFormatter();
            byte[] EncodeAction(ActionBody body) =>
                BattlementMessagePack.SerializeClientMessage(
                    new ClientMessage<SampleError, SamplePayload>.ActionMessage(
                        new Action(actionId, sessionId, body)
                    ),
                    errorFormatter,
                    payloadFormatter
                );
            return new Dictionary<string, byte[]>
            {
                ["csharp-client-pointer-enter.msgpack"] = EncodeAction(
                    new ActionBody.PointerEnter(
                        new ObjectId(GuidAt(5)),
                        new ScreenPosition(12.5, 24.5),
                        new Vector3(1, 2, 3),
                        7
                    )
                ),
                ["csharp-client-pointer-exit.msgpack"] = EncodeAction(
                    new ActionBody.PointerExit(
                        new ObjectId(GuidAt(5)),
                        new ScreenPosition(12.5, 24.5),
                        new Vector3(1, 2, 3),
                        7
                    )
                ),
                ["csharp-client-pointer-down.msgpack"] = EncodeAction(
                    new ActionBody.PointerDown(
                        new ObjectId(GuidAt(5)),
                        new ScreenPosition(12.5, 24.5),
                        new Vector3(1, 2, 3),
                        7,
                        PointerButton.Middle
                    )
                ),
                ["csharp-client-pointer-up.msgpack"] = EncodeAction(
                    new ActionBody.PointerUp(
                        new ObjectId(GuidAt(5)),
                        new ScreenPosition(12.5, 24.5),
                        new Vector3(1, 2, 3),
                        7,
                        PointerButton.Left
                    )
                ),
                ["csharp-client-pointer-click.msgpack"] = EncodeAction(
                    new ActionBody.PointerClick(
                        new ObjectId(GuidAt(5)),
                        new ScreenPosition(12.5, 24.5),
                        new Vector3(1, 2, 3),
                        7,
                        PointerButton.Right
                    )
                ),
                ["csharp-client-drag-start.msgpack"] = EncodeAction(
                    new ActionBody.DragStart(
                        new ObjectId(GuidAt(5)),
                        new ScreenPosition(12.5, 24.5),
                        new Vector3(1, 2, 3),
                        7
                    )
                ),
                ["csharp-client-drag-end.msgpack"] = EncodeAction(
                    new ActionBody.DragEnd(
                        new ObjectId(GuidAt(5)),
                        new ScreenPosition(12.5, 24.5),
                        new Vector3(1, 2, 3),
                        7
                    )
                ),
                ["csharp-client-key-down.msgpack"] = EncodeAction(
                    new ActionBody.KeyDown(KeyCode.KeyA)
                ),
                ["csharp-client-key-up.msgpack"] = EncodeAction(
                    new ActionBody.KeyUp(KeyCode.Escape)
                ),
                ["csharp-client-custom.msgpack"] = BattlementMessagePack.SerializeClientMessage(
                    new ClientMessage<SampleError, SamplePayload>.CustomActionMessage(
                        new CustomAction<SamplePayload>(
                            actionId,
                            sessionId,
                            "cards.choose",
                            new SamplePayload("ace", 4)
                        )
                    ),
                    errorFormatter,
                    payloadFormatter
                ),
                ["csharp-client-batch-failed.msgpack"] =
                    BattlementMessagePack.SerializeClientMessage(
                        new ClientMessage<SampleError, SamplePayload>.BatchFailedMessage(
                            new BatchFailed<SampleError>(
                                sessionId,
                                batchId,
                                SampleError.IllegalMove,
                                "not legal",
                                commandId
                            )
                        ),
                        errorFormatter,
                        payloadFormatter
                    ),
                ["csharp-client-operation-failed.msgpack"] =
                    BattlementMessagePack.SerializeClientMessage(
                        new ClientMessage<SampleError, SamplePayload>.OperationFailedMessage(
                            new OperationFailed<SampleError>(
                                sessionId,
                                batchId,
                                commandId,
                                SampleError.NotReady,
                                "not ready"
                            )
                        ),
                        errorFormatter,
                        payloadFormatter
                    ),
            };
        }

        internal static Response ComprehensiveResponse()
        {
            var factory = new RepresentativeFactory();
            Type[] commandTypes = ConcreteTypes(typeof(CommandBody));
            Command[] commands = commandTypes
                .Select(
                    (type, index) =>
                        new Command(
                            new CommandId(GuidAt(100 + index)),
                            (CommandBody)factory.Create(type),
                            index % 2 == 0
                        )
                )
                .Append(
                    new Command(
                        new CommandId(GuidAt(199)),
                        new CommandBody.Particle.Spawn(
                            new ParticleEffectAddress("game/effect"),
                            new ParticleSpawnLocation.AtGameObject(new ObjectId(GuidAt(299))),
                            TimeSpan.FromMilliseconds(250)
                        )
                    )
                )
                .ToArray();
            BattlementGameObject[] objects = ConcreteTypes(typeof(GameObjectKind))
                .Select(
                    (type, index) =>
                        new BattlementGameObject(
                            new ObjectId(GuidAt(300 + index)),
                            (GameObjectKind)factory.Create(type),
                            (index % 3) switch
                            {
                                0 => new ParentScene.Primary(),
                                1 => new ParentScene.Specific(new SceneId(GuidAt(10))),
                                _ => new ParentScene.Persistent(),
                            },
                            null,
                            true,
                            LocalTransform.Identity,
                            new[] { PointerEvent.Enter, PointerEvent.Click },
                            index % 2 == 0 ? DragMode.SnapToPointer : DragMode.PreserveOffset
                        )
                )
                .ToArray();
            PreparedAsset[] assets = ConcreteTypes(typeof(PreparedAsset))
                .Select(type => (PreparedAsset)factory.Create(type))
                .ToArray();
            SessionId sessionId = new(SessionGuid);
            SceneId sceneId = new(GuidAt(10));
            var snapshot = new Snapshot(
                sessionId,
                assets,
                new[] { new BattlementScene(sceneId, new SceneAddress("game/scene")) },
                objects,
                objects[0].Id,
                sceneId,
                true,
                new[] { KeyCode.KeyA, KeyCode.Escape }
            );
            var batch = new Batch(
                new BatchId(GuidAt(11)),
                sessionId,
                new[] { new ParallelCommandGroup<Command>(commands) },
                new ActionId(GuidAt(12)),
                BatchStart.AfterEarlierBlockingWork
            );
            return new Response(
                sessionId,
                new ResponseMessage<Command>[]
                {
                    new ResponseMessage<Command>.SnapshotMessage(snapshot),
                    new ResponseMessage<Command>.BatchMessage(batch),
                }
            );
        }

        internal static Response<ICommand> CustomResponse() =>
            new(
                new SessionId(SessionGuid),
                new ResponseMessage<ICommand>[]
                {
                    new ResponseMessage<ICommand>.BatchMessage(
                        new Batch<ICommand>(
                            new BatchId(GuidAt(20)),
                            new SessionId(SessionGuid),
                            new[]
                            {
                                new ParallelCommandGroup<ICommand>(
                                    new ICommand[]
                                    {
                                        new CustomCommand<SamplePayload>(
                                            new CommandId(GuidAt(21)),
                                            "cards.reveal",
                                            new SamplePayload("queen", 2),
                                            false
                                        ),
                                    }
                                ),
                            }
                        )
                    ),
                }
            );

        internal static Type[] ConcreteCommandTypes() => ConcreteTypes(typeof(CommandBody));

        internal static Guid GuidAt(int value) => new($"{value:x8}-1234-5678-90ab-{value:x12}");

        private static Type[] ConcreteTypes(Type baseType) =>
            baseType
                .Assembly.GetTypes()
                .Where(type => type.IsSubclassOf(baseType) && !type.IsAbstract)
                .OrderBy(type => type.FullName, StringComparer.Ordinal)
                .ToArray();

        internal sealed class SamplePayload : IEquatable<SamplePayload>
        {
            internal SamplePayload(string name, uint count) => (Name, Count) = (name, count);

            internal string Name { get; }

            internal uint Count { get; }

            public bool Equals(SamplePayload? other) =>
                other is not null && Name == other.Name && Count == other.Count;

            public override bool Equals(object? obj) => obj is SamplePayload other && Equals(other);

            public override int GetHashCode() => (Name, Count).GetHashCode();
        }

        internal enum SampleError
        {
            IllegalMove,
            NotReady,
        }

#pragma warning disable MsgPack014
        internal sealed class SamplePayloadFormatter : IMessagePackFormatter<SamplePayload>
        {
            public void Serialize(
                ref MessagePackWriter writer,
                SamplePayload value,
                MessagePackSerializerOptions options
            )
            {
                writer.WriteArrayHeader(2);
                writer.Write(value.Name);
                writer.Write(value.Count);
            }

            public SamplePayload Deserialize(
                ref MessagePackReader reader,
                MessagePackSerializerOptions options
            )
            {
                if (reader.ReadArrayHeader() != 2)
                {
                    throw new MessagePackSerializationException("Invalid sample payload.");
                }

                return new SamplePayload(reader.ReadString()!, reader.ReadUInt32());
            }
        }
#pragma warning restore MsgPack014

        internal sealed class SampleErrorFormatter : IMessagePackFormatter<SampleError>
        {
            public void Serialize(
                ref MessagePackWriter writer,
                SampleError value,
                MessagePackSerializerOptions options
            ) => writer.Write(value.ToString());

            public SampleError Deserialize(
                ref MessagePackReader reader,
                MessagePackSerializerOptions options
            ) => Enum.Parse<SampleError>(reader.ReadString()!);
        }

        private sealed class RepresentativeFactory
        {
            private int nextGuid = 500;
            private int nextTweenRepeat;

            internal object Create(Type type)
            {
                Type? nullableType = Nullable.GetUnderlyingType(type);
                if (nullableType is not null)
                {
                    return null!;
                }

                if (type == typeof(string))
                {
                    return "sample";
                }

                if (type == typeof(bool))
                {
                    return true;
                }

                if (type == typeof(double))
                {
                    return 1.25;
                }

                if (type == typeof(float))
                {
                    return 1.25f;
                }

                if (type == typeof(int))
                {
                    return 3;
                }

                if (type == typeof(uint))
                {
                    return 3u;
                }

                if (type == typeof(TimeSpan))
                {
                    return TimeSpan.FromMilliseconds(125);
                }

                if (type == typeof(Guid))
                {
                    return GuidAt(nextGuid++);
                }

                if (type.IsEnum)
                {
                    return Enum.GetValues(type).GetValue(0)!;
                }

                if (type == typeof(GameObjectKind))
                {
                    return new GameObjectKind.Empty();
                }

                if (type == typeof(ParentScene))
                {
                    return new ParentScene.Primary();
                }

                if (type == typeof(TweenRepeat))
                {
                    return (nextTweenRepeat++ % 3) switch
                    {
                        0 => new TweenRepeat.Once(),
                        1 => new TweenRepeat.Count(2, RepeatMode.PingPong),
                        _ => new TweenRepeat.Forever(RepeatMode.Restart),
                    };
                }

                if (type == typeof(ParticleSpawnLocation))
                {
                    return new ParticleSpawnLocation.AtWorldPosition(Vector3.One);
                }

                if (TryCreateCollection(type, out object? collection))
                {
                    return collection!;
                }

                ConstructorInfo constructor = type.GetConstructors()
                    .OrderBy(candidate => candidate.GetParameters().Length)
                    .First();
                return constructor.Invoke(
                        constructor
                            .GetParameters()
                            .Select(parameter => Create(parameter.ParameterType))
                            .ToArray()
                    ) ?? throw new InvalidOperationException($"Could not create {type}.");
            }

            private static bool TryCreateCollection(Type type, out object? value)
            {
                if (type.IsGenericType)
                {
                    Type definition = type.GetGenericTypeDefinition();
                    Type[] arguments = type.GetGenericArguments();
                    if (definition == typeof(IReadOnlyList<>))
                    {
                        value = Array.CreateInstance(arguments[0], 0);
                        return true;
                    }

                    if (definition == typeof(IReadOnlyDictionary<,>))
                    {
                        value = Activator.CreateInstance(
                            typeof(Dictionary<,>).MakeGenericType(arguments)
                        );
                        return true;
                    }
                }

                value = null;
                return false;
            }
        }
    }
}
