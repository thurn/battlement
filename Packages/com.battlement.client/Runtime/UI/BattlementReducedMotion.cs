#nullable enable

#if UNITY_STANDALONE_OSX && !UNITY_EDITOR
using System;
#endif
#if (UNITY_WEBGL || UNITY_STANDALONE_OSX) && !UNITY_EDITOR
using System.Runtime.InteropServices;
#endif

namespace Battlement.UI
{
    internal static class BattlementReducedMotion
    {
        public static bool Read()
        {
#if UNITY_WEBGL && !UNITY_EDITOR
            return BattlementPrefersReducedMotion() != 0;
#elif UNITY_STANDALONE_OSX && !UNITY_EDITOR
            IntPtr workspace = SendObject(
                ObjectiveCClass("NSWorkspace"),
                ObjectiveCSelector("sharedWorkspace")
            );
            if (workspace == IntPtr.Zero)
                throw Unavailable();
            return SendBool(
                workspace,
                ObjectiveCSelector("accessibilityDisplayShouldReduceMotion")
            );
#elif UNITY_EDITOR
            return false;
#else
            throw Unavailable();
#endif
        }

#if UNITY_WEBGL && !UNITY_EDITOR
        [DllImport("__Internal")]
        private static extern int BattlementPrefersReducedMotion();
#endif

#if UNITY_STANDALONE_OSX && !UNITY_EDITOR
        [DllImport("/usr/lib/libobjc.A.dylib", EntryPoint = "objc_getClass")]
        private static extern IntPtr ObjectiveCClass(string name);

        [DllImport("/usr/lib/libobjc.A.dylib", EntryPoint = "sel_registerName")]
        private static extern IntPtr ObjectiveCSelector(string name);

        [DllImport("/usr/lib/libobjc.A.dylib", EntryPoint = "objc_msgSend")]
        private static extern IntPtr SendObject(IntPtr receiver, IntPtr selector);

        [return: MarshalAs(UnmanagedType.I1)]
        [DllImport("/usr/lib/libobjc.A.dylib", EntryPoint = "objc_msgSend")]
        private static extern bool SendBool(IntPtr receiver, IntPtr selector);
#endif

        private static BattlementUiException Unavailable() =>
            new(
                CoreErrorCode.InvalidProperty,
                "The platform reduced-motion preference is unavailable."
            );
    }
}
