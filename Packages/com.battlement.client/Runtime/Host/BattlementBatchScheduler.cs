#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using Battlement.UI;

namespace Battlement
{
    /// <summary>Advances admitted batches against the host's monotonic clock.</summary>
    internal sealed class BattlementBatchScheduler
    {
        private readonly List<ScheduledBatch> batches = new();
        private readonly HashSet<ulong> canceledScopes = new();
        private readonly IBattlementClock clock;
        private readonly BattlementCommandExecutor executor;
        private readonly BattlementOperationRegistry operations;
        private readonly Action<BatchFailed<CoreErrorCode>, Exception?> reportFailure;
        private readonly Action<
            BattlementRegisteredCommandException,
            SessionId,
            BatchId,
            CommandId?
        > reportCustomFailure;
        private bool isAdvancing;

        public ulong ActivityVersion { get; private set; }

        public bool HasPendingWork =>
            batches.Any(batch => batch.Outcome == BatchOutcome.Pending)
            || operations.HasFiniteOperations;

        public bool HasInfiniteOperations => operations.HasInfiniteOperations;

        public int FiniteOperationCount => operations.FiniteOperationCount;

        public int InfiniteOperationCount => operations.InfiniteOperationCount;

        private bool IsControlled => clock is DittoMotionClock { IsControlled: true };

        public BattlementBatchScheduler(
            IBattlementClock clock,
            BattlementCommandExecutor executor,
            BattlementOperationRegistry operations,
            Action<BatchFailed<CoreErrorCode>, Exception?> reportFailure,
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
            foreach (ScheduledBatch batch in batches)
                batch.Dispose();
            batches.Clear();
            canceledScopes.Clear();
            operations.BeginSession();
            executor.ResetWorkOwnership();
            ActivityVersion++;
        }

        public void CancelForSnapshot()
        {
            operations.CancelAll();
            executor.ResetWorkOwnership();
            foreach (ScheduledBatch batch in batches)
                batch.Dispose();
            batches.Clear();
        }

        public void Schedule(
            SessionId sessionId,
            IBattlementBatchView batch,
            BattlementBatchAdmissionResult admission
        )
        {
            if (batch.CancelScope is ulong canceled)
                CancelScope(canceled);
            if (batch.WorkScope is ulong owner && canceledScopes.Contains(owner))
            {
                batch.Dispose();
                return;
            }
            batches.Add(new ScheduledBatch(sessionId, batch, admission));
            ActivityVersion++;
            Advance();
        }

        private void CancelScope(ulong scope)
        {
            canceledScopes.Add(scope);
            operations.CancelScope(scope);
            executor.CancelScope(scope);
            foreach (ScheduledBatch batch in batches.Where(item => item.WorkScope == scope))
            {
                batch.BlockingOperations.Clear();
                if (batch.HasStarted && batch.Outcome == BatchOutcome.Pending)
                    executor.EndBatch();
                batch.Outcome = BatchOutcome.Canceled;
                batch.Dispose();
            }
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
                if (HasPendingWork)
                {
                    ActivityVersion++;
                }
                TimeSpan now = clock.Elapsed;
                operations.AdvanceNonblocking(now);
                bool madeProgress;
                do
                {
                    madeProgress = false;
                    foreach (ScheduledBatch batch in batches.ToArray())
                    {
                        bool progressed = AdvanceBatch(batch, now);
                        madeProgress |= progressed;
                        if (IsControlled && progressed)
                        {
                            return;
                        }
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
                executor.BeginBatch();
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
                    Fail(
                        scheduled,
                        exception.ErrorCode,
                        exception.Message,
                        operation.CommandId,
                        exception.DeveloperException
                    );
                    return true;
                }
                catch (Exception exception)
                {
                    Fail(
                        scheduled,
                        CoreErrorCode.UnityException,
                        exception.Message,
                        operation.CommandId,
                        exception
                    );
                    return true;
                }
            }

            if (scheduled.BlockingOperations.Count > 0)
            {
                return previousBlockingCount != scheduled.BlockingOperations.Count;
            }

            if (scheduled.NextGroup >= scheduled.Batch.GroupCount)
            {
                scheduled.Outcome = BatchOutcome.Succeeded;
                executor.EndBatch();
                scheduled.Dispose();
                return true;
            }

            int groupIndex = scheduled.NextGroup++;
            int commandCount = scheduled.Batch.CommandCount(groupIndex);
            for (int commandIndex = 0; commandIndex < commandCount; commandIndex++)
            {
                BattlementCommandExecution command = scheduled.Batch.ReadCommand(
                    groupIndex,
                    commandIndex
                );
                try
                {
                    IBattlementCommandOperation? operation = operations.Launch(
                        scheduled.SessionId,
                        scheduled.Id,
                        command,
                        now,
                        started => Launch(scheduled, command, started),
                        scheduled.WorkScope
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
                    Fail(
                        scheduled,
                        exception.ErrorCode,
                        exception.Message,
                        command.Id,
                        exception.DeveloperException
                    );
                    break;
                }
                catch (Exception exception)
                {
                    Fail(
                        scheduled,
                        CoreErrorCode.UnityException,
                        exception.Message,
                        command.Id,
                        exception
                    );
                    break;
                }
            }

            return true;
        }

        private IBattlementCommandOperation? Launch(
            ScheduledBatch batch,
            BattlementCommandExecution command,
            TimeSpan now
        )
        {
            try
            {
                return executor.Launch(command, now, batch.WorkScope);
            }
            catch (BattlementCommandException error)
                when (batch.IsCancellation && error.ErrorCode == CoreErrorCode.UnknownObject)
            {
                // Its queued creation may have been canceled before reaching the host.
                return null;
            }
            catch (BattlementUiException error)
                when (batch.IsCancellation && error.ErrorCode == CoreErrorCode.UnknownObject)
            {
                return null;
            }
        }

        private bool HasEarlierBlockingWork(ScheduledBatch scheduled) =>
            batches.Any(batch =>
                IsDependency(scheduled, batch) && batch.Outcome == BatchOutcome.Pending
            );

        private static bool IsDependency(ScheduledBatch scheduled, ScheduledBatch earlier)
        {
            if (earlier.Admission.Sequence >= scheduled.Admission.Sequence)
            {
                return false;
            }
            if (scheduled.Start == BatchStart.AfterEarlierAssetPreparation)
            {
                return earlier.ContainsAssetPreparation;
            }
            if (scheduled.WorkScope.HasValue && earlier.WorkScope != scheduled.WorkScope)
                return earlier.ContainsAssetPreparation;
            return scheduled.Admission.WaitsThroughSequence is long through
                && earlier.Admission.Sequence <= through;
        }

        private bool DependsOnFailedPredecessor(ScheduledBatch scheduled)
        {
            if (scheduled.Start == BatchStart.AfterEarlierAssetPreparation)
            {
                return batches.Any(batch =>
                    IsDependency(scheduled, batch) && batch.Outcome == BatchOutcome.Failed
                );
            }
            if (scheduled.Admission.WaitsThroughSequence is null)
            {
                return false;
            }

            ScheduledBatch? predecessor = batches.LastOrDefault(batch =>
                IsDependency(scheduled, batch)
            );
            return predecessor?.Outcome == BatchOutcome.Failed;
        }

        private void Fail(
            ScheduledBatch scheduled,
            CoreErrorCode errorCode,
            string message,
            CommandId? commandId = null,
            Exception? exception = null
        )
        {
            foreach (ScheduledOperation operation in scheduled.BlockingOperations)
            {
                operation.Operation.Cancel();
            }

            scheduled.BlockingOperations.Clear();
            scheduled.Outcome = BatchOutcome.Failed;
            executor.EndBatch();
            reportFailure(
                new BatchFailed<CoreErrorCode>(
                    scheduled.SessionId,
                    scheduled.Id,
                    errorCode,
                    message,
                    commandId
                ),
                exception
            );
            scheduled.Dispose();
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
            executor.EndBatch();
            reportCustomFailure(exception, scheduled.SessionId, scheduled.Id, commandId);
            scheduled.Dispose();
        }

        private enum BatchOutcome
        {
            Pending,
            Succeeded,
            Failed,
            Canceled,
        }

        private sealed class ScheduledBatch : IDisposable
        {
            public ScheduledBatch(
                SessionId sessionId,
                IBattlementBatchView batch,
                BattlementBatchAdmissionResult admission
            )
            {
                SessionId = sessionId;
                Batch = batch;
                Admission = admission;
                Id = batch.Id;
                Start = batch.Start;
                WorkScope = batch.WorkScope;
                IsCancellation = batch.CancelScope.HasValue;
                for (int groupIndex = 0; groupIndex < batch.GroupCount; groupIndex++)
                {
                    for (
                        int commandIndex = 0;
                        commandIndex < batch.CommandCount(groupIndex);
                        commandIndex++
                    )
                    {
                        if (batch.IsAssetPreparation(groupIndex, commandIndex))
                            ContainsAssetPreparation = true;
                    }
                }
            }

            public SessionId SessionId { get; }

            public IBattlementBatchView Batch { get; }

            public BatchId Id { get; }

            public BatchStart Start { get; }

            public ulong? WorkScope { get; }

            public bool IsCancellation { get; }

            public bool ContainsAssetPreparation { get; }

            public BattlementBatchAdmissionResult Admission { get; }

            public List<ScheduledOperation> BlockingOperations { get; } = new();

            public bool HasStarted { get; set; }

            public int NextGroup { get; set; }

            public BatchOutcome Outcome { get; set; }

            public void Dispose() => Batch.Dispose();
        }

        private sealed record ScheduledOperation(
            CommandId CommandId,
            IBattlementCommandOperation Operation
        );
    }
}
