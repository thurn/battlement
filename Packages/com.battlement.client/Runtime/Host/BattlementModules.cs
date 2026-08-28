#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;

namespace Battlement
{
    /// <summary>Serialized opt-in capability owned by a Battlement runner.</summary>
    public abstract class BattlementModule : ScriptableObject
    {
        /// <summary>Stable namespaced module identifier.</summary>
        public abstract string ModuleId { get; }

        /// <summary>Creates one isolated runtime clone for a session.</summary>
        public abstract IBattlementModuleRuntime Prepare();
    }

    /// <summary>Per-session runtime clone for a selected module.</summary>
    public interface IBattlementModuleRuntime : IDisposable
    {
        /// <summary>Stable namespaced module identifier.</summary>
        string ModuleId { get; }
    }

    /// <summary>Runtime contract implemented by the Diagnostics service module.</summary>
    public interface IBattlementDiagnosticsRuntime : IBattlementModuleRuntime
    {
        /// <summary>Executes one synchronous Diagnostics command.</summary>
        void Execute(DiagnosticsCommand command);
    }

    /// <summary>Stable module execution failure consumed by the core batch pipeline.</summary>
    public sealed class BattlementModuleException : InvalidOperationException
    {
        public BattlementModuleException(
            CoreErrorCode errorCode,
            string message,
            Exception? innerException = null
        )
            : base(message, innerException) => ErrorCode = errorCode;

        public CoreErrorCode ErrorCode { get; }
    }

    internal sealed class BattlementModules : IDisposable
    {
        private readonly IReadOnlyList<BattlementModule> selected;
        private readonly List<IBattlementModuleRuntime> runtimes = new();

        public BattlementModules(IReadOnlyList<BattlementModule> selected) =>
            this.selected = selected;

        public IReadOnlyList<string> ModuleIds =>
            runtimes.Select(runtime => runtime.ModuleId).ToArray();

        public void Prepare()
        {
            DisposeRuntimes();
            var ids = new HashSet<string>(StringComparer.Ordinal);
            foreach (BattlementModule asset in selected)
            {
                if (asset == null)
                    throw new InvalidOperationException("A selected Battlement module is missing.");
                if (string.IsNullOrWhiteSpace(asset.ModuleId) || !ids.Add(asset.ModuleId))
                    throw new InvalidOperationException("Battlement module IDs must be unique.");
                IBattlementModuleRuntime runtime = asset.Prepare();
                if (runtime is null || runtime.ModuleId != asset.ModuleId)
                {
                    runtime?.Dispose();
                    throw new InvalidOperationException(
                        "A Battlement module returned an invalid runtime clone."
                    );
                }
                runtimes.Add(runtime);
            }
        }

        public void Execute(DiagnosticsCommand command)
        {
            IBattlementDiagnosticsRuntime? runtime = runtimes
                .OfType<IBattlementDiagnosticsRuntime>()
                .FirstOrDefault(value => value.ModuleId == "battlement.diagnostics");
            if (runtime is null)
            {
                throw new BattlementModuleException(
                    CoreErrorCode.ModuleUnavailable,
                    "No selected Diagnostics module owns the command."
                );
            }
            runtime.Execute(command);
        }

        public void Dispose() => DisposeRuntimes();

        private void DisposeRuntimes()
        {
            for (int index = runtimes.Count - 1; index >= 0; index--)
            {
                runtimes[index].Dispose();
            }
            runtimes.Clear();
        }
    }
}
