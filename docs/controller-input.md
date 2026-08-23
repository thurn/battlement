# Controller input protocol

Battlement exposes controller input as opt-in, platform-neutral actions. Rules engines select
buttons and discrete navigation behavior in `Snapshot.controller_input` or replace the selection
with `InputSetController`. Unity never turns a controller into a pointer: the D-pad and left stick
produce `ControllerNavigate` actions containing a cardinal direction, source, controller device
ID, and repeat flag.

`ControllerInputSettings` can override the analog dead zone and held-navigation timing. When these
values are omitted, the Unity client uses the Input System's native stick processing and UI
navigation timing. A configured dead zone is applied to the unprocessed normalized stick value so
it does not stack with Unity's processor. Unity resolves slightly diagonal stick input to its
dominant axis. A new direction emits exactly one initial action, and normal navigation never wraps;
any wrapping remains a rules-engine decision. D-pad input takes priority when both controls are
active.

Selected buttons emit `ControllerButtonDown` and `ControllerButtonUp`. Face buttons use physical
positions (`South`, `East`, `West`, and `North`) so games can describe Xbox A / PlayStation Cross
without binding protocol behavior to a vendor glyph. Shoulder, stick-button, Start, and Select
controls follow Unity's neutral names and use the same action pair. Start maps to Menu, Options, or
Plus; Select maps to View, Create/Share, or Minus. Right-stick axes and triggers are intentionally
absent until a game requests a concrete use for them.

`ControllerVibrate` runs low- and high-frequency motors at intensities from zero through one for a
bounded millisecond duration. Unsupported or disconnected controllers safely turn the command into
a no-op. Stopping or disposing a Battlement session resets haptics.

Controller state follows the common input gate. A held button or stick direction across a snapshot,
focus change, device change, or disabled-input interval is synchronized without manufacturing a
new down or navigation action when input resumes.

The chess sample demonstrates the contract with a board-square cursor. A/Cross selects and moves,
B/Circle cancels, the D-pad and left stick move one square, shoulders wrap through movable pieces
or legal destinations, and Start (Menu/Options) opens the guarded New Game and audio controls. The
cursor is a separate particle effect from legal-square planes, remains on the player's destination
while Black thinks, shrinks to a dim state while gameplay input is gated, and returns to normal
after the computer move. Invalid destinations play the existing error sound and request a short
vibration.
Chess explicitly overrides the native navigation behavior with a `0.35` dead zone, `275 ms` repeat
delay, and `125 ms` repeat interval.
