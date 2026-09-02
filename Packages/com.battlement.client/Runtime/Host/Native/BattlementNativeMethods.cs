#nullable enable

using System;
using System.Runtime.InteropServices;

namespace Battlement
{
    [StructLayout(LayoutKind.Sequential)]
    internal readonly struct BattlementNativeBuffer
    {
        internal readonly IntPtr Data;
        internal readonly ulong Length;

        internal BattlementNativeBuffer(IntPtr data, ulong length) =>
            (Data, Length) = (data, length);

        internal string? ValidateShape(ulong maximumBytes)
        {
            if ((Data == IntPtr.Zero) != (Length == 0))
            {
                return "Native output did not use the required {NULL,0} empty representation.";
            }

            return Length > maximumBytes
                ? $"Native output exceeded the {maximumBytes}-byte limit."
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
        internal static extern int battlement_engine_create(
            out IntPtr engine,
            out BattlementNativeBuffer error
        );

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int battlement_engine_destroy(
            IntPtr engine,
            out BattlementNativeBuffer error
        );

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int battlement_connect(
            IntPtr engine,
            [In] byte[] json,
            ulong length,
            out BattlementNativeBuffer output
        );

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int battlement_submit(
            IntPtr engine,
            [In] byte[] json,
            ulong length,
            out BattlementNativeBuffer output
        );

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int battlement_submit_ui_event(
            IntPtr engine,
            [In] byte[] json,
            ulong length,
            out uint disposition,
            out BattlementNativeBuffer output
        );

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int battlement_poll(
            IntPtr engine,
            out BattlementNativeBuffer output
        );

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void battlement_buffer_free(BattlementNativeBuffer buffer);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int battlement_logging_drain(out BattlementNativeBuffer records);
    }
}
