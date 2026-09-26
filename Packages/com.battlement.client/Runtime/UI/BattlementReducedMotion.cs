#nullable enable

#if UNITY_STANDALONE_OSX && !UNITY_EDITOR
using System;
#endif
#if !UNITY_EDITOR
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
#elif UNITY_STANDALONE_WIN && !UNITY_EDITOR
            if (!SystemParametersInfo(0x1042, 0, out bool animationsEnabled, 0))
                return ReducedMotionPreference.Unavailable;
            return animationsEnabled
                ? ReducedMotionPreference.NoPreference
                : ReducedMotionPreference.Reduce;
#elif UNITY_IOS && !UNITY_EDITOR
            return UIAccessibilityIsReduceMotionEnabled()
                ? ReducedMotionPreference.Reduce
                : ReducedMotionPreference.NoPreference;
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

#if UNITY_STANDALONE_WIN && !UNITY_EDITOR
        // SPI_GETCLIENTAREAANIMATION returns the system animation preference.
        [DllImport("user32.dll", EntryPoint = "SystemParametersInfoW")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool SystemParametersInfo(
            uint action,
            uint parameter,
            [MarshalAs(UnmanagedType.Bool)] out bool value,
            uint flags
        );
#endif

#if UNITY_IOS && !UNITY_EDITOR
        [DllImport("__Internal")]
        [return: MarshalAs(UnmanagedType.I1)]
        private static extern bool UIAccessibilityIsReduceMotionEnabled();
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
