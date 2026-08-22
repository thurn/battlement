#nullable enable

using System;

namespace Battlement
{
    /// <summary>A three-dimensional value in world units.</summary>
    public readonly struct Vector3 : IEquatable<Vector3>
    {
        /// <summary>Creates a vector from its components.</summary>
        public Vector3(double x, double y, double z) => (X, Y, Z) = (x, y, z);

        /// <summary>Gets the X component.</summary>
        public double X { get; }

        /// <summary>Gets the Y component.</summary>
        public double Y { get; }

        /// <summary>Gets the Z component.</summary>
        public double Z { get; }

        /// <summary>Gets the zero vector.</summary>
        public static Vector3 Zero { get; } = new(0, 0, 0);

        /// <summary>Gets the vector whose components are all one.</summary>
        public static Vector3 One { get; } = new(1, 1, 1);

        public bool Equals(Vector3 other) =>
            X.Equals(other.X) && Y.Equals(other.Y) && Z.Equals(other.Z);

        public override bool Equals(object? obj) => obj is Vector3 other && Equals(other);

        public override int GetHashCode() => (X, Y, Z).GetHashCode();

        public static bool operator ==(Vector3 left, Vector3 right) => left.Equals(right);

        public static bool operator !=(Vector3 left, Vector3 right) => !left.Equals(right);
    }

    /// <summary>A screen position measured in pixels from the bottom-left.</summary>
    public readonly struct ScreenPosition : IEquatable<ScreenPosition>
    {
        /// <summary>Creates a screen position from its coordinates.</summary>
        public ScreenPosition(double x, double y) => (X, Y) = (x, y);

        /// <summary>Gets the horizontal coordinate.</summary>
        public double X { get; }

        /// <summary>Gets the vertical coordinate.</summary>
        public double Y { get; }

        public bool Equals(ScreenPosition other) => X.Equals(other.X) && Y.Equals(other.Y);

        public override bool Equals(object? obj) => obj is ScreenPosition other && Equals(other);

        public override int GetHashCode() => (X, Y).GetHashCode();

        public static bool operator ==(ScreenPosition left, ScreenPosition right) =>
            left.Equals(right);

        public static bool operator !=(ScreenPosition left, ScreenPosition right) =>
            !left.Equals(right);
    }

    /// <summary>A screen size in physical pixels.</summary>
    public readonly struct ScreenSize : IEquatable<ScreenSize>
    {
        /// <summary>Creates a screen size from its dimensions.</summary>
        public ScreenSize(uint width, uint height) => (Width, Height) = (width, height);

        /// <summary>Gets the screen width in pixels.</summary>
        public uint Width { get; }

        /// <summary>Gets the screen height in pixels.</summary>
        public uint Height { get; }

        public bool Equals(ScreenSize other) => Width == other.Width && Height == other.Height;

        public override bool Equals(object? obj) => obj is ScreenSize other && Equals(other);

        public override int GetHashCode() => (Width, Height).GetHashCode();

        public static bool operator ==(ScreenSize left, ScreenSize right) => left.Equals(right);

        public static bool operator !=(ScreenSize left, ScreenSize right) => !left.Equals(right);
    }

    /// <summary>A quaternion in <c>{x, y, z, w}</c> order.</summary>
    /// <remarks>The value must have nonzero length and is normalized before use.</remarks>
    public readonly struct Quaternion : IEquatable<Quaternion>
    {
        /// <summary>Creates a quaternion from its components.</summary>
        public Quaternion(double x, double y, double z, double w) => (X, Y, Z, W) = (x, y, z, w);

        /// <summary>Gets the X component.</summary>
        public double X { get; }

        /// <summary>Gets the Y component.</summary>
        public double Y { get; }

        /// <summary>Gets the Z component.</summary>
        public double Z { get; }

        /// <summary>Gets the scalar component.</summary>
        public double W { get; }

        /// <summary>Gets the identity rotation.</summary>
        public static Quaternion Identity { get; } = new(0, 0, 0, 1);

        public bool Equals(Quaternion other) =>
            X.Equals(other.X) && Y.Equals(other.Y) && Z.Equals(other.Z) && W.Equals(other.W);

        public override bool Equals(object? obj) => obj is Quaternion other && Equals(other);

        public override int GetHashCode() => (X, Y, Z, W).GetHashCode();

        public static bool operator ==(Quaternion left, Quaternion right) => left.Equals(right);

        public static bool operator !=(Quaternion left, Quaternion right) => !left.Equals(right);
    }

    /// <summary>A linear RGB color without alpha.</summary>
    public readonly struct RgbColor : IEquatable<RgbColor>
    {
        /// <summary>Creates a color from its linear components.</summary>
        public RgbColor(double red, double green, double blue) =>
            (Red, Green, Blue) = (red, green, blue);

        /// <summary>Gets the red intensity in the inclusive range [0, 1].</summary>
        public double Red { get; }

        /// <summary>Gets the green intensity in the inclusive range [0, 1].</summary>
        public double Green { get; }

        /// <summary>Gets the blue intensity in the inclusive range [0, 1].</summary>
        public double Blue { get; }

        /// <summary>Gets white in linear color space.</summary>
        public static RgbColor White { get; } = new(1, 1, 1);

        /// <summary>Gets black in linear color space.</summary>
        public static RgbColor Black { get; } = new(0, 0, 0);

        public bool Equals(RgbColor other) =>
            Red.Equals(other.Red) && Green.Equals(other.Green) && Blue.Equals(other.Blue);

        public override bool Equals(object? obj) => obj is RgbColor other && Equals(other);

        public override int GetHashCode() => (Red, Green, Blue).GetHashCode();

        public static bool operator ==(RgbColor left, RgbColor right) => left.Equals(right);

        public static bool operator !=(RgbColor left, RgbColor right) => !left.Equals(right);
    }

    /// <summary>A linear RGBA color.</summary>
    public readonly struct Color : IEquatable<Color>
    {
        /// <summary>Creates a color from its linear components.</summary>
        public Color(double red, double green, double blue, double alpha = 1) =>
            (Red, Green, Blue, Alpha) = (red, green, blue, alpha);

        /// <summary>Gets the red intensity in the inclusive range [0, 1].</summary>
        public double Red { get; }

        /// <summary>Gets the green intensity in the inclusive range [0, 1].</summary>
        public double Green { get; }

        /// <summary>Gets the blue intensity in the inclusive range [0, 1].</summary>
        public double Blue { get; }

        /// <summary>Gets the alpha value in the inclusive range [0, 1].</summary>
        public double Alpha { get; }

        /// <summary>Gets opaque white in linear color space.</summary>
        public static Color White { get; } = new(1, 1, 1);

        /// <summary>Gets opaque black in linear color space.</summary>
        public static Color Black { get; } = new(0, 0, 0);

        public bool Equals(Color other) =>
            Red.Equals(other.Red)
            && Green.Equals(other.Green)
            && Blue.Equals(other.Blue)
            && Alpha.Equals(other.Alpha);

        public override bool Equals(object? obj) => obj is Color other && Equals(other);

        public override int GetHashCode() => (Red, Green, Blue, Alpha).GetHashCode();

        public static bool operator ==(Color left, Color right) => left.Equals(right);

        public static bool operator !=(Color left, Color right) => !left.Equals(right);
    }

    /// <summary>An object's local transform relative to its parent or scene container.</summary>
    public readonly struct LocalTransform : IEquatable<LocalTransform>
    {
        /// <summary>Creates a local transform.</summary>
        public LocalTransform(Vector3 position, Quaternion rotation, Vector3 scale) =>
            (Position, Rotation, Scale) = (position, rotation, scale);

        /// <summary>Gets the local position.</summary>
        public Vector3 Position { get; }

        /// <summary>Gets the local rotation.</summary>
        public Quaternion Rotation { get; }

        /// <summary>Gets the local scale.</summary>
        public Vector3 Scale { get; }

        /// <summary>Gets the identity transform.</summary>
        public static LocalTransform Identity { get; } =
            new(Vector3.Zero, Quaternion.Identity, Vector3.One);

        public bool Equals(LocalTransform other) =>
            Position.Equals(other.Position)
            && Rotation.Equals(other.Rotation)
            && Scale.Equals(other.Scale);

        public override bool Equals(object? obj) => obj is LocalTransform other && Equals(other);

        public override int GetHashCode() => (Position, Rotation, Scale).GetHashCode();

        public static bool operator ==(LocalTransform left, LocalTransform right) =>
            left.Equals(right);

        public static bool operator !=(LocalTransform left, LocalTransform right) =>
            !left.Equals(right);
    }

    /// <summary>The event kinds an object may emit after pointer raycasting.</summary>
    public enum PointerEvent
    {
        /// <summary>The pointer began hovering the object.</summary>
        Enter,

        /// <summary>The pointer stopped hovering the object.</summary>
        Exit,

        /// <summary>A pointer button was pressed over the object.</summary>
        Down,

        /// <summary>A pointer button was released over the object.</summary>
        Up,

        /// <summary>A press and release resolved to the same object.</summary>
        Click,
    }

    /// <summary>How a draggable object's position relates to the pointer at pickup.</summary>
    public enum DragMode
    {
        /// <summary>Move the object's center to the pointer immediately.</summary>
        SnapToPointer,

        /// <summary>Preserve the world-space offset between pointer and object.</summary>
        PreserveOffset,
    }

    /// <summary>A mouse-style button reported with pointer button actions.</summary>
    public enum PointerButton
    {
        /// <summary>The primary mouse button, also used for touch.</summary>
        Left,

        /// <summary>The middle mouse button.</summary>
        Middle,

        /// <summary>The secondary mouse button.</summary>
        Right,
    }

    /// <summary>How a newly received batch relates to earlier blocking batches.</summary>
    public enum BatchStart
    {
        /// <summary>Start as soon as scheduling permits.</summary>
        Now,

        /// <summary>Wait until blocking work in earlier batches has completed.</summary>
        AfterEarlierBlockingWork,
    }

    /// <summary>
    /// What a property-writing command does when another operation controls the property.
    /// </summary>
    public enum ConflictPolicy
    {
        /// <summary>Cancel the older operation and start from the displayed value.</summary>
        Cancel,

        /// <summary>Wait for the older operation to finish before starting.</summary>
        Wait,
    }

    /// <summary>How an image texture is fitted into its requested world-space dimensions.</summary>
    public enum ImageFit
    {
        /// <summary>Fill both dimensions without preserving aspect ratio.</summary>
        Stretch,

        /// <summary>Preserve aspect ratio and leave transparent space as needed.</summary>
        Contain,

        /// <summary>Preserve aspect ratio and crop centered UVs as needed.</summary>
        Cover,
    }

    /// <summary>A camera's projection mode.</summary>
    public enum CameraProjection
    {
        /// <summary>Perspective projection.</summary>
        Perspective,

        /// <summary>Orthographic projection.</summary>
        Orthographic,
    }

    /// <summary>Which buffers a camera clears before rendering.</summary>
    public enum CameraClearMode
    {
        /// <summary>Draw the configured skybox.</summary>
        Skybox,

        /// <summary>Fill with the configured clear color.</summary>
        SolidColor,

        /// <summary>Clear only depth.</summary>
        Depth,

        /// <summary>Do not clear.</summary>
        Nothing,
    }

    /// <summary>A standard light type.</summary>
    public enum LightType
    {
        /// <summary>A light with a direction but no position or range.</summary>
        Directional,

        /// <summary>A point light.</summary>
        Point,

        /// <summary>A spot light.</summary>
        Spot,
    }

    /// <summary>A light's shadow rendering mode.</summary>
    public enum ShadowMode
    {
        /// <summary>Do not render shadows.</summary>
        None,

        /// <summary>Render hard-edged shadows.</summary>
        Hard,

        /// <summary>Render soft-edged shadows.</summary>
        Soft,
    }

    /// <summary>Horizontal alignment for world-space text.</summary>
    public enum HorizontalAlignment
    {
        /// <summary>Align to the left edge.</summary>
        Left,

        /// <summary>Center the text.</summary>
        Center,

        /// <summary>Align to the right edge.</summary>
        Right,

        /// <summary>Expand spacing to align both edges.</summary>
        Justified,
    }

    /// <summary>Vertical alignment for world-space text.</summary>
    public enum VerticalAlignment
    {
        /// <summary>Align to the top edge.</summary>
        Top,

        /// <summary>Center vertically.</summary>
        Middle,

        /// <summary>Align to the bottom edge.</summary>
        Bottom,
    }

    /// <summary>How a repeated tween begins its next traversal.</summary>
    public enum RepeatMode
    {
        /// <summary>Jump to the captured start value and move forward again.</summary>
        Restart,

        /// <summary>Reverse direction for each additional traversal.</summary>
        PingPong,
    }

    /// <summary>A built-in easing curve supported by Battlement.</summary>
    public enum Easing
    {
        /// <summary>Linear interpolation.</summary>
        Linear,

        /// <summary>Sine ease in.</summary>
        InSine,

        /// <summary>Sine ease out.</summary>
        OutSine,

        /// <summary>Sine ease in and out.</summary>
        InOutSine,

        /// <summary>Quadratic ease in.</summary>
        InQuad,

        /// <summary>Quadratic ease out.</summary>
        OutQuad,

        /// <summary>Quadratic ease in and out.</summary>
        InOutQuad,

        /// <summary>Cubic ease in.</summary>
        InCubic,

        /// <summary>Cubic ease out.</summary>
        OutCubic,

        /// <summary>Cubic ease in and out.</summary>
        InOutCubic,

        /// <summary>Quartic ease in.</summary>
        InQuart,

        /// <summary>Quartic ease out.</summary>
        OutQuart,

        /// <summary>Quartic ease in and out.</summary>
        InOutQuart,

        /// <summary>Quintic ease in.</summary>
        InQuint,

        /// <summary>Quintic ease out.</summary>
        OutQuint,

        /// <summary>Quintic ease in and out.</summary>
        InOutQuint,

        /// <summary>Exponential ease in.</summary>
        InExpo,

        /// <summary>Exponential ease out.</summary>
        OutExpo,

        /// <summary>Exponential ease in and out.</summary>
        InOutExpo,

        /// <summary>Circular ease in.</summary>
        InCirc,

        /// <summary>Circular ease out.</summary>
        OutCirc,

        /// <summary>Circular ease in and out.</summary>
        InOutCirc,

        /// <summary>Overshooting ease in.</summary>
        InBack,

        /// <summary>Overshooting ease out.</summary>
        OutBack,

        /// <summary>Overshooting ease in and out.</summary>
        InOutBack,

        /// <summary>Elastic ease in.</summary>
        InElastic,

        /// <summary>Elastic ease out.</summary>
        OutElastic,

        /// <summary>Elastic ease in and out.</summary>
        InOutElastic,

        /// <summary>Bounce ease in.</summary>
        InBounce,

        /// <summary>Bounce ease out.</summary>
        OutBounce,

        /// <summary>Bounce ease in and out.</summary>
        InOutBounce,
    }

    /// <summary>Timing and repetition shared by all tween commands.</summary>
    /// <remarks>
    /// A zero-duration tween cannot repeat, and a forever tween must be nonblocking.
    /// </remarks>
    /// <param name="Duration">Duration of one traversal.</param>
    /// <param name="Delay">Initial delay applied before the first traversal.</param>
    /// <param name="Easing">Easing curve used for each traversal.</param>
    /// <param name="Repeat">Whether and how the tween repeats.</param>
    public sealed record Tween(TimeSpan Duration, TimeSpan Delay, Easing Easing, TweenRepeat Repeat)
    {
        /// <summary>Creates a non-repeating tween with the default easing curve.</summary>
        public Tween(TimeSpan duration)
            : this(duration, TimeSpan.Zero, Easing.InOutSine, new TweenRepeat.Once()) { }
    }

    /// <summary>Repetition behavior after a tween's first traversal.</summary>
    public abstract record TweenRepeat
    {
        private TweenRepeat() { }

        /// <summary>Stop after the first traversal.</summary>
        public sealed record Once : TweenRepeat;

        /// <summary>Perform a bounded number of additional traversals.</summary>
        /// <param name="AdditionalTraversals">Number of traversals after the first.</param>
        /// <param name="Mode">How each additional traversal proceeds.</param>
        public sealed record Count(uint AdditionalTraversals, RepeatMode Mode) : TweenRepeat;

        /// <summary>Repeat until explicitly canceled.</summary>
        public sealed record Forever(RepeatMode Mode) : TweenRepeat;
    }

    /// <summary>A physical W3C <c>KeyboardEvent.code</c> supported by Battlement.</summary>
    public enum KeyCode
    {
        /// <summary>Escape.</summary>
        Escape,

        /// <summary>Function key F1.</summary>
        F1,

        /// <summary>Function key F2.</summary>
        F2,

        /// <summary>Function key F3.</summary>
        F3,

        /// <summary>Function key F4.</summary>
        F4,

        /// <summary>Function key F5.</summary>
        F5,

        /// <summary>Function key F6.</summary>
        F6,

        /// <summary>Function key F7.</summary>
        F7,

        /// <summary>Function key F8.</summary>
        F8,

        /// <summary>Function key F9.</summary>
        F9,

        /// <summary>Function key F10.</summary>
        F10,

        /// <summary>Function key F11.</summary>
        F11,

        /// <summary>Function key F12.</summary>
        F12,

        /// <summary>Backquote.</summary>
        Backquote,

        /// <summary>Digit 0.</summary>
        Digit0,

        /// <summary>Digit 1.</summary>
        Digit1,

        /// <summary>Digit 2.</summary>
        Digit2,

        /// <summary>Digit 3.</summary>
        Digit3,

        /// <summary>Digit 4.</summary>
        Digit4,

        /// <summary>Digit 5.</summary>
        Digit5,

        /// <summary>Digit 6.</summary>
        Digit6,

        /// <summary>Digit 7.</summary>
        Digit7,

        /// <summary>Digit 8.</summary>
        Digit8,

        /// <summary>Digit 9.</summary>
        Digit9,

        /// <summary>Minus.</summary>
        Minus,

        /// <summary>Equal.</summary>
        Equal,

        /// <summary>Backspace.</summary>
        Backspace,

        /// <summary>Tab.</summary>
        Tab,

        /// <summary>Letter A.</summary>
        KeyA,

        /// <summary>Letter B.</summary>
        KeyB,

        /// <summary>Letter C.</summary>
        KeyC,

        /// <summary>Letter D.</summary>
        KeyD,

        /// <summary>Letter E.</summary>
        KeyE,

        /// <summary>Letter F.</summary>
        KeyF,

        /// <summary>Letter G.</summary>
        KeyG,

        /// <summary>Letter H.</summary>
        KeyH,

        /// <summary>Letter I.</summary>
        KeyI,

        /// <summary>Letter J.</summary>
        KeyJ,

        /// <summary>Letter K.</summary>
        KeyK,

        /// <summary>Letter L.</summary>
        KeyL,

        /// <summary>Letter M.</summary>
        KeyM,

        /// <summary>Letter N.</summary>
        KeyN,

        /// <summary>Letter O.</summary>
        KeyO,

        /// <summary>Letter P.</summary>
        KeyP,

        /// <summary>Letter Q.</summary>
        KeyQ,

        /// <summary>Letter R.</summary>
        KeyR,

        /// <summary>Letter S.</summary>
        KeyS,

        /// <summary>Letter T.</summary>
        KeyT,

        /// <summary>Letter U.</summary>
        KeyU,

        /// <summary>Letter V.</summary>
        KeyV,

        /// <summary>Letter W.</summary>
        KeyW,

        /// <summary>Letter X.</summary>
        KeyX,

        /// <summary>Letter Y.</summary>
        KeyY,

        /// <summary>Letter Z.</summary>
        KeyZ,

        /// <summary>Left bracket.</summary>
        BracketLeft,

        /// <summary>Right bracket.</summary>
        BracketRight,

        /// <summary>Backslash.</summary>
        Backslash,

        /// <summary>Caps Lock.</summary>
        CapsLock,

        /// <summary>Semicolon.</summary>
        Semicolon,

        /// <summary>Quote.</summary>
        Quote,

        /// <summary>Enter.</summary>
        Enter,

        /// <summary>Left Shift.</summary>
        ShiftLeft,

        /// <summary>Right Shift.</summary>
        ShiftRight,

        /// <summary>Left Control.</summary>
        ControlLeft,

        /// <summary>Right Control.</summary>
        ControlRight,

        /// <summary>Left Alt.</summary>
        AltLeft,

        /// <summary>Right Alt.</summary>
        AltRight,

        /// <summary>Left Meta, Command, or Windows key.</summary>
        MetaLeft,

        /// <summary>Right Meta, Command, or Windows key.</summary>
        MetaRight,

        /// <summary>Comma.</summary>
        Comma,

        /// <summary>Period.</summary>
        Period,

        /// <summary>Slash.</summary>
        Slash,

        /// <summary>Space.</summary>
        Space,

        /// <summary>Context menu.</summary>
        ContextMenu,

        /// <summary>Insert.</summary>
        Insert,

        /// <summary>Delete.</summary>
        Delete,

        /// <summary>Home.</summary>
        Home,

        /// <summary>End.</summary>
        End,

        /// <summary>Page Up.</summary>
        PageUp,

        /// <summary>Page Down.</summary>
        PageDown,

        /// <summary>Left arrow.</summary>
        ArrowLeft,

        /// <summary>Right arrow.</summary>
        ArrowRight,

        /// <summary>Up arrow.</summary>
        ArrowUp,

        /// <summary>Down arrow.</summary>
        ArrowDown,

        /// <summary>Print Screen.</summary>
        PrintScreen,

        /// <summary>Scroll Lock.</summary>
        ScrollLock,

        /// <summary>Pause.</summary>
        Pause,

        /// <summary>Num Lock.</summary>
        NumLock,

        /// <summary>Numpad digit 0.</summary>
        Numpad0,

        /// <summary>Numpad digit 1.</summary>
        Numpad1,

        /// <summary>Numpad digit 2.</summary>
        Numpad2,

        /// <summary>Numpad digit 3.</summary>
        Numpad3,

        /// <summary>Numpad digit 4.</summary>
        Numpad4,

        /// <summary>Numpad digit 5.</summary>
        Numpad5,

        /// <summary>Numpad digit 6.</summary>
        Numpad6,

        /// <summary>Numpad digit 7.</summary>
        Numpad7,

        /// <summary>Numpad digit 8.</summary>
        Numpad8,

        /// <summary>Numpad digit 9.</summary>
        Numpad9,

        /// <summary>Numpad decimal separator.</summary>
        NumpadDecimal,

        /// <summary>Numpad addition.</summary>
        NumpadAdd,

        /// <summary>Numpad subtraction.</summary>
        NumpadSubtract,

        /// <summary>Numpad multiplication.</summary>
        NumpadMultiply,

        /// <summary>Numpad division.</summary>
        NumpadDivide,

        /// <summary>Numpad Enter.</summary>
        NumpadEnter,
    }
}
