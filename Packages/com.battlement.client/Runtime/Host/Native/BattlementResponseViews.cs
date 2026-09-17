#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement
{
    internal interface IBattlementDirectCustomCommand
    {
        IBattlementCommandOperation? Launch(
            IBattlementFlatBufferCustomCommandDispatcher dispatcher,
            TimeSpan now
        );
    }

    internal readonly struct BattlementCommandExecution
    {
        private BattlementCommandExecution(CommandId id, bool isBlocking)
        {
            Id = id;
            IsBlocking = isBlocking;
            DirectLocalPosition = null;
            DirectWorldPosition = null;
            DirectLabelUpdate = null;
            DirectTextContent = null;
            DirectSetMaterial = null;
            DirectDestroyObject = null;
            DirectInputEnabled = null;
            DirectTweenLocalPosition = null;
            DirectImageObjectCreate = null;
            DirectObjectActive = null;
            DirectRotation = null;
            DirectScale = null;
            DirectTweenRotation = null;
            DirectTweenScale = null;
            DirectPrimitiveObjectCreate = null;
            DirectPrefabObjectCreate = null;
            DirectEmptyObjectCreate = null;
            DirectTextObjectCreate = null;
            DirectCameraObjectCreate = null;
            DirectLightObjectCreate = null;
            DirectObjectReparent = null;
            DirectParticleSpawn = null;
            DirectAudioPlay = null;
            DirectParticlePlay = null;
            DirectAudioStop = null;
            DirectAudioVolume = null;
            DirectWait = null;
            DirectVibration = null;
            DirectDebugUi = null;
            DirectAudioControl = null;
            DirectTweenAudioVolume = null;
            DirectSceneCommand = null;
            DirectCancel = null;
            DirectInputConfiguration = null;
            DirectParticleStop = null;
            DirectOpenUrl = null;
            DirectComponent = null;
            DirectAnimator = null;
            DirectGeometry = null;
            DirectAccessibility = null;
            DirectDiagnostics = null;
            DirectAssets = null;
            DirectUiDestroy = null;
            DirectUiAction = null;
            DirectUiPlacement = null;
            DirectMotion = null;
            DirectMotionValue = null;
            DirectMotionControl = null;
            DirectMotionScope = null;
            DirectUiProperties = null;
            DirectUiScalar = null;
            DirectUiCreate = null;
            DirectCustomCommand = null;
        }

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectLocalPosition directLocalPosition
        )
            : this(id, isBlocking) => DirectLocalPosition = directLocalPosition;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectWorldPosition directWorldPosition
        )
            : this(id, isBlocking) => DirectWorldPosition = directWorldPosition;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectLabelUpdate directLabelUpdate
        )
            : this(id, isBlocking) => DirectLabelUpdate = directLabelUpdate;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectTextContent directTextContent
        )
            : this(id, isBlocking) => DirectTextContent = directTextContent;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectSetMaterial directSetMaterial
        )
            : this(id, isBlocking) => DirectSetMaterial = directSetMaterial;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectDestroyObject directDestroyObject
        )
            : this(id, isBlocking) => DirectDestroyObject = directDestroyObject;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectInputEnabled directInputEnabled
        )
            : this(id, isBlocking) => DirectInputEnabled = directInputEnabled;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectTweenLocalPosition directTweenLocalPosition
        )
            : this(id, isBlocking) => DirectTweenLocalPosition = directTweenLocalPosition;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectImageObjectCreate directImageObjectCreate
        )
            : this(id, isBlocking) => DirectImageObjectCreate = directImageObjectCreate;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectObjectActive directObjectActive
        )
            : this(id, isBlocking) => DirectObjectActive = directObjectActive;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectRotation directRotation
        )
            : this(id, isBlocking) => DirectRotation = directRotation;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectScale directScale
        )
            : this(id, isBlocking) => DirectScale = directScale;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectTweenRotation directTweenRotation
        )
            : this(id, isBlocking) => DirectTweenRotation = directTweenRotation;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectTweenScale directTweenScale
        )
            : this(id, isBlocking) => DirectTweenScale = directTweenScale;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectPrimitiveObjectCreate directPrimitiveObjectCreate
        )
            : this(id, isBlocking) => DirectPrimitiveObjectCreate = directPrimitiveObjectCreate;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectPrefabObjectCreate directPrefabObjectCreate
        )
            : this(id, isBlocking) => DirectPrefabObjectCreate = directPrefabObjectCreate;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectEmptyObjectCreate directEmptyObjectCreate
        )
            : this(id, isBlocking) => DirectEmptyObjectCreate = directEmptyObjectCreate;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectTextObjectCreate directTextObjectCreate
        )
            : this(id, isBlocking) => DirectTextObjectCreate = directTextObjectCreate;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectCameraObjectCreate directCameraObjectCreate
        )
            : this(id, isBlocking) => DirectCameraObjectCreate = directCameraObjectCreate;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectLightObjectCreate directLightObjectCreate
        )
            : this(id, isBlocking) => DirectLightObjectCreate = directLightObjectCreate;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectObjectReparent directObjectReparent
        )
            : this(id, isBlocking) => DirectObjectReparent = directObjectReparent;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectParticleSpawn directParticleSpawn
        )
            : this(id, isBlocking) => DirectParticleSpawn = directParticleSpawn;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectAudioPlay directAudioPlay
        )
            : this(id, isBlocking) => DirectAudioPlay = directAudioPlay;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectParticlePlay directParticlePlay
        )
            : this(id, isBlocking) => DirectParticlePlay = directParticlePlay;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectAudioStop directAudioStop
        )
            : this(id, isBlocking) => DirectAudioStop = directAudioStop;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectAudioVolume directAudioVolume
        )
            : this(id, isBlocking) => DirectAudioVolume = directAudioVolume;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectWait directWait
        )
            : this(id, isBlocking) => DirectWait = directWait;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectVibration directVibration
        )
            : this(id, isBlocking) => DirectVibration = directVibration;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectDebugUi directDebugUi
        )
            : this(id, isBlocking) => DirectDebugUi = directDebugUi;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectAudioControl directAudioControl
        )
            : this(id, isBlocking) => DirectAudioControl = directAudioControl;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectTweenAudioVolume directTweenAudioVolume
        )
            : this(id, isBlocking) => DirectTweenAudioVolume = directTweenAudioVolume;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectSceneCommand directSceneCommand
        )
            : this(id, isBlocking) => DirectSceneCommand = directSceneCommand;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectCancel directCancel
        )
            : this(id, isBlocking) => DirectCancel = directCancel;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectInputConfiguration directInputConfiguration
        )
            : this(id, isBlocking) => DirectInputConfiguration = directInputConfiguration;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectParticleStop directParticleStop
        )
            : this(id, isBlocking) => DirectParticleStop = directParticleStop;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectOpenUrl directOpenUrl
        )
            : this(id, isBlocking) => DirectOpenUrl = directOpenUrl;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectComponentCommand directComponent
        )
            : this(id, isBlocking) => DirectComponent = directComponent;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectAnimatorCommand directAnimator
        )
            : this(id, isBlocking) => DirectAnimator = directAnimator;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectGeometryCommand directGeometry
        )
            : this(id, isBlocking) => DirectGeometry = directGeometry;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectAccessibilityCommand directAccessibility
        )
            : this(id, isBlocking) => DirectAccessibility = directAccessibility;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectDiagnostics directDiagnostics
        )
            : this(id, isBlocking) => DirectDiagnostics = directDiagnostics;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectAssetSet directAssets
        )
            : this(id, isBlocking) => DirectAssets = directAssets;

        internal BattlementCommandExecution(CommandId id, bool isBlocking, ObjectId directUiDestroy)
            : this(id, isBlocking) => DirectUiDestroy = directUiDestroy;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectVisualElementAction directUiAction
        )
            : this(id, isBlocking) => DirectUiAction = directUiAction;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectVisualElementPlacement directUiPlacement
        )
            : this(id, isBlocking) => DirectUiPlacement = directUiPlacement;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectMotionCommand directMotion
        )
            : this(id, isBlocking) => DirectMotion = directMotion;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectMotionValueCommand directMotionValue
        )
            : this(id, isBlocking) => DirectMotionValue = directMotionValue;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectMotionControlCommand directMotionControl
        )
            : this(id, isBlocking) => DirectMotionControl = directMotionControl;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectMotionScopeCommand directMotionScope
        )
            : this(id, isBlocking) => DirectMotionScope = directMotionScope;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectUiProperties directUiProperties
        )
            : this(id, isBlocking) => DirectUiProperties = directUiProperties;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectUiScalar directUiScalar
        )
            : this(id, isBlocking) => DirectUiScalar = directUiScalar;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            BattlementDirectUiCreate directUiCreate
        )
            : this(id, isBlocking) => DirectUiCreate = directUiCreate;

        internal BattlementCommandExecution(
            CommandId id,
            bool isBlocking,
            IBattlementDirectCustomCommand directCustomCommand
        )
            : this(id, isBlocking) => DirectCustomCommand = directCustomCommand;

        internal CommandId Id { get; }
        internal bool IsBlocking { get; }
        internal BattlementDirectLocalPosition? DirectLocalPosition { get; }
        internal BattlementDirectWorldPosition? DirectWorldPosition { get; }
        internal BattlementDirectLabelUpdate? DirectLabelUpdate { get; }
        internal BattlementDirectTextContent? DirectTextContent { get; }
        internal BattlementDirectSetMaterial? DirectSetMaterial { get; }
        internal BattlementDirectDestroyObject? DirectDestroyObject { get; }
        internal BattlementDirectInputEnabled? DirectInputEnabled { get; }
        internal BattlementDirectTweenLocalPosition? DirectTweenLocalPosition { get; }
        internal BattlementDirectImageObjectCreate? DirectImageObjectCreate { get; }
        internal BattlementDirectObjectActive? DirectObjectActive { get; }
        internal BattlementDirectRotation? DirectRotation { get; }
        internal BattlementDirectScale? DirectScale { get; }
        internal BattlementDirectTweenRotation? DirectTweenRotation { get; }
        internal BattlementDirectTweenScale? DirectTweenScale { get; }
        internal BattlementDirectPrimitiveObjectCreate? DirectPrimitiveObjectCreate { get; }
        internal BattlementDirectPrefabObjectCreate? DirectPrefabObjectCreate { get; }
        internal BattlementDirectEmptyObjectCreate? DirectEmptyObjectCreate { get; }
        internal BattlementDirectTextObjectCreate? DirectTextObjectCreate { get; }
        internal BattlementDirectCameraObjectCreate? DirectCameraObjectCreate { get; }
        internal BattlementDirectLightObjectCreate? DirectLightObjectCreate { get; }
        internal BattlementDirectObjectReparent? DirectObjectReparent { get; }
        internal BattlementDirectParticleSpawn? DirectParticleSpawn { get; }
        internal BattlementDirectAudioPlay? DirectAudioPlay { get; }
        internal BattlementDirectParticlePlay? DirectParticlePlay { get; }
        internal BattlementDirectAudioStop? DirectAudioStop { get; }
        internal BattlementDirectAudioVolume? DirectAudioVolume { get; }
        internal BattlementDirectWait? DirectWait { get; }
        internal BattlementDirectVibration? DirectVibration { get; }
        internal BattlementDirectDebugUi? DirectDebugUi { get; }
        internal BattlementDirectAudioControl? DirectAudioControl { get; }
        internal BattlementDirectTweenAudioVolume? DirectTweenAudioVolume { get; }
        internal BattlementDirectSceneCommand? DirectSceneCommand { get; }
        internal BattlementDirectCancel? DirectCancel { get; }
        internal BattlementDirectInputConfiguration? DirectInputConfiguration { get; }
        internal BattlementDirectParticleStop? DirectParticleStop { get; }
        internal BattlementDirectOpenUrl? DirectOpenUrl { get; }
        internal BattlementDirectComponentCommand? DirectComponent { get; }
        internal BattlementDirectAnimatorCommand? DirectAnimator { get; }
        internal BattlementDirectGeometryCommand? DirectGeometry { get; }
        internal BattlementDirectAccessibilityCommand? DirectAccessibility { get; }
        internal BattlementDirectDiagnostics? DirectDiagnostics { get; }
        internal BattlementDirectAssetSet? DirectAssets { get; }
        internal ObjectId? DirectUiDestroy { get; }
        internal BattlementDirectVisualElementAction? DirectUiAction { get; }
        internal BattlementDirectVisualElementPlacement? DirectUiPlacement { get; }
        internal BattlementDirectMotionCommand? DirectMotion { get; }
        internal BattlementDirectMotionValueCommand? DirectMotionValue { get; }
        internal BattlementDirectMotionControlCommand? DirectMotionControl { get; }
        internal BattlementDirectMotionScopeCommand? DirectMotionScope { get; }
        internal BattlementDirectUiProperties? DirectUiProperties { get; }
        internal BattlementDirectUiScalar? DirectUiScalar { get; }
        internal BattlementDirectUiCreate? DirectUiCreate { get; }
        internal IBattlementDirectCustomCommand? DirectCustomCommand { get; }
        internal ConflictPolicy? DirectConflictPolicy =>
            DirectLocalPosition?.OnConflict
            ?? DirectWorldPosition?.OnConflict
            ?? DirectTweenLocalPosition?.OnConflict
            ?? DirectRotation?.OnConflict
            ?? DirectScale?.OnConflict
            ?? DirectTweenRotation?.OnConflict
            ?? DirectTweenScale?.OnConflict
            ?? DirectAudioVolume?.OnConflict
            ?? DirectTweenAudioVolume?.OnConflict
            ?? DirectSetMaterial?.OnConflict
            ?? DirectComponent?.OnConflict;
    }

    internal readonly struct BattlementDirectUiCreate : IBattlementUiForestView
    {
        private readonly IBattlementFlatBufferViewOwner owner;
        private readonly Wire.VisualElementCreatePayload value;

        internal BattlementDirectUiCreate(
            IBattlementFlatBufferViewOwner owner,
            Wire.VisualElementCreatePayload value
        ) => (this.owner, this.value) = (owner, value);

        internal ObjectId ParentId
        {
            get
            {
                Check();
                return new ObjectId(
                    BattlementFlatBufferCore.ReadUuid(value.ParentId, "visual create parent")
                );
            }
        }
        internal uint? ChildIndex
        {
            get
            {
                Check();
                return value.ChildIndex;
            }
        }
        internal ObjectId RootId
        {
            get
            {
                Check();
                return new ObjectId(
                    BattlementFlatBufferCore.ReadUuid(value.RootId, "visual create root")
                );
            }
        }
        public int NodeCount
        {
            get
            {
                Check();
                return value.NodesLength;
            }
        }

        public ObjectId ReadNodeId(int index)
        {
            Wire.UiNode node = Node(index);
            return new ObjectId(
                BattlementFlatBufferCore.ReadUuid(node.ObjectId, "visual create node")
            );
        }

        public UiElement ReadNodeElement(int index) =>
            BattlementFlatBufferRetainedCopy.UiElement(
                Node(index).Element
                    ?? throw new InvalidDataException("A visual create element is absent.")
            );

        public int ReadChildCount(int index) => Node(index).ChildIdsLength;

        public ObjectId ReadChildId(int nodeIndex, int childIndex) =>
            new(
                BattlementFlatBufferCore.ReadUuid(
                    Node(nodeIndex).ChildIds(childIndex),
                    "visual create child"
                )
            );

        private Wire.UiNode Node(int index)
        {
            Check();
            return value.Nodes(index)
                ?? throw new InvalidDataException("A visual create node is absent.");
        }

        private void Check() => owner.RequireLiveView();
    }

    internal readonly struct BattlementDirectUiProperties
    {
        private readonly IBattlementFlatBufferViewOwner owner;
        private readonly Wire.VisualElementUpdatePayload value;

        internal BattlementDirectUiProperties(
            IBattlementFlatBufferViewOwner owner,
            Wire.VisualElementUpdatePayload value
        ) => (this.owner, this.value) = (owner, value);

        internal ObjectId ObjectId
        {
            get
            {
                Check();
                return new ObjectId(
                    BattlementFlatBufferCore.ReadUuid(value.ObjectId, "visual update object")
                );
            }
        }

        internal UiElement ReadElement()
        {
            Check();
            return BattlementFlatBufferRetainedCopy.UiElement(
                value.Element
                    ?? throw new InvalidDataException("A visual property update is absent.")
            );
        }

        private void Check() => owner.RequireLiveView();
    }

    internal readonly struct BattlementDirectUiScalar : IBattlementUiScalarUpdateView
    {
        private readonly IBattlementFlatBufferViewOwner owner;
        private readonly Wire.UiElement element;

        internal BattlementDirectUiScalar(
            IBattlementFlatBufferViewOwner owner,
            Wire.UiElement element,
            BattlementUiScalarUpdateKind kind,
            ObjectId objectId,
            bool boolean = default,
            int integer = default,
            uint unsigned = default,
            uint unsignedSecond = default,
            float first = default,
            float second = default
        ) =>
            (
                this.owner,
                this.element,
                Kind,
                ObjectId,
                Boolean,
                Integer,
                Unsigned,
                UnsignedSecond,
                First,
                Second
            ) = (
                owner,
                element,
                kind,
                objectId,
                boolean,
                integer,
                unsigned,
                unsignedSecond,
                first,
                second
            );

        public BattlementUiScalarUpdateKind Kind { get; }
        public ObjectId ObjectId { get; }
        public bool Boolean { get; }
        public int Integer { get; }
        public uint Unsigned { get; }
        public uint UnsignedSecond { get; }
        public float First { get; }
        public float Second { get; }

        public string? ReadText()
        {
            Check();
            for (int index = 0; index < element.PropertiesLength; index++)
            {
                Wire.UiProperty property = element.Properties(index)!.Value;
                if (property.ValueType == Wire.UiPropertyValue.TextPropertyValue)
                    return property.ValueAsTextPropertyValue().Value;
                if (property.ValueType == Wire.UiPropertyValue.ChoicePropertyValue)
                    return property.ValueAsChoicePropertyValue().Value;
            }
            return null;
        }

        public int IndexCount
        {
            get
            {
                Check();
                return Indices().ValuesLength;
            }
        }

        public uint ReadIndex(int index)
        {
            Check();
            return Indices().Values(index);
        }

        private Wire.UIntListPropertyValue Indices()
        {
            Wire.UiProperty property = element.Properties(0)!.Value;
            if (property.ValueType != Wire.UiPropertyValue.UIntListPropertyValue)
                throw new InvalidDataException("A UI scalar does not contain selected indices.");
            return property.ValueAsUIntListPropertyValue();
        }

        private void Check() => owner.RequireLiveView();
    }

    internal readonly struct BattlementDirectMotionControlCommand
    {
        private readonly IBattlementFlatBufferViewOwner owner;
        private readonly Wire.MotionControlOperation value;

        internal BattlementDirectMotionControlCommand(
            IBattlementFlatBufferViewOwner owner,
            Wire.MotionControlOperation value
        ) => (this.owner, this.value) = (owner, value);

        internal ObjectId ControlId
        {
            get
            {
                Check();
                return Object(value.ControlId, "motion control");
            }
        }
        internal MotionControlOperationKind Kind
        {
            get
            {
                Check();
                return (MotionControlOperationKind)(byte)value.Command;
            }
        }
        internal ObjectId PlaybackId
        {
            get
            {
                Check();
                return Object(value.PlaybackId, "motion control playback");
            }
        }
        internal uint Generation
        {
            get
            {
                Check();
                return value.Generation;
            }
        }

        internal MotionControlTarget ReadTarget()
        {
            Check();
            Wire.MotionControlTarget target =
                value.Target
                ?? throw new InvalidDataException("A motion control target is absent.");
            return target.Kind switch
            {
                Wire.MotionControlTargetKind.Target when target.Target.HasValue =>
                    new MotionControlTarget.Target(
                        BattlementFlatBufferRetainedCopy.MotionTarget(target.Target.Value)
                    ),
                Wire.MotionControlTargetKind.Variant => new MotionControlTarget.Variant(
                    target.Variant ?? throw new InvalidDataException("A motion variant is absent.")
                ),
                _ => throw new InvalidDataException("A motion control target kind is invalid."),
            };
        }

        private static ObjectId Object(Wire.Uuid? id, string name) =>
            new(BattlementFlatBufferCore.ReadUuid(id, name));

        private void Check() => owner.RequireLiveView();
    }

    internal readonly struct BattlementDirectMotionScopeCommand : IBattlementMotionScopeView
    {
        private readonly IBattlementFlatBufferViewOwner owner;
        private readonly Wire.MotionScopeOperation value;

        internal BattlementDirectMotionScopeCommand(
            IBattlementFlatBufferViewOwner owner,
            Wire.MotionScopeOperation value
        ) => (this.owner, this.value) = (owner, value);

        public ObjectId ScopeId
        {
            get
            {
                Check();
                return Object(value.ScopeId, "motion scope");
            }
        }
        public MotionScopeOperationKind Kind
        {
            get
            {
                Check();
                return (MotionScopeOperationKind)(byte)value.Command;
            }
        }
        public ObjectId PlaybackId
        {
            get
            {
                Check();
                return Object(value.PlaybackId, "motion scope playback");
            }
        }
        public uint Generation
        {
            get
            {
                Check();
                return value.Generation;
            }
        }
        public int StepCount
        {
            get
            {
                Check();
                return value.StepsLength;
            }
        }

        public MotionSelector ReadStepSelector(int index)
        {
            Wire.MotionSequenceStep step = Step(index);
            return BattlementFlatBufferRetainedCopy.MotionSelector(
                step.Selector ?? throw new InvalidDataException("A motion step selector is absent.")
            );
        }

        public MotionTargetDescriptor ReadStepTarget(int index)
        {
            Wire.MotionSequenceStep step = Step(index);
            return BattlementFlatBufferRetainedCopy.MotionTarget(
                step.Target ?? throw new InvalidDataException("A motion step target is absent.")
            );
        }

        public ulong ReadStepStartMicros(int index) => Step(index).StartMicros;

        public MotionSelector ReadSelector()
        {
            Check();
            return BattlementFlatBufferRetainedCopy.MotionSelector(
                value.Selector
                    ?? throw new InvalidDataException("A motion scope selector is absent.")
            );
        }

        public MotionTargetDescriptor ReadTarget()
        {
            Check();
            return BattlementFlatBufferRetainedCopy.MotionTarget(
                value.Target ?? throw new InvalidDataException("A motion scope target is absent.")
            );
        }

        private Wire.MotionSequenceStep Step(int index)
        {
            Check();
            return value.Steps(index)
                ?? throw new InvalidDataException("A motion scope step is absent.");
        }

        private static ObjectId Object(Wire.Uuid? id, string name) =>
            new(BattlementFlatBufferCore.ReadUuid(id, name));

        private void Check() => owner.RequireLiveView();
    }

    internal readonly struct BattlementDirectMotionValueCommand
    {
        private readonly IBattlementFlatBufferViewOwner owner;
        private readonly Wire.MotionValueOperation value;

        internal BattlementDirectMotionValueCommand(
            IBattlementFlatBufferViewOwner owner,
            Wire.MotionValueOperation value
        ) => (this.owner, this.value) = (owner, value);

        internal ObjectId ValueId
        {
            get
            {
                Check();
                return new ObjectId(
                    BattlementFlatBufferCore.ReadUuid(value.ValueId, "motion value")
                );
            }
        }

        internal MotionValueOperationKind Kind
        {
            get
            {
                Check();
                return (MotionValueOperationKind)(byte)value.Command;
            }
        }

        internal ObjectId PlaybackId
        {
            get
            {
                Check();
                return new ObjectId(
                    BattlementFlatBufferCore.ReadUuid(value.PlaybackId, "motion value playback")
                );
            }
        }

        internal uint Generation
        {
            get
            {
                Check();
                return value.Generation;
            }
        }

        internal MotionValue ReadValue()
        {
            Check();
            return BattlementFlatBufferRetainedCopy.MotionValue(value);
        }

        internal TransitionDefinition ReadTransition()
        {
            Check();
            return BattlementFlatBufferRetainedCopy.Transition(
                value.Transition
                    ?? throw new InvalidDataException("A motion-value transition is absent.")
            );
        }

        private void Check() => owner.RequireLiveView();
    }

    internal enum BattlementDirectMotionCommandKind : byte
    {
        ValuePlayback,
        Playback,
        ControlledClockSet,
        ControlledClockAdvance,
        DragControl,
    }

    internal readonly struct BattlementDirectMotionCommand
    {
        internal BattlementDirectMotionCommand(
            BattlementDirectMotionCommandKind kind,
            ObjectId objectId,
            MotionPlaybackOperationKind playback = MotionPlaybackOperationKind.Play,
            ulong slot = 0,
            uint generation = 0,
            ulong micros = 0,
            double number = 0,
            MotionPlaybackDirection direction = MotionPlaybackDirection.Forward,
            int pointerId = 0,
            MotionPointerDevice device = MotionPointerDevice.Mouse,
            float x = 0,
            float y = 0,
            bool snapToCursor = false
        ) =>
            (
                Kind,
                ObjectId,
                Playback,
                Slot,
                Generation,
                Micros,
                Number,
                Direction,
                PointerId,
                Device,
                X,
                Y,
                SnapToCursor
            ) = (
                kind,
                objectId,
                playback,
                slot,
                generation,
                micros,
                number,
                direction,
                pointerId,
                device,
                x,
                y,
                snapToCursor
            );

        internal BattlementDirectMotionCommandKind Kind { get; }
        internal ObjectId ObjectId { get; }
        internal MotionPlaybackOperationKind Playback { get; }
        internal ulong Slot { get; }
        internal uint Generation { get; }
        internal ulong Micros { get; }
        internal double Number { get; }
        internal MotionPlaybackDirection Direction { get; }
        internal int PointerId { get; }
        internal MotionPointerDevice Device { get; }
        internal float X { get; }
        internal float Y { get; }
        internal bool SnapToCursor { get; }
    }

    internal enum BattlementDirectComponentCommandKind : byte
    {
        CameraSetEnabled,
        CameraSetPerspective,
        CameraTweenFieldOfView,
        CameraSetOrthographic,
        CameraTweenOrthographicSize,
        CameraSetClipping,
        CameraSetClear,
        LightSetEnabled,
        LightSetType,
        LightSetColor,
        LightTweenColor,
        LightSetIntensity,
        LightTweenIntensity,
        LightSetRange,
        LightSetSpotAngle,
        LightSetShadows,
        ImageSetTexture,
        ImageSetSize,
        ImageSetFit,
        ImageSetTint,
        ImageTweenTint,
        ImageSetOpacity,
        ImageTweenOpacity,
        ImageSetFaceCamera,
        TextSetFont,
        TextSetSize,
        TextTweenSize,
        TextSetColor,
        TextTweenColor,
        TextSetAlignment,
        TextSetWrapping,
        TextSetRichText,
        TextSetFaceCamera,
    }

    internal readonly struct BattlementDirectComponentCommand
    {
        internal BattlementDirectComponentCommand(
            BattlementDirectComponentCommandKind kind,
            ObjectId objectId,
            double first = 0,
            double second = 0,
            double third = 0,
            double fourth = 0,
            byte option = 0,
            bool enabled = false,
            bool hasValue = false,
            string? address = null,
            ConflictPolicy onConflict = ConflictPolicy.Cancel,
            BattlementDirectTweenSettings? tween = null
        ) =>
            (
                Kind,
                ObjectId,
                First,
                Second,
                Third,
                Fourth,
                Option,
                Enabled,
                HasValue,
                Address,
                OnConflict,
                Tween
            ) = (
                kind,
                objectId,
                first,
                second,
                third,
                fourth,
                option,
                enabled,
                hasValue,
                address,
                onConflict,
                tween
            );

        internal BattlementDirectComponentCommandKind Kind { get; }
        internal ObjectId ObjectId { get; }
        internal double First { get; }
        internal double Second { get; }
        internal double Third { get; }
        internal double Fourth { get; }
        internal byte Option { get; }
        internal bool Enabled { get; }
        internal bool HasValue { get; }
        internal string? Address { get; }
        internal ConflictPolicy OnConflict { get; }
        internal BattlementDirectTweenSettings? Tween { get; }
    }

    internal enum BattlementDirectAnimatorCommandKind : byte
    {
        Play,
        CrossFade,
        SetBool,
        SetInt,
        SetFloat,
        SetTrigger,
        SetSpeed,
    }

    internal readonly struct BattlementDirectAnimatorCommand
    {
        internal BattlementDirectAnimatorCommand(
            BattlementDirectAnimatorCommandKind kind,
            ObjectId objectId,
            string value = "",
            uint layer = 0,
            double number = 0,
            long integer = 0,
            bool enabled = false,
            ulong waitMilliseconds = 0,
            ulong crossFadeMilliseconds = 0
        ) =>
            (
                Kind,
                ObjectId,
                Value,
                Layer,
                Number,
                Integer,
                Enabled,
                WaitMilliseconds,
                CrossFadeMilliseconds
            ) = (
                kind,
                objectId,
                value,
                layer,
                number,
                integer,
                enabled,
                waitMilliseconds,
                crossFadeMilliseconds
            );

        internal BattlementDirectAnimatorCommandKind Kind { get; }
        internal ObjectId ObjectId { get; }
        internal string Value { get; }
        internal uint Layer { get; }
        internal double Number { get; }
        internal long Integer { get; }
        internal bool Enabled { get; }
        internal ulong WaitMilliseconds { get; }
        internal ulong CrossFadeMilliseconds { get; }
    }

    internal readonly struct BattlementDirectDiagnostics
    {
        internal BattlementDirectDiagnostics(string key, string? value) =>
            (Key, Value) = (key, value);

        internal string Key { get; }
        internal string? Value { get; }
    }

    internal readonly struct BattlementDirectAssetSet
    {
        private readonly IBattlementFlatBufferViewOwner owner;
        private readonly Wire.ReplaceAssetSetPayload value;

        internal BattlementDirectAssetSet(
            IBattlementFlatBufferViewOwner owner,
            Wire.ReplaceAssetSetPayload value
        ) => (this.owner, this.value) = (owner, value);

        internal int Count
        {
            get
            {
                owner.RequireLiveView();
                return value.AssetsLength;
            }
        }

        internal PreparedAsset Read(int index)
        {
            owner.RequireLiveView();
            return BattlementDirectCommandReader.ReadPreparedAsset(
                value.Assets(index) ?? throw new InvalidDataException("A prepared asset is absent.")
            );
        }
    }

    internal readonly struct BattlementDirectGeometryCommand
    {
        private readonly IBattlementFlatBufferViewOwner owner;
        private readonly Wire.GeometryObservationUpdate value;

        internal BattlementDirectGeometryCommand(
            IBattlementFlatBufferViewOwner owner,
            Wire.GeometryObservationUpdate value
        ) => (this.owner, this.value) = (owner, value);

        internal int AddedCount
        {
            get
            {
                Check();
                return value.AddedLength;
            }
        }
        internal int RemovedCount
        {
            get
            {
                Check();
                return value.RemovedLength;
            }
        }

        internal GeometryObservation ReadAdded(int index)
        {
            Check();
            Wire.GeometryObservation observation =
                value.Added(index)
                ?? throw new InvalidDataException("A geometry observation is absent.");
            return new GeometryObservation(
                new GeometryObservationId(
                    BattlementFlatBufferCore.ReadUuid(
                        observation.ObservationId,
                        "geometry observation"
                    )
                ),
                ReadTarget(
                    observation.Target
                        ?? throw new InvalidDataException("A geometry target is absent.")
                )
            );
        }

        internal GeometryObservationId ReadRemoved(int index)
        {
            Check();
            return new GeometryObservationId(
                BattlementFlatBufferCore.ReadUuid(
                    value.Removed(index),
                    "removed geometry observation"
                )
            );
        }

        private GeometryObservationTarget ReadTarget(Wire.GeometryObservationTarget target) =>
            target.Kind switch
            {
                Wire.GeometryTargetKind.UiElement => new GeometryObservationTarget.UiElement(
                    Object(target.ObjectId, "geometry UI element")
                ),
                Wire.GeometryTargetKind.Viewport => new GeometryObservationTarget.Viewport(
                    new DisplayId(target.DisplayId)
                ),
                Wire.GeometryTargetKind.WorldOrigin => new GeometryObservationTarget.WorldOrigin(
                    Object(target.ObjectId, "geometry world object"),
                    Camera(target)
                ),
                Wire.GeometryTargetKind.WorldAnchor => new GeometryObservationTarget.WorldAnchor(
                    Object(target.ObjectId, "geometry world object"),
                    new AnchorName(target.Anchor),
                    Camera(target)
                ),
                Wire.GeometryTargetKind.WorldRenderedBounds =>
                    new GeometryObservationTarget.WorldRenderedBounds(
                        Object(target.ObjectId, "geometry world object"),
                        Camera(target)
                    ),
                _ => throw new InvalidDataException("A geometry target kind is unknown."),
            };

        private static CameraTarget Camera(Wire.GeometryObservationTarget target) =>
            target.CameraKind switch
            {
                Wire.CameraTargetKind.Input => new CameraTarget.Input(),
                Wire.CameraTargetKind.Object => new CameraTarget.Object(
                    Object(target.CameraObjectId, "geometry camera")
                ),
                _ => throw new InvalidDataException("A geometry camera kind is unknown."),
            };

        private static ObjectId Object(Wire.Uuid? value, string name) =>
            new(BattlementFlatBufferCore.ReadUuid(value, name));

        private void Check() => owner.RequireLiveView();
    }

    internal readonly struct BattlementDirectAccessibilityCommand
        : IBattlementAccessibilityUpdateView
    {
        private readonly IBattlementFlatBufferViewOwner owner;
        private readonly Wire.AccessibilityUpdate value;

        internal BattlementDirectAccessibilityCommand(
            IBattlementFlatBufferViewOwner owner,
            Wire.AccessibilityUpdate value
        ) => (this.owner, this.value) = (owner, value);

        public bool HasSnapshot
        {
            get
            {
                Check();
                return value.Snapshot.HasValue;
            }
        }
        public ulong CommitSequence
        {
            get
            {
                Check();
                return Snapshot().CommitSequence;
            }
        }
        public int RootCount
        {
            get
            {
                Check();
                return Snapshot().RootsLength;
            }
        }
        public int NodeCount
        {
            get
            {
                Check();
                return Snapshot().NodesLength;
            }
        }
        public int AnnouncementCount
        {
            get
            {
                Check();
                return value.AnnouncementsLength;
            }
        }

        public ObjectId ReadRoot(int index)
        {
            Check();
            return Object(Snapshot().Roots(index), "accessibility root");
        }

        public string ReadAnnouncement(int index)
        {
            Check();
            return value.Announcements(index)
                ?? throw new InvalidDataException("An accessibility announcement is absent.");
        }

        public AccessibilityNodeSnapshot ReadNode(int index)
        {
            Check();
            Wire.AccessibilityNodeSnapshot node =
                Snapshot().Nodes(index)
                ?? throw new InvalidDataException("An accessibility node is absent.");
            var children = new ObjectId[node.ChildrenLength];
            for (int child = 0; child < children.Length; child++)
                children[child] = Object(node.Children(child), "accessibility child");
            Wire.SemanticState state =
                node.State ?? throw new InvalidDataException("An accessibility state is absent.");
            Wire.AccessibilityActionSet actions =
                node.Actions ?? throw new InvalidDataException("Accessibility actions are absent.");
            var scroll = new AccessibilityScrollDirection[actions.ScrollLength];
            for (int item = 0; item < scroll.Length; item++)
                scroll[item] = (AccessibilityScrollDirection)(byte)actions.Scroll(item);
            return new AccessibilityNodeSnapshot(
                Object(node.ObjectId, "accessibility node"),
                node.ParentId.HasValue ? Object(node.ParentId, "accessibility parent") : null,
                children,
                (SemanticRole)(byte)node.Role,
                node.Label,
                node.Hint,
                new SemanticState(
                    state.Disabled,
                    state.Checked.HasValue ? (CheckedState?)(byte)state.Checked.Value : null,
                    state.Selected,
                    state.Expanded,
                    state.Busy,
                    state.Current.HasValue ? (CurrentPage?)(byte)state.Current.Value : null,
                    state.Popup.HasValue ? (PopupKind?)(byte)state.Popup.Value : null
                ),
                node.Value.HasValue ? Range(node.Value.Value) : null,
                new AccessibilityActionSet(
                    actions.Activate,
                    actions.Increment,
                    actions.Decrement,
                    actions.Dismiss,
                    scroll
                ),
                node.HeadingLevel,
                node.ScrollAxis.HasValue
                    ? (AccessibilityScrollAxis?)(byte)node.ScrollAxis.Value
                    : null
            );
        }

        private Wire.AccessibilitySnapshot Snapshot() =>
            value.Snapshot
            ?? throw new InvalidDataException("An accessibility snapshot is absent.");

        private static AccessibilityRangeValue Range(Wire.AccessibilityRangeValue range) =>
            new(range.Current, range.Minimum, range.Maximum, range.Text);

        private static ObjectId Object(Wire.Uuid? value, string name) =>
            new(BattlementFlatBufferCore.ReadUuid(value, name));

        private void Check() => owner.RequireLiveView();
    }

    internal readonly struct BattlementDirectLocalPosition
    {
        internal BattlementDirectLocalPosition(
            ObjectId objectId,
            double x,
            double y,
            double z,
            ConflictPolicy onConflict
        ) => (ObjectId, X, Y, Z, OnConflict) = (objectId, x, y, z, onConflict);

        internal ObjectId ObjectId { get; }
        internal double X { get; }
        internal double Y { get; }
        internal double Z { get; }
        internal ConflictPolicy OnConflict { get; }
    }

    internal readonly struct BattlementDirectLabelUpdate
    {
        private readonly IBattlementFlatBufferViewOwner owner;
        private readonly Wire.UiProperty property;

        internal BattlementDirectLabelUpdate(
            IBattlementFlatBufferViewOwner owner,
            ObjectId objectId,
            Wire.UiProperty property
        ) => (this.owner, ObjectId, this.property) = (owner, objectId, property);

        internal ObjectId ObjectId { get; }

        internal string ReadText()
        {
            owner.RequireLiveView();
            return property.ValueAsTextPropertyValue().Value;
        }
    }

    internal readonly struct BattlementDirectWorldPosition
    {
        internal BattlementDirectWorldPosition(
            ObjectId objectId,
            double x,
            double y,
            double z,
            ConflictPolicy onConflict
        ) => (ObjectId, X, Y, Z, OnConflict) = (objectId, x, y, z, onConflict);

        internal ObjectId ObjectId { get; }
        internal double X { get; }
        internal double Y { get; }
        internal double Z { get; }
        internal ConflictPolicy OnConflict { get; }
    }

    internal readonly struct BattlementDirectTextContent
    {
        private readonly IBattlementFlatBufferViewOwner owner;
        private readonly Wire.TextContentPayload value;

        internal BattlementDirectTextContent(
            IBattlementFlatBufferViewOwner owner,
            ObjectId objectId,
            Wire.TextContentPayload value
        ) => (this.owner, ObjectId, this.value) = (owner, objectId, value);

        internal ObjectId ObjectId { get; }

        internal string ReadContent()
        {
            owner.RequireLiveView();
            return value.Content;
        }
    }

    internal readonly struct BattlementDirectSetMaterial
    {
        private readonly IBattlementFlatBufferViewOwner owner;
        private readonly Wire.SetMaterialPayload value;

        internal BattlementDirectSetMaterial(
            IBattlementFlatBufferViewOwner owner,
            ObjectId objectId,
            Wire.SetMaterialPayload value,
            uint? slot,
            ConflictPolicy onConflict
        ) =>
            (this.owner, ObjectId, this.value, Slot, OnConflict) = (
                owner,
                objectId,
                value,
                slot,
                onConflict
            );

        internal ObjectId ObjectId { get; }

        internal string ReadAddress()
        {
            owner.RequireLiveView();
            return value.Address;
        }

        internal uint? Slot { get; }
        internal ConflictPolicy OnConflict { get; }
    }

    internal readonly struct BattlementDirectDestroyObject
    {
        internal BattlementDirectDestroyObject(ObjectId objectId) => ObjectId = objectId;

        internal ObjectId ObjectId { get; }
    }

    internal readonly struct BattlementDirectInputEnabled
    {
        internal BattlementDirectInputEnabled(bool enabled) => Enabled = enabled;

        internal bool Enabled { get; }
    }

    internal readonly struct BattlementDirectObjectActive
    {
        internal BattlementDirectObjectActive(ObjectId objectId, bool active) =>
            (ObjectId, Active) = (objectId, active);

        internal ObjectId ObjectId { get; }
        internal bool Active { get; }
    }

    internal readonly struct BattlementDirectObjectReparent
    {
        internal BattlementDirectObjectReparent(
            ObjectId objectId,
            ObjectId? parentId,
            bool worldPositionStays
        ) => (ObjectId, ParentId, WorldPositionStays) = (objectId, parentId, worldPositionStays);

        internal ObjectId ObjectId { get; }
        internal ObjectId? ParentId { get; }
        internal bool WorldPositionStays { get; }
    }

    internal readonly struct BattlementDirectParticleSpawn
    {
        internal BattlementDirectParticleSpawn(
            string address,
            ObjectId? objectId,
            double x,
            double y,
            double z,
            ulong lifetimeMilliseconds
        ) =>
            (Address, ObjectId, X, Y, Z, LifetimeMilliseconds) = (
                address,
                objectId,
                x,
                y,
                z,
                lifetimeMilliseconds
            );

        internal string Address { get; }
        internal ObjectId? ObjectId { get; }
        internal double X { get; }
        internal double Y { get; }
        internal double Z { get; }
        internal ulong LifetimeMilliseconds { get; }
    }

    internal readonly struct BattlementDirectAudioPlay
    {
        internal BattlementDirectAudioPlay(
            string address,
            double volume,
            double pitch,
            bool loop,
            ulong fadeInMilliseconds
        ) =>
            (Address, Volume, Pitch, Loop, FadeInMilliseconds) = (
                address,
                volume,
                pitch,
                loop,
                fadeInMilliseconds
            );

        internal string Address { get; }
        internal double Volume { get; }
        internal double Pitch { get; }
        internal bool Loop { get; }
        internal ulong FadeInMilliseconds { get; }
    }

    internal readonly struct BattlementDirectParticlePlay
    {
        internal BattlementDirectParticlePlay(ObjectId objectId, bool restart) =>
            (ObjectId, Restart) = (objectId, restart);

        internal ObjectId ObjectId { get; }
        internal bool Restart { get; }
    }

    internal readonly struct BattlementDirectAudioStop
    {
        internal BattlementDirectAudioStop(CommandId audioCommandId, ulong fadeOutMilliseconds) =>
            (AudioCommandId, FadeOutMilliseconds) = (audioCommandId, fadeOutMilliseconds);

        internal CommandId AudioCommandId { get; }
        internal ulong FadeOutMilliseconds { get; }
    }

    internal readonly struct BattlementDirectAudioVolume
    {
        internal BattlementDirectAudioVolume(
            CommandId audioCommandId,
            double volume,
            ConflictPolicy onConflict
        ) => (AudioCommandId, Volume, OnConflict) = (audioCommandId, volume, onConflict);

        internal CommandId AudioCommandId { get; }
        internal double Volume { get; }
        internal ConflictPolicy OnConflict { get; }
    }

    internal readonly struct BattlementDirectWait
    {
        internal BattlementDirectWait(ulong durationMilliseconds) =>
            DurationMilliseconds = durationMilliseconds;

        internal ulong DurationMilliseconds { get; }
    }

    internal readonly struct BattlementDirectVibration
    {
        internal BattlementDirectVibration(
            double lowFrequency,
            double highFrequency,
            ulong durationMilliseconds
        ) =>
            (LowFrequency, HighFrequency, DurationMilliseconds) = (
                lowFrequency,
                highFrequency,
                durationMilliseconds
            );

        internal double LowFrequency { get; }
        internal double HighFrequency { get; }
        internal ulong DurationMilliseconds { get; }
    }

    internal readonly struct BattlementDirectDebugUi
    {
        internal BattlementDirectDebugUi(byte surface, bool visible) =>
            (Surface, Visible) = (surface, visible);

        internal byte Surface { get; }
        internal bool Visible { get; }
    }

    internal enum BattlementDirectAudioControlKind : byte
    {
        Pause,
        Resume,
        Seek,
        SetBuffering,
        Replace,
    }

    internal readonly struct BattlementDirectAudioControl
    {
        internal BattlementDirectAudioControl(
            BattlementDirectAudioControlKind kind,
            CommandId audioCommandId,
            ulong positionMilliseconds = 0,
            bool buffering = false,
            string? address = null
        ) =>
            (Kind, AudioCommandId, PositionMilliseconds, Buffering, Address) = (
                kind,
                audioCommandId,
                positionMilliseconds,
                buffering,
                address
            );

        internal BattlementDirectAudioControlKind Kind { get; }
        internal CommandId AudioCommandId { get; }
        internal ulong PositionMilliseconds { get; }
        internal bool Buffering { get; }
        internal string? Address { get; }
    }

    internal readonly struct BattlementDirectTweenAudioVolume
    {
        internal BattlementDirectTweenAudioVolume(
            CommandId audioCommandId,
            double volume,
            BattlementDirectTweenSettings tween
        ) => (AudioCommandId, Volume, Tween) = (audioCommandId, volume, tween);

        internal CommandId AudioCommandId { get; }
        internal double Volume { get; }
        internal BattlementDirectTweenSettings Tween { get; }
        internal ConflictPolicy OnConflict => Tween.OnConflict;
    }

    internal enum BattlementDirectSceneCommandKind : byte
    {
        Load,
        Unload,
        SetPrimary,
    }

    internal readonly struct BattlementDirectSceneCommand
    {
        internal BattlementDirectSceneCommand(
            BattlementDirectSceneCommandKind kind,
            SceneId sceneId,
            string? address = null,
            bool makePrimary = false
        ) => (Kind, SceneId, Address, MakePrimary) = (kind, sceneId, address, makePrimary);

        internal BattlementDirectSceneCommandKind Kind { get; }
        internal SceneId SceneId { get; }
        internal string? Address { get; }
        internal bool MakePrimary { get; }
    }

    internal readonly struct BattlementDirectCancel
    {
        internal BattlementDirectCancel(CommandId commandId) => CommandId = commandId;

        internal CommandId CommandId { get; }
    }

    internal enum BattlementDirectInputConfigurationKind : byte
    {
        Camera,
        PointerEvents,
        GlobalKeys,
        Controller,
    }

    internal readonly struct BattlementDirectInputConfiguration
    {
        private readonly IBattlementFlatBufferViewOwner owner;
        private readonly Wire.PointerEventsPayload? pointerEvents;
        private readonly Wire.GlobalKeysPayload? globalKeys;
        private readonly Wire.ControllerInputSettings? controller;

        internal BattlementDirectInputConfiguration(
            IBattlementFlatBufferViewOwner owner,
            BattlementDirectInputConfigurationKind kind,
            ObjectId? objectId = null,
            Wire.PointerEventsPayload? pointerEvents = null,
            Wire.GlobalKeysPayload? globalKeys = null,
            Wire.ControllerInputSettings? controller = null
        ) =>
            (this.owner, Kind, ObjectId, this.pointerEvents, this.globalKeys, this.controller) = (
                owner,
                kind,
                objectId,
                pointerEvents,
                globalKeys,
                controller
            );

        internal BattlementDirectInputConfigurationKind Kind { get; }
        internal ObjectId? ObjectId { get; }

        internal PointerEvent[] ReadPointerEvents()
        {
            owner.RequireLiveView();
            Wire.PointerEventsPayload value =
                pointerEvents
                ?? throw new InvalidDataException("Pointer-event settings are absent.");
            var result = new PointerEvent[value.EventsLength];
            for (int index = 0; index < result.Length; index++)
                result[index] = (PointerEvent)(byte)value.Events(index);
            return result;
        }

        internal PhysicalKey[] ReadGlobalKeys()
        {
            owner.RequireLiveView();
            Wire.GlobalKeysPayload value =
                globalKeys ?? throw new InvalidDataException("Global-key settings are absent.");
            var result = new PhysicalKey[value.KeysLength];
            for (int index = 0; index < result.Length; index++)
                result[index] = (PhysicalKey)(ushort)value.Keys(index);
            return result;
        }

        internal ControllerInputSettings ReadController()
        {
            owner.RequireLiveView();
            return BattlementDirectCommandReader.ReadController(
                controller
                    ?? throw new InvalidDataException("Controller input settings are absent.")
            );
        }
    }

    internal readonly struct BattlementDirectParticleStop
    {
        internal BattlementDirectParticleStop(ObjectId objectId, bool clear) =>
            (ObjectId, Clear) = (objectId, clear);

        internal ObjectId ObjectId { get; }
        internal bool Clear { get; }
    }

    internal readonly struct BattlementDirectOpenUrl
    {
        internal BattlementDirectOpenUrl(string url) => Url = url;

        internal string Url { get; }
    }

    internal readonly struct BattlementDirectRotation
    {
        internal BattlementDirectRotation(
            ObjectId objectId,
            double x,
            double y,
            double z,
            double w,
            bool world,
            ConflictPolicy onConflict
        ) => (ObjectId, X, Y, Z, W, World, OnConflict) = (objectId, x, y, z, w, world, onConflict);

        internal ObjectId ObjectId { get; }
        internal double X { get; }
        internal double Y { get; }
        internal double Z { get; }
        internal double W { get; }
        internal bool World { get; }
        internal ConflictPolicy OnConflict { get; }
    }

    internal readonly struct BattlementDirectScale
    {
        internal BattlementDirectScale(
            ObjectId objectId,
            double x,
            double y,
            double z,
            ConflictPolicy onConflict
        ) => (ObjectId, X, Y, Z, OnConflict) = (objectId, x, y, z, onConflict);

        internal ObjectId ObjectId { get; }
        internal double X { get; }
        internal double Y { get; }
        internal double Z { get; }
        internal ConflictPolicy OnConflict { get; }
    }

    internal readonly struct BattlementDirectTweenRotation
    {
        internal BattlementDirectTweenRotation(
            ObjectId objectId,
            double x,
            double y,
            double z,
            double w,
            bool world,
            ConflictPolicy onConflict,
            BattlementDirectTweenSettings tween
        ) =>
            (ObjectId, X, Y, Z, W, World, OnConflict, Tween) = (
                objectId,
                x,
                y,
                z,
                w,
                world,
                onConflict,
                tween
            );

        internal ObjectId ObjectId { get; }
        internal double X { get; }
        internal double Y { get; }
        internal double Z { get; }
        internal double W { get; }
        internal bool World { get; }
        internal ConflictPolicy OnConflict { get; }
        internal BattlementDirectTweenSettings Tween { get; }
    }

    internal readonly struct BattlementDirectTweenScale
    {
        internal BattlementDirectTweenScale(
            ObjectId objectId,
            double x,
            double y,
            double z,
            ConflictPolicy onConflict,
            BattlementDirectTweenSettings tween
        ) => (ObjectId, X, Y, Z, OnConflict, Tween) = (objectId, x, y, z, onConflict, tween);

        internal ObjectId ObjectId { get; }
        internal double X { get; }
        internal double Y { get; }
        internal double Z { get; }
        internal ConflictPolicy OnConflict { get; }
        internal BattlementDirectTweenSettings Tween { get; }
    }

    internal readonly struct BattlementDirectTweenSettings
    {
        internal BattlementDirectTweenSettings(
            ulong delayMilliseconds,
            ulong durationMilliseconds,
            Easing easing,
            byte repeatKind,
            uint repeatCount,
            RepeatMode repeatMode,
            ConflictPolicy onConflict
        ) =>
            (
                DelayMilliseconds,
                DurationMilliseconds,
                Easing,
                RepeatKind,
                RepeatCount,
                RepeatMode,
                OnConflict
            ) = (
                delayMilliseconds,
                durationMilliseconds,
                easing,
                repeatKind,
                repeatCount,
                repeatMode,
                onConflict
            );

        internal ulong DelayMilliseconds { get; }
        internal ulong DurationMilliseconds { get; }
        internal Easing Easing { get; }
        internal byte RepeatKind { get; }
        internal uint RepeatCount { get; }
        internal RepeatMode RepeatMode { get; }
        internal ConflictPolicy OnConflict { get; }
    }

    internal readonly struct BattlementDirectTweenLocalPosition
    {
        internal BattlementDirectTweenLocalPosition(
            ObjectId objectId,
            double x,
            double y,
            double z,
            bool world,
            BattlementDirectTweenSettings tween
        ) => (ObjectId, X, Y, Z, World, Tween) = (objectId, x, y, z, world, tween);

        internal ObjectId ObjectId { get; }
        internal double X { get; }
        internal double Y { get; }
        internal double Z { get; }
        internal bool World { get; }
        internal BattlementDirectTweenSettings Tween { get; }
        internal ConflictPolicy OnConflict => Tween.OnConflict;
    }

    internal readonly struct BattlementDirectObjectPlacement
    {
        internal BattlementDirectObjectPlacement(
            ObjectId objectId,
            byte parentSceneKind,
            SceneId? sceneId,
            ObjectId? parentId,
            bool active,
            double positionX,
            double positionY,
            double positionZ,
            double rotationX,
            double rotationY,
            double rotationZ,
            double rotationW,
            double scaleX,
            double scaleY,
            double scaleZ,
            PointerEvent[] pointerEvents,
            DragMode? dragMode
        ) =>
            (
                ObjectId,
                ParentSceneKind,
                SceneId,
                ParentId,
                Active,
                PositionX,
                PositionY,
                PositionZ,
                RotationX,
                RotationY,
                RotationZ,
                RotationW,
                ScaleX,
                ScaleY,
                ScaleZ,
                PointerEvents,
                DragMode
            ) = (
                objectId,
                parentSceneKind,
                sceneId,
                parentId,
                active,
                positionX,
                positionY,
                positionZ,
                rotationX,
                rotationY,
                rotationZ,
                rotationW,
                scaleX,
                scaleY,
                scaleZ,
                pointerEvents,
                dragMode
            );

        internal ObjectId ObjectId { get; }
        internal byte ParentSceneKind { get; }
        internal SceneId? SceneId { get; }
        internal ObjectId? ParentId { get; }
        internal bool Active { get; }
        internal double PositionX { get; }
        internal double PositionY { get; }
        internal double PositionZ { get; }
        internal double RotationX { get; }
        internal double RotationY { get; }
        internal double RotationZ { get; }
        internal double RotationW { get; }
        internal double ScaleX { get; }
        internal double ScaleY { get; }
        internal double ScaleZ { get; }
        internal PointerEvent[] PointerEvents { get; }
        internal DragMode? DragMode { get; }
    }

    internal readonly struct BattlementDirectImageObjectCreate
    {
        internal BattlementDirectImageObjectCreate(
            BattlementDirectObjectPlacement placement,
            string texture,
            double width,
            double height,
            ImageFit fit,
            double red,
            double green,
            double blue,
            double opacity,
            bool facesCamera
        ) =>
            (Placement, Texture, Width, Height, Fit, Red, Green, Blue, Opacity, FacesCamera) = (
                placement,
                texture,
                width,
                height,
                fit,
                red,
                green,
                blue,
                opacity,
                facesCamera
            );

        internal BattlementDirectObjectPlacement Placement { get; }
        internal string Texture { get; }
        internal double Width { get; }
        internal double Height { get; }
        internal ImageFit Fit { get; }
        internal double Red { get; }
        internal double Green { get; }
        internal double Blue { get; }
        internal double Opacity { get; }
        internal bool FacesCamera { get; }
    }

    internal readonly struct BattlementDirectMaterialAssignment
    {
        internal BattlementDirectMaterialAssignment(uint slot, string address) =>
            (Slot, Address) = (slot, address);

        internal uint Slot { get; }
        internal string Address { get; }
    }

    internal readonly struct BattlementDirectPrimitiveObjectCreate
    {
        internal BattlementDirectPrimitiveObjectCreate(
            BattlementDirectObjectPlacement placement,
            Wire.GameObjectKind kind,
            BattlementDirectMaterialAssignment[] materials
        ) => (Placement, Kind, Materials) = (placement, kind, materials);

        internal BattlementDirectObjectPlacement Placement { get; }
        internal Wire.GameObjectKind Kind { get; }
        internal BattlementDirectMaterialAssignment[] Materials { get; }
    }

    internal readonly struct BattlementDirectPrefabObjectCreate
    {
        internal BattlementDirectPrefabObjectCreate(
            BattlementDirectObjectPlacement placement,
            string address,
            BattlementDirectMaterialAssignment[] materials,
            BattlementDirectAnimatorState? animator
        ) => (Placement, Address, Materials, Animator) = (placement, address, materials, animator);

        internal BattlementDirectObjectPlacement Placement { get; }
        internal string Address { get; }
        internal BattlementDirectMaterialAssignment[] Materials { get; }
        internal BattlementDirectAnimatorState? Animator { get; }
    }

    internal readonly struct BattlementDirectAnimatorState
    {
        internal BattlementDirectAnimatorState(
            string state,
            uint layer,
            double normalizedStartTime,
            BattlementDirectAnimatorBool[] boolParameters,
            BattlementDirectAnimatorInt[] intParameters,
            BattlementDirectAnimatorFloat[] floatParameters,
            double speed
        ) =>
            (
                State,
                Layer,
                NormalizedStartTime,
                BoolParameters,
                IntParameters,
                FloatParameters,
                Speed
            ) = (
                state,
                layer,
                normalizedStartTime,
                boolParameters,
                intParameters,
                floatParameters,
                speed
            );

        internal string State { get; }
        internal uint Layer { get; }
        internal double NormalizedStartTime { get; }
        internal BattlementDirectAnimatorBool[] BoolParameters { get; }
        internal BattlementDirectAnimatorInt[] IntParameters { get; }
        internal BattlementDirectAnimatorFloat[] FloatParameters { get; }
        internal double Speed { get; }
    }

    internal readonly struct BattlementDirectAnimatorBool
    {
        internal BattlementDirectAnimatorBool(string name, bool value) =>
            (Name, Value) = (name, value);

        internal string Name { get; }
        internal bool Value { get; }
    }

    internal readonly struct BattlementDirectAnimatorInt
    {
        internal BattlementDirectAnimatorInt(string name, int value) =>
            (Name, Value) = (name, value);

        internal string Name { get; }
        internal int Value { get; }
    }

    internal readonly struct BattlementDirectAnimatorFloat
    {
        internal BattlementDirectAnimatorFloat(string name, double value) =>
            (Name, Value) = (name, value);

        internal string Name { get; }
        internal double Value { get; }
    }

    internal readonly struct BattlementDirectEmptyObjectCreate
    {
        internal BattlementDirectEmptyObjectCreate(BattlementDirectObjectPlacement placement) =>
            Placement = placement;

        internal BattlementDirectObjectPlacement Placement { get; }
    }

    internal readonly struct BattlementDirectTextObjectCreate
    {
        internal BattlementDirectTextObjectCreate(
            BattlementDirectObjectPlacement placement,
            string text,
            string font,
            double size,
            double red,
            double green,
            double blue,
            double alpha,
            byte horizontal,
            byte vertical,
            double? wrapWidth,
            bool richText,
            bool facesCamera
        ) =>
            (
                Placement,
                Text,
                Font,
                Size,
                Red,
                Green,
                Blue,
                Alpha,
                Horizontal,
                Vertical,
                WrapWidth,
                RichText,
                FacesCamera
            ) = (
                placement,
                text,
                font,
                size,
                red,
                green,
                blue,
                alpha,
                horizontal,
                vertical,
                wrapWidth,
                richText,
                facesCamera
            );

        internal BattlementDirectObjectPlacement Placement { get; }
        internal string Text { get; }
        internal string Font { get; }
        internal double Size { get; }
        internal double Red { get; }
        internal double Green { get; }
        internal double Blue { get; }
        internal double Alpha { get; }
        internal byte Horizontal { get; }
        internal byte Vertical { get; }
        internal double? WrapWidth { get; }
        internal bool RichText { get; }
        internal bool FacesCamera { get; }
    }

    internal readonly struct BattlementDirectCameraObjectCreate
    {
        internal BattlementDirectCameraObjectCreate(
            BattlementDirectObjectPlacement placement,
            bool enabled,
            byte projection,
            double fieldOfView,
            double orthographicSize,
            double near,
            double far,
            byte clearMode,
            double red,
            double green,
            double blue,
            double alpha
        ) =>
            (
                Placement,
                Enabled,
                Projection,
                FieldOfView,
                OrthographicSize,
                Near,
                Far,
                ClearMode,
                Red,
                Green,
                Blue,
                Alpha
            ) = (
                placement,
                enabled,
                projection,
                fieldOfView,
                orthographicSize,
                near,
                far,
                clearMode,
                red,
                green,
                blue,
                alpha
            );

        internal BattlementDirectObjectPlacement Placement { get; }
        internal bool Enabled { get; }
        internal byte Projection { get; }
        internal double FieldOfView { get; }
        internal double OrthographicSize { get; }
        internal double Near { get; }
        internal double Far { get; }
        internal byte ClearMode { get; }
        internal double Red { get; }
        internal double Green { get; }
        internal double Blue { get; }
        internal double Alpha { get; }
    }

    internal readonly struct BattlementDirectLightObjectCreate
    {
        internal BattlementDirectLightObjectCreate(
            BattlementDirectObjectPlacement placement,
            bool enabled,
            byte lightType,
            double red,
            double green,
            double blue,
            double alpha,
            double intensity,
            double range,
            double outerSpotAngle,
            double innerSpotAngle,
            byte shadows
        ) =>
            (
                Placement,
                Enabled,
                LightType,
                Red,
                Green,
                Blue,
                Alpha,
                Intensity,
                Range,
                OuterSpotAngle,
                InnerSpotAngle,
                Shadows
            ) = (
                placement,
                enabled,
                lightType,
                red,
                green,
                blue,
                alpha,
                intensity,
                range,
                outerSpotAngle,
                innerSpotAngle,
                shadows
            );

        internal BattlementDirectObjectPlacement Placement { get; }
        internal bool Enabled { get; }
        internal byte LightType { get; }
        internal double Red { get; }
        internal double Green { get; }
        internal double Blue { get; }
        internal double Alpha { get; }
        internal double Intensity { get; }
        internal double Range { get; }
        internal double OuterSpotAngle { get; }
        internal double InnerSpotAngle { get; }
        internal byte Shadows { get; }
    }

    /// <summary>
    /// A snapshot object reduced to the runtime fields Unity must persist. The description is
    /// one of the flattened object-create structs above, never an unpacked protocol record.
    /// </summary>
    internal sealed class BattlementDirectSnapshotObject
    {
        internal BattlementDirectSnapshotObject(object description)
        {
            Description = description;
            Placement = description switch
            {
                BattlementDirectImageObjectCreate value => value.Placement,
                BattlementDirectPrimitiveObjectCreate value => value.Placement,
                BattlementDirectPrefabObjectCreate value => value.Placement,
                BattlementDirectEmptyObjectCreate value => value.Placement,
                BattlementDirectTextObjectCreate value => value.Placement,
                BattlementDirectCameraObjectCreate value => value.Placement,
                BattlementDirectLightObjectCreate value => value.Placement,
                BattlementDirectUiDocumentObjectCreate value => value.Placement,
                _ => throw new InvalidDataException(
                    "A direct snapshot object kind is unsupported."
                ),
            };
        }

        internal object Description { get; }
        internal BattlementDirectObjectPlacement Placement { get; }
    }

    internal readonly struct BattlementDirectUiDocumentObjectCreate
    {
        internal BattlementDirectUiDocumentObjectCreate(
            BattlementDirectObjectPlacement placement,
            GameObjectKind.UiDocumentState state
        ) => (Placement, State) = (placement, state);

        internal BattlementDirectObjectPlacement Placement { get; }
        internal GameObjectKind.UiDocumentState State { get; }
    }

    internal static class BattlementDirectCommandReader
    {
        internal static BattlementDirectSnapshotObject ReadSnapshotObject(Wire.GameObject value)
        {
            BattlementDirectObjectPlacement placement = ReadPlacement(value);
            object description = value.Kind switch
            {
                Wire.GameObjectKind.Empty => new BattlementDirectEmptyObjectCreate(placement),
                Wire.GameObjectKind.Image => ReadImage(value, placement),
                Wire.GameObjectKind.Text => ReadText(value, placement),
                Wire.GameObjectKind.Camera => ReadCamera(value, placement),
                Wire.GameObjectKind.Light => ReadLight(value, placement),
                Wire.GameObjectKind.UiDocument => new BattlementDirectUiDocumentObjectCreate(
                    placement,
                    BattlementFlatBufferRetainedCopy.UiDocumentState(
                        value.ContentAsUiDocumentObject()
                    )
                ),
                Wire.GameObjectKind.Prefab => ReadPrefab(value, placement),
                _ when IsPrimitive(value.Kind) => new BattlementDirectPrimitiveObjectCreate(
                    placement,
                    value.Kind,
                    ReadMaterials(value.ContentAsPrimitiveObject())
                ),
                _ => throw new InvalidDataException(
                    $"Snapshot object kind {value.Kind} requires a dedicated direct reader."
                ),
            };
            return new BattlementDirectSnapshotObject(description);
        }

        private static BattlementDirectImageObjectCreate ReadImage(
            Wire.GameObject value,
            BattlementDirectObjectPlacement placement
        )
        {
            Wire.ImageObject image = value.ContentAsImageObject();
            Wire.RgbColor tint = image.Tint!.Value;
            return new BattlementDirectImageObjectCreate(
                placement,
                image.Texture,
                image.Width,
                image.Height,
                (ImageFit)(byte)image.Fit,
                tint.R,
                tint.G,
                tint.B,
                image.Opacity,
                image.FaceCamera
            );
        }

        private static BattlementDirectTextObjectCreate ReadText(
            Wire.GameObject value,
            BattlementDirectObjectPlacement placement
        )
        {
            Wire.TextObject text = value.ContentAsTextObject();
            Wire.RgbaColor color = text.Color!.Value;
            return new BattlementDirectTextObjectCreate(
                placement,
                text.Text,
                text.Font,
                text.Size,
                color.R,
                color.G,
                color.B,
                color.A,
                (byte)text.Horizontal,
                (byte)text.Vertical,
                text.WrapWidth,
                text.RichText,
                text.FaceCamera
            );
        }

        private static BattlementDirectCameraObjectCreate ReadCamera(
            Wire.GameObject value,
            BattlementDirectObjectPlacement placement
        )
        {
            Wire.CameraObject camera = value.ContentAsCameraObject();
            Wire.RgbaColor color = camera.ClearColor!.Value;
            return new BattlementDirectCameraObjectCreate(
                placement,
                camera.Enabled,
                (byte)camera.Projection,
                camera.FieldOfView,
                camera.OrthographicSize,
                camera.Near,
                camera.Far,
                (byte)camera.ClearMode,
                color.R,
                color.G,
                color.B,
                color.A
            );
        }

        private static BattlementDirectLightObjectCreate ReadLight(
            Wire.GameObject value,
            BattlementDirectObjectPlacement placement
        )
        {
            Wire.LightObject light = value.ContentAsLightObject();
            Wire.RgbaColor color = light.Color!.Value;
            return new BattlementDirectLightObjectCreate(
                placement,
                light.Enabled,
                (byte)light.LightType,
                color.R,
                color.G,
                color.B,
                color.A,
                light.Intensity,
                light.Range,
                light.OuterSpotAngle,
                light.InnerSpotAngle,
                (byte)light.Shadows
            );
        }

        private static BattlementDirectPrefabObjectCreate ReadPrefab(
            Wire.GameObject value,
            BattlementDirectObjectPlacement placement
        )
        {
            Wire.PrefabObject prefab = value.ContentAsPrefabObject();
            return new BattlementDirectPrefabObjectCreate(
                placement,
                prefab.Address,
                ReadMaterials(prefab),
                prefab.Animator.HasValue ? ReadAnimator(prefab.Animator.Value) : null
            );
        }

        internal static bool TryRead(
            Wire.CoreCommand command,
            CommandId commandId,
            IBattlementFlatBufferViewOwner owner,
            out BattlementCommandExecution execution
        )
        {
            if (command.Kind == Wire.CoreCommandKind.VisualElementCreate)
            {
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectUiCreate(
                        owner,
                        command.PayloadAsVisualElementCreatePayload()
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.AssetsReplaceSet)
            {
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectAssetSet(owner, command.PayloadAsReplaceAssetSetPayload())
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.VisualElementDestroy)
            {
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new ObjectId(
                        BattlementFlatBufferCore.ReadUuid(
                            command.PayloadAsVisualElementDestroyPayload().ObjectId,
                            "visual destroy object"
                        )
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.VisualElementPerformAction)
            {
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    ReadVisualElementAction(command.PayloadAsVisualElementActionPayload())
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.VisualElementUpdate)
            {
                Wire.VisualElementUpdatePayload update =
                    command.PayloadAsVisualElementUpdatePayload();
                if (update.Kind != Wire.VisualElementUpdateKind.Properties)
                {
                    ObjectId objectId = ComponentObject(update.ObjectId, "visual update object");
                    execution = new BattlementCommandExecution(
                        commandId,
                        command.Blocking,
                        new BattlementDirectVisualElementPlacement(
                            objectId,
                            update.Kind == Wire.VisualElementUpdateKind.Parent
                                ? ComponentObject(update.ParentId, "visual update parent")
                                : null,
                            update.ChildIndex,
                            update.Kind == Wire.VisualElementUpdateKind.Parent
                        )
                    );
                    return true;
                }
            }
            if (command.Kind == Wire.CoreCommandKind.GeometryObservationUpdate)
            {
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectGeometryCommand(
                        owner,
                        command.PayloadAsGeometryObservationUpdate()
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.AccessibilityUpdate)
            {
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectAccessibilityCommand(
                        owner,
                        command.PayloadAsAccessibilityUpdate()
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.Diagnostics)
            {
                Wire.DiagnosticsPayload payload = command.PayloadAsDiagnosticsPayload();
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectDiagnostics(payload.Key, payload.Value)
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.MotionValue)
            {
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectMotionValueCommand(
                        owner,
                        command.PayloadAsMotionValueOperation()
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.MotionControl)
            {
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectMotionControlCommand(
                        owner,
                        command.PayloadAsMotionControlOperation()
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.MotionScope)
            {
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectMotionScopeCommand(
                        owner,
                        command.PayloadAsMotionScopeOperation()
                    )
                );
                return true;
            }
            if (TryReadMotion(command, commandId, out execution))
                return true;
            if (TryReadAnimator(command, commandId, out execution))
                return true;
            if (TryReadComponent(command, commandId, out execution))
                return true;
            if (command.Kind == Wire.CoreCommandKind.ApplicationOpenUrl)
            {
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectOpenUrl(command.PayloadAsExternalUrlPayload().Url)
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.SceneLoad)
            {
                Wire.SceneLoadPayload payload = command.PayloadAsSceneLoadPayload();
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectSceneCommand(
                        BattlementDirectSceneCommandKind.Load,
                        new SceneId(BattlementFlatBufferCore.ReadUuid(payload.SceneId, "scene")),
                        payload.Address,
                        payload.MakePrimary
                    )
                );
                return true;
            }
            if (
                command.Kind == Wire.CoreCommandKind.SceneUnload
                || command.Kind == Wire.CoreCommandKind.SceneSetPrimary
            )
            {
                Wire.SceneIdPayload payload = command.PayloadAsSceneIdPayload();
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectSceneCommand(
                        command.Kind == Wire.CoreCommandKind.SceneUnload
                            ? BattlementDirectSceneCommandKind.Unload
                            : BattlementDirectSceneCommandKind.SetPrimary,
                        new SceneId(BattlementFlatBufferCore.ReadUuid(payload.SceneId, "scene"))
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.TransformSetLocalPosition)
            {
                Wire.PositionPayload payload = command.PayloadAsPositionPayload();
                Wire.Vector3d position = payload.Position!.Value;
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectLocalPosition(
                        new ObjectId(
                            BattlementFlatBufferCore.ReadUuid(payload.ObjectId, "position object")
                        ),
                        position.X,
                        position.Y,
                        position.Z,
                        (ConflictPolicy)(byte)payload.OnConflict
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.TransformSetWorldPosition)
            {
                Wire.PositionPayload payload = command.PayloadAsPositionPayload();
                Wire.Vector3d position = payload.Position!.Value;
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectWorldPosition(
                        new ObjectId(
                            BattlementFlatBufferCore.ReadUuid(payload.ObjectId, "position object")
                        ),
                        position.X,
                        position.Y,
                        position.Z,
                        (ConflictPolicy)(byte)payload.OnConflict
                    )
                );
                return true;
            }
            if (
                command.Kind == Wire.CoreCommandKind.TransformTweenLocalPosition
                || command.Kind == Wire.CoreCommandKind.TransformTweenWorldPosition
            )
            {
                Wire.TweenPositionPayload payload = command.PayloadAsTweenPositionPayload();
                Wire.Vector3d position = payload.Position!.Value;
                Wire.Tween tween = payload.Tween!.Value;
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectTweenLocalPosition(
                        new ObjectId(
                            BattlementFlatBufferCore.ReadUuid(payload.ObjectId, "position object")
                        ),
                        position.X,
                        position.Y,
                        position.Z,
                        command.Kind == Wire.CoreCommandKind.TransformTweenWorldPosition,
                        new BattlementDirectTweenSettings(
                            tween.DelayMs,
                            tween.DurationMs,
                            (Easing)(byte)tween.Easing,
                            (byte)tween.RepeatKind,
                            tween.RepeatCount,
                            (RepeatMode)(byte)tween.RepeatMode,
                            (ConflictPolicy)(byte)payload.OnConflict
                        )
                    )
                );
                return true;
            }
            if (
                command.Kind == Wire.CoreCommandKind.TransformTweenLocalRotation
                || command.Kind == Wire.CoreCommandKind.TransformTweenWorldRotation
            )
            {
                Wire.TweenRotationPayload payload = command.PayloadAsTweenRotationPayload();
                Wire.Quaterniond rotation = payload.Rotation!.Value;
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectTweenRotation(
                        new ObjectId(
                            BattlementFlatBufferCore.ReadUuid(payload.ObjectId, "rotation object")
                        ),
                        rotation.X,
                        rotation.Y,
                        rotation.Z,
                        rotation.W,
                        command.Kind == Wire.CoreCommandKind.TransformTweenWorldRotation,
                        (ConflictPolicy)(byte)payload.OnConflict,
                        TweenSettings(payload.Tween!.Value, payload.OnConflict)
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.TransformTweenLocalScale)
            {
                Wire.TweenScalePayload payload = command.PayloadAsTweenScalePayload();
                Wire.Vector3d scale = payload.Scale!.Value;
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectTweenScale(
                        new ObjectId(
                            BattlementFlatBufferCore.ReadUuid(payload.ObjectId, "scale object")
                        ),
                        scale.X,
                        scale.Y,
                        scale.Z,
                        (ConflictPolicy)(byte)payload.OnConflict,
                        TweenSettings(payload.Tween!.Value, payload.OnConflict)
                    )
                );
                return true;
            }
            if (
                command.Kind == Wire.CoreCommandKind.TransformSetLocalRotation
                || command.Kind == Wire.CoreCommandKind.TransformSetWorldRotation
            )
            {
                Wire.RotationPayload payload = command.PayloadAsRotationPayload();
                Wire.Quaterniond rotation = payload.Rotation!.Value;
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectRotation(
                        new ObjectId(
                            BattlementFlatBufferCore.ReadUuid(payload.ObjectId, "rotation object")
                        ),
                        rotation.X,
                        rotation.Y,
                        rotation.Z,
                        rotation.W,
                        command.Kind == Wire.CoreCommandKind.TransformSetWorldRotation,
                        (ConflictPolicy)(byte)payload.OnConflict
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.TransformSetLocalScale)
            {
                Wire.ScalePayload payload = command.PayloadAsScalePayload();
                Wire.Vector3d scale = payload.Scale!.Value;
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectScale(
                        new ObjectId(
                            BattlementFlatBufferCore.ReadUuid(payload.ObjectId, "scale object")
                        ),
                        scale.X,
                        scale.Y,
                        scale.Z,
                        (ConflictPolicy)(byte)payload.OnConflict
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.TextSetContent)
            {
                Wire.TextContentPayload payload = command.PayloadAsTextContentPayload();
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectTextContent(
                        owner,
                        new ObjectId(
                            BattlementFlatBufferCore.ReadUuid(payload.ObjectId, "text object")
                        ),
                        payload
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.RendererSetMaterial)
            {
                Wire.SetMaterialPayload payload = command.PayloadAsSetMaterialPayload();
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectSetMaterial(
                        owner,
                        new ObjectId(
                            BattlementFlatBufferCore.ReadUuid(payload.ObjectId, "renderer object")
                        ),
                        payload,
                        payload.Slot,
                        (ConflictPolicy)(byte)payload.OnConflict
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.ObjectDestroy)
            {
                Wire.ObjectIdPayload payload = command.PayloadAsObjectIdPayload();
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectDestroyObject(
                        new ObjectId(
                            BattlementFlatBufferCore.ReadUuid(payload.ObjectId, "destroyed object")
                        )
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.ObjectSetActive)
            {
                Wire.ObjectSetActivePayload payload = command.PayloadAsObjectSetActivePayload();
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectObjectActive(
                        new ObjectId(
                            BattlementFlatBufferCore.ReadUuid(payload.ObjectId, "active object")
                        ),
                        payload.Active
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.ObjectReparent)
            {
                Wire.ObjectReparentPayload payload = command.PayloadAsObjectReparentPayload();
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectObjectReparent(
                        new ObjectId(
                            BattlementFlatBufferCore.ReadUuid(payload.ObjectId, "reparented object")
                        ),
                        payload.ParentId.HasValue
                            ? new ObjectId(
                                BattlementFlatBufferCore.ReadUuid(
                                    payload.ParentId,
                                    "new object parent"
                                )
                            )
                            : null,
                        payload.WorldPositionStays
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.InputSetEnabled)
            {
                Wire.SetInputEnabledPayload payload = command.PayloadAsSetInputEnabledPayload();
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectInputEnabled(payload.Enabled)
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.ParticleSpawn)
            {
                Wire.ParticleSpawnPayload payload = command.PayloadAsParticleSpawnPayload();
                ObjectId? target =
                    payload.LocationKind == Wire.ParticleSpawnLocationKind.GameObject
                        ? new ObjectId(
                            BattlementFlatBufferCore.ReadUuid(payload.ObjectId, "particle target")
                        )
                        : null;
                Wire.Vector3d? position = payload.WorldPosition;
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectParticleSpawn(
                        payload.Address,
                        target,
                        position?.X ?? 0,
                        position?.Y ?? 0,
                        position?.Z ?? 0,
                        payload.LifetimeMs
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.ParticlePlay)
            {
                Wire.ParticlePlayPayload payload = command.PayloadAsParticlePlayPayload();
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectParticlePlay(
                        new ObjectId(
                            BattlementFlatBufferCore.ReadUuid(payload.ObjectId, "particle object")
                        ),
                        payload.Restart
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.ParticleStop)
            {
                Wire.ParticleStopPayload payload = command.PayloadAsParticleStopPayload();
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectParticleStop(
                        new ObjectId(
                            BattlementFlatBufferCore.ReadUuid(payload.ObjectId, "particle object")
                        ),
                        payload.Clear
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.AudioPlay)
            {
                Wire.AudioPlayPayload payload = command.PayloadAsAudioPlayPayload();
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectAudioPlay(
                        payload.Address,
                        payload.Volume,
                        payload.Pitch,
                        payload.Loop,
                        payload.FadeInMs
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.AudioStop)
            {
                Wire.AudioStopPayload payload = command.PayloadAsAudioStopPayload();
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectAudioStop(
                        new CommandId(
                            BattlementFlatBufferCore.ReadUuid(
                                payload.AudioCommandId,
                                "audio command"
                            )
                        ),
                        payload.FadeOutMs
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.AudioSetVolume)
            {
                Wire.AudioVolumePayload payload = command.PayloadAsAudioVolumePayload();
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectAudioVolume(
                        new CommandId(
                            BattlementFlatBufferCore.ReadUuid(
                                payload.AudioCommandId,
                                "audio command"
                            )
                        ),
                        payload.Volume,
                        (ConflictPolicy)(byte)payload.OnConflict
                    )
                );
                return true;
            }
            if (
                command.Kind == Wire.CoreCommandKind.AudioPause
                || command.Kind == Wire.CoreCommandKind.AudioResume
            )
            {
                Wire.AudioPlaybackPayload payload = command.PayloadAsAudioPlaybackPayload();
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectAudioControl(
                        command.Kind == Wire.CoreCommandKind.AudioPause
                            ? BattlementDirectAudioControlKind.Pause
                            : BattlementDirectAudioControlKind.Resume,
                        new CommandId(
                            BattlementFlatBufferCore.ReadUuid(
                                payload.AudioCommandId,
                                "audio command"
                            )
                        )
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.AudioSeek)
            {
                Wire.AudioSeekPayload payload = command.PayloadAsAudioSeekPayload();
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectAudioControl(
                        BattlementDirectAudioControlKind.Seek,
                        new CommandId(
                            BattlementFlatBufferCore.ReadUuid(
                                payload.AudioCommandId,
                                "audio command"
                            )
                        ),
                        positionMilliseconds: payload.PositionMs
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.AudioSetBuffering)
            {
                Wire.AudioBufferingPayload payload = command.PayloadAsAudioBufferingPayload();
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectAudioControl(
                        BattlementDirectAudioControlKind.SetBuffering,
                        new CommandId(
                            BattlementFlatBufferCore.ReadUuid(
                                payload.AudioCommandId,
                                "audio command"
                            )
                        ),
                        buffering: payload.Buffering
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.AudioReplace)
            {
                Wire.AudioReplacePayload payload = command.PayloadAsAudioReplacePayload();
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectAudioControl(
                        BattlementDirectAudioControlKind.Replace,
                        new CommandId(
                            BattlementFlatBufferCore.ReadUuid(
                                payload.AudioCommandId,
                                "audio command"
                            )
                        ),
                        address: payload.Address
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.AudioTweenVolume)
            {
                Wire.TweenAudioVolumePayload payload = command.PayloadAsTweenAudioVolumePayload();
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectTweenAudioVolume(
                        new CommandId(
                            BattlementFlatBufferCore.ReadUuid(
                                payload.AudioCommandId,
                                "audio command"
                            )
                        ),
                        payload.Volume,
                        TweenSettings(payload.Tween!.Value, payload.OnConflict)
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.TimeWait)
            {
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectWait(command.PayloadAsWaitPayload().DurationMs)
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.OperationCancel)
            {
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectCancel(
                        new CommandId(
                            BattlementFlatBufferCore.ReadUuid(
                                command.PayloadAsCancelOperationPayload().CommandId,
                                "canceled command"
                            )
                        )
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.InputSetCamera)
            {
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectInputConfiguration(
                        owner,
                        BattlementDirectInputConfigurationKind.Camera,
                        objectId: new ObjectId(
                            BattlementFlatBufferCore.ReadUuid(
                                command.PayloadAsObjectIdPayload().ObjectId,
                                "input camera"
                            )
                        )
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.InputSetPointerEvents)
            {
                Wire.PointerEventsPayload payload = command.PayloadAsPointerEventsPayload();
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectInputConfiguration(
                        owner,
                        BattlementDirectInputConfigurationKind.PointerEvents,
                        objectId: new ObjectId(
                            BattlementFlatBufferCore.ReadUuid(payload.ObjectId, "input object")
                        ),
                        pointerEvents: payload
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.InputSetGlobalKeys)
            {
                Wire.GlobalKeysPayload payload = command.PayloadAsGlobalKeysPayload();
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectInputConfiguration(
                        owner,
                        BattlementDirectInputConfigurationKind.GlobalKeys,
                        globalKeys: payload
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.InputSetController)
            {
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectInputConfiguration(
                        owner,
                        BattlementDirectInputConfigurationKind.Controller,
                        controller: command.PayloadAsControllerInputSettings()
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.ControllerVibrate)
            {
                Wire.ControllerVibrationPayload payload =
                    command.PayloadAsControllerVibrationPayload();
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectVibration(
                        payload.LowFrequency,
                        payload.HighFrequency,
                        payload.DurationMs
                    )
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.DebugUi)
            {
                Wire.DebugUiPayload payload = command.PayloadAsDebugUiPayload();
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectDebugUi((byte)payload.Surface, payload.Visible)
                );
                return true;
            }
            if (command.Kind == Wire.CoreCommandKind.ObjectCreate)
            {
                Wire.GameObject value = command.PayloadAsObjectCreatePayload().Object!.Value;
                BattlementDirectObjectPlacement placement = ReadPlacement(value);
                if (value.Kind == Wire.GameObjectKind.Empty)
                {
                    execution = new BattlementCommandExecution(
                        commandId,
                        command.Blocking,
                        new BattlementDirectEmptyObjectCreate(placement)
                    );
                    return true;
                }
                if (value.Kind == Wire.GameObjectKind.Image)
                {
                    Wire.ImageObject image = value.ContentAsImageObject();
                    Wire.RgbColor tint = image.Tint!.Value;
                    execution = new BattlementCommandExecution(
                        commandId,
                        command.Blocking,
                        new BattlementDirectImageObjectCreate(
                            placement,
                            image.Texture,
                            image.Width,
                            image.Height,
                            (ImageFit)(byte)image.Fit,
                            tint.R,
                            tint.G,
                            tint.B,
                            image.Opacity,
                            image.FaceCamera
                        )
                    );
                    return true;
                }
                if (value.Kind == Wire.GameObjectKind.Text)
                {
                    Wire.TextObject text = value.ContentAsTextObject();
                    Wire.RgbaColor color = text.Color!.Value;
                    execution = new BattlementCommandExecution(
                        commandId,
                        command.Blocking,
                        new BattlementDirectTextObjectCreate(
                            placement,
                            text.Text,
                            text.Font,
                            text.Size,
                            color.R,
                            color.G,
                            color.B,
                            color.A,
                            (byte)text.Horizontal,
                            (byte)text.Vertical,
                            text.WrapWidth,
                            text.RichText,
                            text.FaceCamera
                        )
                    );
                    return true;
                }
                if (value.Kind == Wire.GameObjectKind.Camera)
                {
                    Wire.CameraObject camera = value.ContentAsCameraObject();
                    Wire.RgbaColor color = camera.ClearColor!.Value;
                    execution = new BattlementCommandExecution(
                        commandId,
                        command.Blocking,
                        new BattlementDirectCameraObjectCreate(
                            placement,
                            camera.Enabled,
                            (byte)camera.Projection,
                            camera.FieldOfView,
                            camera.OrthographicSize,
                            camera.Near,
                            camera.Far,
                            (byte)camera.ClearMode,
                            color.R,
                            color.G,
                            color.B,
                            color.A
                        )
                    );
                    return true;
                }
                if (value.Kind == Wire.GameObjectKind.Light)
                {
                    Wire.LightObject light = value.ContentAsLightObject();
                    Wire.RgbaColor color = light.Color!.Value;
                    execution = new BattlementCommandExecution(
                        commandId,
                        command.Blocking,
                        new BattlementDirectLightObjectCreate(
                            placement,
                            light.Enabled,
                            (byte)light.LightType,
                            color.R,
                            color.G,
                            color.B,
                            color.A,
                            light.Intensity,
                            light.Range,
                            light.OuterSpotAngle,
                            light.InnerSpotAngle,
                            (byte)light.Shadows
                        )
                    );
                    return true;
                }
                if (IsPrimitive(value.Kind))
                {
                    execution = new BattlementCommandExecution(
                        commandId,
                        command.Blocking,
                        new BattlementDirectPrimitiveObjectCreate(
                            placement,
                            value.Kind,
                            ReadMaterials(value.ContentAsPrimitiveObject())
                        )
                    );
                    return true;
                }
                if (value.Kind == Wire.GameObjectKind.Prefab)
                {
                    Wire.PrefabObject prefab = value.ContentAsPrefabObject();
                    execution = new BattlementCommandExecution(
                        commandId,
                        command.Blocking,
                        new BattlementDirectPrefabObjectCreate(
                            placement,
                            prefab.Address,
                            ReadMaterials(prefab),
                            prefab.Animator.HasValue ? ReadAnimator(prefab.Animator.Value) : null
                        )
                    );
                    return true;
                }
            }
            if (command.Kind == Wire.CoreCommandKind.VisualElementUpdate)
            {
                Wire.VisualElementUpdatePayload payload =
                    command.PayloadAsVisualElementUpdatePayload();
                Wire.UiElement? optionalElement = payload.Element;
                if (
                    payload.Kind == Wire.VisualElementUpdateKind.Properties
                    && optionalElement.HasValue
                    && optionalElement.Value.Kind == Wire.UiElementKind.Label
                    && optionalElement.Value.UsageHints == 0
                    && optionalElement.Value.PropertiesLength == 1
                    && optionalElement.Value.EventSubscriptionsLength == 0
                    && optionalElement.Value.PartStylesLength == 0
                )
                {
                    Wire.UiProperty property = optionalElement.Value.Properties(0)!.Value;
                    if (
                        property.Key == Wire.UiPropertyKey.Text
                        && property.State == Wire.PropState.Set
                        && property.ValueType == Wire.UiPropertyValue.TextPropertyValue
                    )
                    {
                        execution = new BattlementCommandExecution(
                            commandId,
                            command.Blocking,
                            new BattlementDirectLabelUpdate(
                                owner,
                                new ObjectId(
                                    BattlementFlatBufferCore.ReadUuid(
                                        payload.ObjectId,
                                        "label object"
                                    )
                                ),
                                property
                            )
                        );
                        return true;
                    }
                }
                if (
                    optionalElement.HasValue
                    && TryReadUiScalar(
                        owner,
                        payload,
                        optionalElement.Value,
                        out BattlementDirectUiScalar scalar
                    )
                )
                {
                    execution = new BattlementCommandExecution(commandId, command.Blocking, scalar);
                    return true;
                }
                execution = new BattlementCommandExecution(
                    commandId,
                    command.Blocking,
                    new BattlementDirectUiProperties(owner, payload)
                );
                return true;
            }
            execution = default;
            return false;
        }

        private static bool TryReadUiScalar(
            IBattlementFlatBufferViewOwner owner,
            Wire.VisualElementUpdatePayload payload,
            Wire.UiElement element,
            out BattlementDirectUiScalar scalar
        )
        {
            scalar = default;
            if (
                payload.Kind != Wire.VisualElementUpdateKind.Properties
                || element.UsageHints != 0
                || element.EventSubscriptionsLength != 0
                || element.PartStylesLength != 0
            )
                return false;
            ObjectId objectId = new(
                BattlementFlatBufferCore.ReadUuid(payload.ObjectId, "UI scalar object")
            );
            if (element.PropertiesLength == 1)
            {
                Wire.UiProperty property = element.Properties(0)!.Value;
                if (property.State != Wire.PropState.Set)
                    return false;
                switch (element.Kind)
                {
                    case Wire.UiElementKind.TextField
                        when Is(
                            property,
                            Wire.UiPropertyKey.Value,
                            Wire.UiPropertyValue.TextPropertyValue
                        ):
                        scalar = new(
                            owner,
                            element,
                            BattlementUiScalarUpdateKind.TextFieldValue,
                            objectId
                        );
                        return true;
                    case Wire.UiElementKind.Toggle
                        when Is(
                            property,
                            Wire.UiPropertyKey.Value,
                            Wire.UiPropertyValue.BoolPropertyValue
                        ):
                    case Wire.UiElementKind.RadioButton
                        when Is(
                            property,
                            Wire.UiPropertyKey.Value,
                            Wire.UiPropertyValue.BoolPropertyValue
                        ):
                        scalar = new(
                            owner,
                            element,
                            BattlementUiScalarUpdateKind.BooleanValue,
                            objectId,
                            boolean: property.ValueAsBoolPropertyValue().Value
                        );
                        return true;
                    case Wire.UiElementKind.RadioButtonGroup
                        when Is(
                            property,
                            Wire.UiPropertyKey.SelectedIndex,
                            Wire.UiPropertyValue.UIntPropertyValue
                        ):
                        scalar = new(
                            owner,
                            element,
                            BattlementUiScalarUpdateKind.RadioSelection,
                            objectId,
                            unsigned: property.ValueAsUIntPropertyValue().Value
                        );
                        return true;
                    case Wire.UiElementKind.ToggleButtonGroup
                        when Is(
                            property,
                            Wire.UiPropertyKey.SelectedIndices,
                            Wire.UiPropertyValue.UIntListPropertyValue
                        ):
                    {
                        scalar = new(
                            owner,
                            element,
                            BattlementUiScalarUpdateKind.ToggleSelection,
                            objectId
                        );
                        return true;
                    }
                    case Wire.UiElementKind.DropdownField
                        when Is(
                            property,
                            Wire.UiPropertyKey.Selection,
                            Wire.UiPropertyValue.ChoicePropertyValue
                        ):
                    {
                        Wire.ChoicePropertyValue choice = property.ValueAsChoicePropertyValue();
                        scalar = new(
                            owner,
                            element,
                            BattlementUiScalarUpdateKind.DropdownSelection,
                            objectId,
                            boolean: choice.Kind == Wire.ChoiceKind.Index,
                            unsigned: choice.Index
                        );
                        return true;
                    }
                    case Wire.UiElementKind.Scroller
                        when Is(
                            property,
                            Wire.UiPropertyKey.Value,
                            Wire.UiPropertyValue.FloatPropertyValue
                        ):
                        scalar = new(
                            owner,
                            element,
                            BattlementUiScalarUpdateKind.ScrollerValue,
                            objectId,
                            first: property.ValueAsFloatPropertyValue().Value
                        );
                        return true;
                    case Wire.UiElementKind.Slider
                        when Is(
                            property,
                            Wire.UiPropertyKey.Value,
                            Wire.UiPropertyValue.FloatPropertyValue
                        ):
                        scalar = new(
                            owner,
                            element,
                            BattlementUiScalarUpdateKind.SliderValue,
                            objectId,
                            first: property.ValueAsFloatPropertyValue().Value
                        );
                        return true;
                    case Wire.UiElementKind.SliderInt
                        when Is(
                            property,
                            Wire.UiPropertyKey.Value,
                            Wire.UiPropertyValue.IntPropertyValue
                        ):
                        scalar = new(
                            owner,
                            element,
                            BattlementUiScalarUpdateKind.SliderIntValue,
                            objectId,
                            integer: property.ValueAsIntPropertyValue().Value
                        );
                        return true;
                    case Wire.UiElementKind.TabView
                        when Is(
                            property,
                            Wire.UiPropertyKey.SelectedTabIndex,
                            Wire.UiPropertyValue.UIntPropertyValue
                        ):
                        scalar = new(
                            owner,
                            element,
                            BattlementUiScalarUpdateKind.TabSelection,
                            objectId,
                            unsigned: property.ValueAsUIntPropertyValue().Value
                        );
                        return true;
                    case Wire.UiElementKind.Button
                        when Is(
                            property,
                            Wire.UiPropertyKey.Text,
                            Wire.UiPropertyValue.TextPropertyValue
                        ):
                        scalar = new(
                            owner,
                            element,
                            BattlementUiScalarUpdateKind.ButtonText,
                            objectId
                        );
                        return true;
                    case Wire.UiElementKind.Button
                        when Is(
                            property,
                            Wire.UiPropertyKey.Enabled,
                            Wire.UiPropertyValue.BoolPropertyValue
                        ):
                        scalar = new(
                            owner,
                            element,
                            BattlementUiScalarUpdateKind.ButtonEnabled,
                            objectId,
                            boolean: property.ValueAsBoolPropertyValue().Value
                        );
                        return true;
                    case Wire.UiElementKind.VisualElement:
                        break;
                    case Wire.UiElementKind.Flex:
                        break;
                    case Wire.UiElementKind.Grid:
                        break;
                    case Wire.UiElementKind.Stack:
                        break;
                    case Wire.UiElementKind.Box:
                        break;
                    case Wire.UiElementKind.Label:
                        break;
                    case Wire.UiElementKind.TextElement:
                        break;
                    case Wire.UiElementKind.TextField:
                        break;
                    case Wire.UiElementKind.Toggle:
                        break;
                    case Wire.UiElementKind.RadioButton:
                        break;
                    case Wire.UiElementKind.RadioButtonGroup:
                        break;
                    case Wire.UiElementKind.ToggleButtonGroup:
                        break;
                    case Wire.UiElementKind.DropdownField:
                        break;
                    case Wire.UiElementKind.Button:
                        break;
                    case Wire.UiElementKind.RepeatButton:
                        break;
                    case Wire.UiElementKind.GroupBox:
                        break;
                    case Wire.UiElementKind.PopupWindow:
                        break;
                    case Wire.UiElementKind.ScrollView:
                        break;
                    case Wire.UiElementKind.Scroller:
                        break;
                    case Wire.UiElementKind.Slider:
                        break;
                    case Wire.UiElementKind.SliderInt:
                        break;
                    case Wire.UiElementKind.MinMaxSlider:
                        break;
                    case Wire.UiElementKind.ProgressBar:
                        break;
                    case Wire.UiElementKind.Tab:
                        break;
                    case Wire.UiElementKind.TabView:
                        break;
                    case Wire.UiElementKind.Image:
                        break;
                    default:
                        break;
                }
            }
            if (element.Kind == Wire.UiElementKind.MinMaxSlider && element.PropertiesLength == 2)
            {
                float? min = null;
                float? max = null;
                for (int index = 0; index < 2; index++)
                {
                    Wire.UiProperty property = element.Properties(index)!.Value;
                    if (
                        property.State != Wire.PropState.Set
                        || property.ValueType != Wire.UiPropertyValue.FloatPropertyValue
                    )
                        return false;
                    if (property.Key == Wire.UiPropertyKey.MinValue)
                        min = property.ValueAsFloatPropertyValue().Value;
                    else if (property.Key == Wire.UiPropertyKey.MaxValue)
                        max = property.ValueAsFloatPropertyValue().Value;
                    else
                        return false;
                }
                if (min.HasValue && max.HasValue)
                {
                    scalar = new(
                        owner,
                        element,
                        BattlementUiScalarUpdateKind.RangeValue,
                        objectId,
                        first: min.Value,
                        second: max.Value
                    );
                    return true;
                }
            }
            if (element.Kind == Wire.UiElementKind.Button && element.PropertiesLength == 2)
            {
                bool hasText = false;
                bool? enabled = null;
                for (int index = 0; index < 2; index++)
                {
                    Wire.UiProperty property = element.Properties(index)!.Value;
                    if (property.State != Wire.PropState.Set)
                        return false;
                    if (
                        Is(
                            property,
                            Wire.UiPropertyKey.Text,
                            Wire.UiPropertyValue.TextPropertyValue
                        )
                    )
                        hasText = true;
                    else if (
                        Is(
                            property,
                            Wire.UiPropertyKey.Enabled,
                            Wire.UiPropertyValue.BoolPropertyValue
                        )
                    )
                        enabled = property.ValueAsBoolPropertyValue().Value;
                    else
                        return false;
                }
                if (hasText && enabled.HasValue)
                {
                    scalar = new(
                        owner,
                        element,
                        BattlementUiScalarUpdateKind.ButtonTextAndEnabled,
                        objectId,
                        boolean: enabled.Value
                    );
                    return true;
                }
            }
            if (element.Kind == Wire.UiElementKind.RepeatButton && element.PropertiesLength == 2)
            {
                uint? delay = null;
                uint? interval = null;
                for (int index = 0; index < 2; index++)
                {
                    Wire.UiProperty property = element.Properties(index)!.Value;
                    if (
                        property.State != Wire.PropState.Set
                        || property.ValueType != Wire.UiPropertyValue.UIntPropertyValue
                    )
                        return false;
                    if (property.Key == Wire.UiPropertyKey.DelayMs)
                        delay = property.ValueAsUIntPropertyValue().Value;
                    else if (property.Key == Wire.UiPropertyKey.IntervalMs)
                        interval = property.ValueAsUIntPropertyValue().Value;
                    else
                        return false;
                }
                if (delay.HasValue && interval.HasValue && interval.Value > 0)
                {
                    scalar = new(
                        owner,
                        element,
                        BattlementUiScalarUpdateKind.RepeatTiming,
                        objectId,
                        unsigned: delay.Value,
                        unsignedSecond: interval.Value
                    );
                    return true;
                }
            }
            return false;
        }

        private static bool Is(
            Wire.UiProperty property,
            Wire.UiPropertyKey key,
            Wire.UiPropertyValue type
        ) => property.Key == key && property.ValueType == type;

        private static bool TryReadMotion(
            Wire.CoreCommand command,
            CommandId commandId,
            out BattlementCommandExecution execution
        )
        {
            switch (command.Kind)
            {
                case Wire.CoreCommandKind.MotionValuePlayback:
                {
                    Wire.MotionValuePlaybackOperation value =
                        command.PayloadAsMotionValuePlaybackOperation();
                    Wire.MotionPlaybackCommand playback = value.Command!.Value;
                    execution = new BattlementCommandExecution(
                        commandId,
                        command.Blocking,
                        ReadPlayback(
                            BattlementDirectMotionCommandKind.ValuePlayback,
                            ComponentObject(value.PlaybackId, "motion value playback"),
                            playback,
                            generation: value.Generation
                        )
                    );
                    return true;
                }
                case Wire.CoreCommandKind.MotionPlayback:
                {
                    Wire.MotionPlaybackOperation value = command.PayloadAsMotionPlaybackOperation();
                    Wire.MotionPlaybackCommand playback = value.Command!.Value;
                    execution = new BattlementCommandExecution(
                        commandId,
                        command.Blocking,
                        ReadPlayback(
                            BattlementDirectMotionCommandKind.Playback,
                            ComponentObject(value.DescriptorId, "motion descriptor"),
                            playback,
                            value.Slot,
                            value.Generation
                        )
                    );
                    return true;
                }
                case Wire.CoreCommandKind.MotionControlledClock:
                {
                    Wire.MotionControlledClockOperation value =
                        command.PayloadAsMotionControlledClockOperation();
                    var kind =
                        value.Command == Wire.MotionControlledClockCommandKind.Set
                            ? BattlementDirectMotionCommandKind.ControlledClockSet
                            : BattlementDirectMotionCommandKind.ControlledClockAdvance;
                    execution = new BattlementCommandExecution(
                        commandId,
                        command.Blocking,
                        new BattlementDirectMotionCommand(
                            kind,
                            ComponentObject(value.ClockId, "motion clock"),
                            micros: value.Micros
                        )
                    );
                    return true;
                }
                case Wire.CoreCommandKind.MotionDragControl:
                {
                    Wire.MotionDragControlOperation value =
                        command.PayloadAsMotionDragControlOperation();
                    Wire.MotionVector2 point = value.Point!.Value;
                    execution = new BattlementCommandExecution(
                        commandId,
                        command.Blocking,
                        new BattlementDirectMotionCommand(
                            BattlementDirectMotionCommandKind.DragControl,
                            ComponentObject(value.ControlId, "motion drag control"),
                            pointerId: value.PointerId,
                            device: (MotionPointerDevice)(byte)value.Device,
                            x: point.X,
                            y: point.Y,
                            snapToCursor: value.SnapToCursor
                        )
                    );
                    return true;
                }

                case Wire.CoreCommandKind.ApplicationOpenUrl:
                    break;
                case Wire.CoreCommandKind.Diagnostics:
                    break;
                case Wire.CoreCommandKind.AssetsReplaceSet:
                    break;
                case Wire.CoreCommandKind.SceneLoad:
                    break;
                case Wire.CoreCommandKind.SceneUnload:
                    break;
                case Wire.CoreCommandKind.SceneSetPrimary:
                    break;
                case Wire.CoreCommandKind.ObjectCreate:
                    break;
                case Wire.CoreCommandKind.ObjectDestroy:
                    break;
                case Wire.CoreCommandKind.ObjectSetActive:
                    break;
                case Wire.CoreCommandKind.ObjectReparent:
                    break;
                case Wire.CoreCommandKind.TransformSetLocalPosition:
                    break;
                case Wire.CoreCommandKind.TransformSetWorldPosition:
                    break;
                case Wire.CoreCommandKind.TransformTweenLocalPosition:
                    break;
                case Wire.CoreCommandKind.TransformTweenWorldPosition:
                    break;
                case Wire.CoreCommandKind.TransformSetLocalRotation:
                    break;
                case Wire.CoreCommandKind.TransformSetWorldRotation:
                    break;
                case Wire.CoreCommandKind.TransformTweenLocalRotation:
                    break;
                case Wire.CoreCommandKind.TransformTweenWorldRotation:
                    break;
                case Wire.CoreCommandKind.TransformSetLocalScale:
                    break;
                case Wire.CoreCommandKind.TransformTweenLocalScale:
                    break;
                case Wire.CoreCommandKind.RendererSetMaterial:
                    break;
                case Wire.CoreCommandKind.CameraSetEnabled:
                    break;
                case Wire.CoreCommandKind.CameraSetPerspective:
                    break;
                case Wire.CoreCommandKind.CameraTweenFieldOfView:
                    break;
                case Wire.CoreCommandKind.CameraSetOrthographic:
                    break;
                case Wire.CoreCommandKind.CameraTweenOrthographicSize:
                    break;
                case Wire.CoreCommandKind.CameraSetClipping:
                    break;
                case Wire.CoreCommandKind.CameraSetClear:
                    break;
                case Wire.CoreCommandKind.LightSetEnabled:
                    break;
                case Wire.CoreCommandKind.LightSetType:
                    break;
                case Wire.CoreCommandKind.LightSetColor:
                    break;
                case Wire.CoreCommandKind.LightTweenColor:
                    break;
                case Wire.CoreCommandKind.LightSetIntensity:
                    break;
                case Wire.CoreCommandKind.LightTweenIntensity:
                    break;
                case Wire.CoreCommandKind.LightSetRange:
                    break;
                case Wire.CoreCommandKind.LightSetSpotAngle:
                    break;
                case Wire.CoreCommandKind.LightSetShadows:
                    break;
                case Wire.CoreCommandKind.ImageSetTexture:
                    break;
                case Wire.CoreCommandKind.ImageSetSize:
                    break;
                case Wire.CoreCommandKind.ImageSetFit:
                    break;
                case Wire.CoreCommandKind.ImageSetTint:
                    break;
                case Wire.CoreCommandKind.ImageTweenTint:
                    break;
                case Wire.CoreCommandKind.ImageSetOpacity:
                    break;
                case Wire.CoreCommandKind.ImageTweenOpacity:
                    break;
                case Wire.CoreCommandKind.ImageSetFaceCamera:
                    break;
                case Wire.CoreCommandKind.TextSetContent:
                    break;
                case Wire.CoreCommandKind.TextSetFont:
                    break;
                case Wire.CoreCommandKind.TextSetSize:
                    break;
                case Wire.CoreCommandKind.TextTweenSize:
                    break;
                case Wire.CoreCommandKind.TextSetColor:
                    break;
                case Wire.CoreCommandKind.TextTweenColor:
                    break;
                case Wire.CoreCommandKind.TextSetAlignment:
                    break;
                case Wire.CoreCommandKind.TextSetWrapping:
                    break;
                case Wire.CoreCommandKind.TextSetRichText:
                    break;
                case Wire.CoreCommandKind.TextSetFaceCamera:
                    break;
                case Wire.CoreCommandKind.AnimatorPlay:
                    break;
                case Wire.CoreCommandKind.AnimatorCrossFade:
                    break;
                case Wire.CoreCommandKind.AnimatorSetBool:
                    break;
                case Wire.CoreCommandKind.AnimatorSetInt:
                    break;
                case Wire.CoreCommandKind.AnimatorSetFloat:
                    break;
                case Wire.CoreCommandKind.AnimatorSetTrigger:
                    break;
                case Wire.CoreCommandKind.AnimatorSetSpeed:
                    break;
                case Wire.CoreCommandKind.ParticlePlay:
                    break;
                case Wire.CoreCommandKind.ParticleStop:
                    break;
                case Wire.CoreCommandKind.ParticleSpawn:
                    break;
                case Wire.CoreCommandKind.AudioPlay:
                    break;
                case Wire.CoreCommandKind.AudioStop:
                    break;
                case Wire.CoreCommandKind.AudioPause:
                    break;
                case Wire.CoreCommandKind.AudioResume:
                    break;
                case Wire.CoreCommandKind.AudioSeek:
                    break;
                case Wire.CoreCommandKind.AudioSetBuffering:
                    break;
                case Wire.CoreCommandKind.AudioReplace:
                    break;
                case Wire.CoreCommandKind.AudioSetVolume:
                    break;
                case Wire.CoreCommandKind.AudioTweenVolume:
                    break;
                case Wire.CoreCommandKind.TimeWait:
                    break;
                case Wire.CoreCommandKind.OperationCancel:
                    break;
                case Wire.CoreCommandKind.InputSetEnabled:
                    break;
                case Wire.CoreCommandKind.InputSetCamera:
                    break;
                case Wire.CoreCommandKind.InputSetPointerEvents:
                    break;
                case Wire.CoreCommandKind.InputSetGlobalKeys:
                    break;
                case Wire.CoreCommandKind.InputSetController:
                    break;
                case Wire.CoreCommandKind.ControllerVibrate:
                    break;
                case Wire.CoreCommandKind.DebugUi:
                    break;
                case Wire.CoreCommandKind.VisualElementCreate:
                    break;
                case Wire.CoreCommandKind.VisualElementUpdate:
                    break;
                case Wire.CoreCommandKind.VisualElementDestroy:
                    break;
                case Wire.CoreCommandKind.VisualElementPerformAction:
                    break;
                case Wire.CoreCommandKind.MotionValue:
                    break;
                case Wire.CoreCommandKind.MotionControl:
                    break;
                case Wire.CoreCommandKind.MotionScope:
                    break;
                case Wire.CoreCommandKind.GeometryObservationUpdate:
                    break;
                case Wire.CoreCommandKind.AccessibilityUpdate:
                    break;
                default:
                    execution = default;
                    return false;
            }
            execution = default;
            return false;
        }

        private static BattlementDirectMotionCommand ReadPlayback(
            BattlementDirectMotionCommandKind kind,
            ObjectId objectId,
            Wire.MotionPlaybackCommand value,
            ulong slot = 0,
            uint generation = 0
        ) =>
            new(
                kind,
                objectId,
                (MotionPlaybackOperationKind)(byte)value.Kind,
                slot,
                generation,
                value.ElapsedMicros,
                value.Speed,
                (MotionPlaybackDirection)(byte)value.Direction
            );

        private static bool TryReadComponent(
            Wire.CoreCommand command,
            CommandId commandId,
            out BattlementCommandExecution execution
        )
        {
            BattlementDirectComponentCommand? direct = command.Kind switch
            {
                Wire.CoreCommandKind.CameraSetEnabled => Enabled(
                    command,
                    BattlementDirectComponentCommandKind.CameraSetEnabled
                ),
                Wire.CoreCommandKind.LightSetEnabled => Enabled(
                    command,
                    BattlementDirectComponentCommandKind.LightSetEnabled
                ),
                Wire.CoreCommandKind.ImageSetFaceCamera => Enabled(
                    command,
                    BattlementDirectComponentCommandKind.ImageSetFaceCamera
                ),
                Wire.CoreCommandKind.TextSetRichText => Enabled(
                    command,
                    BattlementDirectComponentCommandKind.TextSetRichText
                ),
                Wire.CoreCommandKind.TextSetFaceCamera => Enabled(
                    command,
                    BattlementDirectComponentCommandKind.TextSetFaceCamera
                ),
                Wire.CoreCommandKind.CameraSetPerspective => Perspective(command, false),
                Wire.CoreCommandKind.CameraTweenFieldOfView => Perspective(command, true),
                Wire.CoreCommandKind.CameraSetOrthographic => Orthographic(command, false),
                Wire.CoreCommandKind.CameraTweenOrthographicSize => Orthographic(command, true),
                Wire.CoreCommandKind.CameraSetClipping => CameraClipping(command),
                Wire.CoreCommandKind.CameraSetClear => CameraClear(command),
                Wire.CoreCommandKind.LightSetType => LightType(command),
                Wire.CoreCommandKind.LightSetColor => Color(
                    command,
                    BattlementDirectComponentCommandKind.LightSetColor,
                    false
                ),
                Wire.CoreCommandKind.LightTweenColor => Color(
                    command,
                    BattlementDirectComponentCommandKind.LightTweenColor,
                    true
                ),
                Wire.CoreCommandKind.TextSetColor => Color(
                    command,
                    BattlementDirectComponentCommandKind.TextSetColor,
                    false
                ),
                Wire.CoreCommandKind.TextTweenColor => Color(
                    command,
                    BattlementDirectComponentCommandKind.TextTweenColor,
                    true
                ),
                Wire.CoreCommandKind.LightSetIntensity => Scalar(
                    command,
                    BattlementDirectComponentCommandKind.LightSetIntensity,
                    false
                ),
                Wire.CoreCommandKind.LightTweenIntensity => Scalar(
                    command,
                    BattlementDirectComponentCommandKind.LightTweenIntensity,
                    true
                ),
                Wire.CoreCommandKind.LightSetRange => LightRange(command),
                Wire.CoreCommandKind.LightSetSpotAngle => SpotAngle(command),
                Wire.CoreCommandKind.LightSetShadows => LightShadows(command),
                Wire.CoreCommandKind.ImageSetTexture => Asset(
                    command,
                    BattlementDirectComponentCommandKind.ImageSetTexture
                ),
                Wire.CoreCommandKind.TextSetFont => Asset(
                    command,
                    BattlementDirectComponentCommandKind.TextSetFont
                ),
                Wire.CoreCommandKind.ImageSetSize => ImageSize(command),
                Wire.CoreCommandKind.ImageSetFit => ImageFit(command),
                Wire.CoreCommandKind.ImageSetTint => Tint(command, false),
                Wire.CoreCommandKind.ImageTweenTint => Tint(command, true),
                Wire.CoreCommandKind.ImageSetOpacity => Opacity(command, false),
                Wire.CoreCommandKind.ImageTweenOpacity => Opacity(command, true),
                Wire.CoreCommandKind.TextSetSize => TextSize(command, false),
                Wire.CoreCommandKind.TextTweenSize => TextSize(command, true),
                Wire.CoreCommandKind.TextSetAlignment => TextAlignment(command),
                Wire.CoreCommandKind.TextSetWrapping => TextWrapping(command),
                _ => null,
            };
            if (direct is not BattlementDirectComponentCommand value)
            {
                execution = default;
                return false;
            }
            execution = new BattlementCommandExecution(commandId, command.Blocking, value);
            return true;
        }

        private static bool TryReadAnimator(
            Wire.CoreCommand command,
            CommandId commandId,
            out BattlementCommandExecution execution
        )
        {
            BattlementDirectAnimatorCommand? direct = command.Kind switch
            {
                Wire.CoreCommandKind.AnimatorPlay => AnimatorPlay(command, false),
                Wire.CoreCommandKind.AnimatorCrossFade => AnimatorPlay(command, true),
                Wire.CoreCommandKind.AnimatorSetBool => AnimatorBool(command),
                Wire.CoreCommandKind.AnimatorSetInt => AnimatorInt(command),
                Wire.CoreCommandKind.AnimatorSetFloat => AnimatorFloat(command),
                Wire.CoreCommandKind.AnimatorSetTrigger => AnimatorTrigger(command),
                Wire.CoreCommandKind.AnimatorSetSpeed => AnimatorSpeed(command),
                _ => null,
            };
            if (direct is not BattlementDirectAnimatorCommand value)
            {
                execution = default;
                return false;
            }
            execution = new BattlementCommandExecution(commandId, command.Blocking, value);
            return true;
        }

        private static BattlementDirectAnimatorCommand AnimatorPlay(
            Wire.CoreCommand command,
            bool crossFade
        )
        {
            if (crossFade)
            {
                Wire.AnimatorCrossFadePayload value = command.PayloadAsAnimatorCrossFadePayload();
                return new(
                    BattlementDirectAnimatorCommandKind.CrossFade,
                    ComponentObject(value.ObjectId, "animator object"),
                    value.State,
                    value.Layer,
                    value.NormalizedStartTime,
                    waitMilliseconds: value.WaitMs,
                    crossFadeMilliseconds: value.CrossFadeMs
                );
            }
            Wire.AnimatorPlayPayload play = command.PayloadAsAnimatorPlayPayload();
            return new(
                BattlementDirectAnimatorCommandKind.Play,
                ComponentObject(play.ObjectId, "animator object"),
                play.State,
                play.Layer,
                play.NormalizedStartTime,
                waitMilliseconds: play.WaitMs
            );
        }

        private static BattlementDirectAnimatorCommand AnimatorBool(Wire.CoreCommand command)
        {
            Wire.AnimatorBoolPayload value = command.PayloadAsAnimatorBoolPayload();
            return new(
                BattlementDirectAnimatorCommandKind.SetBool,
                ComponentObject(value.ObjectId, "animator object"),
                value.Parameter,
                enabled: value.Value
            );
        }

        private static BattlementDirectAnimatorCommand AnimatorInt(Wire.CoreCommand command)
        {
            Wire.AnimatorIntPayload value = command.PayloadAsAnimatorIntPayload();
            return new(
                BattlementDirectAnimatorCommandKind.SetInt,
                ComponentObject(value.ObjectId, "animator object"),
                value.Parameter,
                integer: value.Value
            );
        }

        private static BattlementDirectAnimatorCommand AnimatorFloat(Wire.CoreCommand command)
        {
            Wire.AnimatorFloatPayload value = command.PayloadAsAnimatorFloatPayload();
            return new(
                BattlementDirectAnimatorCommandKind.SetFloat,
                ComponentObject(value.ObjectId, "animator object"),
                value.Parameter,
                number: value.Value
            );
        }

        private static BattlementDirectAnimatorCommand AnimatorTrigger(Wire.CoreCommand command)
        {
            Wire.AnimatorParameterPayload value = command.PayloadAsAnimatorParameterPayload();
            return new(
                BattlementDirectAnimatorCommandKind.SetTrigger,
                ComponentObject(value.ObjectId, "animator object"),
                value.Parameter
            );
        }

        private static BattlementDirectAnimatorCommand AnimatorSpeed(Wire.CoreCommand command)
        {
            Wire.AnimatorSpeedPayload value = command.PayloadAsAnimatorSpeedPayload();
            return new(
                BattlementDirectAnimatorCommandKind.SetSpeed,
                ComponentObject(value.ObjectId, "animator object"),
                number: value.Speed
            );
        }

        private static ObjectId ComponentObject(Wire.Uuid? value, string name) =>
            new(BattlementFlatBufferCore.ReadUuid(value, name));

        private static BattlementDirectComponentCommand Enabled(
            Wire.CoreCommand command,
            BattlementDirectComponentCommandKind kind
        )
        {
            Wire.ObjectEnabledPayload value = command.PayloadAsObjectEnabledPayload();
            return new(
                kind,
                ComponentObject(value.ObjectId, "component object"),
                enabled: value.Enabled
            );
        }

        private static BattlementDirectComponentCommand Perspective(
            Wire.CoreCommand command,
            bool tweened
        )
        {
            if (tweened)
            {
                Wire.TweenFieldOfViewPayload value = command.PayloadAsTweenFieldOfViewPayload();
                return new(
                    BattlementDirectComponentCommandKind.CameraTweenFieldOfView,
                    ComponentObject(value.ObjectId, "camera object"),
                    value.FieldOfView,
                    onConflict: (ConflictPolicy)(byte)value.OnConflict,
                    tween: TweenSettings(value.Tween!.Value, value.OnConflict)
                );
            }
            Wire.PerspectivePayload set = command.PayloadAsPerspectivePayload();
            return new(
                BattlementDirectComponentCommandKind.CameraSetPerspective,
                ComponentObject(set.ObjectId, "camera object"),
                set.FieldOfView,
                onConflict: (ConflictPolicy)(byte)set.OnConflict
            );
        }

        private static BattlementDirectComponentCommand Orthographic(
            Wire.CoreCommand command,
            bool tweened
        )
        {
            if (tweened)
            {
                Wire.TweenOrthographicSizePayload value =
                    command.PayloadAsTweenOrthographicSizePayload();
                return new(
                    BattlementDirectComponentCommandKind.CameraTweenOrthographicSize,
                    ComponentObject(value.ObjectId, "camera object"),
                    value.Size,
                    onConflict: (ConflictPolicy)(byte)value.OnConflict,
                    tween: TweenSettings(value.Tween!.Value, value.OnConflict)
                );
            }
            Wire.OrthographicPayload set = command.PayloadAsOrthographicPayload();
            return new(
                BattlementDirectComponentCommandKind.CameraSetOrthographic,
                ComponentObject(set.ObjectId, "camera object"),
                set.Size,
                onConflict: (ConflictPolicy)(byte)set.OnConflict
            );
        }

        private static BattlementDirectComponentCommand CameraClipping(Wire.CoreCommand command)
        {
            Wire.CameraClippingPayload value = command.PayloadAsCameraClippingPayload();
            return new(
                BattlementDirectComponentCommandKind.CameraSetClipping,
                ComponentObject(value.ObjectId, "camera object"),
                value.Near,
                value.Far
            );
        }

        private static BattlementDirectComponentCommand CameraClear(Wire.CoreCommand command)
        {
            Wire.CameraClearPayload value = command.PayloadAsCameraClearPayload();
            Wire.RgbaColor? color = value.ClearColor;
            return new(
                BattlementDirectComponentCommandKind.CameraSetClear,
                ComponentObject(value.ObjectId, "camera object"),
                color?.R ?? 0,
                color?.G ?? 0,
                color?.B ?? 0,
                color?.A ?? 0,
                (byte)value.ClearMode,
                hasValue: value.ClearColor.HasValue
            );
        }

        private static BattlementDirectComponentCommand LightType(Wire.CoreCommand command)
        {
            Wire.LightTypePayload value = command.PayloadAsLightTypePayload();
            return new(
                BattlementDirectComponentCommandKind.LightSetType,
                ComponentObject(value.ObjectId, "light object"),
                option: (byte)value.LightType
            );
        }

        private static BattlementDirectComponentCommand Color(
            Wire.CoreCommand command,
            BattlementDirectComponentCommandKind kind,
            bool tweened
        )
        {
            if (tweened)
            {
                Wire.TweenColorPayload value = command.PayloadAsTweenColorPayload();
                Wire.RgbaColor color = value.Color!.Value;
                return new(
                    kind,
                    ComponentObject(value.ObjectId, "color object"),
                    color.R,
                    color.G,
                    color.B,
                    color.A,
                    onConflict: (ConflictPolicy)(byte)value.OnConflict,
                    tween: TweenSettings(value.Tween!.Value, value.OnConflict)
                );
            }
            Wire.ColorPayload set = command.PayloadAsColorPayload();
            Wire.RgbaColor setColor = set.Color!.Value;
            return new(
                kind,
                ComponentObject(set.ObjectId, "color object"),
                setColor.R,
                setColor.G,
                setColor.B,
                setColor.A,
                onConflict: (ConflictPolicy)(byte)set.OnConflict
            );
        }

        private static BattlementDirectComponentCommand Scalar(
            Wire.CoreCommand command,
            BattlementDirectComponentCommandKind kind,
            bool tweened
        )
        {
            if (tweened)
            {
                Wire.TweenIntensityPayload value = command.PayloadAsTweenIntensityPayload();
                return new(
                    kind,
                    ComponentObject(value.ObjectId, "light object"),
                    value.Intensity,
                    onConflict: (ConflictPolicy)(byte)value.OnConflict,
                    tween: TweenSettings(value.Tween!.Value, value.OnConflict)
                );
            }
            Wire.IntensityPayload set = command.PayloadAsIntensityPayload();
            return new(
                kind,
                ComponentObject(set.ObjectId, "light object"),
                set.Intensity,
                onConflict: (ConflictPolicy)(byte)set.OnConflict
            );
        }

        private static BattlementDirectComponentCommand LightRange(Wire.CoreCommand command)
        {
            Wire.LightRangePayload value = command.PayloadAsLightRangePayload();
            return new(
                BattlementDirectComponentCommandKind.LightSetRange,
                ComponentObject(value.ObjectId, "light object"),
                value.Range
            );
        }

        private static BattlementDirectComponentCommand SpotAngle(Wire.CoreCommand command)
        {
            Wire.SpotAnglePayload value = command.PayloadAsSpotAnglePayload();
            return new(
                BattlementDirectComponentCommandKind.LightSetSpotAngle,
                ComponentObject(value.ObjectId, "light object"),
                value.OuterSpotAngle,
                value.InnerSpotAngle
            );
        }

        private static BattlementDirectComponentCommand LightShadows(Wire.CoreCommand command)
        {
            Wire.LightShadowsPayload value = command.PayloadAsLightShadowsPayload();
            return new(
                BattlementDirectComponentCommandKind.LightSetShadows,
                ComponentObject(value.ObjectId, "light object"),
                option: (byte)value.Shadows
            );
        }

        private static BattlementDirectComponentCommand Asset(
            Wire.CoreCommand command,
            BattlementDirectComponentCommandKind kind
        )
        {
            if (kind == BattlementDirectComponentCommandKind.ImageSetTexture)
            {
                Wire.SetTexturePayload value = command.PayloadAsSetTexturePayload();
                return new(
                    kind,
                    ComponentObject(value.ObjectId, "image object"),
                    address: value.Address
                );
            }
            Wire.SetFontPayload font = command.PayloadAsSetFontPayload();
            return new(kind, ComponentObject(font.ObjectId, "text object"), address: font.Address);
        }

        private static BattlementDirectComponentCommand ImageSize(Wire.CoreCommand command)
        {
            Wire.ImageSizePayload value = command.PayloadAsImageSizePayload();
            return new(
                BattlementDirectComponentCommandKind.ImageSetSize,
                ComponentObject(value.ObjectId, "image object"),
                value.Width,
                value.Height
            );
        }

        private static BattlementDirectComponentCommand ImageFit(Wire.CoreCommand command)
        {
            Wire.ImageFitPayload value = command.PayloadAsImageFitPayload();
            return new(
                BattlementDirectComponentCommandKind.ImageSetFit,
                ComponentObject(value.ObjectId, "image object"),
                option: (byte)value.Fit
            );
        }

        private static BattlementDirectComponentCommand Tint(Wire.CoreCommand command, bool tweened)
        {
            if (tweened)
            {
                Wire.TweenTintPayload value = command.PayloadAsTweenTintPayload();
                Wire.RgbColor color = value.Tint!.Value;
                return new(
                    BattlementDirectComponentCommandKind.ImageTweenTint,
                    ComponentObject(value.ObjectId, "image object"),
                    color.R,
                    color.G,
                    color.B,
                    onConflict: (ConflictPolicy)(byte)value.OnConflict,
                    tween: TweenSettings(value.Tween!.Value, value.OnConflict)
                );
            }
            Wire.TintPayload set = command.PayloadAsTintPayload();
            Wire.RgbColor tint = set.Tint!.Value;
            return new(
                BattlementDirectComponentCommandKind.ImageSetTint,
                ComponentObject(set.ObjectId, "image object"),
                tint.R,
                tint.G,
                tint.B,
                onConflict: (ConflictPolicy)(byte)set.OnConflict
            );
        }

        private static BattlementDirectComponentCommand Opacity(
            Wire.CoreCommand command,
            bool tweened
        )
        {
            if (tweened)
            {
                Wire.TweenOpacityPayload value = command.PayloadAsTweenOpacityPayload();
                return new(
                    BattlementDirectComponentCommandKind.ImageTweenOpacity,
                    ComponentObject(value.ObjectId, "image object"),
                    value.Opacity,
                    onConflict: (ConflictPolicy)(byte)value.OnConflict,
                    tween: TweenSettings(value.Tween!.Value, value.OnConflict)
                );
            }
            Wire.OpacityPayload set = command.PayloadAsOpacityPayload();
            return new(
                BattlementDirectComponentCommandKind.ImageSetOpacity,
                ComponentObject(set.ObjectId, "image object"),
                set.Opacity,
                onConflict: (ConflictPolicy)(byte)set.OnConflict
            );
        }

        private static BattlementDirectComponentCommand TextSize(
            Wire.CoreCommand command,
            bool tweened
        )
        {
            if (tweened)
            {
                Wire.TweenTextSizePayload value = command.PayloadAsTweenTextSizePayload();
                return new(
                    BattlementDirectComponentCommandKind.TextTweenSize,
                    ComponentObject(value.ObjectId, "text object"),
                    value.Size,
                    onConflict: (ConflictPolicy)(byte)value.OnConflict,
                    tween: TweenSettings(value.Tween!.Value, value.OnConflict)
                );
            }
            Wire.TextSizePayload set = command.PayloadAsTextSizePayload();
            return new(
                BattlementDirectComponentCommandKind.TextSetSize,
                ComponentObject(set.ObjectId, "text object"),
                set.Size,
                onConflict: (ConflictPolicy)(byte)set.OnConflict
            );
        }

        private static BattlementDirectComponentCommand TextAlignment(Wire.CoreCommand command)
        {
            Wire.TextAlignmentPayload value = command.PayloadAsTextAlignmentPayload();
            return new(
                BattlementDirectComponentCommandKind.TextSetAlignment,
                ComponentObject(value.ObjectId, "text object"),
                option: (byte)value.Horizontal,
                first: (byte)value.Vertical
            );
        }

        private static BattlementDirectComponentCommand TextWrapping(Wire.CoreCommand command)
        {
            Wire.TextWrappingPayload value = command.PayloadAsTextWrappingPayload();
            return new(
                BattlementDirectComponentCommandKind.TextSetWrapping,
                ComponentObject(value.ObjectId, "text object"),
                value.WrapWidth ?? 0,
                hasValue: value.WrapWidth.HasValue
            );
        }

        private static BattlementDirectVisualElementAction ReadVisualElementAction(
            Wire.VisualElementActionPayload value
        )
        {
            ObjectId objectId = ComponentObject(value.ObjectId, "visual action object");
            if (value.Kind == Wire.VisualElementActionKind.ParticleStreaks)
            {
                var streaks = new UiParticleStreak[value.StreaksLength];
                for (int index = 0; index < streaks.Length; index++)
                {
                    Wire.UiParticleStreak streak =
                        value.Streaks(index)
                        ?? throw new InvalidDataException("A UI particle streak is absent.");
                    Wire.F32Vector2 origin = streak.Origin!.Value;
                    Wire.F32Vector2 travel = streak.Travel!.Value;
                    Wire.F32Vector2 size = streak.Size!.Value;
                    Wire.RgbaColor color = streak.Color!.Value;
                    streaks[index] = new UiParticleStreak(
                        new[] { origin.X, origin.Y },
                        new[] { travel.X, travel.Y },
                        new[] { size.X, size.Y },
                        streak.Rotation,
                        new Color(color.R, color.G, color.B, color.A),
                        streak.LifetimeMs,
                        streak.DelayMs
                    );
                }
                return new BattlementDirectVisualElementAction(
                    objectId,
                    BattlementDirectVisualElementActionKind.ParticleStreaks,
                    streaks
                );
            }
            return value.Kind switch
            {
                Wire.VisualElementActionKind.Focus => new(
                    objectId,
                    BattlementDirectVisualElementActionKind.Focus
                ),
                Wire.VisualElementActionKind.Blur => new(
                    objectId,
                    BattlementDirectVisualElementActionKind.Blur
                ),
                Wire.VisualElementActionKind.CapturePointer => new(
                    objectId,
                    BattlementDirectVisualElementActionKind.CapturePointer,
                    pointerId: value.PointerId
                ),
                Wire.VisualElementActionKind.ReleasePointer => new(
                    objectId,
                    BattlementDirectVisualElementActionKind.ReleasePointer,
                    pointerId: value.PointerId
                ),
                Wire.VisualElementActionKind.ScrollTo => new(
                    objectId,
                    BattlementDirectVisualElementActionKind.ScrollTo,
                    descendantId: ComponentObject(value.DescendantId, "scroll descendant")
                ),
                Wire.VisualElementActionKind.SelectText => new(
                    objectId,
                    BattlementDirectVisualElementActionKind.SelectText,
                    cursorIndex: value.CursorIndex,
                    selectionIndex: value.SelectionIndex
                ),
                _ => throw new InvalidDataException("A visual-element action kind is unknown."),
            };
        }

        internal static ControllerInputSettings ReadController(Wire.ControllerInputSettings value)
        {
            var buttons = new ControllerButton[value.ButtonsLength];
            for (int index = 0; index < buttons.Length; index++)
                buttons[index] = (ControllerButton)(byte)value.Buttons(index);
            return new ControllerInputSettings(
                buttons,
                value.NavigationEnabled,
                value.StickDeadZone,
                value.RepeatDelayMs.HasValue
                    ? TimeSpan.FromMilliseconds(value.RepeatDelayMs.Value)
                    : null,
                value.RepeatIntervalMs.HasValue
                    ? TimeSpan.FromMilliseconds(value.RepeatIntervalMs.Value)
                    : null
            );
        }

        private static BattlementDirectTweenSettings TweenSettings(
            Wire.Tween tween,
            Wire.ConflictPolicy onConflict
        ) =>
            new(
                tween.DelayMs,
                tween.DurationMs,
                (Easing)(byte)tween.Easing,
                (byte)tween.RepeatKind,
                tween.RepeatCount,
                (RepeatMode)(byte)tween.RepeatMode,
                (ConflictPolicy)(byte)onConflict
            );

        internal static BattlementDirectObjectPlacement ReadPlacement(Wire.GameObject value)
        {
            Wire.ParentScene parentScene = value.ParentScene!.Value;
            Wire.LocalTransform transform = value.LocalTransform!.Value;
            Wire.Vector3d position = transform.Position;
            Wire.Quaterniond rotation = transform.Rotation;
            Wire.Vector3d scale = transform.Scale;
            var pointerEvents = new PointerEvent[value.PointerEventsLength];
            for (int index = 0; index < pointerEvents.Length; index++)
                pointerEvents[index] = (PointerEvent)(byte)value.PointerEvents(index);
            return new BattlementDirectObjectPlacement(
                new ObjectId(BattlementFlatBufferCore.ReadUuid(value.ObjectId, "created object")),
                (byte)parentScene.Kind,
                parentScene.SceneId.HasValue
                    ? new SceneId(
                        BattlementFlatBufferCore.ReadUuid(parentScene.SceneId, "parent scene")
                    )
                    : null,
                value.ParentId.HasValue
                    ? new ObjectId(
                        BattlementFlatBufferCore.ReadUuid(value.ParentId, "object parent")
                    )
                    : null,
                value.Active,
                position.X,
                position.Y,
                position.Z,
                rotation.X,
                rotation.Y,
                rotation.Z,
                rotation.W,
                scale.X,
                scale.Y,
                scale.Z,
                pointerEvents,
                value.DragMode == Wire.DragMode.None ? null : (DragMode)((byte)value.DragMode - 1)
            );
        }

        internal static PreparedAsset ReadPreparedAsset(Wire.PreparedAsset value)
        {
            string address = value.Address;
            return value.Kind switch
            {
                Wire.PreparedAssetKind.Scene => new PreparedAsset.Scene(new SceneAddress(address)),
                Wire.PreparedAssetKind.Prefab => new PreparedAsset.Prefab(
                    new PrefabAddress(address)
                ),
                Wire.PreparedAssetKind.ParticleEffect => new PreparedAsset.ParticleEffect(
                    new ParticleEffectAddress(address)
                ),
                Wire.PreparedAssetKind.Material => new PreparedAsset.Material(
                    new MaterialAddress(address)
                ),
                Wire.PreparedAssetKind.Texture => new PreparedAsset.Texture(
                    new TextureAddress(address)
                ),
                Wire.PreparedAssetKind.Sprite => new PreparedAsset.Sprite(
                    new SpriteAddress(address)
                ),
                Wire.PreparedAssetKind.VectorImage => new PreparedAsset.VectorImage(
                    new VectorImageAddress(address)
                ),
                Wire.PreparedAssetKind.RenderTexture => new PreparedAsset.RenderTexture(
                    new RenderTextureAddress(address)
                ),
                Wire.PreparedAssetKind.AudioClip => new PreparedAsset.AudioClip(
                    new AudioClipAddress(address)
                ),
                Wire.PreparedAssetKind.TextMeshProFont => new PreparedAsset.TextMeshProFont(
                    new TextMeshProFontAddress(address)
                ),
                Wire.PreparedAssetKind.UiFont => new PreparedAsset.UiFont(
                    new UiFontAddress(address)
                ),
                _ => throw new InvalidDataException("A prepared asset kind is unknown."),
            };
        }

        private static bool IsPrimitive(Wire.GameObjectKind kind) =>
            (byte)kind >= (byte)Wire.GameObjectKind.Cube
            && (byte)kind <= (byte)Wire.GameObjectKind.Quad;

        private static BattlementDirectMaterialAssignment[] ReadMaterials(
            Wire.PrimitiveObject value
        ) => ReadMaterials(value.MaterialsLength, index => value.Materials(index)!.Value);

        private static BattlementDirectMaterialAssignment[] ReadMaterials(
            Wire.PrefabObject value
        ) => ReadMaterials(value.MaterialsLength, index => value.Materials(index)!.Value);

        private static BattlementDirectMaterialAssignment[] ReadMaterials(
            int count,
            Func<int, Wire.MaterialAssignment> item
        )
        {
            var result = new BattlementDirectMaterialAssignment[count];
            for (int index = 0; index < count; index++)
            {
                Wire.MaterialAssignment value = item(index);
                result[index] = new BattlementDirectMaterialAssignment(value.Slot, value.Address);
            }
            return result;
        }

        private static BattlementDirectAnimatorState ReadAnimator(Wire.AnimatorState value)
        {
            var bools = new BattlementDirectAnimatorBool[value.BoolParametersLength];
            for (int index = 0; index < bools.Length; index++)
            {
                Wire.AnimatorBoolParameter item = value.BoolParameters(index)!.Value;
                bools[index] = new BattlementDirectAnimatorBool(item.Name, item.Value);
            }
            var ints = new BattlementDirectAnimatorInt[value.IntParametersLength];
            for (int index = 0; index < ints.Length; index++)
            {
                Wire.AnimatorIntParameter item = value.IntParameters(index)!.Value;
                ints[index] = new BattlementDirectAnimatorInt(item.Name, item.Value);
            }
            var floats = new BattlementDirectAnimatorFloat[value.FloatParametersLength];
            for (int index = 0; index < floats.Length; index++)
            {
                Wire.AnimatorFloatParameter item = value.FloatParameters(index)!.Value;
                floats[index] = new BattlementDirectAnimatorFloat(item.Name, item.Value);
            }
            return new BattlementDirectAnimatorState(
                value.State,
                value.Layer,
                value.NormalizedStartTime,
                bools,
                ints,
                floats,
                value.Speed
            );
        }
    }

    internal interface IBattlementResponseView : IDisposable
    {
        SessionId SessionId { get; }
        int MessageCount { get; }
        bool IsSnapshot(int index);
        IBattlementSnapshotView ReadSnapshot(int index);
        IBattlementBatchView ReadBatch(int index);
    }

    internal interface IBattlementSnapshotView : IDisposable
    {
        SessionId SessionId { get; }
        ObjectId? InputCameraId { get; }
        SceneId? PrimarySceneId { get; }
        bool IsInputDisabled { get; }
        PanelInputConfigurationValue PanelInputConfiguration { get; }
    }

    internal interface IBattlementFlatBufferViewOwner
    {
        IDisposable RetainView();
        void RequireLiveView();
    }

    internal interface IBattlementBatchView : IDisposable
    {
        BatchId Id { get; }
        SessionId SessionId { get; }
        ActionId? CausedByActionId { get; }
        BatchStart Start { get; }
        ulong? WorkScope { get; }
        ulong? CancelScope { get; }
        int GroupCount { get; }
        int CommandCount(int groupIndex);
        CommandId CommandId(int groupIndex, int commandIndex);
        BattlementCommandExecution ReadCommand(int groupIndex, int commandIndex);
        bool IsAssetPreparation(int groupIndex, int commandIndex);
    }

    internal sealed class BattlementFlatBufferSnapshotView
        : IBattlementSnapshotView,
            IBattlementUiDocumentCollectionView
    {
        private readonly IBattlementFlatBufferViewOwner owner;
        private readonly Wire.Snapshot value;
        private IDisposable? lease;

        internal BattlementFlatBufferSnapshotView(
            IBattlementFlatBufferViewOwner owner,
            Wire.Snapshot value,
            bool retain = true
        ) => (this.owner, this.value, lease) = (owner, value, retain ? owner.RetainView() : null);

        public SessionId SessionId =>
            new(BattlementFlatBufferResponse.ReadUuid(Checked().SessionId, "snapshot session"));

        public ObjectId? InputCameraId =>
            Checked().InputCameraId.HasValue
                ? new ObjectId(
                    BattlementFlatBufferResponse.ReadUuid(value.InputCameraId, "input camera")
                )
                : null;

        public SceneId? PrimarySceneId =>
            Checked().PrimarySceneId.HasValue
                ? new SceneId(
                    BattlementFlatBufferResponse.ReadUuid(value.PrimarySceneId, "primary scene")
                )
                : null;

        public bool IsInputDisabled => Checked().InputDisabled;

        public int DocumentCount => Checked().UiLength;

        public IBattlementUiDocumentView ReadDocument(int index)
        {
            Wire.Snapshot snapshot = Checked();
            if ((uint)index >= (uint)snapshot.UiLength)
                throw new ArgumentOutOfRangeException(nameof(index));
            return new BattlementFlatBufferUiDocumentView(
                this,
                snapshot.Ui(index) ?? throw new InvalidDataException("A UI document is absent.")
            );
        }

        public PanelInputConfigurationValue PanelInputConfiguration
        {
            get
            {
                Wire.PanelInputConfiguration panel = Checked().PanelInputConfiguration!.Value;
                InteractionDistance distance = panel.DistanceKind switch
                {
                    Wire.InteractionDistanceKind.Unbounded => new InteractionDistance.Unbounded(),
                    Wire.InteractionDistanceKind.Inclusive => new InteractionDistance.Inclusive(
                        panel.MaximumInteractionDistance
                    ),
                    _ => throw new InvalidDataException("Unknown panel interaction distance kind."),
                };
                return new PanelInputConfigurationValue(
                    new InteractionLayerMask(panel.InteractionLayers),
                    distance,
                    (PanelInputRedirection)(byte)panel.InputRedirection
                );
            }
        }

        internal bool CanApplyDirectly
        {
            get
            {
                _ = Checked();
                return true;
            }
        }

        internal int DirectObjectCount => Checked().ObjectsLength;

        internal int DirectPreparedAssetCount => Checked().PreparedAssetsLength;

        internal Wire.PreparedAssetKind DirectPreparedAssetKind(int index) =>
            DirectPreparedAsset(index).Kind;

        internal string DirectPreparedAssetAddress(int index) => DirectPreparedAsset(index).Address;

        internal PreparedAsset ReadDirectPreparedAsset(int index) =>
            BattlementDirectCommandReader.ReadPreparedAsset(DirectPreparedAsset(index));

        internal int DirectSceneCount => Checked().ScenesLength;

        internal BattlementScene ReadDirectScene(int index)
        {
            Wire.Snapshot snapshot = Checked();
            if ((uint)index >= (uint)snapshot.ScenesLength)
                throw new ArgumentOutOfRangeException(nameof(index));
            Wire.Scene value =
                snapshot.Scenes(index)
                ?? throw new InvalidDataException("A snapshot scene is absent.");
            return new BattlementScene(
                new SceneId(BattlementFlatBufferCore.ReadUuid(value.SceneId, "snapshot scene")),
                new SceneAddress(value.Address)
            );
        }

        internal IReadOnlyList<PhysicalKey> ReadDirectGlobalKeys()
        {
            Wire.Snapshot snapshot = Checked();
            var result = new PhysicalKey[snapshot.GlobalKeysLength];
            for (int index = 0; index < result.Length; index++)
                result[index] = (PhysicalKey)(ushort)snapshot.GlobalKeys(index);
            return result;
        }

        internal ControllerInputSettings? ReadDirectControllerInput()
        {
            Wire.ControllerInputSettings? optional = Checked().ControllerInput;
            if (!optional.HasValue)
                return null;
            Wire.ControllerInputSettings value = optional.Value;
            var buttons = new ControllerButton[value.ButtonsLength];
            for (int index = 0; index < buttons.Length; index++)
                buttons[index] = (ControllerButton)(byte)value.Buttons(index);
            return new ControllerInputSettings(
                buttons,
                value.NavigationEnabled,
                value.StickDeadZone,
                value.RepeatDelayMs.HasValue
                    ? TimeSpan.FromMilliseconds(value.RepeatDelayMs.Value)
                    : null,
                value.RepeatIntervalMs.HasValue
                    ? TimeSpan.FromMilliseconds(value.RepeatIntervalMs.Value)
                    : null
            );
        }

        internal BattlementDirectSnapshotObject ReadDirectObject(int index)
        {
            Wire.Snapshot snapshot = Checked();
            if ((uint)index >= (uint)snapshot.ObjectsLength)
                throw new ArgumentOutOfRangeException(nameof(index));
            return BattlementDirectCommandReader.ReadSnapshotObject(
                snapshot.Objects(index)
                    ?? throw new InvalidDataException("A snapshot object is absent.")
            );
        }

        private Wire.PreparedAsset DirectPreparedAsset(int index)
        {
            Wire.Snapshot snapshot = Checked();
            if ((uint)index >= (uint)snapshot.PreparedAssetsLength)
                throw new ArgumentOutOfRangeException(nameof(index));
            return snapshot.PreparedAssets(index)
                ?? throw new InvalidDataException("A prepared asset is absent.");
        }

        public void Dispose()
        {
            lease?.Dispose();
            lease = null;
        }

        private Wire.Snapshot Checked()
        {
            owner.RequireLiveView();
            if (lease is null)
                throw new ObjectDisposedException(nameof(BattlementFlatBufferSnapshotView));
            return value;
        }

        internal void RequireLiveDocumentView() => _ = Checked();
    }

    internal sealed class BattlementFlatBufferUiDocumentView : IBattlementUiDocumentView
    {
        private readonly BattlementFlatBufferSnapshotView snapshot;
        private readonly Wire.UiDocument value;

        internal BattlementFlatBufferUiDocumentView(
            BattlementFlatBufferSnapshotView snapshot,
            Wire.UiDocument value
        ) => (this.snapshot, this.value) = (snapshot, value);

        public ObjectId DocumentId =>
            new(BattlementFlatBufferResponse.ReadUuid(Checked().DocumentId, "UI document"));

        public ObjectId RootId =>
            new(BattlementFlatBufferResponse.ReadUuid(Checked().RootId, "UI document root"));

        public UiDocument ReadRoot() => BattlementFlatBufferRetainedCopy.UiDocumentRoot(Checked());

        public int RootChildCount => Checked().RootChildIdsLength;

        public ObjectId ReadRootChildId(int index) =>
            new(
                BattlementFlatBufferResponse.ReadUuid(
                    Checked().RootChildIds(index),
                    "UI document root child"
                )
            );

        public int NodeCount => Checked().NodesLength;

        public ObjectId ReadNodeId(int index) =>
            new(BattlementFlatBufferResponse.ReadUuid(Node(index).ObjectId, "UI document node"));

        public UiElement ReadNodeElement(int index) =>
            BattlementFlatBufferRetainedCopy.UiElement(
                Node(index).Element
                    ?? throw new InvalidDataException("A UI node element is absent.")
            );

        public int ReadChildCount(int index) => Node(index).ChildIdsLength;

        public ObjectId ReadChildId(int nodeIndex, int childIndex) =>
            new(
                BattlementFlatBufferResponse.ReadUuid(
                    Node(nodeIndex).ChildIds(childIndex),
                    "UI document child"
                )
            );

        private Wire.UiNode Node(int index)
        {
            Wire.UiDocument document = Checked();
            if ((uint)index >= (uint)document.NodesLength)
                throw new ArgumentOutOfRangeException(nameof(index));
            return document.Nodes(index) ?? throw new InvalidDataException("A UI node is absent.");
        }

        private Wire.UiDocument Checked()
        {
            snapshot.RequireLiveDocumentView();
            return value;
        }
    }

    internal sealed class BattlementFlatBufferBatchView : IBattlementBatchView
    {
        private readonly BattlementFlatBufferResponse response;
        private readonly Wire.Batch value;
        private IDisposable? lease;

        internal BattlementFlatBufferBatchView(
            BattlementFlatBufferResponse response,
            Wire.Batch value
        ) => (this.response, this.value, lease) = (response, value, response.Retain());

        public BatchId Id => new(BattlementFlatBufferResponse.ReadUuid(Checked().BatchId, "batch"));
        public SessionId SessionId =>
            new(BattlementFlatBufferResponse.ReadUuid(Checked().SessionId, "batch session"));
        public ActionId? CausedByActionId =>
            Checked().CausedByActionId.HasValue
                ? new ActionId(
                    BattlementFlatBufferResponse.ReadUuid(value.CausedByActionId, "causing action")
                )
                : null;
        public BatchStart Start => (BatchStart)(byte)Checked().Start;
        public ulong? WorkScope => Checked().WorkScope;
        public ulong? CancelScope => Checked().CancelScope;
        public int GroupCount => Checked().GroupsLength;

        public int CommandCount(int groupIndex) => Group(groupIndex).CommandsLength;

        public CommandId CommandId(int groupIndex, int commandIndex) =>
            new(
                BattlementFlatBufferResponse.ReadUuid(
                    Command(groupIndex, commandIndex).CommandId,
                    "command"
                )
            );

        public BattlementCommandExecution ReadCommand(int groupIndex, int commandIndex)
        {
            Wire.CoreCommand command = Command(groupIndex, commandIndex);
            CommandId id = CommandId(groupIndex, commandIndex);
            if (BattlementDirectCommandReader.TryRead(command, id, response, out var direct))
                return direct;
            throw new InvalidDataException(
                $"Core FlatBuffer command {command.Kind} has no direct host reader."
            );
        }

        public bool IsAssetPreparation(int groupIndex, int commandIndex) =>
            Command(groupIndex, commandIndex).Kind == Wire.CoreCommandKind.AssetsReplaceSet;

        public void Dispose()
        {
            lease?.Dispose();
            lease = null;
        }

        private Wire.Batch Checked()
        {
            response.RequireLiveView();
            if (lease is null)
                throw new ObjectDisposedException(nameof(BattlementFlatBufferBatchView));
            return value;
        }

        private Wire.ParallelCommandGroup Group(int index)
        {
            Wire.Batch batch = Checked();
            if ((uint)index >= (uint)batch.GroupsLength)
                throw new ArgumentOutOfRangeException(nameof(index));
            return batch.Groups(index)
                ?? throw new InvalidDataException("A command group is absent.");
        }

        private Wire.CoreCommand Command(int groupIndex, int commandIndex)
        {
            Wire.ParallelCommandGroup group = Group(groupIndex);
            if ((uint)commandIndex >= (uint)group.CommandsLength)
                throw new ArgumentOutOfRangeException(nameof(commandIndex));
            Wire.CommandEntry entry =
                group.Commands(commandIndex)
                ?? throw new InvalidDataException("A command entry is absent.");
            if (entry.CommandType != Wire.CommandEntryPayload.CoreCommand)
                throw new InvalidDataException("A core response contains a custom command tag.");
            return entry.CommandAsCoreCommand();
        }
    }
}
