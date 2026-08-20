#nullable enable

using System;
using System.Collections.Generic;

namespace Masonry
{
    /// <summary>Validates and records batches before command scheduling begins.</summary>
    internal sealed class MasonryBatchAdmission
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

        public MasonryBatchAdmissionResult Admit(SessionId responseSession, Batch<ICommand> batch)
        {
            if (batch.Id.Value == Guid.Empty)
            {
                throw new MasonryUnorderableBatchException("A batch UUID must be nonzero.");
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
                return new MasonryBatchAdmissionResult(true, sequence, null);
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

            if (batch.Groups is null || batch.Groups.Count is 0 or > MaximumGroups)
            {
                CoreErrorCode code =
                    batch.Groups?.Count > MaximumGroups
                        ? CoreErrorCode.LimitExceeded
                        : CoreErrorCode.InvalidProperty;
                throw Invalid(
                    code,
                    $"A batch must contain between 1 and {MaximumGroups} command groups."
                );
            }

            var commandIds = new HashSet<Guid>();
            foreach (ParallelCommandGroup<ICommand>? group in batch.Groups)
            {
                if (
                    group?.Commands is null
                    || group.Commands.Count is 0 or > MaximumCommandsPerGroup
                )
                {
                    CoreErrorCode code =
                        group?.Commands?.Count > MaximumCommandsPerGroup
                            ? CoreErrorCode.LimitExceeded
                            : CoreErrorCode.InvalidProperty;
                    throw Invalid(
                        code,
                        "A command group must contain between 1 and "
                            + $"{MaximumCommandsPerGroup} commands."
                    );
                }

                foreach (ICommand? command in group.Commands)
                {
                    if (command is null || command is Command { Body: null })
                    {
                        throw Invalid(
                            CoreErrorCode.InvalidProperty,
                            "Every command must contain its common fields."
                        );
                    }

                    if (command.Id.Value == Guid.Empty)
                    {
                        throw Invalid(
                            CoreErrorCode.InvalidProperty,
                            "A command UUID must be nonzero."
                        );
                    }

                    if (!commandIds.Add(command.Id.Value))
                    {
                        throw Invalid(
                            CoreErrorCode.DuplicateId,
                            $"Command UUID {command.Id} appeared more than once in a batch.",
                            command.Id
                        );
                    }
                }
            }

            long earlierSequence = sequence;
            sequence++;
            admittedIds.Add(batch.Id.Value);
            return new MasonryBatchAdmissionResult(
                false,
                sequence,
                batch.Start == BatchStart.AfterEarlierBlockingWork ? earlierSequence : null
            );
        }

        private static MasonryBatchAdmissionException Invalid(
            CoreErrorCode errorCode,
            string message,
            CommandId? commandId = null
        ) => new(errorCode, message, commandId);
    }

    internal sealed record MasonryBatchAdmissionResult(
        bool IsDuplicate,
        long Sequence,
        long? WaitsThroughSequence
    );

    internal sealed class MasonryBatchAdmissionException : InvalidOperationException
    {
        public MasonryBatchAdmissionException(
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

    internal sealed class MasonryUnorderableBatchException : InvalidOperationException
    {
        public MasonryUnorderableBatchException(string message)
            : base(message) { }
    }
}
