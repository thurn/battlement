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
        private TimeSpan startedAt;
        private bool started;

        public DittoPlayerStateReset(
            BattlementRunner runner,
            DittoNativeEngineSession? engine,
            Func<TimeSpan> currentTime,
            DittoVirtualInput? input = null
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
        }

        public bool IsComplete { get; private set; }

        public bool IsReusable => IsComplete && Failure is null;

        public DittoPlayerResetFailure? Failure { get; private set; }

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
                    return true;
                }
            }
            catch (Exception exception)
            {
                Fail(DittoBoundaryStage.Reset, exception.Message);
                IsComplete = true;
                return true;
            }

            if (now() - startedAt < Timeout)
            {
                return false;
            }

            Fail(DittoBoundaryStage.Reset, "Battlement-owned state reset exceeded 10 seconds.");
            IsComplete = true;
            return true;
        }

        private void Fail(DittoBoundaryStage stage, string diagnostic) =>
            Failure ??= new DittoPlayerResetFailure(stage, diagnostic);
    }
}
