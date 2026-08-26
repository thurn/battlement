#nullable enable

using UnityEngine;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal static class BattlementUiKeyboardMapper
    {
        public static PhysicalKey? Physical(KeyCode value)
        {
            if (value >= KeyCode.A && value <= KeyCode.Z)
                return (PhysicalKey)((int)PhysicalKey.KeyA + (value - KeyCode.A));
            if (value >= KeyCode.Alpha0 && value <= KeyCode.Alpha9)
                return (PhysicalKey)((int)PhysicalKey.Digit0 + (value - KeyCode.Alpha0));
            if (value >= KeyCode.F1 && value <= KeyCode.F12)
                return (PhysicalKey)((int)PhysicalKey.F1 + (value - KeyCode.F1));
            if (value >= KeyCode.Keypad0 && value <= KeyCode.Keypad9)
                return (PhysicalKey)((int)PhysicalKey.Numpad0 + (value - KeyCode.Keypad0));
            return value switch
            {
                KeyCode.Escape => PhysicalKey.Escape,
                KeyCode.BackQuote => PhysicalKey.Backquote,
                KeyCode.Minus => PhysicalKey.Minus,
                KeyCode.Equals => PhysicalKey.Equal,
                KeyCode.Backspace => PhysicalKey.Backspace,
                KeyCode.Tab => PhysicalKey.Tab,
                KeyCode.LeftBracket => PhysicalKey.BracketLeft,
                KeyCode.RightBracket => PhysicalKey.BracketRight,
                KeyCode.Backslash => PhysicalKey.Backslash,
                KeyCode.CapsLock => PhysicalKey.CapsLock,
                KeyCode.Semicolon => PhysicalKey.Semicolon,
                KeyCode.Quote => PhysicalKey.Quote,
                KeyCode.Return => PhysicalKey.Enter,
                KeyCode.LeftShift => PhysicalKey.ShiftLeft,
                KeyCode.RightShift => PhysicalKey.ShiftRight,
                KeyCode.LeftControl => PhysicalKey.ControlLeft,
                KeyCode.RightControl => PhysicalKey.ControlRight,
                KeyCode.LeftAlt => PhysicalKey.AltLeft,
                KeyCode.RightAlt => PhysicalKey.AltRight,
                KeyCode.LeftCommand => PhysicalKey.MetaLeft,
                KeyCode.RightCommand => PhysicalKey.MetaRight,
                KeyCode.LeftWindows => PhysicalKey.MetaLeft,
                KeyCode.RightWindows => PhysicalKey.MetaRight,
                KeyCode.Comma => PhysicalKey.Comma,
                KeyCode.Period => PhysicalKey.Period,
                KeyCode.Slash => PhysicalKey.Slash,
                KeyCode.Space => PhysicalKey.Space,
                KeyCode.Menu => PhysicalKey.ContextMenu,
                KeyCode.Insert => PhysicalKey.Insert,
                KeyCode.Delete => PhysicalKey.Delete,
                KeyCode.Home => PhysicalKey.Home,
                KeyCode.End => PhysicalKey.End,
                KeyCode.PageUp => PhysicalKey.PageUp,
                KeyCode.PageDown => PhysicalKey.PageDown,
                KeyCode.LeftArrow => PhysicalKey.ArrowLeft,
                KeyCode.RightArrow => PhysicalKey.ArrowRight,
                KeyCode.UpArrow => PhysicalKey.ArrowUp,
                KeyCode.DownArrow => PhysicalKey.ArrowDown,
                KeyCode.Print => PhysicalKey.PrintScreen,
                KeyCode.ScrollLock => PhysicalKey.ScrollLock,
                KeyCode.Pause => PhysicalKey.Pause,
                KeyCode.Numlock => PhysicalKey.NumLock,
                KeyCode.KeypadPeriod => PhysicalKey.NumpadDecimal,
                KeyCode.KeypadPlus => PhysicalKey.NumpadAdd,
                KeyCode.KeypadMinus => PhysicalKey.NumpadSubtract,
                KeyCode.KeypadMultiply => PhysicalKey.NumpadMultiply,
                KeyCode.KeypadDivide => PhysicalKey.NumpadDivide,
                KeyCode.KeypadEnter => PhysicalKey.NumpadEnter,
                _ => null,
            };
        }

        public static UiNavigationDirection Navigation(NavigationMoveEvent.Direction direction) =>
            direction switch
            {
                NavigationMoveEvent.Direction.Left => UiNavigationDirection.Left,
                NavigationMoveEvent.Direction.Up => UiNavigationDirection.Up,
                NavigationMoveEvent.Direction.Right => UiNavigationDirection.Right,
                NavigationMoveEvent.Direction.Down => UiNavigationDirection.Down,
                NavigationMoveEvent.Direction.Next => UiNavigationDirection.Next,
                NavigationMoveEvent.Direction.Previous => UiNavigationDirection.Previous,
                _ => UiNavigationDirection.None,
            };

        public static UiFocusDirection? Focus(FocusChangeDirection? direction)
        {
            if (direction == null || direction == FocusChangeDirection.none)
                return null;
            if (direction == FocusChangeDirection.unspecified)
                return new UiFocusDirection.Unspecified();
            if (direction == VisualElementFocusChangeDirection.left)
                return new UiFocusDirection.Left();
            if (direction == VisualElementFocusChangeDirection.right)
                return new UiFocusDirection.Right();
            return new UiFocusDirection.Other((int)direction);
        }
    }
}
