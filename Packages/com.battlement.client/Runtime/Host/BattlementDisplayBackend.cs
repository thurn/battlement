#nullable enable

using System;
using UnityEngine;

namespace Battlement
{
    internal interface IDisplayBackend
    {
        DisplayConfiguration SafeWindow { get; }
        bool WindowFits(DisplayResolution resolution);
        void Apply(DisplayConfiguration configuration);
    }

    internal sealed class BattlementDisplayBackend : IDisplayBackend
    {
        public DisplayConfiguration SafeWindow
        {
            get
            {
                ScreenSize display =
                    ReadWindowBounds()
                    ?? throw new InvalidOperationException(
                        "Current monitor bounds are unavailable."
                    );
                return new DisplayConfiguration(
                    DisplayMode.Windowed,
                    new DisplayResolution(
                        Math.Max(1U, Math.Min(1600U, display.Width) * 4 / 5),
                        Math.Max(1U, Math.Min(900U, display.Height) * 4 / 5),
                        0,
                        1
                    )
                );
            }
        }

        public bool WindowFits(DisplayResolution resolution)
        {
            ScreenSize? bounds = ReadWindowBounds();
            if (bounds is not ScreenSize display)
                return false;
            return resolution.Width <= display.Width && resolution.Height <= display.Height;
        }

        public static ScreenSize? ReadWindowBounds()
        {
            try
            {
                DisplayInfo display = Screen.mainWindowDisplayInfo;
                if (display.width <= 0 || display.height <= 0)
                    return null;
                return new ScreenSize(checked((uint)display.width), checked((uint)display.height));
            }
            catch (Exception)
            {
                return null;
            }
        }

        public void Apply(DisplayConfiguration configuration)
        {
            FullScreenMode mode = configuration.Mode switch
            {
                DisplayMode.Windowed => FullScreenMode.Windowed,
                DisplayMode.Borderless => FullScreenMode.FullScreenWindow,
                DisplayMode.Fullscreen when Application.platform == RuntimePlatform.WindowsPlayer =>
                    FullScreenMode.ExclusiveFullScreen,
                _ => throw new InvalidOperationException("This display mode is unavailable."),
            };
            DisplayResolution resolution = configuration.Resolution;
            Screen.SetResolution(
                checked((int)resolution.Width),
                checked((int)resolution.Height),
                mode,
                new RefreshRate
                {
                    numerator = resolution.RefreshNumerator,
                    denominator = resolution.RefreshDenominator,
                }
            );
        }
    }
}
