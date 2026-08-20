#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;

namespace Masonry
{
    /// <summary>Advances admitted batches against the host's monotonic clock.</summary>
    internal sealed class MasonryBatchScheduler
    {
        private readonly List<ScheduledBatch> batches = new();
        private readonly List<IMasonryCommandOperation> nonblockingOperations = new();
        private readonly IMasonryClock clock;
        private readonly MasonryCommandExecutor executor;
        private readonly Action<BatchFailed<CoreErrorCode>> reportFailure;
        private bool isAdvancing;

        public MasonryBatchScheduler(
            IMasonryClock clock,
            MasonryCommandExecutor executor,
            Action<BatchFailed<CoreErrorCode>> reportFailure
        )
        {
            this.clock = clock;
            this.executor = executor;
            this.reportFailure = reportFailure;
        }

        public void BeginSession()
        {
            batches.Clear();
            nonblockingOperations.Clear();
        }

        public void Schedule(
            SessionId sessionId,
            Batch<Command> batch,
            MasonryBatchAdmissionResult admission
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
                nonblockingOperations.RemoveAll(operation => operation.IsComplete(now));
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
            scheduled.BlockingOperations.RemoveAll(operation => operation.IsComplete(now));
            if (scheduled.BlockingOperations.Count > 0)
            {
                return previousBlockingCount != scheduled.BlockingOperations.Count;
            }

            if (scheduled.NextGroup >= scheduled.Batch.Groups.Count)
            {
                scheduled.Outcome = BatchOutcome.Succeeded;
                return true;
            }

            ParallelCommandGroup<Command> group = scheduled.Batch.Groups[scheduled.NextGroup++];
            foreach (Command command in group.Commands)
            {
                try
                {
                    IMasonryCommandOperation? operation = executor.Launch(command, now);
                    if (operation is null)
                    {
                        continue;
                    }

                    if (command.IsBlocking)
                    {
                        scheduled.BlockingOperations.Add(operation);
                    }
                    else
                    {
                        nonblockingOperations.Add(operation);
                    }
                }
                catch (MasonryCommandException exception)
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
                Batch<Command> batch,
                MasonryBatchAdmissionResult admission
            )
            {
                SessionId = sessionId;
                Batch = batch;
                Admission = admission;
            }

            public SessionId SessionId { get; }

            public Batch<Command> Batch { get; }

            public MasonryBatchAdmissionResult Admission { get; }

            public List<IMasonryCommandOperation> BlockingOperations { get; } = new();

            public bool HasStarted { get; set; }

            public int NextGroup { get; set; }

            public BatchOutcome Outcome { get; set; }
        }
    }
}
