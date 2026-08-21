#nullable enable

using System;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;

namespace Masonry
{
    /// <summary>Synchronous main-thread transport for the fixed Masonry native ABI.</summary>
    public sealed class MasonryNativeTransport : IMasonryTransport
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

        public MasonryNativeTransport() => owningThreadId = Thread.CurrentThread.ManagedThreadId;

        public MasonryTransportKind Kind => MasonryTransportKind.Native;

        /// <summary>The plugin filename required on the current Unity target.</summary>
        public static string RequiredPluginName
        {
            get
            {
#if (UNITY_IOS || UNITY_WEBGL) && !UNITY_EDITOR
                return "__Internal";
#elif UNITY_STANDALONE_WIN || UNITY_EDITOR_WIN
                return "masonry_rules.dll";
#elif UNITY_STANDALONE_OSX || UNITY_EDITOR_OSX
                return "libmasonry_rules.dylib";
#elif UNITY_ANDROID
                return "libmasonry_rules.so";
#else
                throw new PlatformNotSupportedException(
                    "The current platform is not a Masonry v1 native target."
                );
#endif
            }
        }

        /// <summary>The most recent connect result, if connect has been called.</summary>
        public MasonryTransportResult? LastConnectResult { get; private set; }

        public MasonryTransportResult Connect(ReadOnlyMemory<byte> messagePack)
        {
            lock (callGate)
            {
                MasonryTransportResult? rejected = RejectCall();
                if (rejected is not null)
                {
                    return LastConnectResult = rejected;
                }

                MasonryTransportResult? creation = EnsureEngine();
                if (creation is not null)
                {
                    return LastConnectResult = creation;
                }

                LastConnectResult = InvokeRequest(
                    messagePack,
                    MasonryNativeMethods.masonry_connect
                );
                return LastConnectResult;
            }
        }

        public MasonryTransportResult Submit(ReadOnlyMemory<byte> messagePack)
        {
            lock (callGate)
            {
                MasonryTransportResult? rejected = RejectCall();
                if (rejected is not null)
                {
                    return rejected;
                }

                if (engine == IntPtr.Zero)
                {
                    return AbiError("Connect must create the native engine before submit.");
                }

                return InvokeRequest(messagePack, MasonryNativeMethods.masonry_submit);
            }
        }

        public MasonryTransportResult Poll()
        {
            lock (callGate)
            {
                MasonryTransportResult? rejected = RejectCall();
                if (rejected is not null)
                {
                    return rejected;
                }

                if (engine == IntPtr.Zero)
                {
                    return AbiError("Connect must create the native engine before poll.");
                }

                MasonryNativeBuffer output = default;
                try
                {
                    int status = MasonryNativeMethods.masonry_poll(engine, out output);
                    return Translate(status, output, true);
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
                if (engine != IntPtr.Zero)
                {
                    MasonryNativeMethods.masonry_engine_destroy(engine);
                    engine = IntPtr.Zero;
                }

                isDisposed = true;
            }
        }

        private MasonryTransportResult? EnsureEngine()
        {
            if (engine != IntPtr.Zero)
            {
                return null;
            }

            IntPtr createdEngine = IntPtr.Zero;
            MasonryNativeBuffer output = default;
            bool destroyCreatedEngine = false;
            try
            {
                int status = MasonryNativeMethods.masonry_engine_create(
                    out createdEngine,
                    out output
                );
                destroyCreatedEngine = status == Ok && createdEngine != IntPtr.Zero;
                MasonryTransportResult result = Translate(
                    status,
                    output,
                    false,
                    creation: true,
                    createdEngine
                );
                if (result.Status != MasonryTransportStatus.Success)
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
                    MasonryNativeMethods.masonry_engine_destroy(createdEngine);
                }
            }
        }

        private MasonryTransportResult InvokeRequest(
            ReadOnlyMemory<byte> messagePack,
            NativeRequest request
        )
        {
            byte[] synchronousInput = messagePack.ToArray();
            MasonryNativeBuffer output = default;
            try
            {
                int status = request(
                    engine,
                    synchronousInput,
                    checked((ulong)synchronousInput.LongLength),
                    out output
                );
                return Translate(status, output, false);
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

        private static MasonryTransportResult Translate(
            int status,
            MasonryNativeBuffer output,
            bool isPoll,
            bool creation = false,
            IntPtr createdEngine = default
        )
        {
            string? shapeError = ValidateShape(output);
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
                        : new MasonryTransportResult(MasonryTransportStatus.Success);
                }

                if (output.Length == 0)
                {
                    return AbiError("Native success returned an empty response.", status);
                }

                return new MasonryTransportResult(
                    MasonryTransportStatus.Success,
                    Copy(output),
                    nativeStatus: status
                );
            }

            if (status == NoMessage)
            {
                return isPoll && output.Length == 0
                    ? new MasonryTransportResult(
                        MasonryTransportStatus.NoMessage,
                        nativeStatus: status
                    )
                    : AbiError("NO_MESSAGE is valid only for an empty poll result.", status);
            }

            string? diagnostic = DecodeDiagnostic(output, out string? decodeError);
            if (decodeError is not null)
            {
                return AbiError(decodeError, status);
            }

            MasonryTransportStatus mapped = status switch
            {
                InvalidArgument => MasonryTransportStatus.InvalidArgument,
                EngineError => MasonryTransportStatus.EngineError,
                Panic => MasonryTransportStatus.Panic,
                _ => MasonryTransportStatus.AbiError,
            };
            return new MasonryTransportResult(mapped, diagnostic: diagnostic, nativeStatus: status);
        }

        private MasonryTransportResult? RejectCall()
        {
            if (isDisposed)
            {
                throw new ObjectDisposedException(nameof(MasonryNativeTransport));
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

        private static string? ValidateShape(MasonryNativeBuffer output)
        {
            if ((output.Data == IntPtr.Zero) != (output.Length == 0))
            {
                return "Native output did not use the required {NULL,0} empty representation.";
            }

            return output.Length > MaximumPayloadBytes
                ? $"Native output exceeded the {MaximumPayloadBytes}-byte limit."
                : null;
        }

        private static byte[] Copy(MasonryNativeBuffer output)
        {
            var bytes = new byte[checked((int)output.Length)];
            Marshal.Copy(output.Data, bytes, 0, bytes.Length);
            return bytes;
        }

        private static string? DecodeDiagnostic(MasonryNativeBuffer output, out string? decodeError)
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

        private static void Free(MasonryNativeBuffer output)
        {
            if (output.Data != IntPtr.Zero && output.Length != 0)
            {
                MasonryNativeMethods.masonry_buffer_free(output);
            }
        }

        private static MasonryTransportResult ManagedFailure(Exception exception) =>
            AbiError($"Managed native transport failure: {exception.Message}");

        private static MasonryTransportResult AbiError(
            string diagnostic,
            int? nativeStatus = null
        ) =>
            new(
                MasonryTransportStatus.AbiError,
                diagnostic: diagnostic,
                nativeStatus: nativeStatus
            );

        private delegate int NativeRequest(
            IntPtr engine,
            byte[] messagePack,
            ulong length,
            out MasonryNativeBuffer output
        );
    }
}
