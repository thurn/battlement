#nullable enable

using UnityEngine;
#if (UNITY_IOS || UNITY_STANDALONE_OSX) && !UNITY_EDITOR
using System;
#endif
#if (UNITY_IOS || UNITY_WEBGL || UNITY_STANDALONE_OSX || UNITY_STANDALONE_WIN) && !UNITY_EDITOR
using System.Runtime.InteropServices;
#endif

namespace Battlement.UI
{
    /// <summary>Converts framebuffer pixels to CSS-compatible logical pixels.</summary>
    internal static class BattlementLogicalPixels
    {
        private const float DefaultScale = 1;
#if BATTLEMENT_DITTO_DIAGNOSTICS
        private static float? dittoScale;
#endif

        public static float BackingScale
        {
            get
            {
#if BATTLEMENT_DITTO_DIAGNOSTICS
                if (dittoScale.HasValue)
                    return dittoScale.Value;
#endif
#if UNITY_WEBGL && !UNITY_EDITOR
                return Valid((float)BattlementWebLogicalPixelScale());
#elif UNITY_STANDALONE_OSX && !UNITY_EDITOR
                IntPtr application = SendObject(
                    ObjectiveCClass("NSApplication"),
                    ObjectiveCSelector("sharedApplication")
                );
                IntPtr window = SendObject(application, ObjectiveCSelector("keyWindow"));
                if (window == IntPtr.Zero)
                    window = SendObject(application, ObjectiveCSelector("mainWindow"));
                if (window == IntPtr.Zero)
                {
                    IntPtr windows = SendObject(application, ObjectiveCSelector("windows"));
                    window = SendObject(windows, ObjectiveCSelector("firstObject"));
                }
                return window == IntPtr.Zero
                    ? DefaultScale
                    : Valid((float)SendDouble(window, ObjectiveCSelector("backingScaleFactor")));
#elif UNITY_IOS && !UNITY_EDITOR
                IntPtr screen = SendObject(
                    ObjectiveCClass("UIScreen"),
                    ObjectiveCSelector("mainScreen")
                );
                return screen == IntPtr.Zero
                    ? DefaultScale
                    : Valid((float)SendDouble(screen, ObjectiveCSelector("scale")));
#elif UNITY_STANDALONE_WIN && !UNITY_EDITOR
                IntPtr window = GetActiveWindow();
                return window == IntPtr.Zero ? DefaultScale : Valid(GetDpiForWindow(window) / 96f);
#elif UNITY_ANDROID && !UNITY_EDITOR
                return Valid(Screen.dpi / 160f);
#else
                return DefaultScale;
#endif
            }
        }

#if BATTLEMENT_DITTO_DIAGNOSTICS
        public static void UseDittoScale(double value) => dittoScale = Valid((float)value);
#endif

        public static float PanelScale => BackingScale;

        public static ScreenSize ScreenSize
        {
            get
            {
                float backingScale = BackingScale;
                return new(
                    checked((uint)Mathf.RoundToInt(Screen.width / backingScale)),
                    checked((uint)Mathf.RoundToInt(Screen.height / backingScale))
                );
            }
        }

        private static float Valid(float value) =>
            float.IsFinite(value) && value > 0 ? value : DefaultScale;

#if UNITY_WEBGL && !UNITY_EDITOR
        [DllImport("__Internal")]
        private static extern double BattlementWebLogicalPixelScale();
#endif

#if (UNITY_IOS || UNITY_STANDALONE_OSX) && !UNITY_EDITOR
#if UNITY_IOS
        private const string ObjectiveCLibrary = "__Internal";
#else
        private const string ObjectiveCLibrary = "/usr/lib/libobjc.A.dylib";
#endif

        [DllImport(ObjectiveCLibrary, EntryPoint = "objc_getClass")]
        private static extern IntPtr ObjectiveCClass(string name);

        [DllImport(ObjectiveCLibrary, EntryPoint = "sel_registerName")]
        private static extern IntPtr ObjectiveCSelector(string name);

        [DllImport(ObjectiveCLibrary, EntryPoint = "objc_msgSend")]
        private static extern IntPtr SendObject(IntPtr receiver, IntPtr selector);

        [DllImport(ObjectiveCLibrary, EntryPoint = "objc_msgSend")]
        private static extern double SendDouble(IntPtr receiver, IntPtr selector);
#endif

#if UNITY_STANDALONE_WIN && !UNITY_EDITOR
        [DllImport("user32.dll")]
        private static extern IntPtr GetActiveWindow();

        [DllImport("user32.dll")]
        private static extern uint GetDpiForWindow(IntPtr window);
#endif
    }
}
