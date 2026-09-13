#nullable enable

using System;
using System.Buffers;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Text;
using System.Threading;

namespace Battlement
{
    /// <summary>Current ownership accounting for one native transport instance.</summary>
    public readonly struct BattlementNativeTransportDiagnostics
    {
        internal BattlementNativeTransportDiagnostics(
            int generation,
            int liveBufferCount,
            long liveAllocationBytes,
            int pendingFinalizerReleases,
            ulong buildersCreated,
            ulong buildersReused,
            ulong builderGrowths,
            ulong builderCopiedBytes,
            ulong idleBuilderBytes,
            ulong handoffPayloadCopies,
            ulong clientBuilderGrowths,
            ulong clientBuilderCopiedBytes,
            int clientBuilderRetainedBytes
        ) =>
            (
                Generation,
                LiveBufferCount,
                LiveAllocationBytes,
                PendingFinalizerReleases,
                BuildersCreated,
                BuildersReused,
                BuilderGrowths,
                BuilderCopiedBytes,
                IdleBuilderBytes,
                HandoffPayloadCopies,
                ClientBuilderGrowths,
                ClientBuilderCopiedBytes,
                ClientBuilderRetainedBytes
            ) = (
                generation,
                liveBufferCount,
                liveAllocationBytes,
                pendingFinalizerReleases,
                buildersCreated,
                buildersReused,
                builderGrowths,
                builderCopiedBytes,
                idleBuilderBytes,
                handoffPayloadCopies,
                clientBuilderGrowths,
                clientBuilderCopiedBytes,
                clientBuilderRetainedBytes
            );

        public int Generation { get; }
        public int LiveBufferCount { get; }
        public long LiveAllocationBytes { get; }
        public int PendingFinalizerReleases { get; }
        public ulong BuildersCreated { get; }
        public ulong BuildersReused { get; }
        public ulong BuilderGrowths { get; }
        public ulong BuilderCopiedBytes { get; }
        public ulong IdleBuilderBytes { get; }
        public ulong HandoffPayloadCopies { get; }
        public ulong ClientBuilderGrowths { get; }
        public ulong ClientBuilderCopiedBytes { get; }
        public int ClientBuilderRetainedBytes { get; }
    }

    /// <summary>Synchronous main-thread transport for the fixed Battlement native ABI.</summary>
    public sealed class BattlementNativeTransport : IBattlementTransport
    {
        public const int MaximumPayloadBytes = 16 * 1024 * 1024;
        internal const int MaximumBufferAllocationBytes = 64 * 1024 * 1024;

        private const int Ok = 0;
        private const int NoMessage = 1;
        private const int InvalidArgument = 2;
        private const int EngineError = 3;
        private const int Panic = 4;

        private static readonly UTF8Encoding StrictUtf8 = new(false, true);

        private readonly object callGate = new();
        private readonly BattlementConnectRequestWriter connectRequests = new();
        private readonly BattlementCoreClientMessageWriter coreRequests = new();
        private readonly BattlementUiEventRequestWriter uiEventRequests = new();
        private readonly Dictionary<ulong, LiveBuffer> liveBuffers = new();
        private readonly ConcurrentQueue<ulong> finalizerReleases = new();
        private readonly int owningThreadId;
        private long liveAllocationBytes;
        private ulong clientBuilderGrowths;
        private ulong clientBuilderCopiedBytes;
        private int generation = 1;
        private ulong engine;
        private string expectedWireContractDigest = BattlementNativeContract.WireContractDigest;
        private bool contractVerified;
        private bool isDisposed;

        public BattlementNativeTransport()
        {
            owningThreadId = Thread.CurrentThread.ManagedThreadId;
        }

        /// <summary>The plugin filename required on the current Unity target.</summary>
        public static string RequiredPluginName
        {
            get
            {
#if (UNITY_IOS || UNITY_WEBGL) && !UNITY_EDITOR
                return "__Internal";
#elif UNITY_STANDALONE_WIN || UNITY_EDITOR_WIN
                return "battlement_rules.dll";
#elif UNITY_STANDALONE_OSX || UNITY_EDITOR_OSX
                return "libbattlement_rules.dylib";
#elif UNITY_ANDROID
                return "libbattlement_rules.so";
#else
                throw new PlatformNotSupportedException(
                    "The current platform is not a supported Battlement native target."
                );
#endif
            }
        }

        /// <summary>The most recent connect result, if connect has been called.</summary>
        public BattlementTransportResult? LastConnectResult { get; private set; }

        /// <summary>Returns live response ownership and deferred-release accounting.</summary>
        public BattlementNativeTransportDiagnostics Diagnostics
        {
            get
            {
                lock (callGate)
                {
                    if (isDisposed)
                        throw new ObjectDisposedException(nameof(BattlementNativeTransport));
                    RequireOwningThread();
                    DrainFinalizerReleases();
                    int status = BattlementNativeMethods.battlement_transport_diagnostics(
                        out ulong buildersCreated,
                        out ulong buildersReused,
                        out ulong builderGrowths,
                        out ulong builderCopiedBytes,
                        out ulong idleBuilderBytes,
                        out ulong handoffPayloadCopies
                    );
                    if (status != Ok)
                        throw new InvalidOperationException("Native transport diagnostics failed.");
                    return new BattlementNativeTransportDiagnostics(
                        generation,
                        liveBuffers.Count,
                        liveAllocationBytes,
                        finalizerReleases.Count,
                        buildersCreated,
                        buildersReused,
                        builderGrowths,
                        builderCopiedBytes,
                        idleBuilderBytes,
                        handoffPayloadCopies,
                        clientBuilderGrowths,
                        clientBuilderCopiedBytes,
                        checked(
                            connectRequests.AllocationBytes
                            + coreRequests.AllocationBytes
                            + uiEventRequests.AllocationBytes
                        )
                    );
                }
            }
        }

        internal bool HasEngine
        {
            get
            {
                lock (callGate)
                {
                    return engine != 0;
                }
            }
        }

        internal void SetExpectedWireContractDigest(string digest)
        {
            if (digest is null)
                throw new ArgumentNullException(nameof(digest));
            if (
                digest.Length != 64
                || Array.Exists(
                    digest.ToCharArray(),
                    value => !(value is >= '0' and <= '9' or >= 'a' and <= 'f')
                )
            )
            {
                throw new ArgumentException(
                    "A wire contract digest must be 64 lowercase hexadecimal characters.",
                    nameof(digest)
                );
            }

            lock (callGate)
            {
                if (contractVerified || engine != 0)
                {
                    throw new InvalidOperationException(
                        "The native wire contract cannot change after engine initialization."
                    );
                }
                expectedWireContractDigest = digest;
            }
        }

        public BattlementTransportResult Connect(ReadOnlyMemory<byte> message)
        {
            lock (callGate)
            {
                BattlementTransportResult? rejected = RejectCall();
                if (rejected is not null)
                {
                    return LastConnectResult = rejected;
                }

                BeginGeneration();
                BattlementTransportResult? creation = EnsureEngine();
                if (creation is not null)
                {
                    return LastConnectResult = creation;
                }

                LastConnectResult = InvokeRequest(
                    message,
                    BattlementNativeMethods.battlement_connect
                );
                return LastConnectResult;
            }
        }

        internal BattlementTransportResult Connect(Connect value) => Connect(WriteConnect(value));

        internal ReadOnlyMemory<byte> WriteConnect(Connect value)
        {
            int before = connectRequests.AllocationBytes;
            bool completed = false;
            try
            {
                ReadOnlyMemory<byte> result = connectRequests.Write(value);
                completed = true;
                return result;
            }
            finally
            {
                RecordClientBuilderGrowth(before, connectRequests.AllocationBytes);
                if (!completed)
                    connectRequests.TrimOversized();
            }
        }

        public BattlementTransportResult Submit(ReadOnlyMemory<byte> message)
        {
            lock (callGate)
            {
                BattlementTransportResult? rejected = RejectCall();
                if (rejected is not null)
                {
                    return rejected;
                }

                if (engine == 0)
                {
                    return AbiError("Connect must create the native engine before submit.");
                }

                return InvokeRequest(message, BattlementNativeMethods.battlement_submit);
            }
        }

        internal BattlementTransportResult Submit(Action action) => Submit(WriteCore(action));

        internal ReadOnlyMemory<byte> WriteCore(Action action)
        {
            int before = coreRequests.AllocationBytes;
            bool completed = false;
            try
            {
                ReadOnlyMemory<byte> result = coreRequests.WriteAction(action);
                completed = true;
                return result;
            }
            finally
            {
                RecordClientBuilderGrowth(before, coreRequests.AllocationBytes);
                if (!completed)
                    coreRequests.TrimOversized();
            }
        }

        internal BattlementTransportResult Submit(BatchFailed<CoreErrorCode> failure) =>
            Submit(WriteCore(failure));

        internal BattlementTransportResult Submit(OperationFailed<CoreErrorCode> failure) =>
            Submit(WriteCore(failure));

        private ReadOnlyMemory<byte> WriteCore(BatchFailed<CoreErrorCode> failure)
        {
            int before = coreRequests.AllocationBytes;
            bool completed = false;
            try
            {
                ReadOnlyMemory<byte> result = coreRequests.WriteBatchFailure(failure);
                completed = true;
                return result;
            }
            finally
            {
                RecordClientBuilderGrowth(before, coreRequests.AllocationBytes);
                if (!completed)
                    coreRequests.TrimOversized();
            }
        }

        private ReadOnlyMemory<byte> WriteCore(OperationFailed<CoreErrorCode> failure)
        {
            int before = coreRequests.AllocationBytes;
            bool completed = false;
            try
            {
                ReadOnlyMemory<byte> result = coreRequests.WriteOperationFailure(failure);
                completed = true;
                return result;
            }
            finally
            {
                RecordClientBuilderGrowth(before, coreRequests.AllocationBytes);
                if (!completed)
                    coreRequests.TrimOversized();
            }
        }

        public unsafe BattlementUiEventTransportResult SubmitUiEvent(ReadOnlyMemory<byte> message)
        {
            lock (callGate)
            {
                BattlementTransportResult? rejected = RejectCall();
                if (rejected is not null)
                {
                    return UiFailure(rejected);
                }
                if (engine == 0)
                {
                    return UiFailure(
                        AbiError("Connect must create the native engine before UI submission.")
                    );
                }

                ulong output = 0;
                try
                {
                    using MemoryHandle pinned = message.Pin();
                    int status = BattlementNativeMethods.battlement_submit_ui_event(
                        engine,
                        new IntPtr(pinned.Pointer),
                        checked((ulong)message.Length),
                        out uint disposition,
                        out output
                    );
                    BattlementNativeLogging.Drain();
                    BattlementUiEventTransportResult result = TranslateUiEvent(
                        status,
                        disposition,
                        Inspect(output)
                    );
                    if (result.PayloadOwner is not null)
                        output = 0;
                    if (result.Status == BattlementTransportStatus.Panic)
                    {
                        _ = DestroyEngine();
                    }
                    return result;
                }
                catch (Exception exception)
                {
                    return UiFailure(ManagedFailure(exception));
                }
                finally
                {
                    Release(output);
                    TrimOversizedRequestBuilders();
                }
            }
        }

        internal BattlementUiEventTransportResult SubmitUiEvent(UiEventAction action) =>
            SubmitUiEvent(WriteUiEvent(action));

        private ReadOnlyMemory<byte> WriteUiEvent(UiEventAction action)
        {
            int before = uiEventRequests.AllocationBytes;
            bool completed = false;
            try
            {
                ReadOnlyMemory<byte> result = uiEventRequests.Write(action);
                completed = true;
                return result;
            }
            finally
            {
                RecordClientBuilderGrowth(before, uiEventRequests.AllocationBytes);
                if (!completed)
                    uiEventRequests.TrimOversized();
            }
        }

        public BattlementTransportResult Poll()
        {
            lock (callGate)
            {
                BattlementTransportResult? rejected = RejectCall();
                if (rejected is not null)
                {
                    return rejected;
                }

                if (engine == 0)
                {
                    return AbiError("Connect must create the native engine before poll.");
                }

                ulong output = 0;
                try
                {
                    int status = BattlementNativeMethods.battlement_poll(engine, out output);
                    BattlementNativeLogging.Drain();
                    BattlementTransportResult result = Translate(status, Inspect(output), true);
                    if (result.PayloadOwner is not null)
                        output = 0;
                    if (result.Status == BattlementTransportStatus.Panic)
                    {
                        _ = DestroyEngine();
                    }

                    return result;
                }
                catch (Exception exception)
                {
                    return ManagedFailure(exception);
                }
                finally
                {
                    Release(output);
                }
            }
        }

        public void Stop()
        {
            lock (callGate)
            {
                RequireOwningThread();
                BeginGeneration();
            }
        }

        internal BattlementTransportResult CreateDittoEngine()
        {
            lock (callGate)
            {
                BattlementTransportResult? rejected = RejectCall();
                if (rejected is not null)
                {
                    return rejected;
                }
                if (engine != 0)
                {
                    return AbiError("A native engine is already active for another scenario.");
                }

                return EnsureEngine()
                    ?? new BattlementTransportResult(BattlementTransportStatus.Success);
            }
        }

        internal bool SupportsDittoDeterminism
        {
            get
            {
                try
                {
                    const ulong requiredCapabilities = 0b11_1111;
                    return BattlementNativeMethods.battlement_ditto_determinism_contract() == 3
                        && BattlementNativeMethods.battlement_ditto_determinism_capabilities()
                            == requiredCapabilities;
                }
                catch (EntryPointNotFoundException)
                {
                    return false;
                }
            }
        }

        internal BattlementTransportResult ConnectDittoEngine(ReadOnlyMemory<byte> message)
        {
            lock (callGate)
            {
                BattlementTransportResult? rejected = RejectCall();
                if (rejected is not null)
                {
                    return LastConnectResult = rejected;
                }
                if (engine == 0)
                {
                    return LastConnectResult = AbiError(
                        "A Ditto scenario must create a fresh native engine before connect."
                    );
                }

                BeginGeneration();
                return LastConnectResult = InvokeRequest(
                    message,
                    BattlementNativeMethods.battlement_connect
                );
            }
        }

        internal BattlementTransportResult ConnectDittoEngine(Connect value)
        {
            ReadOnlyMemory<byte> request = WriteConnect(value);
            return ConnectDittoEngine(request);
        }

        internal BattlementTransportResult DestroyDittoEngine()
        {
            lock (callGate)
            {
                BattlementTransportResult? rejected = RejectCall();
                return rejected ?? DestroyEngine();
            }
        }

        public void Dispose()
        {
            lock (callGate)
            {
                if (isDisposed)
                {
                    return;
                }

                RequireOwningThread();
                _ = DestroyEngine();

                isDisposed = true;
            }
        }

        private BattlementTransportResult? EnsureEngine()
        {
            if (engine != 0)
            {
                return null;
            }

            ulong createdEngine = 0;
            ulong output = 0;
            bool destroyCreatedEngine = false;
            try
            {
                if (!contractVerified)
                {
                    BattlementNativeContract.Verify(expectedWireContractDigest);
                    contractVerified = true;
                }
                int status = BattlementNativeMethods.battlement_engine_create(
                    out createdEngine,
                    out output
                );
                BattlementNativeLogging.Drain();
                destroyCreatedEngine = createdEngine != 0;
                BattlementTransportResult result = Translate(
                    status,
                    Inspect(output),
                    false,
                    creation: true,
                    createdEngine
                );
                if (result.Status != BattlementTransportStatus.Success)
                {
                    return result;
                }

                engine = createdEngine;
                destroyCreatedEngine = false;
                return null;
            }
            catch (Exception exception)
            {
                return ManagedFailure(exception);
            }
            finally
            {
                Release(output);
                if (destroyCreatedEngine)
                {
                    _ = DestroyNativeEngine(createdEngine);
                }
            }
        }

        private unsafe BattlementTransportResult InvokeRequest(
            ReadOnlyMemory<byte> message,
            NativeRequest request
        )
        {
            ulong output = 0;
            try
            {
                using MemoryHandle pinned = message.Pin();
                int status = request(
                    engine,
                    new IntPtr(pinned.Pointer),
                    checked((ulong)message.Length),
                    out output
                );
                BattlementNativeLogging.Drain();
                BattlementTransportResult result = Translate(status, Inspect(output), false);
                if (result.PayloadOwner is not null)
                    output = 0;
                if (result.Status == BattlementTransportStatus.Panic)
                {
                    _ = DestroyEngine();
                }

                return result;
            }
            catch (Exception exception)
            {
                return ManagedFailure(exception);
            }
            finally
            {
                Release(output);
                TrimOversizedRequestBuilders();
            }
        }

        private BattlementTransportResult Translate(
            int status,
            BattlementNativeBuffer output,
            bool isPoll,
            bool creation = false,
            ulong createdEngine = default
        )
        {
            string? shapeError = output.ValidateShape(
                MaximumPayloadBytes,
                MaximumBufferAllocationBytes
            );
            if (shapeError is not null)
            {
                return AbiError(shapeError, status);
            }

            if (status == Ok)
            {
                if (creation)
                {
                    return createdEngine == 0 || output.Length != 0
                        ? AbiError("Native create returned an invalid success value.", status)
                        : new BattlementTransportResult(BattlementTransportStatus.Success);
                }

                if (output.Length == 0)
                {
                    return AbiError("Native success returned an empty response.", status);
                }

                return Adopt(output, status);
            }

            if (status == NoMessage)
            {
                return isPoll && output.Length == 0
                    ? new BattlementTransportResult(
                        BattlementTransportStatus.NoMessage,
                        nativeStatus: status
                    )
                    : AbiError("NO_MESSAGE is valid only for an empty poll result.", status);
            }

            string? diagnostic = DecodeDiagnostic(output, out string? decodeError);
            if (decodeError is not null)
            {
                return AbiError(decodeError, status);
            }

            BattlementTransportStatus mapped = status switch
            {
                InvalidArgument => BattlementTransportStatus.InvalidArgument,
                EngineError => BattlementTransportStatus.EngineError,
                Panic => BattlementTransportStatus.Panic,
                _ => BattlementTransportStatus.AbiError,
            };
            return new BattlementTransportResult(
                mapped,
                diagnostic: diagnostic,
                nativeStatus: status
            );
        }

        private BattlementUiEventTransportResult TranslateUiEvent(
            int status,
            uint disposition,
            BattlementNativeBuffer output
        )
        {
            UiEventDisposition validated = disposition switch
            {
                0 => UiEventDisposition.Continue,
                1 => UiEventDisposition.PreventDefault,
                _ when status == Ok => throw new InvalidOperationException(
                    $"Native UI event returned unknown disposition {disposition}."
                ),
                _ => UiEventDisposition.Continue,
            };
            using BattlementTransportResult translated = Translate(status, output, false);
            if (translated.Status != BattlementTransportStatus.Success)
            {
                return UiFailure(translated);
            }
            return new BattlementUiEventTransportResult(
                BattlementTransportStatus.Success,
                validated,
                translated.BorrowedPayload,
                nativeStatus: status
            ).OwnPayload(translated.DetachPayloadOwner());
        }

        private BattlementTransportResult? RejectCall()
        {
            if (isDisposed)
            {
                throw new ObjectDisposedException(nameof(BattlementNativeTransport));
            }

            try
            {
                RequireOwningThread();
                DrainFinalizerReleases();
            }
            catch (InvalidOperationException exception)
            {
                return ManagedFailure(exception);
            }

            return null;
        }

        private void RequireOwningThread()
        {
            if (Thread.CurrentThread.ManagedThreadId != owningThreadId)
            {
                throw new InvalidOperationException(
                    "Native transport calls must remain on their creating Unity thread."
                );
            }
        }

        private BattlementTransportResult Adopt(BattlementNativeBuffer output, int status)
        {
            long allocation = checked((long)output.AllocationBytes);
            if (liveAllocationBytes > MaximumBufferAllocationBytes - allocation)
                return AbiError(
                    "Native response allocations exceeded the 64 MiB live budget.",
                    status
                );
            var owner = new BattlementNativeBufferMemory(
                output,
                owningThreadId,
                generation,
                () => generation,
                ReleaseOwnedBuffer,
                finalizerReleases.Enqueue
            );
            liveBuffers.Add(output.Handle, new LiveBuffer(owner, allocation));
            liveAllocationBytes += allocation;
            return new BattlementTransportResult(
                BattlementTransportStatus.Success,
                owner.ReadOnlyMemory,
                nativeStatus: status
            ).OwnPayload(owner);
        }

        private void BeginGeneration()
        {
            DrainFinalizerReleases();
            generation = checked(generation + 1);
            foreach (LiveBuffer buffer in new List<LiveBuffer>(liveBuffers.Values))
            {
                if (buffer.Owner.TryGetTarget(out BattlementNativeBufferMemory owner))
                    owner.ForceRelease();
                else
                    ReleaseOwnedBuffer(buffer.Handle);
            }
        }

        private void DrainFinalizerReleases()
        {
            while (finalizerReleases.TryDequeue(out ulong handle))
                ReleaseOwnedBuffer(handle);
        }

        private void ReleaseOwnedBuffer(ulong handle)
        {
            if (!liveBuffers.TryGetValue(handle, out LiveBuffer buffer))
                return;
            liveBuffers.Remove(handle);
            liveAllocationBytes -= buffer.AllocationBytes;
            Release(handle);
        }

        private sealed class LiveBuffer
        {
            internal LiveBuffer(BattlementNativeBufferMemory owner, long allocationBytes)
            {
                Handle = owner.Handle;
                Owner = new WeakReference<BattlementNativeBufferMemory>(owner);
                AllocationBytes = allocationBytes;
            }

            internal ulong Handle { get; }
            internal WeakReference<BattlementNativeBufferMemory> Owner { get; }
            internal long AllocationBytes { get; }
        }

        private static string? DecodeDiagnostic(
            BattlementNativeBuffer output,
            out string? decodeError
        )
        {
            decodeError = null;
            if (output.Length == 0)
            {
                return null;
            }

            try
            {
                unsafe
                {
                    return StrictUtf8.GetString(
                        new ReadOnlySpan<byte>(output.Data.ToPointer(), checked((int)output.Length))
                    );
                }
            }
            catch (DecoderFallbackException exception)
            {
                decodeError = $"Native diagnostic was not UTF-8: {exception.Message}";
                return null;
            }
        }

        private static BattlementNativeBuffer Inspect(ulong handle)
        {
            if (handle == 0)
            {
                return default;
            }
            int status = BattlementNativeMethods.battlement_buffer_info(
                handle,
                out IntPtr data,
                out ulong length,
                out ulong allocationBytes
            );
            if (status != Ok)
            {
                throw new InvalidOperationException(
                    $"Native buffer inspection failed with status {status}."
                );
            }
            return new BattlementNativeBuffer(handle, data, length, allocationBytes);
        }

        private static void Release(ulong handle)
        {
            if (handle != 0 && BattlementNativeMethods.battlement_release_buffer(handle) != Ok)
            {
                throw new InvalidOperationException("Native buffer release failed.");
            }
        }

        private BattlementTransportResult DestroyEngine()
        {
            if (engine == 0)
            {
                return new BattlementTransportResult(BattlementTransportStatus.Success);
            }

            BeginGeneration();
            ulong poisoned = engine;
            engine = 0;
            return DestroyNativeEngine(poisoned);
        }

        private static BattlementTransportResult DestroyNativeEngine(ulong engineToDestroy)
        {
            ulong output = 0;
            try
            {
                int status = BattlementNativeMethods.battlement_engine_destroy(
                    engineToDestroy,
                    out output
                );
                BattlementNativeLogging.Drain();
                BattlementTransportResult result = TranslateDestroy(status, Inspect(output));
                if (result.Status == BattlementTransportStatus.Success)
                {
                    return result;
                }

                string detail =
                    result.Diagnostic
                    ?? $"Native destroy returned status {status} without a diagnostic.";
                bool panicked = result.Status == BattlementTransportStatus.Panic;
                BattlementUnityLogging.Log(
                    "rust",
                    new BattlementLogRecord(
                        BattlementLogSeverity.Error,
                        panicked
                            ? "battlement.rust.destroy_panic"
                            : "battlement.native.destroy_failed",
                        panicked
                            ? "Rust engine panicked during destruction."
                            : "Rust engine destruction failed.",
                        new System.Collections.Generic.Dictionary<string, string>
                        {
                            ["native_status"] = status.ToString(
                                System.Globalization.CultureInfo.InvariantCulture
                            ),
                        },
                        StackTrace: Errors.BattlementAnsiText.Format(detail).PlainText
                    )
                );
                return result;
            }
            catch (Exception exception)
            {
                BattlementUnityLogging.Log(
                    "battlement",
                    new BattlementLogRecord(
                        BattlementLogSeverity.Error,
                        "battlement.native.destroy_failed",
                        "Rust engine destruction failed.",
                        Exception: exception
                    )
                );
                return ManagedFailure(exception);
            }
            finally
            {
                Release(output);
            }
        }

        internal static BattlementTransportResult TranslateDestroy(
            int status,
            BattlementNativeBuffer output
        )
        {
            string? shapeError = output.ValidateShape(
                MaximumPayloadBytes,
                MaximumBufferAllocationBytes
            );
            if (shapeError is not null)
            {
                return AbiError(shapeError, status);
            }
            if (status == Ok)
            {
                return output.Length == 0
                    ? new BattlementTransportResult(
                        BattlementTransportStatus.Success,
                        nativeStatus: status
                    )
                    : AbiError("Native destroy success returned a diagnostic.", status);
            }

            string? diagnostic = DecodeDiagnostic(output, out string? decodeError);
            if (decodeError is not null)
            {
                return AbiError(decodeError, status);
            }
            if (diagnostic is null)
            {
                return AbiError(
                    $"Native destroy returned status {status} without a diagnostic.",
                    status
                );
            }

            BattlementTransportStatus mapped = status switch
            {
                InvalidArgument => BattlementTransportStatus.InvalidArgument,
                EngineError => BattlementTransportStatus.EngineError,
                Panic => BattlementTransportStatus.Panic,
                _ => BattlementTransportStatus.AbiError,
            };
            return new BattlementTransportResult(
                mapped,
                diagnostic: diagnostic,
                nativeStatus: status
            );
        }

        private void RecordClientBuilderGrowth(int before, int after)
        {
            if (after <= before)
                return;
            int allocation = before;
            while (allocation < after)
            {
                clientBuilderGrowths = checked(clientBuilderGrowths + 1);
                clientBuilderCopiedBytes = checked(clientBuilderCopiedBytes + (ulong)allocation);
                allocation = checked(allocation * 2);
            }
        }

        private void TrimOversizedRequestBuilders()
        {
            connectRequests.TrimOversized();
            coreRequests.TrimOversized();
            uiEventRequests.TrimOversized();
        }

        private static BattlementTransportResult ManagedFailure(Exception exception) =>
            AbiError($"Managed native transport failure: {exception.Message}");

        private static BattlementUiEventTransportResult UiFailure(
            BattlementTransportResult result
        ) =>
            new(
                result.Status,
                UiEventDisposition.Continue,
                ReadOnlyMemory<byte>.Empty,
                result.Diagnostic,
                result.NativeStatus
            );

        private static BattlementTransportResult AbiError(
            string diagnostic,
            int? nativeStatus = null
        ) =>
            new(
                BattlementTransportStatus.AbiError,
                diagnostic: diagnostic,
                nativeStatus: nativeStatus
            );

        private delegate int NativeRequest(
            ulong engine,
            IntPtr input,
            ulong length,
            out ulong output
        );
    }
}
