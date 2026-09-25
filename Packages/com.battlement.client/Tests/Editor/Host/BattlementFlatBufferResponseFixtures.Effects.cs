#nullable enable

using Google.FlatBuffers;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement.Tests
{
    internal static partial class BattlementFlatBufferResponseFixtures
    {
        private static Payload AnimatorPlay(
            FlatBufferBuilder builder,
            CommandBody.Animator.Play value
        )
        {
            StringOffset state = builder.CreateString(value.State);
            Wire.AnimatorPlayPayload.StartAnimatorPlayPayload(builder);
            Wire.AnimatorPlayPayload.AddWaitMs(builder, Milliseconds(value.Wait));
            Wire.AnimatorPlayPayload.AddNormalizedStartTime(builder, value.NormalizedStartTime);
            Wire.AnimatorPlayPayload.AddLayer(builder, value.Layer);
            Wire.AnimatorPlayPayload.AddState(builder, state);
            Wire.AnimatorPlayPayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            return new(
                Wire.CoreCommandKind.AnimatorPlay,
                Wire.CoreCommandPayload.AnimatorPlayPayload,
                Wire.AnimatorPlayPayload.EndAnimatorPlayPayload(builder).Value
            );
        }

        private static Payload AnimatorCrossFade(
            FlatBufferBuilder builder,
            CommandBody.Animator.CrossFade value
        )
        {
            StringOffset state = builder.CreateString(value.State);
            Wire.AnimatorCrossFadePayload.StartAnimatorCrossFadePayload(builder);
            Wire.AnimatorCrossFadePayload.AddCrossFadeMs(
                builder,
                Milliseconds(value.CrossFadeDuration)
            );
            Wire.AnimatorCrossFadePayload.AddWaitMs(builder, Milliseconds(value.Wait));
            Wire.AnimatorCrossFadePayload.AddNormalizedStartTime(
                builder,
                value.NormalizedStartTime
            );
            Wire.AnimatorCrossFadePayload.AddLayer(builder, value.Layer);
            Wire.AnimatorCrossFadePayload.AddState(builder, state);
            Wire.AnimatorCrossFadePayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            return new(
                Wire.CoreCommandKind.AnimatorCrossFade,
                Wire.CoreCommandPayload.AnimatorCrossFadePayload,
                Wire.AnimatorCrossFadePayload.EndAnimatorCrossFadePayload(builder).Value
            );
        }

        private static Payload AnimatorBool(
            FlatBufferBuilder builder,
            CommandBody.Animator.SetBool value
        )
        {
            StringOffset parameter = builder.CreateString(value.Parameter);
            Wire.AnimatorBoolPayload.StartAnimatorBoolPayload(builder);
            Wire.AnimatorBoolPayload.AddValue(builder, value.Value);
            Wire.AnimatorBoolPayload.AddParameter(builder, parameter);
            Wire.AnimatorBoolPayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            return new(
                Wire.CoreCommandKind.AnimatorSetBool,
                Wire.CoreCommandPayload.AnimatorBoolPayload,
                Wire.AnimatorBoolPayload.EndAnimatorBoolPayload(builder).Value
            );
        }

        private static Payload AnimatorInt(
            FlatBufferBuilder builder,
            CommandBody.Animator.SetInt value
        )
        {
            StringOffset parameter = builder.CreateString(value.Parameter);
            Wire.AnimatorIntPayload.StartAnimatorIntPayload(builder);
            Wire.AnimatorIntPayload.AddValue(builder, value.Value);
            Wire.AnimatorIntPayload.AddParameter(builder, parameter);
            Wire.AnimatorIntPayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            return new(
                Wire.CoreCommandKind.AnimatorSetInt,
                Wire.CoreCommandPayload.AnimatorIntPayload,
                Wire.AnimatorIntPayload.EndAnimatorIntPayload(builder).Value
            );
        }

        private static Payload AnimatorFloat(
            FlatBufferBuilder builder,
            CommandBody.Animator.SetFloat value
        )
        {
            StringOffset parameter = builder.CreateString(value.Parameter);
            Wire.AnimatorFloatPayload.StartAnimatorFloatPayload(builder);
            Wire.AnimatorFloatPayload.AddValue(builder, value.Value);
            Wire.AnimatorFloatPayload.AddParameter(builder, parameter);
            Wire.AnimatorFloatPayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            return new(
                Wire.CoreCommandKind.AnimatorSetFloat,
                Wire.CoreCommandPayload.AnimatorFloatPayload,
                Wire.AnimatorFloatPayload.EndAnimatorFloatPayload(builder).Value
            );
        }

        private static Payload AnimatorParameter(
            FlatBufferBuilder builder,
            ObjectId objectId,
            string value,
            Wire.CoreCommandKind kind
        )
        {
            StringOffset parameter = builder.CreateString(value);
            Wire.AnimatorParameterPayload.StartAnimatorParameterPayload(builder);
            Wire.AnimatorParameterPayload.AddParameter(builder, parameter);
            Wire.AnimatorParameterPayload.AddObjectId(builder, Uuid(builder, objectId.Value));
            return new(
                kind,
                Wire.CoreCommandPayload.AnimatorParameterPayload,
                Wire.AnimatorParameterPayload.EndAnimatorParameterPayload(builder).Value
            );
        }

        private static Payload AnimatorSpeed(
            FlatBufferBuilder builder,
            CommandBody.Animator.SetSpeed value
        )
        {
            Wire.AnimatorSpeedPayload.StartAnimatorSpeedPayload(builder);
            Wire.AnimatorSpeedPayload.AddSpeed(builder, value.Speed);
            Wire.AnimatorSpeedPayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            return new(
                Wire.CoreCommandKind.AnimatorSetSpeed,
                Wire.CoreCommandPayload.AnimatorSpeedPayload,
                Wire.AnimatorSpeedPayload.EndAnimatorSpeedPayload(builder).Value
            );
        }

        private static Payload ParticlePlay(
            FlatBufferBuilder builder,
            CommandBody.Particle.Play value
        )
        {
            Wire.ParticlePlayPayload.StartParticlePlayPayload(builder);
            Wire.ParticlePlayPayload.AddRestart(builder, value.Restart);
            Wire.ParticlePlayPayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            return new(
                Wire.CoreCommandKind.ParticlePlay,
                Wire.CoreCommandPayload.ParticlePlayPayload,
                Wire.ParticlePlayPayload.EndParticlePlayPayload(builder).Value
            );
        }

        private static Payload ParticleStop(
            FlatBufferBuilder builder,
            CommandBody.Particle.Stop value
        )
        {
            Wire.ParticleStopPayload.StartParticleStopPayload(builder);
            Wire.ParticleStopPayload.AddClear(builder, value.Clear);
            Wire.ParticleStopPayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            return new(
                Wire.CoreCommandKind.ParticleStop,
                Wire.CoreCommandPayload.ParticleStopPayload,
                Wire.ParticleStopPayload.EndParticleStopPayload(builder).Value
            );
        }

        private static Payload ParticleSpawn(
            FlatBufferBuilder builder,
            CommandBody.Particle.Spawn value
        )
        {
            StringOffset address = builder.CreateString(value.Address.Value);
            Wire.ParticleSpawnPayload.StartParticleSpawnPayload(builder);
            Wire.ParticleSpawnPayload.AddLifetimeMs(builder, Milliseconds(value.Lifetime));
            switch (value.Location)
            {
                case ParticleSpawnLocation.AtGameObject atObject:
                    Wire.ParticleSpawnPayload.AddObjectId(
                        builder,
                        Uuid(builder, atObject.ObjectId.Value)
                    );
                    Wire.ParticleSpawnPayload.AddLocationKind(
                        builder,
                        Wire.ParticleSpawnLocationKind.GameObject
                    );
                    break;
                case ParticleSpawnLocation.AtWorldPosition atWorld:
                    Wire.ParticleSpawnPayload.AddWorldPosition(
                        builder,
                        Vector3(builder, atWorld.Position)
                    );
                    Wire.ParticleSpawnPayload.AddLocationKind(
                        builder,
                        Wire.ParticleSpawnLocationKind.WorldPosition
                    );
                    break;
                default:
                    break;
            }
            Wire.ParticleSpawnPayload.AddAddress(builder, address);
            return new(
                Wire.CoreCommandKind.ParticleSpawn,
                Wire.CoreCommandPayload.ParticleSpawnPayload,
                Wire.ParticleSpawnPayload.EndParticleSpawnPayload(builder).Value
            );
        }

        private static Payload AudioPlay(FlatBufferBuilder builder, CommandBody.Audio.Play value)
        {
            return new(
                Wire.CoreCommandKind.AudioPlay,
                Wire.CoreCommandPayload.AudioPlayPayload,
                Wire.AudioPlayPayload.CreateAudioPlayPayload(
                    builder,
                    builder.CreateString(value.Address.Value),
                    (Wire.AudioBus)value.Bus,
                    value.Volume,
                    value.Pitch,
                    value.Loop,
                    Milliseconds(value.FadeIn)
                ).Value
            );
        }

        private static Payload AudioMix(
            FlatBufferBuilder builder,
            CommandBody.Audio.SetMix value
        ) =>
            new(
                Wire.CoreCommandKind.AudioSetMix,
                Wire.CoreCommandPayload.AudioMixPayload,
                Wire.AudioMixPayload.CreateAudioMixPayload(
                    builder,
                    value.Mix.Master,
                    value.Mix.Music,
                    value.Mix.Effects,
                    value.Mix.Muted
                ).Value
            );

        private static Payload AudioStop(FlatBufferBuilder builder, CommandBody.Audio.Stop value)
        {
            Wire.AudioStopPayload.StartAudioStopPayload(builder);
            Wire.AudioStopPayload.AddFadeOutMs(builder, Milliseconds(value.FadeOut));
            Wire.AudioStopPayload.AddAudioCommandId(
                builder,
                Uuid(builder, value.AudioCommandId.Value)
            );
            return new(
                Wire.CoreCommandKind.AudioStop,
                Wire.CoreCommandPayload.AudioStopPayload,
                Wire.AudioStopPayload.EndAudioStopPayload(builder).Value
            );
        }

        private static Payload AudioPlayback(
            FlatBufferBuilder builder,
            CommandId value,
            Wire.CoreCommandKind kind
        )
        {
            Wire.AudioPlaybackPayload.StartAudioPlaybackPayload(builder);
            Wire.AudioPlaybackPayload.AddAudioCommandId(builder, Uuid(builder, value.Value));
            return new(
                kind,
                Wire.CoreCommandPayload.AudioPlaybackPayload,
                Wire.AudioPlaybackPayload.EndAudioPlaybackPayload(builder).Value
            );
        }

        private static Payload AudioSeek(FlatBufferBuilder builder, CommandBody.Audio.Seek value)
        {
            Wire.AudioSeekPayload.StartAudioSeekPayload(builder);
            Wire.AudioSeekPayload.AddPositionMs(builder, Milliseconds(value.Position));
            Wire.AudioSeekPayload.AddAudioCommandId(
                builder,
                Uuid(builder, value.AudioCommandId.Value)
            );
            return new(
                Wire.CoreCommandKind.AudioSeek,
                Wire.CoreCommandPayload.AudioSeekPayload,
                Wire.AudioSeekPayload.EndAudioSeekPayload(builder).Value
            );
        }

        private static Payload AudioBuffering(
            FlatBufferBuilder builder,
            CommandBody.Audio.SetBuffering value
        )
        {
            Wire.AudioBufferingPayload.StartAudioBufferingPayload(builder);
            Wire.AudioBufferingPayload.AddBuffering(builder, value.Buffering);
            Wire.AudioBufferingPayload.AddAudioCommandId(
                builder,
                Uuid(builder, value.AudioCommandId.Value)
            );
            return new(
                Wire.CoreCommandKind.AudioSetBuffering,
                Wire.CoreCommandPayload.AudioBufferingPayload,
                Wire.AudioBufferingPayload.EndAudioBufferingPayload(builder).Value
            );
        }

        private static Payload AudioReplace(
            FlatBufferBuilder builder,
            CommandBody.Audio.Replace value
        )
        {
            StringOffset address = builder.CreateString(value.Address.Value);
            Wire.AudioReplacePayload.StartAudioReplacePayload(builder);
            Wire.AudioReplacePayload.AddAddress(builder, address);
            Wire.AudioReplacePayload.AddAudioCommandId(
                builder,
                Uuid(builder, value.AudioCommandId.Value)
            );
            return new(
                Wire.CoreCommandKind.AudioReplace,
                Wire.CoreCommandPayload.AudioReplacePayload,
                Wire.AudioReplacePayload.EndAudioReplacePayload(builder).Value
            );
        }

        private static Payload AudioVolume(
            FlatBufferBuilder builder,
            CommandBody.Audio.SetVolume value
        )
        {
            Wire.AudioVolumePayload.StartAudioVolumePayload(builder);
            Wire.AudioVolumePayload.AddVolume(builder, value.Volume);
            Wire.AudioVolumePayload.AddAudioCommandId(
                builder,
                Uuid(builder, value.AudioCommandId.Value)
            );
            Wire.AudioVolumePayload.AddOnConflict(builder, Conflict(value.OnConflict));
            return new(
                Wire.CoreCommandKind.AudioSetVolume,
                Wire.CoreCommandPayload.AudioVolumePayload,
                Wire.AudioVolumePayload.EndAudioVolumePayload(builder).Value
            );
        }

        private static Payload TweenAudioVolume(
            FlatBufferBuilder builder,
            CommandBody.Audio.TweenVolume value
        )
        {
            Wire.TweenAudioVolumePayload.StartTweenAudioVolumePayload(builder);
            Wire.TweenAudioVolumePayload.AddTween(builder, Tween(builder, value.Tween));
            Wire.TweenAudioVolumePayload.AddVolume(builder, value.Volume);
            Wire.TweenAudioVolumePayload.AddAudioCommandId(
                builder,
                Uuid(builder, value.AudioCommandId.Value)
            );
            Wire.TweenAudioVolumePayload.AddOnConflict(builder, Conflict(value.OnConflict));
            return new(
                Wire.CoreCommandKind.AudioTweenVolume,
                Wire.CoreCommandPayload.TweenAudioVolumePayload,
                Wire.TweenAudioVolumePayload.EndTweenAudioVolumePayload(builder).Value
            );
        }

        private static ulong Milliseconds(System.TimeSpan value) =>
            checked((ulong)value.TotalMilliseconds);
    }
}
