#nullable enable

using System;
using System.Runtime.InteropServices;

namespace Battlement
{
    internal readonly struct BattlementNativeBuffer
    {
        internal readonly ulong Handle;
        internal readonly IntPtr Data;
        internal readonly ulong Length;
        internal readonly ulong AllocationBytes;

        internal BattlementNativeBuffer(
            ulong handle,
            IntPtr data,
            ulong length,
            ulong allocationBytes
        ) => (Handle, Data, Length, AllocationBytes) = (handle, data, length, allocationBytes);

        internal string? ValidateShape(ulong maximumBytes) =>
            ValidateShape(maximumBytes, maximumBytes);

        internal string? ValidateShape(ulong maximumMessageBytes, ulong maximumAllocationBytes)
        {
            if (Handle == 0)
            {
                return Data == IntPtr.Zero && Length == 0 && AllocationBytes == 0
                    ? null
                    : "Native empty output carried nonempty buffer metadata.";
            }
            if (Data == IntPtr.Zero || Length == 0)
            {
                return "Native output handle did not describe a nonempty finished range.";
            }
            if (Length > AllocationBytes)
            {
                return "Native output length exceeded its allocation size.";
            }
            if (Length > maximumMessageBytes)
                return $"Native output exceeded the {maximumMessageBytes}-byte limit "
                    + "for a message.";
            return AllocationBytes > maximumAllocationBytes
                ? $"Native output exceeded the {maximumAllocationBytes}-byte allocation limit."
                : null;
        }
    }

    internal static class BattlementNativeMethods
    {
#if (UNITY_IOS || UNITY_WEBGL) && !UNITY_EDITOR
        internal const string LibraryName = "__Internal";
#else
        internal const string LibraryName = "battlement_rules";
#endif

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr battlement_native_abi_digest();

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr battlement_wire_contract_digest();

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int battlement_engine_create(out ulong engine, out ulong error);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int battlement_engine_destroy(ulong engine, out ulong error);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int battlement_connect(
            ulong engine,
            IntPtr input,
            ulong length,
            out ulong output
        );

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int battlement_submit(
            ulong engine,
            IntPtr input,
            ulong length,
            out ulong output
        );

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int battlement_submit_ui_event(
            ulong engine,
            IntPtr input,
            ulong length,
            out uint disposition,
            out ulong output
        );

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int battlement_poll(ulong engine, out ulong output);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int battlement_buffer_info(
            ulong buffer,
            out IntPtr data,
            out ulong length,
            out ulong allocationBytes
        );

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int battlement_release_buffer(ulong buffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int battlement_transport_diagnostics(
            out ulong buildersCreated,
            out ulong buildersReused,
            out ulong builderGrowths,
            out ulong builderCopiedBytes,
            out ulong idleBuilderBytes,
            out ulong handoffPayloadCopies
        );

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int battlement_logging_drain(out ulong records);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern uint battlement_ditto_determinism_contract();

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern ulong battlement_ditto_determinism_capabilities();
    }
}
