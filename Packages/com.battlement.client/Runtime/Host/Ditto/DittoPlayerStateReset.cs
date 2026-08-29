#nullable enable

using System;

namespace Battlement
{
    internal sealed record DittoPlayerResetFailure(DittoBoundaryStage Stage, string Diagnostic);

    internal sealed class DittoPlayerStateReset
    {
        private static readonly TimeSpan Timeout = TimeSpan.FromSeconds(10);

        private readonly BattlementRunner runner;
        private readonly DittoNativeEngineSession? engine;
        private readonly DittoVirtualInput? input;
        private readonly Func<TimeSpan> now;
        private readonly Action<BattlementTransportResult> engineDestroyed;
        private TimeSpan startedAt;
        private TimeSpan completedAt;
        private bool started;

        public DittoPlayerStateReset(
            BattlementRunner runner,
            DittoNativeEngineSession? engine,
            Func<TimeSpan> currentTime,
            DittoVirtualInput? input = null,
            Action<BattlementTransportResult>? onEngineDestroyed = null
        )
        {
            if (runner == null)
            {
                throw new ArgumentNullException(nameof(runner));
            }

            this.runner = runner;
            this.engine = engine;
            this.input = input;
            now = currentTime ?? throw new ArgumentNullException(nameof(currentTime));
            engineDestroyed = onEngineDestroyed ?? (_ => { });
        }

        public bool IsComplete { get; private set; }

        public bool IsReusable => IsComplete && Failure is null;

        public DittoPlayerResetFailure? Failure { get; private set; }

        public ulong DurationMs =>
            !started
                ? 0
                : checked(
                    (ulong)
                        Math.Floor(
                            Math.Max(
                                0,
                                ((IsComplete ? completedAt : now()) - startedAt).TotalMilliseconds
                            )
                        )
                );

        public void Begin()
        {
            if (started)
            {
                return;
            }

            started = true;
            startedAt = now();
            if (input?.HeldInputDiagnostic() is string inputDiagnostic)
            {
                Fail(DittoBoundaryStage.Reset, inputDiagnostic);
            }
            input?.Dispose();
            if (engine is not null)
            {
                BattlementTransportResult result = engine.Destroy();
                engineDestroyed(result);
                if (result.Status != BattlementTransportStatus.Success)
                {
                    Fail(
                        DittoBoundaryStage.Destroy,
                        result.Diagnostic ?? $"Engine destruction returned {result.Status}."
                    );
                }
            }

            try
            {
                runner.BeginDittoReset();
            }
            catch (Exception exception)
            {
                Fail(DittoBoundaryStage.Reset, exception.Message);
            }

            Advance();
        }

        public bool Advance()
        {
            if (!started)
            {
                throw new InvalidOperationException("The player reset has not started.");
            }
            if (IsComplete)
            {
                return true;
            }

            try
            {
                if (runner.TryCompleteDittoReset(out Exception? error))
                {
                    if (error is not null)
                    {
                        Fail(DittoBoundaryStage.Reset, error.Message);
                    }
                    IsComplete = true;
                    completedAt = now();
                    return true;
                }
            }
            catch (Exception exception)
            {
                Fail(DittoBoundaryStage.Reset, exception.Message);
                IsComplete = true;
                completedAt = now();
                return true;
            }

            if (now() - startedAt < Timeout)
            {
                return false;
            }

            Fail(DittoBoundaryStage.Reset, "Battlement-owned state reset exceeded 10 seconds.");
            IsComplete = true;
            completedAt = now();
            return true;
        }

        private void Fail(DittoBoundaryStage stage, string diagnostic) =>
            Failure ??= new DittoPlayerResetFailure(stage, diagnostic);
    }
}
