#nullable enable

using System;
using System.Runtime.InteropServices;

namespace Masonry
{
    [StructLayout(LayoutKind.Sequential)]
    internal readonly struct MasonryNativeBuffer
    {
        internal readonly IntPtr Data;
        internal readonly ulong Length;
    }

    internal static class MasonryNativeMethods
    {
#if UNITY_IOS && !UNITY_EDITOR
        internal const string LibraryName = "__Internal";
#else
        internal const string LibraryName = "masonry_rules";
#endif

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int masonry_engine_create(
            out IntPtr engine,
            out MasonryNativeBuffer error
        );

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void masonry_engine_destroy(IntPtr engine);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int masonry_connect(
            IntPtr engine,
            [In] byte[] messagePack,
            ulong length,
            out MasonryNativeBuffer output
        );

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int masonry_submit(
            IntPtr engine,
            [In] byte[] messagePack,
            ulong length,
            out MasonryNativeBuffer output
        );

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int masonry_poll(IntPtr engine, out MasonryNativeBuffer output);

        [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void masonry_buffer_free(MasonryNativeBuffer buffer);
    }
}
