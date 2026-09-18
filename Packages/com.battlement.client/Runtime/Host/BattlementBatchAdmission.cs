#nullable enable

using System;
using System.Collections.Generic;

namespace Battlement
{
    /// <summary>Validates and records batches before command scheduling begins.</summary>
    internal sealed class BattlementBatchAdmission
    {
        private const int MaximumGroups = 256;
        private const int MaximumCommandsPerGroup = 4_096;

        private readonly HashSet<Guid> admittedIds = new();
        private long sequence;

        public void BeginSession()
        {
            admittedIds.Clear();
            sequence = 0;
        }

        public BattlementBatchAdmissionResult Admit(
            SessionId responseSession,
            IBattlementBatchView batch
        )
        {
            if (batch.Id.Value == Guid.Empty)
            {
                throw new BattlementUnorderableBatchException("A batch UUID must be nonzero.");
            }

            if (batch.SessionId.Value == Guid.Empty)
            {
                throw Invalid(
                    CoreErrorCode.InvalidProperty,
                    "A batch session UUID must be nonzero."
                );
            }

            if (batch.SessionId != responseSession)
            {
                throw Invalid(CoreErrorCode.WrongSession, "The batch used the wrong session.");
            }

            if (admittedIds.Contains(batch.Id.Value))
            {
                return new BattlementBatchAdmissionResult(true, sequence, null);
            }

            if (!Enum.IsDefined(typeof(BatchStart), batch.Start))
            {
                throw Invalid(CoreErrorCode.InvalidProperty, "The batch start mode is unknown.");
            }

            if (
                batch.CausedByActionId is ActionId { Value: var actionId }
                && actionId == Guid.Empty
            )
            {
                throw Invalid(
                    CoreErrorCode.InvalidProperty,
                    "A causing action UUID must be nonzero."
                );
            }

            if (
                batch.GroupCount > MaximumGroups
                || (
                    batch.GroupCount == 0
                    && !batch.CancelScope.HasValue
                    && batch.PresentationControl is null
                )
            )
            {
                CoreErrorCode code =
                    batch.GroupCount > MaximumGroups
                        ? CoreErrorCode.LimitExceeded
                        : CoreErrorCode.InvalidProperty;
                throw Invalid(
                    code,
                    $"A batch must contain between 1 and {MaximumGroups} command groups."
                );
            }

            if (batch.PresentationControl is PresentationControl control)
            {
                if (control.WorkScope == 0 || control.OwnerId.Value == Guid.Empty)
                    throw Invalid(
                        CoreErrorCode.InvalidProperty,
                        "Presentation control requires nonzero scope and owner identifiers."
                    );
                if (
                    batch.WorkScope.HasValue
                    || batch.CancelScope.HasValue
                    || batch.Start != BatchStart.Now
                    || batch.GroupCount != 0
                )
                    throw Invalid(
                        CoreErrorCode.InvalidProperty,
                        "Presentation control must be independent unowned work."
                    );
            }

            var commandIds = new HashSet<Guid>();
            for (int groupIndex = 0; groupIndex < batch.GroupCount; groupIndex++)
            {
                int commandCount = batch.CommandCount(groupIndex);
                if (commandCount is 0 or > MaximumCommandsPerGroup)
                {
                    CoreErrorCode code =
                        commandCount > MaximumCommandsPerGroup
                            ? CoreErrorCode.LimitExceeded
                            : CoreErrorCode.InvalidProperty;
                    throw Invalid(
                        code,
                        "A command group must contain between 1 and "
                            + $"{MaximumCommandsPerGroup} commands."
                    );
                }

                for (int commandIndex = 0; commandIndex < commandCount; commandIndex++)
                {
                    CommandId commandId = batch.CommandId(groupIndex, commandIndex);
                    if (commandId.Value == Guid.Empty)
                    {
                        throw Invalid(
                            CoreErrorCode.InvalidProperty,
                            "A command UUID must be nonzero."
                        );
                    }

                    if (!commandIds.Add(commandId.Value))
                    {
                        throw Invalid(
                            CoreErrorCode.DuplicateId,
                            $"Command UUID {commandId} appeared more than once in a batch.",
                            commandId
                        );
                    }
                }
            }

            long earlierSequence = sequence;
            sequence++;
            admittedIds.Add(batch.Id.Value);
            return new BattlementBatchAdmissionResult(
                false,
                sequence,
                batch.Start == BatchStart.AfterEarlierBlockingWork ? earlierSequence : null
            );
        }

        private static BattlementBatchAdmissionException Invalid(
            CoreErrorCode errorCode,
            string message,
            CommandId? commandId = null
        ) => new(errorCode, message, commandId);
    }

    internal sealed record BattlementBatchAdmissionResult(
        bool IsDuplicate,
        long Sequence,
        long? WaitsThroughSequence
    );

    internal sealed class BattlementBatchAdmissionException : InvalidOperationException
    {
        public BattlementBatchAdmissionException(
            CoreErrorCode errorCode,
            string message,
            CommandId? commandId
        )
            : base(message)
        {
            ErrorCode = errorCode;
            CommandId = commandId;
        }

        public CoreErrorCode ErrorCode { get; }

        public CommandId? CommandId { get; }
    }

    internal sealed class BattlementUnorderableBatchException : InvalidOperationException
    {
        public BattlementUnorderableBatchException(string message)
            : base(message) { }
    }
}
