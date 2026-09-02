# Chess UI Reactant Port Implementation Plan

## Introduction

This plan defines a complete Rust port of the player-visible `mockups` React
application using **Reactant**, Battlement's declarative Rust component runtime.
The result is a standalone `chess-ui` sample whose appearance and behavior
match the pinned TypeScript source at desktop resolution.

The sample is also a deliberate Reactant refinement project. Each small
migration is reviewed for unnecessary differences from TypeScript and for
general framework improvements. The sample matters primarily as evidence that
Reactant can express a polished application with React-like component
boundaries, controlled props, hooks, accessibility, focus, assets, and motion.

The port is developed through exactly 40 selectable review pages. Each page
isolates one responsibility, states what must work, and states what that page
does not yet assert. The last page composes the pieces into the complete app.

## Related Information

The authoritative TypeScript source is
`git@github.com:thurn/mockups.git` at commit
`2451ea9cc6f76b356b1102ee37b82c478853122a`. The existing checkout is
`/Users/dthurn/Documents/mockups`. That checkout remains unchanged while
reference states and screenshots are captured.

The reference uses Node 22.13 or newer, React 19.2.6, TypeScript 5.9.3,
Framer Motion 13.1.1, and Vinext 1.0.0-beta.3. Before the first capture, run:

```text
npm install
npm run format:check
npm run build
npm run dev
```

The pinned commit must build without source changes. Dependency installation
and generated build output do not become port inputs.

Implementation depends on the certified implementations of these designs:

- [Layout and stacking](layout-and-stacking.md)
- [Focus and navigation](focus-and-navigation.md)
- [Accessibility technical design](accessibility-technical-design.md)
- [Accessibility implementation plan](accessibility-implementation-plan.md)
- [Events and default actions](events-and-default-actions.md)
- [Asset generator](asset-generator.md#authoring-api)
- [Mockup animation coverage](animations.md#mockup-translation-coverage)

The events and default-actions work is a transitive prerequisite even though it
was not part of the original requested document list. Sliders, listboxes,
modals, tabs, and input rebinding cannot preserve native default behavior
without it.

The declarations in `samples/reactant/rules/src/assets.rs` are the source for
the 18 existing generated assets. The
[animation coverage ledger](animations.md#coverage-ledger) is the source for
motion timing, easing, direction, interruption, seed, and reduced-motion
requirements. This plan uses those as pinned starting evidence instead of
duplicating or rediscovering either design.

A **certified release** is the repository's `release` ref after Tollgate has
validated and promoted the exact candidate. Implementation begins from the
first certified release containing every prerequisite named above. Each later
task records its exact starting release commit in its handoff, because those
future commit identifiers do not exist while this plan is written.

## Research Basis

Three independent research passes audited the mockup, Reactant, and the
dependency order for a port. They reached these conclusions:

- Visual fidelity is feasible after the prerequisite designs are implemented.
- Focus, default actions, accessibility, controller input, and application
  lifecycle are more significant obstacles than static layout.
- Existing generated assets should be copied into the new sample instead of
  being shared with or referenced from the Reactant rules sample.
- Callback props, behavior-bundle forwarding, asset families, application
  visibility, external links, and stable semantic test targeting are likely
  Reactant improvement opportunities.
- Browser-only diagnostics should not be ported unless they expose a
  generalized requirement that also applies to Unity applications.
- The source contains intentional prototype behavior, including inert actions
  and settings that modify only local state. Fidelity requires preserving those
  behaviors rather than completing the implied product features.

Before the first migration, compile and runtime probes verify that the certified
release supplies:

- Flex, grid, stack, scroll, sticky, overlay, and anchored-popover layout
- Focus scopes, focus restoration, focus-visible modality, and directional
  navigation
- Accessible custom buttons, checkboxes, sliders, tabs, listboxes, dialogs,
  tables, and links
- Pointer capture, committed default actions, and input-capture policies
- Keyboard and normalized controller actions
- Presence, keyed animation, interruption, reduced motion, and audio time
- Audio playback and application-focus or visibility observation
- External-URL activation
- Stable semantic targeting for black-box tests

A missing capability that blocks a page is implemented as a generalized
Reactant feature in that page's base change. Sample-specific framework adapters
are not permitted.

## Port Contract

The port targets player-visible experience, not browser implementation details.
Component names, prop meaning, controlled state ownership, and high-level
composition remain aligned with TypeScript wherever the Unity host permits.

### Runnable sample

The result is a routinely discoverable `samples/chess-ui` sample with:

- Application name `Chess UI.app`
- Unity scene `Assets/Scenes/ChessUi.unity`
- Rules crate `battlement-chess-ui-rules`
- No sample-specific C# scripts
- Standard Unity serialization, Addressables, **Ditto** black-box sample
  scenarios, and sample discovery metadata
- `chess-ui` included in the hard-coded Ditto sample list in `scripts/ci.py`

Task 1 copies these pinned source files into the sample:

```text
public/fonts/barlow-condensed-700.ttf
public/fonts/barlow-condensed-800-italic.ttf
public/fonts/bebas-neue.ttf
public/audio/drag-and-dread.opus
```

Retain `samples/reactant/Assets/Original/BarlowCondensed-OFL.txt` for the Barlow
files and record the repository, source commit, original path, and applicable
license for every imported file. The files are available to every later page
even though audio behavior is not asserted until Task 31.

The ordinary validation commands are:

```text
cargo battlement reactant assets generate --project samples/chess-ui
cargo battlement sample run chess-ui --release
cargo battlement ditto --config samples/chess-ui/ditto.toml run --profile macos
./scripts/ci.py
```

### Component correspondence

Preserve these source-level component, provider, hook, and helper boundaries:

- Shell: `PortraitViewport`, `ScreenFrame`, `ConceptFrame`,
  `BackgroundMusicProvider`, and `FontScaleProvider`
- Routing: `ArcadeRouteTransition`, `ArcadeScreenRouter`, `MainMenu`, and
  `SettingsScreen`
- Controls: `ActionButton`, `ReturnButton`, `SettingRow`, `ToggleControl`,
  `SelectControl`, `VolumeControl`, `EraseControl`, and `SettingsTabs`
- Settings: `GraphicsSettings`, `SoundSettings`, and `InputSettings`; preserve
  the source's inline Gameplay panel rather than inventing a component
- Effects: `ArcadeAttractMode`, `ArcadeButtonEffect`,
  `ArcadeCheckboxEffect`, `ArcadeSliderEffect`, `ArcadeExitSequence`,
  `ArcadeFramePulse`, `ArcadeMenuTransition`, `ArcadeModal`,
  `ArcadeTabTransition`, and `MusicPlaybackIndicator`
- Helpers: `ClippedInset`, `ControlInteraction`, `InputBindingIcons`,
  `RasterFrame`, `ScreenHeader`, `ScreenReaderOnly`, `useInteraction`,
  `useBackgroundMusic`, `useFontScale`, and `useArcadeNavigation`

Rust fields use idiomatic `snake_case`, but preserve the source prop boundary
and required or optional status. Important control contracts are:

```text
ActionButton { children, disabled = false, max_text_scale?, on_click? }
ReturnButton { disabled = false, on_click }
SettingRow { label, label_id?, children, first = false, row_height? }
```

```text
ToggleControl {
  label, first?, offset_y?, row_height?, checked, aria_label?, on_change,
  with_info = false, on_info_click?
}

SelectControl {
  label, first?, offset_y?, row_height?, options, value, on_change
}
```

```text
VolumeControl {
  label, value, on_change, first = false
}

SettingsTabs { active_tab, on_select }
EraseControl { on_click? }
```

The initial application hierarchy remains `BackgroundMusicProvider` containing
`FontScaleProvider`, containing `PortraitViewport`, containing `ScreenFrame`,
containing `ArcadeRouteTransition`, containing `ArcadeScreenRouter`.

The Rust implementation may introduce host elements required by UI Toolkit,
but each extra element needs a line-level explanation. "Rust is different" is
not a sufficient explanation for changing a component boundary or prop.

Primary source ownership is assigned as follows. A boundary can appear in more
than one task when its layout, behavior, and animation are reviewed separately.

- Tasks 2--4 own `PortraitViewport`, `ScreenFrame`, `ConceptFrame`,
  `ClippedInset`, and `SettingRow`.
- Tasks 5, 11--13, 23, and 25 own `ToggleControl`, `ControlInteraction`,
  `useInteraction`, `ScreenReaderOnly`, and its checkbox effect.
- Tasks 6, 14, 15, 23, and 26 own `SelectControl`.
- Tasks 7, 16, 23, 25, 31, and 32 own `VolumeControl` and slider effects.
- Tasks 8, 11, 12, 23, and 25 own `ActionButton` and `ReturnButton`.
- Tasks 9, 10, 17, 23, and 27 own `SettingsTabs` and `ScreenHeader`.
- Tasks 18, 19, and 28 own `ArcadeModal` and `EraseControl` dialog behavior.
- Tasks 20, 21, 24, and 37 own `InputSettings` and `InputBindingIcons`.
- Task 22 owns `FontScaleProvider`, `useFontScale`, and scaling helpers.
- Tasks 23 and 24 own `RasterFrame` and generated or prepared visual assets.
- Tasks 29--32 own attract mode, frame pulse, background music, and playback
  indication.
- Tasks 33, 34, and 39 own menu transitions, exit behavior, and `MainMenu`.
- Tasks 35--38 own the settings components, tabs, dialogs, and Return flow.
- Task 40 owns `ArcadeRouteTransition`, `useArcadeNavigation`, and
  `ArcadeScreenRouter` as an integrated route tree.

`ArcadeRouteTransition` owns the controlled `main` or `settings` route and
reduced-motion context. `ArcadeMenuTransition` paints the animated replacement
of one keyed route with another. `ArcadeScreenRouter` selects `MainMenu` or
`SettingsScreen` and composes both responsibilities.

### Platform substitutions

The following substitutions are part of the port rather than parity failures:

- `RasterFrame` uses generated Reactant assets instead of browser image URLs.
- `ScreenReaderOnly` contributes directly to the native semantic tree.
- Browser history becomes controlled `ArcadeScreenRouter` state.
- Browser autoplay unlocking becomes the Unity audio-start lifecycle.
- `document.hidden` becomes a reactive application-visibility signal.
- Privacy Policy activation uses a typed external-URL host action.
- The WebKit workaround becomes one deterministic transition implementation.
  Add a platform capability only if testing proves it generally necessary.
- `100dvh`, `VisualViewport`, and safe-area code become Unity panel geometry.
- `UiRenderModeProvider`, `DebugRenderModeToggle`, `useUiRenderMode`, and
  `useAppleTouchWebKit` are excluded as browser diagnostic scaffolding.

Potential public Reactant improvements include cloneable callback props,
behavior-bundle forwarding, generated asset families, application visibility,
external-link activation, and stable semantic test identifiers. Add one only
when a page demonstrates a generalized need. Each addition includes public
black-box tests, concise public documentation, and a refactor proving that the
sample uses the improved API.

Backward compatibility and versioning do not constrain these improvements.

## Review Gallery

The gallery contains exactly 40 registered entries. Page 1 demonstrates the
gallery shell itself. Every entry displays its title and a 10--20 word
description of the behavior asserted on that page.

Descriptions identify a page's review responsibility, not a frozen rendering
of the component at that point in history. "Not asserted" means that a later
capability may be visible after shared code evolves, but that page does not own
its acceptance evidence. The source-line ownership record is authoritative for
later-task boundaries.

Each registration supplies:

```text
number, title, description, render_harness, reset_generation,
semantic_target, capture_states
```

`capture_states` is either `static` or an ordered list of named initial,
changed, and reset states. The container is a navigation landmark named
`Chess UI review pages`. Each item is a button named `<number>. <title>`; the
selected button exposes `aria-current="page"`. The content is a region labelled
by its visible page heading. Selection recreates the harness and moves focus to
that heading. Directional controller input moves focus among navigation buttons
without selecting; Submit activates the focused button.

The gallery navigation:

- Is vertically scrollable and remains outside the 1024x1536 design stage.
- Scrolls the selected entry into view.
- Exposes selected-state semantics.
- Supports pointer, keyboard, and controller activation.
- Mounts the current shared component implementations, so earlier pages improve
  as shared components and Reactant evolve.

At desktop sizes, the gallery uses a 320-pixel navigation column, a 24-pixel
gap, and 24-pixel outer padding. The design stage is centered in the remaining
space and uses this scale, capped at 1:

```text
min((viewport_width - 392) / 1024, (viewport_height - 48) / 1536)
```

The navigation scrolls independently. The design stage never causes outer-page
scrolling. Below a 1280x800 review window the same formula continues to shrink
the stage; mockup content does not reflow merely because the gallery is narrow.

Selecting any entry, including the currently selected entry, increments a mount
generation and fully recreates the page harness. Re-entry, application reload,
and relaunch reset:

- Component and provider state
- Focus and focus-visible modality
- Dropdowns, dialogs, input capture, and overlays
- Audio playback and playhead
- Animation clocks and keyed effect generations
- Scroll positions
- Application-visibility simulation
- Final-router dismissal state

On reload or relaunch, Page 1 is selected and its heading is focused. After any
selection or reselection reset, the selected page heading is focused, no pointer
is captured, the host is visible and focused, the application is on the main
route, all overlays are closed, all page scroll offsets are zero, animation
time and heartbeat phase are zero, and audio is stopped at time zero. Audio
begins only when the page or full app explicitly activates its playback
behavior.

Page 40 first displays its title, description, and a launcher. Activating the
launcher opens the complete app in a full-screen overlay without gallery chrome
or sample-only controls. An unconsumed Escape or controller Cancel returns to
the gallery. Pointer-only exit is intentionally absent from this temporary
review overlay; application reload remains available.

The pages remain committed until the user explicitly accepts Task 40's final
parity candidate and every resulting Reactant follow-up has been promoted.
Their possible deletion is a separate, explicitly authorized task and is not
part of this plan.

## Visual Fidelity

The authoritative visual target is the mockup's CSS desktop rendering. The
canonical environment is:

- Inner design stage: exactly 1024x1536 logical pixels
- Device scale: 1
- Screenshot crop: the inner design stage only
- Initial route: main menu
- Motion preference: normal
- Safe-area insets: zero
- Fonts: Bebas Neue, Barlow Condensed 700, and Barlow Condensed 800 italic
- Secondary integration capture: 2560x1440 desktop with the source's 0.75
  outer scale

Run the pinned source unchanged with its existing development command. A
partial page uses the corresponding state in the complete source application,
cropped to the component under review. Transient effects are captured through
normal source interactions. Computed DOM and CSS values supplement the crop
where the source does not expose an isolated specimen. Do not add a temporary
React harness to manufacture references.

Pages before Task 23 compare geometry, native text, semantics, and interaction
only. Generated-skin pixels are excluded from their visual diffs. Task 23
recaptures every earlier page and is the first full-paint approval for those
controls. Task 21 uses text-only binding cells until Task 24 supplies icons.

Static pages may record their changed-state capture as `N/A`. Interactive pages
require initial, changed, and reset captures.

Geometry is measured in the unscaled 1024x1536 coordinate system and must be
within one logical pixel. Screenshots use sRGB and are aligned by the stage
bounds before comparison. At device scale 1, one logical pixel is one captured
pixel. Generated raster output matches its recipe. Outside transparent pixels
and an explicit text-antialiasing mask, no unexplained static difference larger
than two captured pixels or 2/255 per color channel is accepted. The one-pixel
geometry rule remains stricter than the screenshot threshold. The user records
approval in the candidate handoff.

## Task and Reviewer Protocol

Each numbered migration is an independently promoted task. Work begins from the
certified release containing all earlier migrations and reviewer follow-ups.

**Tollgate** is the repository's validation and exact-candidate promotion
service. A **candidate** is one immutable source commit submitted for validation
without promotion authority. A **`wt` worktree** is the isolated checkout
created and owned through Tollgate for one task.

The base-task workflow is:

1. Create a fresh Tollgate-owned `wt` worktree from the current release.
2. Implement one page, its resettable harness, semantic fixture, focused tests,
   and visual evidence. Target roughly 500 non-test lines or fewer.
3. Rerun smoke and reset checks for all registered pages. Recapture earlier
   pages whose shared components changed.
4. Stage all intended changes, run focused checks and `./scripts/ci.py`, create
   one Conventional Commit, and submit `tg candidate HEAD`.
5. Every migration page is web-visible. Start its sample on a verified-free
   non-default port and expose it with a named Cloudflare Quick Tunnel. Record
   both service identities, verify the public URL, and keep both services
   available through review. Apply the same rule to a follow-up that changes
   rendered behavior.
6. Obtain an explicit promotion mandate for the exact candidate. Stop only the
   recorded demo and tunnel services immediately before authorization.
7. After promotion, assign a fresh port-ergonomics reviewer.

The reviewer receives:

- The complete page diff and rendered evidence
- The page description and acceptance checks
- The complete relevant TypeScript files
- The current source-line ownership table
- The Reactant documentation and public tests used by the implementation

Several source files are intentionally divided across pages. The reviewer still
receives the complete file, but every line has one disposition:

- Implemented by this task
- Implemented by an earlier task
- Intentionally assigned to a named later task
- Approved platform substitution

A later-task disposition is valid only when the current page description makes
that behavior out of scope. Task 40 audits every source line and requires a
terminal disposition.

The reviewer asks why each TypeScript line cannot have a direct Rust
counterpart and produces:

- A line-by-line TypeScript-to-Rust correspondence table
- A reason for every source line without a direct counterpart
- A list of extra Rust hosts, state, or glue absent from TypeScript
- A classification of each divergence as a sample defect, generalized Reactant
  friction, Unity limitation, or language-only difference
- A concrete Reactant improvement for every generalized divergence
- Black-box acceptance evidence for the page
- A written no-follow-up rationale when no improvement has merit

Task 1 has no TypeScript counterpart. Its reviewer instead examines gallery
registration, scrolling, reset, selected-state semantics, and authoring
ergonomics.

Every confirmed sample defect is corrected before the next migration. All
meritorious improvements from one page are grouped into zero or one immediate
follow-up commit containing:

- The generalized Reactant change
- Its public tests and documentation
- The page refactor proving the improvement
- No work belonging to the next migration

The follow-up receives ordinary correctness review, CI, candidate validation,
and a separate explicit promotion. It does not receive another specialized
port-ergonomics review. This prevents recursive review while ensuring every
confirmed improvement lands before the next page begins.

Correspondence tables, line ownership, and no-follow-up rationales are Markdown
attachments to the Tollgate candidate handoff. Screenshots are PNG attachments,
and automated evidence is the named Ditto result plus CI run. The handoff stores
the source and tested commit IDs so a later reviewer can retrieve the exact
artifact set. Do not embed planning or historical migration commentary in code
or repository documentation.

## Migration Pages

The page order begins with horizontal layout and controlled props, then adds
interaction, focus, accessibility, assets, motion, audio, and composition.

Three pages intentionally group closely coupled work: Task 19 validates one
help dialog including its link, Task 23 validates one generated-skin batch, and
Task 35 validates the two state-only settings panels. These remain one review
boundary because splitting them would not expose an independently meaningful
player interaction. The approximate 500-line task target still applies.

### Layout and shared controls

1. **Gallery shell**

   "Scrollable navigation selects one isolated demonstration; migrated mockup
   content is intentionally not asserted."

2. **PortraitViewport**

   "Fixed stage scales to fit available space; responsive content reflow is not
   asserted."

3. **ScreenFrame and ConceptFrame**

   "Arcade frame and clipped interior render; pulses, exits, generated skin,
   and controls are not asserted."

4. **SettingRow**

   "SettingRow aligns label and child horizontally; responsive reflow and
   interactive controls are not asserted."

5. **ToggleControl layout and state**

   "ToggleControl renders label, checkbox, and controlled toggling; focus,
   animation, and help remain unasserted."

6. **SelectControl closed state**

   "SelectControl renders selected value and caret; opening, options, focus,
   and animation remain unasserted."

7. **VolumeControl layout**

   "VolumeControl renders track, fill, thumb, value, and controlled changes;
   rich input and effects remain unasserted."

8. **ActionButton**

   "ActionButton renders typed children and invokes clicks; interaction states,
   particles, and navigation remain unasserted."

9. **SettingsTabs layout**

   "SettingsTabs selects controlled tabs horizontally; directional focus,
   panel transitions, and responsive labels remain unasserted."

10. **ScreenHeader**

    "ScreenHeader renders game and settings variants; generated wordmark,
    scaling, and animation remain unasserted."

### Interaction, focus, accessibility, and input

11. **useInteraction**

    "useInteraction drives hover, press, release, and cancellation visuals;
    focus modality and particles remain unasserted."

12. **Focus-visible behavior**

    "Keyboard and controller focus-visible states render correctly; pointer
    focus rings and complete controls remain unasserted."

13. **ToggleControl accessibility**

    "ToggleControl exposes labeled checkbox semantics and help description;
    effects, help modal, and composition remain unasserted."

14. **SelectControl pointer popover**

    "SelectControl opens one anchored listbox, selects options, and dismisses
    outside; keyboard behavior remains unasserted."

15. **SelectControl keyboard and controller behavior**

    "SelectControl supports arrows, Home, End, typeahead, Escape, restoration,
    and semantics; animation remains unasserted."

16. **VolumeControl input**

    "VolumeControl supports drag, keyboard steps, endpoints, pages, and
    controller input; release effects remain unasserted."

17. **SettingsTabs navigation**

    "SettingsTabs preserves four Tab stops and adds arrow and controller
    selection; animated panels remain unasserted."

18. **ArcadeModal behavior**

    "ArcadeModal traps focus, dismisses safely, restores its opener, and
    exposes dialog semantics; animation remains unasserted."

19. **InfoBadge and Privacy Policy**

    "InfoBadge opens accessible crash-report help and activates Privacy Policy;
    data erasure remains absent."

20. **Input settings table**

    "InputSettings scrolls bindings beneath a sticky header; rebinding,
    conflicts, and visual icons remain unasserted."

21. **Keyboard rebinding**

    "InputSettings captures keyboard bindings, rejects conflicts, resets
    defaults, and announces status; icons and controller rebinding are not
    asserted."

22. **FontScale**

    "FontScale reflows rows and scales text and controls; persistence and
    complete screens remain unasserted."

### Assets, effects, animation, and audio

23. **Generated control skin**

    "Generated assets skin controls and labels; interaction behavior, dynamic
    effects, and screen composition are not asserted."

    Copy these existing declarations into `chess-ui`; do not create a shared
    asset crate:

    - `ARCADE_SCREEN_FRAME`: 1024x1536
    - `SETTINGS_PANEL_FRAME`: 887x1021
    - `ACTION_BUTTON_FRAME`: 760x140, slices 24/26/24/26
    - `SMALL_CONTROL_FRAME`: 396x106, slices 15/15/15/15
    - `SETTINGS_TAB_ACTIVE`: 288x154, slices 30/42/18/42
    - `SETTINGS_TAB_INACTIVE`: 288x154, slices 30/42/18/42
    - `GAME_LOGO`: 900x360
    - `ACTION_LABEL_PLAY`, `SETTINGS`, `ABOUT`, `QUIT`, and `RETURN`:
      480x146 each
    - `CHECKBOX_UNCHECKED` and `CHECKBOX_CHECK`: 101x101 each
    - `VOLUME_SLIDER_TRACK`: 308x88, slices 18/18/18/18
    - `VOLUME_SLIDER_FILL`: 278x20
    - `VOLUME_SLIDER_TICKS`: 284x10
    - `VOLUME_SLIDER_HANDLE`: 68x88

    Generate all 18 assets through the ordinary asset command. The generator's
    `manifest.json` is authoritative; each runtime address has the form
    `battlement-reactant/generated/<request-hash>.png`. Validate the declaration
    count, symbol-to-hash mapping, and linked runtime catalog.

24. **Input icons and settings panel skin**

    "InputBindingIcons and the settings panel render precisely; rebinding
    behavior and full composition remain unasserted."

25. **Control shine and release bursts**

    "Buttons, checkboxes, and sliders play shine and keyed release bursts;
    ambient and route effects remain unasserted."

26. **Dropdown animation**

    "Dropdown and options animate presence, stagger, selection flash, and
    interruption; settings composition remains unasserted."

27. **ArcadeTabTransition**

    "ArcadeTabTransition enters, exits, and sweeps by direction; complete tab
    contents and routing remain unasserted."

28. **ArcadeModal animation**

    "ArcadeModal animates backdrop, panel, and shine with reduced-motion
    alternatives; screen composition remains unasserted."

29. **ArcadeAttractMode**

    "ArcadeAttractMode animates seeded grid and particles deterministically;
    menu controls and audio remain unasserted."

30. **ArcadeFramePulse**

    "ArcadeFramePulse animates border comets around the restored Return cutout;
    exits and route effects remain unasserted."

    This is an approved parity exception. The source tests
    `usePathname() === "/settings"`, but routing now stays at the root URL, so
    that condition never succeeds. Reactant applies the cutout when
    `active_screen == ArcadeScreen::Settings`. The source's existing
    `frame-pulse-right-edge-fixed.png` and the following mask geometry are the
    visual authority for the corrected state:

    ```text
    position: 0 0, 0 100%, 100% 100%
    size:     100% 1329px, 297px 75px, 297px 75px
    repeat:   no-repeat
    layers:   three linear-gradient(#000 0 0) masks
    ```

31. **BackgroundMusicProvider**

    "BackgroundMusic loops audio, applies effective volume and background mute,
    and exposes playback context; heartbeat remains unasserted."

32. **Music indicator and heartbeat**

    "MusicPlaybackIndicator mutes or enables sound while controls pulse from
    audio time; complete menu composition is not asserted."

33. **ArcadeMenuTransition**

    "ArcadeMenuTransition swaps keyed screens with beam and reveal effects;
    complete routed screens remain unasserted."

34. **ArcadeExitSequence**

    "ArcadeExitSequence and frame collapse synchronize dismissal; gameplay,
    quitting, and routed composition remain unasserted."

### Screen composition

35. **Gameplay and Graphics settings**

    "Gameplay and Graphics settings compose matching controls and props; other
    tabs and final transitions remain unasserted."

36. **SoundSettings**

    "SoundSettings composes three sliders and background mute against shared
    audio state; Input settings remain unasserted."

37. **InputSettings composition**

    "InputSettings composes bindings, icons, scrolling, rebinding, and its
    modal; cross-tab integration is not asserted."

38. **SettingsScreen**

    "SettingsScreen composes tabs, panels, Return, and both dialogs; main menu
    and route transition remain unasserted."

39. **MainMenu**

    "MainMenu composes background, header, buttons, music, and exit behavior;
    the complete router remains unasserted."

40. **ArcadeScreenRouter**

    "ArcadeScreenRouter composes every accessible mockup behavior; no
    player-visible behavior remains outside this page's scope."

    Before this candidate, run the project's single permitted independent
    review over the complete port. After promotion, run the ordinary required
    port-ergonomics reviewer and the final source-coverage audit.

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

At 100%, settings use the source's two-column rows. At 150% and 200%, each row
stacks its label above its control, grows by the source scale formulas, remains
scrollable, and scrolls the focused control fully into view.

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
  select the destination tab.
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

An action is **unconsumed** when the focused control, input-capture policy,
open listbox, dialog scope, and application router all return it as unhandled.
Only then may the review shell interpret Escape or controller Cancel as an exit
from Page 40.

## Automated Validation

Tests should describe player-visible behavior rather than private Rust
structure. A **fake host** is Reactant's deterministic non-Unity renderer used
to inspect committed UI output. A **semantic fixture** is its expected roles,
names, states, and relationships. Add focused unit tests only for genuinely
complex algorithms.

Pages 1--4, 10, 20, 23, and 24 are static for capture purposes. Every other
page is interactive or time-varying and supplies named initial, changed, and
reset states. All pages receive Ditto smoke and reset coverage; pages with a
meaningful interaction also receive a changed-state scenario.

Every task supplies validation appropriate to its page:

- Fake-host render, initial-state, interaction, reset, and semantic assertions
- An explicit `N/A` changed state for static pages
- Pointer, keyboard, controller, dismissal, focus-restoration, and modality
  matrices for interactive controls
- Controlled clocks and deterministic seeds for Motion behavior
- Numeric animation assertions copied from the exact rows of the pinned
  animation coverage ledger
- Asset-address uniqueness, canvas, slices, fonts, audio, and provenance checks
- Ditto initial, changed, and reset scenarios for each applicable page
- Smoke and reset checks for every previously registered page
- Targeted screenshot recapture whenever a shared component changes
- Unity-backend assertions for roles, names, states, relationships, table
  semantics, dialog isolation, live announcements, and external links
- At Task 40, a VoiceOver path through the macOS player and a TalkBack path
  through an Android player: launch, open Settings, change one tab and control,
  open and close each dialog, activate Return, and dismiss the full app
- A Task 40 audit assigning every source line a terminal disposition

The per-page smoke check opens the registered entry, finds its semantic target,
and asserts that no error or warning was emitted. The reset check mutates every
state domain owned by that page, reselects the same entry, and asserts the
documented default values, zero scroll, closed overlays, reset clock and audio,
and expected focus target.

Task 40 receives the project's single independent-review pass before candidate
submission. The required post-promotion port-ergonomics reviewer remains a
separate review and may produce one final Reactant follow-up.

## Manual QA

1. Launch `chess-ui`. Count 40 entries, read every description, navigate the
   full list with pointer, keyboard, and controller, and reselect entries. Pass
   when selection, focus, independent scrolling, and every reset value match the
   gallery contract.
2. Compare every visually applicable initial, changed, and reset state with its
   unchanged source crop at 1024x1536. Then capture the 2560x1440 integration
   view. Pass when geometry and pixel evidence meet the documented tolerances.
3. Exercise hover, press, release, pointer cancellation, focus-visible changes,
   D-pad, left stick, Submit, ignored shoulder buttons, and Cancel. Pass when
   controller and keyboard actions match and pointer focus never gains a
   keyboard-only ring.
4. Tab and Shift-Tab across all four settings tabs, then use every arrow,
   Home, and End. Test dropdown outside-click dismissal, Escape, typeahead, and
   focus restoration. Pass when focus and selection follow the defined order.
5. Drag sliders and use arrows, Page Up, Page Down, Home, and End. Capture a new
   shortcut, reject a conflict, cancel capture, and reset the binding. Pass when
   values, announcements, modal state, and display-only controller cells match
   the source.
6. Switch among 100%, 150%, and 200% text. Scroll the sticky input table and
   focus its final row. Pass when rows reflow, headings remain visible, focused
   content is revealed, and no text or control is clipped.
7. With VoiceOver and TalkBack, follow the Task 40 accessibility path. Activate
   Privacy Policy through a test host. Pass when roles, names, states,
   relationships, table semantics, dialog isolation, announcements, exact URL,
   and focus restoration are correct.
8. Observe each animation normally, with reduced motion, and while interrupted.
   Toggle sound, simulate unavailable playback, change visibility, background
   mute, and volume. Pass when timing follows the ledger and heartbeat, mute,
   enable, zero-volume restoration, and reset follow the audio contract.
9. Open the complete router. Verify every setting, Return, inert About, erase
   no-op, state-only controls, and identical Play and Quit exits. Pass when
   neither exit invokes gameplay or host shutdown and both finish on black.
10. Confirm the full-screen app contains no gallery chrome. After dismissal,
    press an otherwise unhandled Escape or controller Cancel. Pass when the
    review shell returns with Page 40 reset and no player-visible sample control
    was added.
