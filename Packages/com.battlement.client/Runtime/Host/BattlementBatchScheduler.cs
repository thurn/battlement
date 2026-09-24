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
        private readonly Dictionary<ulong, HashSet<Guid>> pausedScopes = new();
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

        public bool HasRunnableWork =>
            batches.Any(batch =>
                batch.Outcome == BatchOutcome.Pending && !IsPaused(batch.WorkScope)
            ) || operations.HasFiniteOperations;

        public bool HasInfiniteOperations => operations.HasInfiniteOperations;

        public int HeldOperationCount => operations.HeldOperationCount;

        public int FiniteOperationCount => operations.FiniteOperationCount;

        public int InfiniteOperationCount => operations.InfiniteOperationCount;

        internal uint PendingBatchCount =>
            checked((uint)batches.Count(batch => batch.Outcome == BatchOutcome.Pending));

        internal uint BlockingOperationCount =>
            checked(
                (uint)
                    batches
                        .Where(batch => batch.Outcome == BatchOutcome.Pending)
                        .Sum(batch => batch.BlockingOperations.Count)
            );

        internal uint PausedScopeCount => checked((uint)pausedScopes.Count);

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
            pausedScopes.Clear();
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
            if (batch.PresentationControl is PresentationControl control)
            {
                ApplyPresentationControl(control);
                batch.Dispose();
                ActivityVersion++;
                Advance();
                return;
            }
            if (batch.CancelScope is ulong canceled)
                CancelScope(canceled);
            if (batch.WorkScope is ulong owner && canceledScopes.Contains(owner))
            {
                batch.Dispose();
                return;
            }
            var scheduled = new ScheduledBatch(sessionId, batch, admission);
            batches.Add(scheduled);
            ActivityVersion++;
#if UNITY_EDITOR || DEVELOPMENT_BUILD || BATTLEMENT_DITTO_DIAGNOSTICS
            if (FindUnorderedUiReference(scheduled) is string message)
                Fail(scheduled, CoreErrorCode.UnknownObject, message);
#endif
            Advance();
        }

#if UNITY_EDITOR || DEVELOPMENT_BUILD || BATTLEMENT_DITTO_DIAGNOSTICS
        private string? FindUnorderedUiReference(ScheduledBatch scheduled)
        {
            if (scheduled.IsCancellation)
                return null;
            return BattlementBatchOrdering.FindUnorderedReference(
                scheduled.Batch,
                batches
                    .Where(earlier =>
                        earlier != scheduled && earlier.Outcome == BatchOutcome.Pending
                    )
                    .Where(earlier => !earlier.IsCancellation && !IsDependency(scheduled, earlier))
                    .Select(earlier => (earlier.Batch, earlier.NextGroup)),
                executor.LiveUiSubtree
            );
        }
#endif

        private void CancelScope(ulong scope)
        {
            canceledScopes.Add(scope);
            pausedScopes.Remove(scope);
            operations.CancelScope(scope);
            executor.CancelScope(scope);
            foreach (ScheduledBatch batch in batches.Where(item => item.WorkScope == scope))
            {
                batch.BlockingOperations.Clear();
                batch.Outcome = BatchOutcome.Canceled;
                batch.Dispose();
            }
        }

        private void ApplyPresentationControl(PresentationControl control)
        {
            if (canceledScopes.Contains(control.WorkScope))
                return;
            if (control.Paused)
            {
                if (!pausedScopes.TryGetValue(control.WorkScope, out HashSet<Guid>? owners))
                {
                    owners = new HashSet<Guid>();
                    pausedScopes.Add(control.WorkScope, owners);
                }
                if (owners.Add(control.OwnerId.Value) && owners.Count == 1)
                {
                    operations.PauseScope(control.WorkScope, clock.Elapsed);
                    executor.PauseScope(control.WorkScope);
                }
                return;
            }
            if (
                pausedScopes.TryGetValue(control.WorkScope, out HashSet<Guid>? current)
                && current.Remove(control.OwnerId.Value)
                && current.Count == 0
            )
            {
                pausedScopes.Remove(control.WorkScope);
                operations.ResumeScope(control.WorkScope, clock.Elapsed);
                executor.ResumeScope(control.WorkScope);
            }
        }

        public void Advance()
        {
            if (isAdvancing)
            {
                return;
            }

            isAdvancing = true;
            bool commitStarted = false;
            try
            {
                if (HasRunnableWork)
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
                        bool progressed = AdvanceBatch(batch, now, ref commitStarted);
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
                if (commitStarted)
                    executor.EndBatch();
                isAdvancing = false;
            }
        }

        private bool AdvanceBatch(ScheduledBatch scheduled, TimeSpan now, ref bool commitStarted)
        {
            if (scheduled.Outcome != BatchOutcome.Pending)
            {
                return false;
            }

            if (IsPaused(scheduled.WorkScope))
                return false;

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
                scheduled.Dispose();
                return true;
            }

            // Suspend input only while applying commands, never while playback is waiting.
            if (!commitStarted)
            {
                executor.BeginBatch();
                commitStarted = true;
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

                    if (command.IsBlocking && !operation.IsInfinite)
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

        private bool IsPaused(ulong? scope) =>
            scope is ulong value
            && pausedScopes.TryGetValue(value, out HashSet<Guid>? owners)
            && owners.Count != 0;

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
                return earlier.ContainsAssetPreparation
                    || earlier.Start == BatchStart.AfterEarlierAssetPreparation;
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
                    batch.ContainsAssetPreparation
                    && IsDependency(scheduled, batch)
                    && batch.Outcome == BatchOutcome.Failed
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
