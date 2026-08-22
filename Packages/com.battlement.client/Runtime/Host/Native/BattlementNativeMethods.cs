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
        internal static extern void battlement_engine_destroy(IntPtr engine);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int battlement_connect(
            IntPtr engine,
            [In] byte[] messagePack,
            ulong length,
            out BattlementNativeBuffer output
        );

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int battlement_submit(
            IntPtr engine,
            [In] byte[] messagePack,
            ulong length,
            out BattlementNativeBuffer output
        );

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int battlement_poll(
            IntPtr engine,
            out BattlementNativeBuffer output
        );

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void battlement_buffer_free(BattlementNativeBuffer buffer);
    }
}
