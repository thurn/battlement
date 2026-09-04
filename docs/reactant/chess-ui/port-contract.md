# Port contract

[Plan and reading guide](../chess-ui-implementation-plan.md)

## Port Contract

Preserve both the player-visible experience and the source's authoring model.
Component names, prop meaning, controlled state ownership, and high-level
composition must remain aligned with TypeScript. Browser mechanisms may become
Unity mechanisms inside Reactant; their replacement must not automatically
become additional work for every application component.

For every difference, first ask: "Why can't Reactant work this way, and what
would we change in Reactant to make it possible?" Do not substitute the easier
question of how to fit the source into today's API. A limitation of the current
builder, hook, host facade, semantic model, or runtime is a candidate framework
defect. Only a demonstrated language or platform constraint can justify a
necessary divergence, and the reviewer must still consider hiding its mechanics
inside a reusable abstraction.

### Runnable sample

The result is a routinely discoverable `samples/chess-ui` sample with:

- Application name `Chess UI.app`
- Unity scene `Assets/Scenes/ChessUi.unity`
- Rules crate `battlement-chess-ui-rules`
- No sample-specific C# scripts
- Standard Unity serialization, Addressables, Ditto black-box sample
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

Use the [validation sequence](workflow.md#validation-sequence) to schedule these commands;
do not run the entire list after each edit:

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
and required or optional status. Inspect how each prop is actually used, not
just its declaration. A prop accepted through a shared TypeScript type but
ignored by the component must remain behaviorally inert. For example,
`ToggleControl` accepts `first` through `BaseProps` but does not forward it to
`SettingRow`; making it suppress a separator would introduce a feature absent
from the source. Do not complete inert callbacks, add convenient behavior, or
"fix" prototype choices under the guise of porting. Important control contracts
are:

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

Every extra host, hook, ref, state value, callback adapter, mutation, and
intermediate render variable needs scrutiny. Show why it is necessary at the
application call site, not merely why the current implementation uses it.
"Rust is different," "the borrow checker requires it," "Unity is not the DOM,"
"accessibility needs it," and "this follows the existing architecture" are not
sufficient explanations. Investigate the concrete constraint and a simpler API
before accepting the difference.

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

The listed platform substitutions do not exempt their application-facing APIs
from review. A different backend does not by itself require a more complicated
component.

Framework improvements are not limited to small helpers around existing APIs.
Consider redesigning builders, control primitives, label and ID associations,
callback ownership, behavior composition, semantic defaults, and the division
of work between Rust and Unity. Inspect existing native controls and their
styling and event facilities before recreating their behavior. A need exposed
by one page is sufficient when the capability belongs in a general UI framework;
do not demand several applications before addressing obvious authoring friction.

Existing Reactant design documents explain current decisions; they do not prove
those decisions remain appropriate. Challenge and revise them when the source
demonstrates a better authoring contract. Preserve the underlying correctness
requirements, including controlled state, lifecycle safety, focus behavior, and
accessible meaning, while changing where that work is performed. Each accepted
change includes focused public black-box coverage where behavior warrants it,
concise public documentation, and the sample refactor that proves the API works.

Major breaking API changes are explicitly in scope. Backward compatibility,
versioning, sunk implementation effort, and the size of the existing API are
not reasons to retain an inferior authoring model. Large changes still receive
proportionate correctness review and validation; they must not be reduced to
cosmetic fixes merely to keep the diff small.
