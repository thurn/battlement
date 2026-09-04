# Behavioral acceptance

[Plan and reading guide](../chess-ui-implementation-plan.md)

## Behavioral Acceptance

Every page declares its smallest useful provider harness. The default state is:

- Main-menu route
- Gameplay tab
- English
- Font scale 100%
- Reduced motion off
- Crash-report increase and upload options on
- Resolution 1920 × 1080
- Max framerate 144 FPS
- Display mode Borderless
- Screenshake and VSync on
- Master volume 80
- Music volume 65
- Effects volume 75
- Background mute off
- No dialog, dropdown, rebinding capture, exit, or active burst

### Settings value domains

The controlled settings and their complete value domains are:

- Gameplay: Language is English, Español, Français, or Deutsch; Text Size is
  100%, 150%, or 200%; Reduce Motion defaults off; Increase Move Duration and
  Upload Crash Reports default on; Erase Saved Data has no persistent effect.
- Graphics: Resolution is 1920 × 1080, 2560 × 1440, or 3840 × 2160; Max
  Framerate is
  60, 120, 144, or 240 FPS; Display Mode is Borderless, Fullscreen, or Windowed;
  Screenshake and VSync default on.
- Sound: Master, Music, and Effects volume are integers from 0 through 100;
  Mute in Background defaults off.
- Input: Left, Right, Up, Down, Move Piece, Pause, and Restart are displayed
  with keyboard and controller bindings.

Changing a value updates the controlled page state and does not invoke platform
graphics, locale, save-data, or gameplay services.

### Binding defaults and rebinding

The default keyboard/controller binding pairs are:

```text
Left       Left arrow   D-pad left
Right      Right arrow  D-pad right
Up         Up arrow     D-pad up
Down       Down arrow   D-pad down
Move Piece Space        A
Pause      Esc          menu
Restart    R            Y
```

Keyboard capture ignores bare Shift, Control, Alt, and Meta. Escape is a valid
captured shortcut because the dialog's normal Escape close action is disabled
during capture. A key already assigned to another action leaves the dialog open
and announces `Already used by <action>`. A valid unassigned key replaces the
binding and closes the dialog. Cancel closes without changing it. Reset restores
that action's default binding. Controller cells remain display-only.

### Privacy Policy dialog

The crash-report help dialog has the accessible name `Crash report upload
information` and body `We upload crash reports to Unity Diagnostics.` Its
`Privacy Policy` link emits an external-URL request for:

```text
https://unity.com/legal/game-player-and-app-user-privacy-policy
```

Tests replace the host opener and assert that exact request. A host rejection
leaves the dialog open, preserves focus, and exposes the standard
link-activation failure through the host diagnostic channel; the sample does
not invent a second dialog.

### Text scaling and reveal

At 100%, settings use the source's two-column rows. At 150% and 200%, each row
stacks its label above its control, grows by the source scale formulas, remains
scrollable, and scrolls the focused control fully into view. Each focusable
control retains an `ElementRef`; its focus handler calls the containing
ScrollView ref's explicit `scroll_to` action. The focus coordinator does not
perform automatic reveal.

### Audio and heartbeat

Audio volume is `(master / 100) * (music / 100)`. Sound mute takes precedence,
followed by background mute while the application is hidden. Losing focus while
the application remains visible does not mute it. Muting does not pause or
rewind the playhead. Restoring visibility restores the computed volume and
continues from the current playhead. Effects volume remains state-only.
Playback starts when the full app or audio page activates music and loops until
the harness resets or the host reports unavailable playback.

`MusicPlaybackIndicator` mutes sound without pausing audio when sound is
enabled. Enabling sound restores a zero master volume to 80 and a zero music
volume to 65, clears sound mute, and requests playback. A nonzero volume is not
otherwise changed. The same playhead continues across mute and enable actions.

The heartbeat is driven by audio time with a `60 / 56` second period, a second
hit at `0.13393` seconds, and a phase offset of `1.04` seconds. Its strength is
zero after `0.14` seconds from either hit and otherwise follows the source
exponential falloff. Paused, unavailable, reduced-motion, or reset playback
produces no pulse. Background muting alone does not stop it because the
playhead continues.

### Preserved prototype behavior

These source behaviors are intentional acceptance requirements:

- About is inert.
- Play and Quit run the same dismissal sequence.
- After dismissal, the stage remains black until page re-entry or an
  unconsumed Escape or Cancel reaches the review shell.
- Erase confirmation closes without deleting data.
- Gameplay and graphics controls update only controlled in-memory state.
- Effects volume changes visually but does not affect existing audio.
- Controller bindings are display-only and cannot be rebound.

Play and Quit never start gameplay and never request host shutdown. Each starts
the animation sequence defined by the pinned animation ledger, makes exiting
content inert, clears focus, collapses the frame, and leaves a black stage. The
black dismissed state consumes no Escape or controller Cancel action, allowing
the outer review shell to handle that otherwise unhandled action.

These behaviors belong respectively to Tasks 39, 34 and 39, 38, 35, 36, and
37. Each receives before-and-after black-box assertions.

### Keyboard and controller behavior

All four settings tabs remain sequential Tab stops, matching the source rather
than adopting an accessibility-guideline roving-tab-stop variation.

- Arrow keys and Reactant directional controller actions wrap, move focus, and
  select the destination tab through application handlers and queued ref focus.
- Home and End select and focus the first and last tabs.
- Tab and Shift-Tab follow ordinary document order.
- D-pad and left stick use Reactant's normalized direction and repeat policy.
  The sample introduces no private analog threshold.
- Submit mirrors Enter or Space on the focused control.
- Shoulder buttons are ignored.
- Cancel precedence is input capture, listbox, dialog, settings route, and then
  the review shell.
- Exiting Motion content becomes inert immediately and cannot retain focus.

The only application routes are `main` and `settings`. Settings navigates from
main to settings; Return navigates from settings to main. Selecting the current
route is a no-op. The first successful navigation sets `has_navigated`, causing
later replacements to use `ArcadeMenuTransition`. Browser history and URL state
never participate.

Only an unconsumed action may reach the review shell, where Escape or controller
Cancel exits Page 40.
