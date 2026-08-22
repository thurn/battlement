#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;

namespace Battlement
{
    /// <summary>Advances admitted batches against the host's monotonic clock.</summary>
    internal sealed class BattlementBatchScheduler
    {
        private readonly List<ScheduledBatch> batches = new();
        private readonly IBattlementClock clock;
        private readonly BattlementCommandExecutor executor;
        private readonly BattlementOperationRegistry operations;
        private readonly Action<BatchFailed<CoreErrorCode>> reportFailure;
        private readonly Action<
            BattlementRegisteredCommandException,
            SessionId,
            BatchId,
            CommandId?
        > reportCustomFailure;
        private bool isAdvancing;

        public BattlementBatchScheduler(
            IBattlementClock clock,
            BattlementCommandExecutor executor,
            BattlementOperationRegistry operations,
            Action<BatchFailed<CoreErrorCode>> reportFailure,
            Action<
                BattlementRegisteredCommandException,
                SessionId,
                BatchId,
                CommandId?
            > reportCustomFailure
        )
        {
            this.clock = clock;
            this.executor = executor;
            this.operations = operations;
            this.reportFailure = reportFailure;
            this.reportCustomFailure = reportCustomFailure;
        }

        public void BeginSession()
        {
            batches.Clear();
            operations.BeginSession();
        }

        public void CancelForSnapshot()
        {
            operations.CancelAll();
            batches.Clear();
        }

        public void Schedule(
            SessionId sessionId,
            Batch<ICommand> batch,
            BattlementBatchAdmissionResult admission
        )
        {
            batches.Add(new ScheduledBatch(sessionId, batch, admission));
            Advance();
        }

        public void Advance()
        {
            if (isAdvancing)
            {
                return;
            }

            isAdvancing = true;
            try
            {
                TimeSpan now = clock.Elapsed;
                operations.AdvanceNonblocking(now);
                bool madeProgress;
                do
                {
                    madeProgress = false;
                    foreach (ScheduledBatch batch in batches.ToArray())
                    {
                        madeProgress |= AdvanceBatch(batch, now);
                    }
                } while (madeProgress);
            }
            finally
            {
                isAdvancing = false;
            }
        }

        private bool AdvanceBatch(ScheduledBatch scheduled, TimeSpan now)
        {
            if (scheduled.Outcome != BatchOutcome.Pending)
            {
                return false;
            }

            if (!scheduled.HasStarted)
            {
                if (HasEarlierBlockingWork(scheduled))
                {
                    return false;
                }

                scheduled.HasStarted = true;
                if (DependsOnFailedPredecessor(scheduled))
                {
                    Fail(
                        scheduled,
                        CoreErrorCode.EarlierBatchFailed,
                        "An earlier dependent batch failed."
                    );
                    return true;
                }
            }

            int previousBlockingCount = scheduled.BlockingOperations.Count;
            foreach (ScheduledOperation operation in scheduled.BlockingOperations.ToArray())
            {
                try
                {
                    if (operation.Operation.IsComplete(now))
                    {
                        scheduled.BlockingOperations.Remove(operation);
                    }
                }
                catch (BattlementRegisteredCommandException exception)
                {
                    FailCustom(scheduled, exception, operation.CommandId);
                    return true;
                }
                catch (BattlementCommandException exception)
                {
                    Fail(scheduled, exception.ErrorCode, exception.Message, operation.CommandId);
                    return true;
                }
                catch (Exception exception)
                {
                    Fail(
                        scheduled,
                        CoreErrorCode.UnityException,
                        exception.Message,
                        operation.CommandId
                    );
                    return true;
                }
            }

            if (scheduled.BlockingOperations.Count > 0)
            {
                return previousBlockingCount != scheduled.BlockingOperations.Count;
            }

            if (scheduled.NextGroup >= scheduled.Batch.Groups.Count)
            {
                scheduled.Outcome = BatchOutcome.Succeeded;
                return true;
            }

            ParallelCommandGroup<ICommand> group = scheduled.Batch.Groups[scheduled.NextGroup++];
            foreach (ICommand command in group.Commands)
            {
                try
                {
                    IBattlementCommandOperation? operation = operations.Launch(
                        scheduled.SessionId,
                        scheduled.Batch.Id,
                        command,
                        now,
                        started => executor.Launch(command, started)
                    );
                    if (operation is null)
                    {
                        continue;
                    }

                    if (command.IsBlocking)
                    {
                        scheduled.BlockingOperations.Add(
                            new ScheduledOperation(command.Id, operation)
                        );
                    }
                }
                catch (BattlementRegisteredCommandException exception)
                {
                    FailCustom(scheduled, exception, command.Id);
                    break;
                }
                catch (BattlementCommandException exception)
                {
                    Fail(scheduled, exception.ErrorCode, exception.Message, command.Id);
                    break;
                }
                catch (Exception exception)
                {
                    Fail(scheduled, CoreErrorCode.UnityException, exception.Message, command.Id);
                    break;
                }
            }

            return true;
        }

        private bool HasEarlierBlockingWork(ScheduledBatch scheduled) =>
            scheduled.Admission.WaitsThroughSequence is long through
            && batches.Any(batch =>
                batch.Admission.Sequence <= through && batch.Outcome == BatchOutcome.Pending
            );

        private bool DependsOnFailedPredecessor(ScheduledBatch scheduled)
        {
            if (scheduled.Admission.WaitsThroughSequence is null)
            {
                return false;
            }

            ScheduledBatch? predecessor = batches.FirstOrDefault(batch =>
                batch.Admission.Sequence == scheduled.Admission.Sequence - 1
            );
            return predecessor?.Outcome == BatchOutcome.Failed;
        }

        private void Fail(
            ScheduledBatch scheduled,
            CoreErrorCode errorCode,
            string message,
            CommandId? commandId = null
        )
        {
            foreach (ScheduledOperation operation in scheduled.BlockingOperations)
            {
                operation.Operation.Cancel();
            }

            scheduled.BlockingOperations.Clear();
            scheduled.Outcome = BatchOutcome.Failed;
            reportFailure(
                new BatchFailed<CoreErrorCode>(
                    scheduled.SessionId,
                    scheduled.Batch.Id,
                    errorCode,
                    message,
                    commandId
                )
            );
        }

        private void FailCustom(
            ScheduledBatch scheduled,
            BattlementRegisteredCommandException exception,
            CommandId? commandId
        )
        {
            foreach (ScheduledOperation operation in scheduled.BlockingOperations)
            {
                operation.Operation.Cancel();
            }

            scheduled.BlockingOperations.Clear();
            scheduled.Outcome = BatchOutcome.Failed;
            reportCustomFailure(exception, scheduled.SessionId, scheduled.Batch.Id, commandId);
        }

        private enum BatchOutcome
        {
            Pending,
            Succeeded,
            Failed,
        }

        private sealed class ScheduledBatch
        {
            public ScheduledBatch(
                SessionId sessionId,
                Batch<ICommand> batch,
                BattlementBatchAdmissionResult admission
            )
            {
                SessionId = sessionId;
                Batch = batch;
                Admission = admission;
            }

            public SessionId SessionId { get; }

            public Batch<ICommand> Batch { get; }

            public BattlementBatchAdmissionResult Admission { get; }

            public List<ScheduledOperation> BlockingOperations { get; } = new();

            public bool HasStarted { get; set; }

            public int NextGroup { get; set; }

            public BatchOutcome Outcome { get; set; }
        }

        private sealed record ScheduledOperation(
            CommandId CommandId,
            IBattlementCommandOperation Operation
        );
    }
}
