#nullable enable

using System;
using System.IO;
using Google.FlatBuffers;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement
{
    /// <summary>
    /// Constructs closed core client submissions in reusable FlatBuffers storage.
    /// </summary>
    internal sealed class BattlementCoreClientMessageWriter
    {
        private FlatBufferBuilder builder;
        private BattlementGeometryActionWriter geometry;
        private BattlementMotionActionWriter motion;

        internal BattlementCoreClientMessageWriter()
        {
            builder = new FlatBufferBuilder(2048);
            geometry = new BattlementGeometryActionWriter(builder);
            motion = new BattlementMotionActionWriter(builder);
        }

        internal int AllocationBytes => builder.DataBuffer.Length;

        internal void TrimOversized()
        {
            if (AllocationBytes <= 1024 * 1024)
                return;
            builder = new FlatBufferBuilder(2048);
            geometry = new BattlementGeometryActionWriter(builder);
            motion = new BattlementMotionActionWriter(builder);
        }

        internal ReadOnlyMemory<byte> WriteAction(Action action)
        {
            builder.Clear();
            ActionBodyOffset body = WriteActionBody(action.Body);
            Wire.CoreAction.StartCoreAction(builder);
            Wire.CoreAction.AddBody(builder, body.Offset);
            Wire.CoreAction.AddBodyType(builder, body.Type);
            Wire.CoreAction.AddKind(builder, body.Kind);
            Wire.CoreAction.AddSessionId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, action.SessionId.Value)
            );
            Wire.CoreAction.AddActionId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, action.Id.Value)
            );
            Offset<Wire.CoreAction> value = Wire.CoreAction.EndCoreAction(builder);
            return Finish(Wire.CoreClientMessageBody.CoreAction, value.Value);
        }

        internal ReadOnlyMemory<byte> WriteBatchCompleted(BatchCompleted completed)
        {
            builder.Clear();
            Wire.BatchCompleted.StartBatchCompleted(builder);
            Wire.BatchCompleted.AddBatchId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, completed.BatchId.Value)
            );
            Wire.BatchCompleted.AddSessionId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, completed.SessionId.Value)
            );
            Offset<Wire.BatchCompleted> value = Wire.BatchCompleted.EndBatchCompleted(builder);
            return Finish(Wire.CoreClientMessageBody.BatchCompleted, value.Value);
        }

        internal ReadOnlyMemory<byte> WriteBatchFailure(BatchFailed<CoreErrorCode> failure)
        {
            ValidateErrorCode(failure.ErrorCode);
            builder.Clear();
            StringOffset message = builder.CreateString(failure.Message);
            Wire.BatchFailed.StartBatchFailed(builder);
            Wire.BatchFailed.AddMessage(builder, message);
            Wire.BatchFailed.AddErrorCode(builder, (Wire.CoreErrorCode)failure.ErrorCode);
            if (failure.CommandId is CommandId commandId)
            {
                Wire.BatchFailed.AddCommandId(
                    builder,
                    BattlementFlatBufferWriter.WriteUuid(builder, commandId.Value)
                );
            }
            Wire.BatchFailed.AddBatchId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, failure.BatchId.Value)
            );
            Wire.BatchFailed.AddSessionId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, failure.SessionId.Value)
            );
            Offset<Wire.BatchFailed> value = Wire.BatchFailed.EndBatchFailed(builder);
            return Finish(Wire.CoreClientMessageBody.BatchFailed, value.Value);
        }

        internal ReadOnlyMemory<byte> WriteOperationFailure(OperationFailed<CoreErrorCode> failure)
        {
            ValidateErrorCode(failure.ErrorCode);
            builder.Clear();
            StringOffset message = builder.CreateString(failure.Message);
            Wire.OperationFailed.StartOperationFailed(builder);
            Wire.OperationFailed.AddMessage(builder, message);
            Wire.OperationFailed.AddErrorCode(builder, (Wire.CoreErrorCode)failure.ErrorCode);
            Wire.OperationFailed.AddCommandId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, failure.CommandId.Value)
            );
            Wire.OperationFailed.AddBatchId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, failure.BatchId.Value)
            );
            Wire.OperationFailed.AddSessionId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, failure.SessionId.Value)
            );
            Offset<Wire.OperationFailed> value = Wire.OperationFailed.EndOperationFailed(builder);
            return Finish(Wire.CoreClientMessageBody.OperationFailed, value.Value);
        }

        private ReadOnlyMemory<byte> Finish(Wire.CoreClientMessageBody type, int offset)
        {
            Offset<Wire.CoreClientMessage> root = Wire.CoreClientMessage.CreateCoreClientMessage(
                builder,
                type,
                offset
            );
            Wire.CoreClientMessage.FinishSizePrefixedCoreClientMessageBuffer(builder, root);
            if (builder.Offset > BattlementProtocolLimits.MaximumMessageBytes)
                throw new InvalidDataException("A core client message cannot exceed 16 MiB.");
            return builder.DataBuffer.ToReadOnlyMemory(builder.DataBuffer.Position, builder.Offset);
        }

        private ActionBodyOffset WriteActionBody(ActionBody body)
        {
            return body switch
            {
                ActionBody.Activate value => Activation(value),
                ActionBody.PointerEnter value => Pointer(
                    Wire.CoreActionKind.PointerEnter,
                    value.ObjectId,
                    value.PointerId,
                    value.ScreenPosition,
                    value.WorldHit
                ),
                ActionBody.PointerExit value => Pointer(
                    Wire.CoreActionKind.PointerExit,
                    value.ObjectId,
                    value.PointerId,
                    value.ScreenPosition,
                    value.WorldHit
                ),
                ActionBody.PointerDown value => PointerButton(
                    Wire.CoreActionKind.PointerDown,
                    value.ObjectId,
                    value.PointerId,
                    value.ScreenPosition,
                    value.WorldHit,
                    value.Button
                ),
                ActionBody.PointerUp value => PointerButton(
                    Wire.CoreActionKind.PointerUp,
                    value.ObjectId,
                    value.PointerId,
                    value.ScreenPosition,
                    value.WorldHit,
                    value.Button
                ),
                ActionBody.PointerClick value => PointerButton(
                    Wire.CoreActionKind.PointerClick,
                    value.ObjectId,
                    value.PointerId,
                    value.ScreenPosition,
                    value.WorldHit,
                    value.Button
                ),
                ActionBody.DragStart value => Drag(
                    Wire.CoreActionKind.DragStart,
                    value.ObjectId,
                    value.PointerId,
                    value.ScreenPosition,
                    value.WorldPosition
                ),
                ActionBody.DragEnd value => Drag(
                    Wire.CoreActionKind.DragEnd,
                    value.ObjectId,
                    value.PointerId,
                    value.ScreenPosition,
                    value.WorldPosition
                ),
                ActionBody.KeyDown value => Key(Wire.CoreActionKind.KeyDown, value.Key),
                ActionBody.KeyUp value => Key(Wire.CoreActionKind.KeyUp, value.Key),
                ActionBody.ControllerButtonDown value => ControllerButton(
                    Wire.CoreActionKind.ControllerButtonDown,
                    value.ControllerId,
                    value.Button
                ),
                ActionBody.ControllerButtonUp value => ControllerButton(
                    Wire.CoreActionKind.ControllerButtonUp,
                    value.ControllerId,
                    value.Button
                ),
                ActionBody.ControllerNavigate value => ControllerNavigate(value),
                ActionBody.InputCaptured value => new ActionBodyOffset(
                    Wire.CoreActionKind.InputCaptured,
                    Wire.CoreActionBody.InputCaptureAction,
                    BattlementInputCaptureWire.Write(builder, value.Value).Value
                ),
                ActionBody.HostSettingsChanged value => HostSettings(value.Value),
                ActionBody.ApplicationStateChanged value => ApplicationState(value.Value),
                ActionBody.ReducedMotionPreferenceChanged value => ReducedMotion(value.Value),
                ActionBody.GeometryObservations value => new(
                    Wire.CoreActionKind.GeometryObservations,
                    Wire.CoreActionBody.GeometryAction,
                    geometry.Write(value.Value).Value
                ),
                ActionBody.MotionEvents value => new(
                    Wire.CoreActionKind.MotionEvents,
                    Wire.CoreActionBody.MotionAction,
                    motion.Write(value.Value).Value
                ),
                _ => throw new InvalidDataException("Unknown core action body."),
            };
        }

        private ActionBodyOffset Activation(ActionBody.Activate value)
        {
            Wire.ActivationAction.StartActivationAction(builder);
            Wire.ActivationAction.AddObjectId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, value.ObjectId.Value)
            );
            return new(
                Wire.CoreActionKind.Activate,
                Wire.CoreActionBody.ActivationAction,
                Wire.ActivationAction.EndActivationAction(builder).Value
            );
        }

        private ActionBodyOffset Pointer(
            Wire.CoreActionKind kind,
            ObjectId objectId,
            int pointerId,
            ScreenPosition screen,
            Vector3 world
        )
        {
            ValidatePointerId(pointerId);
            ValidateFinite(screen, world);
            Wire.PointerAction.StartPointerAction(builder);
            Wire.PointerAction.AddPointerId(builder, pointerId);
            Wire.PointerAction.AddWorldHit(
                builder,
                Wire.Vector3d.CreateVector3d(builder, world.X, world.Y, world.Z)
            );
            Wire.PointerAction.AddScreenPosition(
                builder,
                Wire.PanelPoint.CreatePanelPoint(builder, screen.X, screen.Y)
            );
            Wire.PointerAction.AddObjectId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, objectId.Value)
            );
            return new(
                kind,
                Wire.CoreActionBody.PointerAction,
                Wire.PointerAction.EndPointerAction(builder).Value
            );
        }

        private ActionBodyOffset PointerButton(
            Wire.CoreActionKind kind,
            ObjectId objectId,
            int pointerId,
            ScreenPosition screen,
            Vector3 world,
            Battlement.PointerButton button
        )
        {
            ValidatePointerId(pointerId);
            ValidateFinite(screen, world);
            Wire.PointerButtonKind wireButton = button switch
            {
                Battlement.PointerButton.Left => Wire.PointerButtonKind.Left,
                Battlement.PointerButton.Middle => Wire.PointerButtonKind.Middle,
                Battlement.PointerButton.Right => Wire.PointerButtonKind.Right,
                _ => throw new InvalidDataException("Unknown pointer button."),
            };
            Offset<Wire.PointerButton> buttonOffset = Wire.PointerButton.CreatePointerButton(
                builder,
                wireButton
            );
            Wire.PointerButtonAction.StartPointerButtonAction(builder);
            Wire.PointerButtonAction.AddButton(builder, buttonOffset);
            Wire.PointerButtonAction.AddPointerId(builder, pointerId);
            Wire.PointerButtonAction.AddWorldHit(
                builder,
                Wire.Vector3d.CreateVector3d(builder, world.X, world.Y, world.Z)
            );
            Wire.PointerButtonAction.AddScreenPosition(
                builder,
                Wire.PanelPoint.CreatePanelPoint(builder, screen.X, screen.Y)
            );
            Wire.PointerButtonAction.AddObjectId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, objectId.Value)
            );
            return new(
                kind,
                Wire.CoreActionBody.PointerButtonAction,
                Wire.PointerButtonAction.EndPointerButtonAction(builder).Value
            );
        }

        private ActionBodyOffset Drag(
            Wire.CoreActionKind kind,
            ObjectId objectId,
            int pointerId,
            ScreenPosition screen,
            Vector3 world
        )
        {
            ValidatePointerId(pointerId);
            ValidateFinite(screen, world);
            Wire.DragAction.StartDragAction(builder);
            Wire.DragAction.AddPointerId(builder, pointerId);
            Wire.DragAction.AddWorldPosition(
                builder,
                Wire.Vector3d.CreateVector3d(builder, world.X, world.Y, world.Z)
            );
            Wire.DragAction.AddScreenPosition(
                builder,
                Wire.PanelPoint.CreatePanelPoint(builder, screen.X, screen.Y)
            );
            Wire.DragAction.AddObjectId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, objectId.Value)
            );
            return new(
                kind,
                Wire.CoreActionBody.DragAction,
                Wire.DragAction.EndDragAction(builder).Value
            );
        }

        private ActionBodyOffset Key(Wire.CoreActionKind kind, PhysicalKey key)
        {
            if ((uint)key > (uint)PhysicalKey.NumpadEnter)
                throw new InvalidDataException("Unknown physical key.");
            Offset<Wire.KeyAction> value = Wire.KeyAction.CreateKeyAction(
                builder,
                (Wire.PhysicalKey)key
            );
            return new(kind, Wire.CoreActionBody.KeyAction, value.Value);
        }

        private ActionBodyOffset ControllerButton(
            Wire.CoreActionKind kind,
            int controllerId,
            Battlement.ControllerButton button
        )
        {
            if (controllerId < 0)
                throw new InvalidDataException("Controller identities must be nonnegative.");
            if ((uint)button > (uint)Battlement.ControllerButton.Select)
                throw new InvalidDataException("Unknown controller button.");
            Offset<Wire.ControllerButtonAction> value =
                Wire.ControllerButtonAction.CreateControllerButtonAction(
                    builder,
                    controllerId,
                    (Wire.ControllerButton)button
                );
            return new(kind, Wire.CoreActionBody.ControllerButtonAction, value.Value);
        }

        private ActionBodyOffset ControllerNavigate(ActionBody.ControllerNavigate value)
        {
            if (value.ControllerId < 0)
                throw new InvalidDataException("Controller identities must be nonnegative.");
            if ((uint)value.Direction > (uint)ControllerDirection.Down)
                throw new InvalidDataException("Unknown controller direction.");
            if ((uint)value.Source > (uint)ControllerNavigationSource.LeftStick)
                throw new InvalidDataException("Unknown controller navigation source.");
            Offset<Wire.ControllerNavigateAction> action =
                Wire.ControllerNavigateAction.CreateControllerNavigateAction(
                    builder,
                    value.ControllerId,
                    (Wire.ControllerDirection)value.Direction,
                    (Wire.ControllerNavigationSource)value.Source,
                    value.Repeat
                );
            return new(
                Wire.CoreActionKind.ControllerNavigate,
                Wire.CoreActionBody.ControllerNavigateAction,
                action.Value
            );
        }

        private ActionBodyOffset HostSettings(HostSettings value)
        {
            var settings = BattlementHostSettingsWriter.Write(builder, value);
            var action = Wire.HostSettingsAction.CreateHostSettingsAction(builder, settings);
            return new(
                Wire.CoreActionKind.HostSettingsChanged,
                Wire.CoreActionBody.HostSettingsAction,
                action.Value
            );
        }

        private ActionBodyOffset ApplicationState(ApplicationState value)
        {
            Offset<Wire.ApplicationState> state = Wire.ApplicationState.CreateApplicationState(
                builder,
                value.Focused,
                value.Paused
            );
            Offset<Wire.ApplicationStateAction> action =
                Wire.ApplicationStateAction.CreateApplicationStateAction(builder, state);
            return new(
                Wire.CoreActionKind.ApplicationStateChanged,
                Wire.CoreActionBody.ApplicationStateAction,
                action.Value
            );
        }

        private ActionBodyOffset ReducedMotion(ReducedMotionPreference value)
        {
            if ((uint)value > (uint)ReducedMotionPreference.NoPreference)
                throw new InvalidDataException("Unknown reduced-motion preference.");
            Offset<Wire.ReducedMotionPreferenceAction> action =
                Wire.ReducedMotionPreferenceAction.CreateReducedMotionPreferenceAction(
                    builder,
                    (Wire.ReducedMotionPreference)value
                );
            return new(
                Wire.CoreActionKind.ReducedMotionPreferenceChanged,
                Wire.CoreActionBody.ReducedMotionPreferenceAction,
                action.Value
            );
        }

        private static void ValidatePointerId(int value)
        {
            if (value < 0)
                throw new InvalidDataException("Pointer identities must be nonnegative.");
        }

        private static void ValidateFinite(ScreenPosition screen, Vector3 world)
        {
            if (
                !double.IsFinite(screen.X)
                || !double.IsFinite(screen.Y)
                || !double.IsFinite(world.X)
                || !double.IsFinite(world.Y)
                || !double.IsFinite(world.Z)
            )
                throw new InvalidDataException("Action coordinates must be finite.");
        }

        private static void ValidateErrorCode(CoreErrorCode value)
        {
            if ((uint)value > (uint)CoreErrorCode.DiagnosticsOperationFailed)
                throw new InvalidDataException("Unknown core error code.");
        }

        private readonly struct ActionBodyOffset
        {
            internal ActionBodyOffset(
                Wire.CoreActionKind kind,
                Wire.CoreActionBody type,
                int offset
            ) => (Kind, Type, Offset) = (kind, type, offset);

            internal Wire.CoreActionKind Kind { get; }
            internal Wire.CoreActionBody Type { get; }
            internal int Offset { get; }
        }
    }
}
