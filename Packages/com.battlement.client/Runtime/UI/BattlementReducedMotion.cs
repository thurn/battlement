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
        public static bool Read() => Preference() == ReducedMotionPreference.Reduce;

        public static ReducedMotionPreference Preference()
        {
#if UNITY_WEBGL && !UNITY_EDITOR
            return BattlementPrefersReducedMotion() switch
            {
                1 => ReducedMotionPreference.Reduce,
                0 => ReducedMotionPreference.NoPreference,
                _ => ReducedMotionPreference.Unavailable,
            };
#elif UNITY_STANDALONE_OSX && !UNITY_EDITOR
            IntPtr workspace = SendObject(
                ObjectiveCClass("NSWorkspace"),
                ObjectiveCSelector("sharedWorkspace")
            );
            if (workspace == IntPtr.Zero)
                return ReducedMotionPreference.Unavailable;
            return SendBool(workspace, ObjectiveCSelector("accessibilityDisplayShouldReduceMotion"))
                ? ReducedMotionPreference.Reduce
                : ReducedMotionPreference.NoPreference;
#else
            return ReducedMotionPreference.Unavailable;
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
    }
}
