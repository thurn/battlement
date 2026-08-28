#nullable enable

using System;
using System.Collections.Generic;
using Battlement.Cloud.Diagnostics;
using NUnit.Framework;

namespace Battlement.Tests
{
    public sealed class BattlementDiagnosticsTests
    {
        [Test]
        public void PreparationAppliesSerializedConfigurationInOrder()
        {
            var backend = new FakeBackend();
            using var runtime = new BattlementDiagnosticsRuntime(backend, false, 20);

            Assert.That(backend.Writes, Is.EqualTo(new[] { "capture=False", "buffer=20" }));
        }

        [Test]
        public void MetadataCommandSetsAndClearsOneKey()
        {
            var backend = new FakeBackend();
            using var runtime = new BattlementDiagnosticsRuntime(backend, true, 10);

            runtime.Execute(new DiagnosticsCommand.SetMetadata("battlement.scene", "castle"));
            runtime.Execute(new DiagnosticsCommand.SetMetadata("battlement.scene"));

            Assert.That(
                backend.Writes,
                Is.EqualTo(
                    new[]
                    {
                        "capture=True",
                        "buffer=10",
                        "metadata=battlement.scene:castle",
                        "metadata=battlement.scene:<clear>",
                    }
                )
            );
        }

        [Test]
        public void InvalidMetadataIsRejectedBeforeUnity()
        {
            var backend = new FakeBackend();
            using var runtime = new BattlementDiagnosticsRuntime(backend, true, 10);

            BattlementModuleException? error = Assert.Throws<BattlementModuleException>(() =>
                runtime.Execute(new DiagnosticsCommand.SetMetadata(" key", "value"))
            );

            Assert.That(error!.ErrorCode, Is.EqualTo(CoreErrorCode.DiagnosticsMetadataInvalid));
            Assert.That(backend.Writes, Has.Count.EqualTo(2));
        }

        [Test]
        public void UnityMetadataFailureUsesTheNormalCommandFailurePath()
        {
            var backend = new FakeBackend { FailMetadata = true };
            using var runtime = new BattlementDiagnosticsRuntime(backend, true, 10);

            BattlementModuleException? error = Assert.Throws<BattlementModuleException>(() =>
                runtime.Execute(new DiagnosticsCommand.SetMetadata("key", "value"))
            );

            Assert.That(error!.ErrorCode, Is.EqualTo(CoreErrorCode.DiagnosticsOperationFailed));
        }

        private sealed class FakeBackend : IDiagnosticsBackend
        {
            public List<string> Writes { get; } = new();
            public bool FailMetadata { get; init; }

            public bool CaptureExceptions
            {
                set => Writes.Add($"capture={value}");
            }

            public uint LogBufferSize
            {
                set => Writes.Add($"buffer={value}");
            }

            public void SetMetadata(string key, string? value)
            {
                if (FailMetadata)
                    throw new InvalidOperationException("injected metadata failure");
                Writes.Add($"metadata={key}:{value ?? "<clear>"}");
            }
        }
    }
}
