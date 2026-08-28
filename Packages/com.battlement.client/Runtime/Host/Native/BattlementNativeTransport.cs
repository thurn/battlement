#nullable enable

using System;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;

namespace Battlement
{
    /// <summary>Synchronous main-thread transport for the fixed Battlement native ABI.</summary>
    public sealed class BattlementNativeTransport : IBattlementTransport
    {
        public const int MaximumPayloadBytes = 16 * 1024 * 1024;

        private const int Ok = 0;
        private const int NoMessage = 1;
        private const int InvalidArgument = 2;
        private const int EngineError = 3;
        private const int Panic = 4;

        private static readonly UTF8Encoding StrictUtf8 = new(false, true);

        private readonly object callGate = new();
        private readonly int owningThreadId;
        private IntPtr engine;
        private bool isDisposed;

        public BattlementNativeTransport()
        {
            owningThreadId = Thread.CurrentThread.ManagedThreadId;
        }

        public BattlementTransportKind Kind => BattlementTransportKind.Native;

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

        public BattlementTransportResult Connect(ReadOnlyMemory<byte> json)
        {
            lock (callGate)
            {
                BattlementTransportResult? rejected = RejectCall();
                if (rejected is not null)
                {
                    return LastConnectResult = rejected;
                }

                BattlementTransportResult? creation = EnsureEngine();
                if (creation is not null)
                {
                    return LastConnectResult = creation;
                }

                LastConnectResult = InvokeRequest(json, BattlementNativeMethods.battlement_connect);
                return LastConnectResult;
            }
        }

        public BattlementTransportResult Submit(ReadOnlyMemory<byte> json)
        {
            lock (callGate)
            {
                BattlementTransportResult? rejected = RejectCall();
                if (rejected is not null)
                {
                    return rejected;
                }

                if (engine == IntPtr.Zero)
                {
                    return AbiError("Connect must create the native engine before submit.");
                }

                return InvokeRequest(json, BattlementNativeMethods.battlement_submit);
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

                if (engine == IntPtr.Zero)
                {
                    return AbiError("Connect must create the native engine before poll.");
                }

                BattlementNativeBuffer output = default;
                try
                {
                    int status = BattlementNativeMethods.battlement_poll(engine, out output);
                    BattlementNativeLogging.Drain();
                    BattlementTransportResult result = Translate(status, output, true);
                    if (result.Status == BattlementTransportStatus.Panic)
                    {
                        DestroyEngine();
                    }

                    return result;
                }
                catch (Exception exception)
                {
                    return ManagedFailure(exception);
                }
                finally
                {
                    Free(output);
                }
            }
        }

        public void Stop() { }

        public void Dispose()
        {
            lock (callGate)
            {
                if (isDisposed)
                {
                    return;
                }

                RequireOwningThread();
                DestroyEngine();

                isDisposed = true;
            }
        }

        private BattlementTransportResult? EnsureEngine()
        {
            if (engine != IntPtr.Zero)
            {
                return null;
            }

            IntPtr createdEngine = IntPtr.Zero;
            BattlementNativeBuffer output = default;
            bool destroyCreatedEngine = false;
            try
            {
                int status = BattlementNativeMethods.battlement_engine_create(
                    out createdEngine,
                    out output
                );
                BattlementNativeLogging.Drain();
                destroyCreatedEngine = createdEngine != IntPtr.Zero;
                BattlementTransportResult result = Translate(
                    status,
                    output,
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
                Free(output);
                if (destroyCreatedEngine)
                {
                    DestroyNativeEngine(createdEngine);
                }
            }
        }

        private BattlementTransportResult InvokeRequest(
            ReadOnlyMemory<byte> json,
            NativeRequest request
        )
        {
            byte[] synchronousInput = json.ToArray();
            BattlementNativeBuffer output = default;
            try
            {
                int status = request(
                    engine,
                    synchronousInput,
                    checked((ulong)synchronousInput.LongLength),
                    out output
                );
                BattlementNativeLogging.Drain();
                BattlementTransportResult result = Translate(status, output, false);
                if (result.Status == BattlementTransportStatus.Panic)
                {
                    DestroyEngine();
                }

                return result;
            }
            catch (Exception exception)
            {
                return ManagedFailure(exception);
            }
            finally
            {
                Free(output);
            }
        }

        private static BattlementTransportResult Translate(
            int status,
            BattlementNativeBuffer output,
            bool isPoll,
            bool creation = false,
            IntPtr createdEngine = default
        )
        {
            string? shapeError = output.ValidateShape(MaximumPayloadBytes);
            if (shapeError is not null)
            {
                return AbiError(shapeError, status);
            }

            if (status == Ok)
            {
                if (creation)
                {
                    return createdEngine == IntPtr.Zero || output.Length != 0
                        ? AbiError("Native create returned an invalid success value.", status)
                        : new BattlementTransportResult(BattlementTransportStatus.Success);
                }

                if (output.Length == 0)
                {
                    return AbiError("Native success returned an empty response.", status);
                }

                return new BattlementTransportResult(
                    BattlementTransportStatus.Success,
                    Copy(output),
                    nativeStatus: status
                );
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

        private BattlementTransportResult? RejectCall()
        {
            if (isDisposed)
            {
                throw new ObjectDisposedException(nameof(BattlementNativeTransport));
            }

            try
            {
                RequireOwningThread();
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

        private static byte[] Copy(BattlementNativeBuffer output)
        {
            var bytes = new byte[checked((int)output.Length)];
            Marshal.Copy(output.Data, bytes, 0, bytes.Length);
            return bytes;
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
                return StrictUtf8.GetString(Copy(output));
            }
            catch (DecoderFallbackException exception)
            {
                decodeError = $"Native diagnostic was not UTF-8: {exception.Message}";
                return null;
            }
        }

        private static void Free(BattlementNativeBuffer output)
        {
            if (output.Data != IntPtr.Zero && output.Length != 0)
            {
                BattlementNativeMethods.battlement_buffer_free(output);
            }
        }

        private void DestroyEngine()
        {
            if (engine == IntPtr.Zero)
            {
                return;
            }

            IntPtr poisoned = engine;
            engine = IntPtr.Zero;
            DestroyNativeEngine(poisoned);
        }

        private static void DestroyNativeEngine(IntPtr engineToDestroy)
        {
            BattlementNativeBuffer output = default;
            try
            {
                int status = BattlementNativeMethods.battlement_engine_destroy(
                    engineToDestroy,
                    out output
                );
                BattlementNativeLogging.Drain();
                string? shapeError = output.ValidateShape(MaximumPayloadBytes);
                if (status == Ok && shapeError is null && output.Length == 0)
                {
                    return;
                }

                string? diagnostic = DecodeDiagnostic(output, out string? decodeError);
                string detail =
                    shapeError
                    ?? decodeError
                    ?? diagnostic
                    ?? $"Native destroy returned status {status} without a diagnostic.";
                bool panicked = status == Panic;
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
            }
            finally
            {
                Free(output);
            }
        }

        private static BattlementTransportResult ManagedFailure(Exception exception) =>
            AbiError($"Managed native transport failure: {exception.Message}");

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
            IntPtr engine,
            byte[] json,
            ulong length,
            out BattlementNativeBuffer output
        );
    }
}
