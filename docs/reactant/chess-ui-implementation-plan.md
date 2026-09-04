# Chess UI Reactant Port Implementation Plan

## Introduction

This plan defines a complete Rust port of the player-visible `mockups` React
application using **Reactant**, Battlement's declarative Rust component runtime.
The result is a standalone `chess-ui` sample whose appearance and behavior
match the pinned TypeScript source at desktop resolution.

The sample is a deliberate challenge to Reactant's API and architecture. Every
migration must strongly question why Reactant cannot express the source as
directly as React can. Consider major public API changes, replacement of
existing abstractions, and changes to runtime responsibilities when they make
application code simpler and preserve the source's behavior. The current
Reactant API is an implementation to improve, not a constraint to defend.

An audit that finds only naming, formatting, or small local cleanups while
leaving unexplained architectural differences has not fulfilled this plan.
Visual parity and passing tests are necessary, but insufficient: the sample
must demonstrate that ordinary application authors can express the same
component boundaries, controlled props, inline composition, labels, controls,
focus, accessibility, assets, and motion without unnecessary framework wiring.

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
- [Shared components](shared-components.md)
- [Shared components](shared-components.md)
- [Events and default actions](events-and-default-actions.md)
- [Asset generator](asset-generator.md#authoring-api)
- [Mockup animation coverage](animations.md#mockup-translation-coverage)

The events and default-actions work is a transitive prerequisite even though it
was not part of the original requested document list. Sliders, listboxes,
modals, tabs, and input rebinding cannot preserve native default behavior
without it.

The current accessibility design deliberately stops before listboxes, tables,
links, landmarks, and current-page state. The chess UI demonstrates a product
need for those semantics. Before Task 1 begins, a focused accessibility
extension must be designed, implemented, and certified. It adds only these
host-backed patterns:

- listbox and option semantics for `SelectControl`;
- table, row, header, and cell semantics for the input bindings;
- link semantics whose activation uses the existing external-URL request;
- navigation and region landmarks for the review gallery; and
- current-page state for its selected review-page button.

The extension composes the existing `SemanticProps`, `InteractionProps`, and
ordinary `FocusProps` contracts. It does not add virtual semantic nodes,
programmatic accessibility focus, a roving-focus engine, or a second input
navigation model. It is prerequisite work rather than a forty-first review
page.

The separate extension design may choose its public type names, but it cannot
weaken these behavior contracts:

- A listbox is one host-backed named container. Each logical option descendant
  is a host-backed named option with application-owned selected and disabled
  state. Option activation requests selection through its target-default
  handler. Arrow, Home, End, typeahead, and focus movement remain
  `SelectControl` handlers using queued ref actions.
- A table is one host-backed named container whose logical children are rows.
  Header cells identify their column or row scope, data cells remain in their
  containing row, and logical ancestry supplies the relationships. These nodes
  have no input-focus behavior unless the rendered host is independently
  interactive.
- A link is one named, ordinarily focusable host with an Activate action. The
  Privacy Policy handler issues the existing typed external-URL request; the
  semantic layer does not open URLs itself.
- Navigation and region landmarks are named host-backed containers with no
  actions and no implied input focus.
- Current-page state has the exact value `Page`, is valid on a button or link,
  and appears on exactly one review-page button in the gallery navigation.
- Rust validation, Unity mapping, **Ditto** black-box sample inspection, and
  VoiceOver and TalkBack evidence cover every added role, relationship, state,
  and action. Ditto is the repository's sample scenario runner.

The declarations in `samples/reactant/rules/src/assets.rs` are the source for
the 18 existing generated assets. The
[animation coverage ledger](animations.md#coverage-ledger) is the source for
motion timing, easing, direction, interruption, seed, and reduced-motion
requirements. This plan uses those as pinned starting evidence instead of
duplicating or rediscovering either design.

**Tollgate** is the repository's validation and exact-promotion service. A
**candidate** is one immutable source commit submitted to it. A **certified
release** is the repository's `release` ref after Tollgate has validated and
promoted that exact candidate. Implementation begins from the first certified
release containing every prerequisite named above. Each later task records its
exact starting release commit in its handoff, because those future commit
identifiers do not exist while this plan is written.

## Research Basis

Three independent research passes audited the mockup, Reactant, and the
dependency order for a port. They reached these conclusions:

- Visual fidelity is feasible after the prerequisite designs are implemented.
- Focus, default actions, accessibility, controller input, and application
  lifecycle are more significant obstacles than static layout.
- Existing generated assets should be copied into the new sample instead of
  being shared with or referenced from the Reactant rules sample.
- Application visibility, external links, and stable semantic test targeting
  are prerequisite Reactant capabilities. Callback props, behavior-bundle
  forwarding, and asset families may still expose authoring friction during the
  port.
- Browser-only diagnostics should not be ported unless they expose a
  generalized requirement that also applies to Unity applications.
- The source contains intentional prototype behavior, including inert actions
  and settings that modify only local state. Fidelity requires preserving those
  behaviors rather than completing the implied product features.

Before the first migration, compile and runtime probes verify that the certified
release supplies:

- Flex, grid, stack, scroll, sticky, overlay, and anchored-popover layout
- Programmatic focus, authored inertness, modal focus containment and
  restoration, focus-visible modality, and native directional navigation
- Accessible custom buttons, checkboxes, sliders, tabs, listboxes, dialogs,
  tables, links, navigation landmarks, regions, and current-page state
- Pointer capture, committed default actions, and input-capture policies
- Keyboard and normalized controller actions
- Presence, keyed animation, interruption, reduced motion, and audio time
- Audio playback and application-focus or visibility observation
- External-URL activation
- Stable semantic targeting for black-box tests

A later missing capability that blocks a page is implemented as a generalized
Reactant feature in that page's base change. The known accessibility extension
above must already be certified before Task 1. Sample-specific framework
adapters are not permitted.

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

## Review Gallery

The gallery contains exactly 40 registered entries. Page 1 demonstrates the
gallery shell itself. Every entry displays its title and a 10--20 word
description of the behavior asserted on that page.

The per-task **Visible result**, **Exercise**, and **Deferred** paragraphs below
are the visual acceptance contract. The short quoted descriptions are gallery
captions, not exhaustive scope definitions. Source-line ownership records must
follow this contract; they cannot independently defer a required visual feature.

Each contract describes the page immediately after its task is complete. Earlier
pages mount current shared components and acquire later improvements; they do
not preserve historical incomplete renderings. A later capability may therefore
appear on an earlier page without transferring ownership of its acceptance
evidence. Do not add feature switches solely to freeze an earlier appearance.

Each registration supplies:

```text
number, title, description, render_harness, reset_generation,
semantic_target, capture_states
```

`capture_states` is either `static` or an ordered list of named initial,
changed, and reset states. The container is a navigation landmark named
`Chess UI review pages`. Each item is a button named `<number>. <title>`; the
selected button exposes the extension's current-page state. The content is a
region labelled by its visible page heading. Selection recreates the harness
and queues focus on that heading. Directional controller input uses native
navigation to move focus among navigation buttons without selecting; Submit
activates the focused button.

The gallery navigation:

- Is vertically scrollable and remains outside the 1024x1536 design stage.
- Uses the navigation ScrollView ref's explicit `scroll_to` action to reveal the
  selected entry.
- Exposes current-page state on the selected review-page button.
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
- Focus target
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

Focus-visible presentation always follows the panel's current physical input
modality. A pointer selection hides the heading's focus-visible treatment;
keyboard or controller selection retains it. Resetting a page does not clear or
replace that panel-local modality.

For the review shell, an action is **unconsumed** when the focused control,
input-capture policy, open listbox, active dialog, and application router all
return it as unhandled.

Page 40 first displays its title, description, and a launcher. Activating the
launcher opens the complete app in an unanchored full-screen `Overlay::layer`
without gallery chrome or sample-only controls. The review shell authors its
gallery-content root inert while the app is open and queues focus on the app's
initial heading. The layer is a logical sibling of that inert root, so portal
ancestry does not make the app inert. This layer is not a dialog and must not
use `Overlay::modal`. An unconsumed Escape or controller Cancel closes the
layer, removes authored inertness, and restores launcher focus. Pointer-only
exit is intentionally absent; application reload remains available.

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

### What must look finished at each step

For numeric/style details not repeated in a task paragraph, the named component
and its imported style helpers at the pinned source revision supply the exact
values. The paragraphs define which state and layers are visible; the source
supplies those layers' complete rendering recipe. Do not approximate a source
value because this document describes its color or shape in words.

**Static appearance is required when a component first appears.** A task named
"layout", "closed state", or "behavior" is not permission to draw a wireframe.
Unless its Deferred paragraph explicitly names an exception, the component must
already match the pinned CSS reference in silhouette, clipping, corner cuts,
border thickness, every gradient stop, solid colors, inset and outer shadows,
glow, opacity, text paint, font, size, weight, spacing, alignment, and resting
transforms. These requirements include active, inactive, checked, unchecked,
disabled, and other settled states that the task exposes. Plain rectangular
borders, solid-color substitutes for gradients, omitted shadows, generic system
fonts, and default native control chrome are not acceptable placeholders.

A generated image is an implementation mechanism, not a visual feature. Task 23
owns the existing batch of generated assets, their declarations, catalog, and
runtime integration. It does not own the first appearance of a tab's shape or
gradient, a button's bevel, or a heading's text effects. Earlier tasks use typed
styles or other supported prepared paint to meet their static reference. Task 23
may replace that paint with generated assets while preserving approved geometry
and appearance. No browser render-mode toggle is added to the sample.

Interactions and time-varying effects are separate requirements. Before the task
that owns an effect, render its settled state without that effect: changing a
tab changes its active paint immediately; opening a dropdown or dialog shows
its fully open appearance immediately. Do not leave an entering element at
zero opacity, half scale, or another animation start value. Task 11 owns hover,
press, and cancellation presentation and its short feedback transitions; Task
12 owns focus-visible paint; Task 25 owns shine and release bursts; Tasks
26--30 and 33--34 own their named animations; Task 32 owns audio-driven pulses.
A new control introduced after one of these tasks must reuse the applicable
completed behavior. Animation alone may not be used to defer static paint.

Only these visual substitutions are permitted before their named owner:

| Visible feature | Required before its owner | First task requiring the final feature |
| --- | --- | --- |
| Generated frame and label textures | Complete matching CSS-reference appearance, rendered with supported paint | 23: generated asset integration |
| Keyboard and controller binding icons | Actual binding names as readable text in correctly sized cells | 24: source keycaps, arrows, and controller glyphs |
| Settings panel frame | No panel required on isolated control pages; behavior harnesses may use a plain dark backdrop | 24: complete panel specimen |
| Text-size-dependent layout | Source appearance at 100%; no guessed large-text layout | 22: 100%, 150%, and 200% specimens |
| Full settings and main-menu composition | Only the components and fixtures listed for the selected page | 35--40: the named assembled panels and screens |

In particular, **Task 9 must show the source's clipped tabs, multicolor active
border, gray inactive border gradient, dark inner gradients, glow, and text
shadows.** Task 17 adds directional navigation, Task 22 adds scaled tab labels,
Task 25 adds release bursts, and Task 27 adds content-panel transitions. None
of those tasks is the owner of Task 9's resting tab paint.

### Specimens, captures, and reset

Each Visible result lists the required specimens, not permission to compose the
whole source screen early. Render one source-sized specimen at a time when
multiple variants cannot fit without rescaling. Put variant selectors, parent
state controls, event counters, and clock controls in a clearly separate harness
area outside the reference crop. They must not replace source content or change
its layout. Use the source component's own colors on a plain dark stage when
its surrounding screen is not yet in scope; omit absent neighboring components
rather than filling their space with invented product UI.

For every page, selecting or reselecting its gallery entry returns to the listed
opening state, clears transient effects, and applies the gallery focus/reset
contract. Unless specified otherwise, 100% text, normal motion, and the defaults
in Behavioral Acceptance apply. "Reset" below means this gallery operation,
not an extra reset button inside the mockup. Static pages have an identical
reset capture and an `N/A` changed state. Variant captures are still required
when a static page lists multiple specimens.

Compare all paint belonging to the current and earlier tasks, including paint
implemented without generated assets. Mask only the explicitly deferred visual
features above or in the selected task's Deferred paragraph; record each mask
and its owner. There is no blanket pre-Task-23 pixel exemption. Task 23
recaptures affected earlier pages to prove that asset integration preserves
appearance. Task 24 recaptures binding pages when text substitutes become icons.

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

A **`wt` worktree** is the isolated checkout created and owned through Tollgate
for one task. Initial candidate submission never grants promotion authority.

The base-task workflow is:

1. Create a fresh Tollgate-owned `wt` worktree from the current release.
2. Implement one page, its resettable harness, **semantic fixture** containing
   its expected roles, names, states, and relationships, focused tests, and
   visual evidence. Apply the architectural questions below during
   implementation, before accepting extra application wiring. Target roughly
   500 non-test lines or fewer, but do not use this target to reject a necessary
   framework redesign; keep that redesign cohesive and reviewable.
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
- The relevant Reactant implementation and native Unity API evidence, which
  the reviewer inspects independently of the implementer's explanation

Several source files are intentionally divided across pages. The reviewer still
receives the complete file, but every line has one disposition:

- Implemented by this task
- Implemented by an earlier task
- Intentionally assigned to a named later task
- Approved platform substitution

A later-task disposition is valid only when the current task's Deferred
paragraph or the explicit visual ownership rules above assign that feature to
that named later task. A short gallery caption alone cannot justify deferral.
Task 40 audits every source line and requires a terminal disposition.

### Mandatory architectural challenge

Review the largest differences in responsibility and composition before minor
style issues. First run a separate, fresh-context subagent using the
[blind idealized Rust port prompt](idealized-rust-port-prompt.md). Give it only
the selected TypeScript snapshots and the prompt's fixed authoring guide,
including its brief generated-asset and Motion API context. It must not read
other Reactant project files, the existing port, this plan, or earlier audits.
The reviewer may inspect the implementation, but must not pass that context to
the blind subagent.

Record the independent draft and its proposed API contracts before comparing
them with the actual port. Then investigate what prevents that simpler version
and what changes would make it possible. Use typed Rust styles throughout;
runtime CSS strings or a CSS-string styling API are not acceptable. The existing
static asset-generator declaration grammar remains available for generated
PNGs, and runtime animation uses typed Motion builders. The goal is equivalent
expressive power and behavior, not matching token counts or manufacturing a
one-to-one translation of JSX syntax.

For every relevant component, answer these questions with concrete evidence:

1. **Are we preserving the source's actual behavior?** Trace defaults, ignored
   props, callbacks, state ownership, and event propagation. Identify every
   behavior added or changed by the port, even when it seems useful or harmless.
2. **What does React or the browser already do for the source?** Identify the
   work supplied by native elements and built-in relationships. Ask why
   Reactant or Unity cannot own the equivalent work. Do not compare a native
   HTML control with a hand-built Rust control and declare the extra wiring
   inevitable.
3. **Does Unity already provide this control or behavior?** Inspect native
   controls, styling of their internal parts, value events, and Reactant's
   existing facades. For a checkbox, examine `Toggle` before accepting a
   `Button` plus manually attached toggle behavior. If the native control is
   unsuitable, identify the exact missing capability and consider improving
   its Reactant facade before rebuilding it.
4. **Why is each hook, ID, ref, or association authored here?** Compare
   `use_label()` with the source's `useId()` and label relationship. Compare
   explicit focus and activation wiring with the source's wrapping `<label>`.
   Could a stable ID API, associated-label primitive, or composed control own
   these mechanics? Distinguish required internal bookkeeping from a required
   public call. A low-level React Aria hook is one API option, not proof that
   every application needs low-level hooks.
5. **Why can't the render tree be written inline and declaratively?** Challenge
   one-use `control` and `row` variables, mutable builders, conditional setters,
   callback adapters, and repeated clones. Try existing option-aware setters,
   consuming builders, and direct child expressions. If those are insufficient,
   propose the API or ownership change that would remove the friction. Retain
   a local binding when it adds clear meaning or supports real reuse, not
   because the first implementation happened to introduce it.
6. **What major API change would remove the largest remaining difference?**
   Consider replacing an abstraction, moving responsibility into Reactant, or
   revisiting an earlier design decision. Compare a concrete simpler call site
   with the current one. Do not stop at extracting boilerplate into a
   sample-specific helper, renaming a hook, or making a cumbersome pattern
   reusable when a better primitive would eliminate the pattern.

Every claimed unavoidable difference carries a burden of proof. Cite the exact
Rust rule or Unity API constraint, show the relevant implementation, and use a
minimal compile or runtime probe when feasibility is uncertain. Explain which
simpler alternatives were considered and why each fails. A missing Reactant
feature is not a Unity limitation. An ownership issue in the present callback
API is not automatically a language limitation. A design document that mandates
explicit semantics does not rule out a control abstraction that supplies those
semantics internally.

Unverified explanations remain unresolved findings. The reviewer must not mark
them justified, classify them as unavoidable, or issue a no-follow-up result.
Passing CI, matching screenshots, prior promotion, and fixing several trivial
issues do not discharge this architectural review.

### Required evidence and follow-up

The reviewer produces:

- The blind subagent's complete Rust draft, proposed contracts, supplied prompt,
  and TypeScript source revision, preserved before the feasibility review
- A line-by-line TypeScript-to-Rust correspondence table
- A reason for every source line without a direct counterpart
- An inventory of extra Rust hosts, hooks, refs, state, mutation, and glue,
  including whether each belongs in application code or inside Reactant
- A classification of each divergence as a sample defect, generalized Reactant
  friction, proven Unity limitation, proven language constraint, or unresolved
- Answers to the architectural questions above, led by the most consequential
  differences; explain any question that is not applicable
- A concrete simpler Rust call-site sketch and Reactant improvement for every
  generalized divergence, including major API changes where appropriate
- Evidence and rejected alternatives for every claimed unavoidable difference
- Black-box acceptance evidence for the page
- A written no-follow-up rationale only when there are no unresolved findings
  and the strongest simpler designs have been investigated and ruled out

Task 1 has no TypeScript counterpart. Its reviewer instead examines gallery
registration, scrolling, reset, current-page state, and authoring
ergonomics.

Every confirmed sample defect is corrected before the next migration. Resolve
architectural findings before advancing; do not carry them forward as optional
cleanup or defer them to an unspecified redesign. Accept an improvement when
it removes unnecessary application work while preserving the source contract
and framework correctness. Reject it only with a concrete explanation of why
the proposed authoring model is worse or infeasible. Implementation effort alone
is not a rejection reason. All accepted improvements from one page are grouped
into zero or one immediate follow-up commit containing:

- The generalized Reactant change
- Its public tests and documentation
- The page refactor proving the improvement
- No work belonging to the next migration

The follow-up receives ordinary correctness review, CI, candidate validation,
and a separate explicit promotion. It does not receive another specialized
port-ergonomics review. This prevents recursive review while ensuring every
confirmed improvement lands before the next page begins.

Correspondence tables, line ownership, architectural findings, simpler API
sketches, constraint evidence, and no-follow-up rationales are Markdown
attachments to the Tollgate candidate handoff. Screenshots are PNG attachments,
and the blind draft is a Rust source attachment. Automated evidence is the named
Ditto result plus CI run. The handoff stores the source and tested commit IDs so
a later reviewer can retrieve the exact artifact set. Do not embed planning or
historical migration commentary in code or repository documentation.

## Migration Pages

The page order begins with horizontal layout and controlled props, then adds
interaction, focus, accessibility, assets, motion, audio, and composition.

Three pages intentionally group closely coupled work: Task 19 validates one
help dialog including its link, Task 23 validates one generated-asset batch, and
Task 35 validates the two state-only settings panels. These remain one review
boundary because splitting them would not expose an independently meaningful
player interaction. The approximate 500-line task target still applies.

### Layout and shared controls

1. **Gallery shell**

   "Scrollable navigation selects one isolated demonstration; migrated mockup
   content is intentionally not asserted."

   **Visible result.** A 320-pixel scrollable navigation column lists all 40
   numbered entries beside the centered design stage, with the padding and scale
   specified under Review Gallery. Page 1 is selected; its heading and caption are
   visible. Future entries show their heading, caption, and an explicit unavailable
   specimen message until implemented. They must not display a fabricated mockup.

   **Exercise.** Select an entry, scroll to entry 40, and reselect the current
   entry. Exactly one navigation item indicates the current page; the content and
   heading change, navigation reveals selection, and reset restores the heading
   focus. Keyboard/controller focus is visibly distinguishable from selection.

   **Deferred.** Mockup components belong to Tasks 2--40. The shell's own readable
   navigation and focus presentation must already be complete.

2. **PortraitViewport**

   "Fixed stage scales to fit available space; responsive content reflow is not
   asserted."

   **Visible result.** One empty 1024x1536 portrait stage fits inside the
   gallery without cropping or stretching. Its aspect ratio stays 2:3; shrinking
   the window scales the entire stage uniformly according to the gallery formula.
   Use a visible stage boundary or measurement markers outside the source crop to
   make its edges reviewable.

   **Exercise.** Capture the canonical and integration window sizes. The stage
   remains centered, navigation scrolls independently, and no outer scrollbar is
   introduced. Reset yields the same empty stage.

   **Deferred.** The arcade frame is Task 3; no frame, logo, controls, responsive
   content rearrangement, animation, or audio is required here.

3. **ScreenFrame and ConceptFrame**

   "Arcade frame and clipped interior match their resting paint; pulses,
   exits, generated textures, and controls remain unasserted."

   **Visible result.** An empty arcade frame fills the portrait stage. Match
   `styles.ts`'s `frameClip` polygon, the 21-pixel outer inset, 111-pixel bottom
   inset, 8-pixel metallic border, cyan/blue/violet/pink border gradient and glow,
   and `ScreenFrame`'s clipped dark radial interior. The interior is empty, not a
   main-menu screenshot or placeholder control stack.

   **Exercise.** Capture the static frame and its reset at both review sizes.
   The corners, notches, border, and interior gradient must match the source.

   **Deferred.** Generated frame substitution is Task 23, moving border comets
   and the corrected Return cutout are Task 30, frame collapse is Task 34, and
   screen contents are Tasks 35--40. The resting frame's shape and gradient are
   required now.

4. **SettingRow**

   "SettingRow aligns label and child horizontally; responsive reflow and
   interactive controls are not asserted."

   **Visible result.** Source-width rows show a label in the 422-pixel left
   column and a plain child specimen in the right column. Include a first row,
   a normal separated row, a multiline label, and an explicit-height variant.
   At 100%, default minimum height is 159 pixels; labels use 61-pixel Bebas Neue,
   uppercase treatment, source horizontal stretch and shadows. The normal row
   has the source 2-pixel translucent separator; the first row does not.

   **Exercise.** Capture each static variant; reset reproduces it exactly.

   **Deferred.** The child is a size-marked specimen, not an unfinished control.
   Actual controls begin at Task 5. Stacked large-text rows belong to Task 22.

5. **ToggleControl layout and state**

   "ToggleControl renders label, checkbox, and controlled toggling; focus,
   animation, and help remain unasserted."

   **Visible result.** A source-styled labeled checkbox row opens checked.
   The 77x77 checkbox has its 4-pixel blue border, 11-pixel corner radius, dark
   vertical gradient, inset shadow, blue glow, and cyan clipped check mark. Show
   an unchecked specimen and the supported row-height/offset variants as well.
   Preserve the source's treatment of `first`; do not remove a separator when the
   source component ignores that prop.

   **Exercise.** Clicking toggles the controlled check mark on and off; changing
   `checked` from the harness changes the same specimen. Reset returns to checked.

   **Deferred.** Hover/press feedback is Task 11, keyboard-only focus paint Task
   12, help description semantics Task 13, the info badge/dialog Task 19, generated
   checkbox parts Task 23, and animated checkbox effects Task 25.

6. **SelectControl closed state**

   "SelectControl renders changing controlled values and its caret; opening,
   options, focus, and animation remain unasserted."

   **Visible result.** A Display Mode row opens with Borderless in the closed
   select trigger. Match the source's 396x106 trigger at 100%, clipped corners,
   3-pixel inset, cyan-to-pink border gradient, dark interior, glow, Barlow Condensed
   value text and shadow, and downward caret. The trigger remains closed.

   **Exercise.** Harness controls set Borderless, Fullscreen, and Windowed;
   only the displayed controlled value changes. Reset restores Borderless.

   **Deferred.** Hover/press feedback is Task 11, focus paint Task 12, opening
   and the option list Task 14, keyboard list navigation Task 15, generated frame
   substitution Task 23, and popover animation Task 26.

7. **VolumeControl layout**

   "VolumeControl renders track, fill, thumb, value, and controlled changes;
   rich input and effects remain unasserted."

   **Visible result.** A Master Volume row opens at 80 with the source's
   284-pixel track, proportional colored fill, tick marks, clipped metallic thumb,
   and numeric value. Match the track's dark gradient, fill gradient, thumb shape,
   shadows and glow; retain the source thumb overhang at both endpoints.

   **Exercise.** Parent controls set 0, 50, and 100. Fill width, thumb position,
   and numeral agree at every value. Reset restores 80.

   **Deferred.** Complete drag, keyboard, and controller input is Task 16;
   generated slider parts Task 23; release effects Task 25; audio behavior Task
   31; heartbeat Task 32. A generic native slider is not a visual substitute.

8. **ActionButton**

   "ActionButton renders typed children and invokes clicks; interaction states,
   particles, and navigation remain unasserted."

   **Visible result.** A source-size 760x140 ActionButton shows PLAY with
   cut corners, multicolor border, dark inset, source glow, and the gradient,
   stroked, shadowed label. Separate specimens cover custom typed children and a
   disabled button. Include ReturnButton at its source rectangle: left 328, top
   1358, width 368, height 120, with its dark backing and RETURN label.

   **Exercise.** Enabled buttons update an external activation counter. Disabled
   buttons do not. Return requests its callback without navigating a screen.
   Reset clears counters. Preserve the source's actual disabled appearance; do
   not invent a gray treatment.

   **Deferred.** Hover/press states are Task 11, focus paint Task 12, generated
   frames/labels Task 23, shine/bursts Task 25, and route/exit integration Tasks
   34 and 38--40.

9. **SettingsTabs layout**

   "SettingsTabs selects controlled tabs horizontally; directional focus,
   panel transitions, and responsive labels remain unasserted."

   **Visible result.** Show only the horizontal SettingsTabs strip, opening
   with Gameplay selected. Columns are 264, 212, 205, and 200 pixels with 2-pixel
   gaps in an 887x129 layout slot. Labels read Gameplay, Graphics, Sound, Input.
   The active tab is 130 pixels high; inactive tabs are 127 pixels high with the
   source's 3-pixel downward resting translation. Preserve bottom alignment and
   visible overflow rather than forcing all painted tops into the slot.

   The shape is mandatory: `tabOuterClip` cuts both top corners by 18 pixels,
   keeps bottom corners square, and encloses a 4-pixel inset whose top cuts are
   15 pixels. The active border is the 112-degree gradient with stops #72f5ff,
   #53afff at 44%, #9a83ff at 68%, and #ff4ed3. Inactive borders use the source's
   110-degree #657287 / #454f64 at 52% / #6f6577 gradient. Active and inactive
   interiors retain their distinct dark vertical gradients and inset shadows;
   the active tab has blue outer glow and a magenta inner bottom edge. Labels
   use Barlow Condensed 700, 55 pixels active and 51 inactive, 1-pixel tracking,
   #f7f7fb text, and the source text shadows. Rectangular solid borders fail this
   page even if their bounding boxes match.

   **Exercise.** Click each tab and show the active appearance moving to the
   chosen label, with exactly one selected tab. A parent control can select Sound;
   reactivating the current tab still emits its selection request. Reset selects
   Gameplay. Changes are immediate until feedback motion is added.

   **Deferred.** No content panel is required. Hover/press feedback is Task 11,
   focus-visible paint Task 12, arrows/Home/End/controller selection Task 17,
   scaled labels Task 22, generated frame substitution Task 23, release bursts
   Task 25, and content-panel transitions Task 27. Tab shape and all resting
   border/interior/text paint are required now.

10. **ScreenHeader**

    "ScreenHeader matches both painted heading variants; generated textures,
    text scaling, and surrounding screen composition remain unasserted."

    **Visible result.** Provide separate game and settings heading specimens
    at their source positions. The game heading reads CHESS CHESS on its first
    line and REVOLUTION on its second; the other reads Settings. Match the
    Barlow Condensed 800 italic letters, gradient fill, stroke, colored offset
    shadows, skew/stretch, and the blue left and pink right clipped stripe bars.
    At 100%, heading containers are left 84, width 854, with top/height 103/330
    for game and 74/122 for settings. Use the source's distinct text transforms.

    **Exercise.** Capture both static variants and their reset.

    **Deferred.** The generated logo is Task 23; its absence does not permit plain
    unpainted heading text. Font scaling is Task 22 and surrounding screen
    composition Tasks 38--39. No title animation is introduced.

### Interaction, focus, accessibility, and input

11. **useInteraction**

    "useInteraction drives hover, press, release, and cancellation visuals;
    focus modality and particles remain unasserted."

    **Visible result.** A specimen selector presents the existing checkbox,
    closed select, slider, action/Return button, and tabs with their completed
    resting paint. Show source hover, held-press, release, and canceled-press
    states, including brightness, border color, scale, and tab vertical offset.
    For example, an inactive hovered tab rises to y=-1 and a pressed tab scales
    to .955; the source's feedback transitions and reduced-motion branches apply.

    **Exercise.** Enter, press, release, drag out/cancel, and leave each specimen.
    Successful activation changes controlled state where applicable; cancellation
    clears pressed presentation without a successful activation. Reset restores
    resting paint and clears counts.

    **Deferred.** Keyboard-only focus appearance is Task 12. Shine sweeps,
    particles, and release bursts are Task 25; audio-driven pulse is Task 32.
    Popover, panel, and route transitions retain their later owners.

12. **Focus-visible behavior**

    "Keyboard and controller focus-visible states render correctly while
    pointer focus hides the keyboard-only ring; complete controls remain
    unasserted."

    **Visible result.** The Task 11 specimens also show their source
    keyboard/controller focus treatment: yellow/gold borders or outlines, white
    and yellow glow, and the appropriate focused gradient. Moving focus moves
    that treatment to one control; checked and selected states remain legible.
    Pointer focus retains ordinary pointer/hover paint without the keyboard ring.

    **Exercise.** Compare pointer click, Tab, Shift-Tab, and controller focus on
    each specimen. Reset restores the gallery heading focus using the current
    physical input modality, so a keyboard ring must not remain on an old control.

    **Deferred.** Full arrow selection for tabs belongs to Task 17; listbox and
    dialog navigation belong to Tasks 15 and 18. This page does not add panels,
    shine, particles, or heartbeat.

13. **ToggleControl accessibility**

    "ToggleControl exposes labeled checkbox semantics and help description;
    effects, help modal, and composition remain unasserted."

    **Visible result.** A labeled Upload Crash Reports checkbox row opens
    checked and has the same finished paint and feedback as the earlier checkbox.
    Its crash-report description is available to assistive technology without
    adding a visible paragraph to the source row. Include the `aria_label`
    override variant in the harness.

    **Exercise.** Activate through pointer, keyboard, controller, and semantic
    Activate; the check mark and checked state agree. Reset returns to checked.
    The description reads “We upload crash reports to Unity Diagnostics.”

    **Deferred.** The clickable info badge and visible help modal are Task 19.
    No help panel or screen composition is required on this page.

14. **SelectControl pointer popover**

    "SelectControl opens one anchored listbox, selects options, and dismisses
    outside; keyboard behavior remains unasserted."

    **Visible result.** Display Mode opens as the fully styled closed
    Borderless trigger. Clicking opens one source-styled list directly below it,
    with the same width and a 6-pixel gap. Show Borderless, Fullscreen, and
    Windowed, the selected check mark, hovered option background, clipped gradient
    frame, dark interior, and shadows. The caret points up while open.

    **Exercise.** Open the list, hover Windowed, select it, reopen, and dismiss by
    clicking outside. The trigger reads Windowed after selection; reset closes
    the list and restores Borderless. Open/close and caret reversal are immediate.

    **Deferred.** Keyboard/controller list navigation is Task 15. Presence,
    stagger, caret rotation timing, and selection-flash animation are Task 26.
    The fully open list's shape, gradients, text, and selected mark are required now.

15. **SelectControl keyboard and controller behavior**

    "SelectControl supports arrows, Home, End, typeahead, Escape, restoration,
    and listbox semantics through handlers and queued ref focus; animation
    remains unasserted."

    **Visible result.** The same listbox now visibly distinguishes the
    keyboard/controller active option from the committed selected option. Source
    focus paint follows the active option, while the check mark continues to
    identify the selected value until commitment.

    **Exercise.** Open from the trigger, use arrows, Home, End, and typeahead,
    commit Windowed, then reopen and Escape without changing it. The trigger
    regains focus and the correct modality treatment. Reset is closed Borderless.

    **Deferred.** This page adds no new decorative shell. Dropdown presence,
    stagger, selection flash, and interruption animation remain Task 26.

16. **VolumeControl input**

    "VolumeControl supports drag, keyboard steps, endpoints, pages, and
    controller input; release effects remain unasserted."

    **Visible result.** The finished Master Volume slider opens at 80.
    Dragging visibly moves its fill, thumb, and numeral together, with existing
    hover, press, and focus paint. Values remain integers between 0 and 100.

    **Exercise.** Drag to both endpoints; use arrows, Page Up/Down, Home/End, and
    controller actions. Match the source's step sizes, clamping, and touch padding.
    Cancel a captured pointer and confirm pressed paint clears. Reset restores 80.

    **Deferred.** Release bursts are Task 25 and playback integration Task 31. Do not
    add an audio visualization or redesign the already approved slider paint.

17. **SettingsTabs navigation**

    "SettingsTabs preserves four Tab stops and adds directional selection
    with visible focus; animated content panels remain unasserted."

    **Visible result.** The completed tab strip opens on Gameplay. Arrow,
    Home/End, and controller selection now move both the active tab paint and
    keyboard/controller focus treatment. All four labels remain sequential Tab
    stops; focused and selected are distinct states when ordinary Tab moves focus.

    **Exercise.** Wrap Input to Gameplay and back, select first/last, and traverse
    all four tabs with Tab/Shift-Tab. Reset selects Gameplay and restores gallery
    heading focus. Selection still renders without a content-panel animation.

    **Deferred.** Scaled labels are Task 22 and panel transition Task 27. This
    page requires no settings content panel and makes no new resting-skin change.

18. **ArcadeModal behavior**

    "ArcadeModal traps focus, dismisses safely, restores its opener, and
    exposes dialog semantics on its modal wrapper; animation remains
    unasserted."

    **Visible result.** An external opener shows a closed-dialog state. Open
    it to display the source erase-confirmation specimen: “Erase Saved Data?”, its
    source warning sentence, Cancel, and Erase. The stage behind it is darkened
    and blurred; the centered clipped panel has the source cyan border, layered
    dark gradients, inset/outer glow, title/body typography, and danger button.
    The panel is at its fully open size and opacity immediately.

    **Exercise.** Open, traverse contained focus, cancel with Escape, reopen,
    confirm, and dismiss through the source backdrop behavior. Each close
    restores opener focus; confirmation records an event without deleting data.
    Reset closes the dialog.

    **Deferred.** Opening/closing transforms and looping shine are Task 28.
    The real EraseControl row and its composition belong to Task 35; no complete
    Gameplay panel is needed here. Static modal paint is required now.

19. **InfoBadge and Privacy Policy**

    "InfoBadge opens accessible crash-report help and activates Privacy Policy;
    data erasure remains absent."

    **Visible result.** Upload Crash Reports appears checked with the source
    small circular blue “i” badge beside its label. Activating the badge opens the
    source help dialog: no invented visible title, the body “We upload crash
    reports to Unity Diagnostics.”, the cyan underlined Privacy Policy link, and
    OK. Match badge and link geometry, typography, border, glow, and placement.

    **Exercise.** Open help without toggling the checkbox; activate the link
    through a test host; dismiss and restore badge focus. Reset returns to the
    checked row with help closed. Host rejection keeps the same dialog visible.

    **Deferred.** Modal animation is Task 28; full Gameplay composition Task 35.
    The help body is not shown permanently beside the checkbox, and no erase
    interaction belongs to this page.

20. **Input settings table**

    "InputSettings scrolls bindings beneath a sticky header; rebinding,
    conflicts, and visual icons remain unasserted."

    **Visible result.** A source-size 839-pixel-wide input table shows Action,
    Keyboard, Controller headings and seven rows: Left, Right, Up, Down, Move
    Piece, Pause, Restart. Initial bindings match Behavioral Acceptance. Match
    row height, column widths, separator paint, header background,
    text, and scroll viewport. Keyboard/controller cells deliberately show binding
    names as text; they do not show placeholder squares or guessed glyphs.

    **Exercise.** Scroll until Restart is visible. The header stays fixed and
    aligned above its columns; reset returns to the top and default bindings.

    **Deferred.** Rebinding/dialog/conflict states are Task 21 and icon artwork
    Task 24. The full settings panel surround is not required.

21. **Keyboard rebinding**

    "InputSettings captures keyboard bindings, rejects conflicts, resets
    defaults, and announces status; icons and controller rebinding are not
    asserted."

    **Visible result.** The Task 20 table gains interactive keyboard cells.
    Opening Move Piece displays the finished “Change Shortcut” modal, “Press a key
    for Move Piece”, a cyan waiting marker, Cancel, and Reset. A conflicting key
    shows “Already used by <action>” in the source pink/red status styling; the
    modal stays open. The waiting marker's source blink is owned here.

    **Exercise.** Assign an unused key and see the cell text update, reject a
    conflicting key, cancel another capture, then reset a binding. Escape can be
    captured as a key. Gallery reset closes capture and restores all defaults.

    **Deferred.** Cells remain text-only until Task 24; controller cells remain
    display-only permanently. The modal's entrance/exit and shine are Task 28.

22. **FontScale**

    "FontScale reflows rows and scales text and controls; persistence and
    complete screens remain unasserted."

    **Visible result.** A text-size harness opens at 100% and offers 150% and
    200%. It presents representative rows of every existing control, the four-tab
    strip, both headings, action/Return buttons, dialogs, and the input table.
    At larger sizes, SettingRow labels stack above controls with the source gaps,
    padding, and height formulas; scroll containers reveal focused controls. Text
    and controls grow by their own source formulas, not one uniform scale.

    Tab widths remain 264/212/205/200 with 2-pixel gaps. Tab text multiplies its
    55/51-pixel base by `1 + (fontScale - 1) * .25`; Gameplay and Graphics also
    multiply by .92 above 100%. No abbreviated labels are introduced. Input columns
    change from 310/310/remainder to 260/340/remainder. Headings, controls, navigation
    labels, and Return's text cap follow `FontScale.tsx` and their component formulas.

    **Exercise.** Compare all three sizes, open a list/dialog at 200%, scroll to
    and focus the final input row, then reset to 100%. No text or focused control
    is clipped. Use separate specimens instead of squeezing everything into one
    stage.

    **Deferred.** Binding icons are Task 24; complete screens Tasks 35--40.
    Window narrowing still scales the portrait stage; it does not trigger reflow.

### Assets, effects, animation, and audio

23. **Generated control skin integration**

    "Generated assets replace matching frame and label paint; catalog integration
    and preserved appearance are asserted across earlier pages."

    **Visible result.** A static specimen selector displays every generated
    frame, label, logo, checkbox state, and slider part integrated at its real
    runtime size. Earlier pages retain the same approved shapes, gradients,
    shadows, text placement, and controlled resting states when generated artwork
    replaces their paint. Include active/inactive tabs, checked/unchecked boxes,
    slider endpoints, all action labels, both frame assets, and the game logo.
    The settings panel frame is an isolated asset specimen here, not a composed
    panel with rows or tabs.

    **Exercise.** Capture the static variants and every affected earlier page.
    Verify asset loading from the generated catalog with no missing-texture blocks,
    transparent holes, seams, stretch artifacts, or duplicated overlaid paint.
    The source CSS remains the visual target; reconcile a recipe mismatch rather
    than approving a changed appearance merely because generation succeeded.

    **Deferred.** Input glyphs and the assembled panel surround are Task 24.
    No new hover, focus, burst, or transition behavior is introduced; retain
    completed interaction behavior on earlier pages. Generated asset integration
    must not be reported as the first completion of their static appearance.

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

    **Visible result.** Two static specimens are required. First, display the
    input table with actual source keycaps, directional arrows, D-pad marks, green
    A, gray menu, and yellow Y glyphs in place of the text-only binding substitutes.
    Second, display the 887x1021 settings panel surround with its clipped bottom
    corners, thin blue/purple gradient border, layered dark interior, inset shadow,
    and 18/24/32-pixel top/horizontal/bottom content padding. Use a plain interior
    specimen to show the padding without assembling settings content.

    **Exercise.** Capture default and long/custom binding variants at each text
    size; reset reproduces them. Earlier input pages now render icons while
    retaining rebinding and scrolling. Check the final row beneath the sticky header.

    **Deferred.** Full Input composition is Task 37 and tabs/panels/Return
    integration Task 38. No new rebinding policy or controller editing is added.

25. **Control shine and release bursts**

    "Buttons, checkboxes, and sliders play shine and keyed release bursts;
    ambient and route effects remain unasserted."

    **Visible result.** Existing action/Return buttons, tabs, select triggers
    and options, checkboxes, and sliders retain their finished paint and gain
    the source shine or keyed effect layers where their source uses them.
    Action-button highlight shows its moving shine; successful releases produce
    the source compact or full particle burst. Checkbox state changes and slider
    release use their own source effect shapes and anchors.

    **Exercise.** Trigger each effect, capture its ledger-defined intermediate
    state and settled result, retrigger before completion, and cancel a press.
    Effects must originate at the control and leave no residue. Reset clears all
    particles and keys; reduced motion follows each source branch.

    **Deferred.** Dropdown presence/selection flash is Task 26; modal shine Task
    28; ambient effects Task 29; heartbeat Task 32. No effect can excuse a
    resting-paint mismatch. EraseControl reuses its source-prescribed compact
    button burst when that control is introduced in Task 35; modal buttons do
    not gain a burst absent from their source.

26. **Dropdown animation**

    "Dropdown and options animate presence, stagger, selection flash, and
    interruption; settings composition remains unasserted."

    **Visible result.** The finished select now animates from its closed
    state into the fully painted list: the panel reveals below the trigger,
    options enter with source stagger, the caret rotates, and selection flashes
    before the list closes. Intermediate scale/translation/opacity match the
    pinned ledger; settled open and closed appearances match Tasks 14--15.

    **Exercise.** Open, select Windowed, reopen during closing, and dismiss
    outside/Escape. Capture opening, selection flash, interrupted replacement, and
    settled states. Reset returns to closed Borderless with no lingering overlay.
    Reduced motion removes the source-disallowed movement.

    **Deferred.** No settings screen composition or tab-panel transition is added.

27. **ArcadeTabTransition**

    "ArcadeTabTransition enters, exits, and sweeps by direction; complete tab
    contents and routing remain unasserted."

    **Visible result.** The completed tab strip sits above a source-sized
    panel viewport containing a simple labeled content specimen for each category.
    Gameplay is initial. Switching categories produces the source directional
    enter/exit motion and sweep; after settling exactly one correctly labeled
    specimen remains visible. The tab strip's own shape and paint do not change.

    **Exercise.** Switch right and left, wrap, and interrupt a transition with
    another selection. Capture both directions and the reduced-motion result.
    Reset restores Gameplay with no outgoing content or sweep remaining.

    **Deferred.** The specimens are not real Gameplay/Graphics/Sound/Input
    contents; those belong to Tasks 35--37. Full SettingsScreen is Task 38.

28. **ArcadeModal animation**

    "ArcadeModal animates backdrop, panel, and shine with reduced-motion
    alternatives; screen composition remains unasserted."

    **Visible result.** The finished erase, help, and rebinding modal
    specimens gain backdrop fades, panel reveal/collapse, skew/brightness changes,
    and looping panel shine as specified in the ledger. Their fully open text,
    buttons, borders, and dimensions remain those already approved.

    **Exercise.** Open and close each variant, interrupt an entrance with close,
    and reopen during exit. Capture entrance, open shine, and exit. Exiting content
    is inert and cannot retain focus. Reset closes all dialogs and clears shine;
    reduced motion uses the source's short fades without the large transforms.

    **Deferred.** No new modal content or settings-screen composition is added.

29. **ArcadeAttractMode**

    "ArcadeAttractMode animates seeded grid and particles deterministically;
    menu controls and audio remain unasserted."

    **Visible result.** The portrait frame encloses only the source attract
    background: its layered grid and seeded ambient particles at the source
    positions, colors, opacity, and clipping. At time zero it shows the defined
    initial seed state, not an arbitrary screenshot of a running effect.

    **Exercise.** Advance the controlled clock to ledger capture times; the grid
    and particles move deterministically. Reset returns to the identical initial
    seed/time image. Reduced motion follows the source's static/reduced alternative.

    **Deferred.** No logo, menu buttons, music indicator, or audio belongs to
    this background specimen; the assembled main menu is Task 39.

30. **ArcadeFramePulse**

    "ArcadeFramePulse animates border comets around the restored Return cutout;
    exits and route effects remain unasserted."

    **Visible result.** The finished empty arcade frame gains moving border
    comets. A harness switches between main and settings frame contexts. The
    settings specimen includes Return at its source location and the corrected
    bottom-center cutout, so comets follow the remaining frame rather than crossing
    through Return. Main has the source main-frame path.

    **Exercise.** Capture both contexts at pinned pulse times and verify the
    cutout geometry below. Reset restores the main context at time zero; reduced
    motion follows the ledger.

    **Deferred.** Frame collapse is Task 34 and complete routed settings Task
    38. Only the documented settings cutout is a parity correction.

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

    **Visible result.** A harness shows the existing Master Volume and Music
    Volume sliders at 80 and 65, background mute off, and external playback,
    visibility, and mute status. Activating playback advances the displayed
    playhead; sliders retain their approved paint. Status is harness UI outside
    the reference crop, not an invented source music player.

    **Exercise.** Change volumes and simulate hidden/visible state. Effective
    volume reflects .8 × .65 initially; background mute affects hidden playback
    without pausing the playhead. Reset stops playback at zero and restores values.
    Unavailable playback is explicit in harness status.

    **Deferred.** The source music indicator and visible heartbeat are Task 32.
    No waveform, equalizer, or new product audio controls are introduced.

32. **Music indicator and heartbeat**

    "MusicPlaybackIndicator mutes or enables sound while controls pulse from
    audio time; complete menu composition is not asserted."

    **Visible result.** The source indicator reads “Playing with sound” and
    “is recommended!” on two centered lines, with its source position, font, and
    shadow. Active sound has no crossed-out speaker below it; muted/unavailable
    sound shows the source gray speaker-slash icon. Representative completed
    controls pulse in scale/brightness from audio time, using the specified two-hit
    heartbeat rather than an unrelated animation clock.

    **Exercise.** Mute and enable without rewinding, restore zero volumes through
    enable, simulate unavailable playback, and compare normal/reduced motion.
    Capture sound-on, muted, and ledger-timed pulse states. Reset stops the
    playhead and clears the pulse until playback is activated again.

    **Deferred.** The surrounding main menu is Task 39. Audio status diagnostics
    remain outside the source crop.

33. **ArcadeMenuTransition**

    "ArcadeMenuTransition swaps keyed screens with beam and reveal effects;
    complete routed screens remain unasserted."

    **Visible result.** A keyed screen-transition harness opens on a clearly
    labeled Main specimen. A harness action replaces it with a Settings specimen
    using the source beam, reveal, overlay, and outgoing/incoming motion. The
    specimens occupy the source screen bounds inside the completed frame; they
    are simple distinguishable contents, not prematurely assembled screens.

    **Exercise.** Transition in both directions and interrupt with a replacement.
    Capture ledger-defined intermediate layers and the settled destination.
    Reduced motion follows the source alternative. Reset restores Main with no
    beam or outgoing screen visible.

    **Deferred.** First-navigation policy and route ownership are Task 40;
    complete MainMenu and SettingsScreen are Tasks 38--39.

34. **ArcadeExitSequence**

    "ArcadeExitSequence and frame collapse synchronize dismissal; gameplay,
    quitting, and routed composition remain unasserted."

    **Visible result.** A completed frame with a representative painted
    button/content specimen starts intact. An external trigger runs the source
    exit overlay and synchronized frame brightness, distortion, and collapse,
    ending on an entirely black stage. Gallery navigation remains outside the
    stage; no invented game, farewell message, or quit dialog appears.

    **Exercise.** Capture the intact, ledger-timed collapsing, and black states;
    repeat with reduced motion. Once exiting, controls become inert and focus
    clears. Reset restores the intact initial frame and specimen.

    **Deferred.** Play/Quit wiring and the complete menu are Task 39; full-app
    review-layer dismissal is Task 40. Neither gameplay nor host shutdown will
    be added by those tasks.

### Screen composition

35. **Gameplay and Graphics settings**

    "Gameplay and Graphics settings compose matching controls and props; other
    tabs and final transitions remain unasserted."

    **Visible result.** Separate Gameplay and Graphics specimens contain
    complete, source-painted row contents within the approved panel surround.
    Gameplay opens with Language English, Text Size 100%, Reduce Motion off,
    Increase Move Duration on, Upload Crash Reports on with its info badge, and
    the fully painted red ERASE row. Graphics opens with Resolution 1920 × 1080,
    Max Framerate 144 FPS, Display Mode Borderless, Screenshake on, and VSync on.
    Match source row order, offsets, line breaks, spacing, and panel scrolling.

    **Exercise.** Change every control and compare all text sizes. Selecting
    150% or 200% reflows the specimen. Harness callbacks may demonstrate the already
    completed help/erase dialogs, but are outside cross-screen routing. Reset
    restores all listed values and closes overlays.

    **Deferred.** Sound and Input composition are Tasks 36--37; the shared tab
    strip, header, Return, and integrated dialog ownership are Task 38. No platform
    locale, graphics, saved-data, or gameplay effects are added.

36. **SoundSettings**

    "SoundSettings composes three sliders and background mute against shared
    audio state; Input settings remain unasserted."

    **Visible result.** The Sound panel contains Master Volume 80, Music
    Volume 65, Effects Volume 75, and unchecked Mute in Background in source order,
    with the source multiline labels, spacing, and approved slider/checkbox paint.
    Its panel surround and large-text layout match earlier specimens.

    **Exercise.** Drag all three sliders, toggle background mute, and simulate
    hidden/visible playback. Fill, thumb, and numeral agree; master/music affect
    the shared audio state. Effects volume changes its visible value only. Reset
    restores all four defaults, scrolling, and the audio lifecycle contract.

    **Deferred.** Input composition is Task 37 and cross-tab settings ownership
    Task 38. This page does not add menu chrome or a new effects sound.

37. **InputSettings composition**

    "InputSettings composes bindings, icons, scrolling, rebinding, and its
    modal; cross-tab integration is not asserted."

    **Visible result.** The Input panel combines the approved surround,
    sticky Action/Keyboard/Controller header, seven binding rows, real icons,
    scrolling, and the animated Change Shortcut modal. At default size the first
    rows appear at the source scroll position; scrolling reveals Restart beneath
    the fixed header. Larger text follows Task 22.

    **Exercise.** Rebind Move Piece, show a conflict, cancel, reset one binding,
    and scroll/focus the last row. Updated key text/icon matches the binding.
    Gallery reset closes capture, restores all default bindings, and scrolls to top.

    **Deferred.** The tab strip, settings title, Return, and state integration
    with the other settings panels are Task 38. Controller cells remain display-only.

38. **SettingsScreen**

    "SettingsScreen composes tabs, panels, Return, and both dialogs; main menu
    and route transition remain unasserted."

    **Visible result.** The complete settings screen opens on Gameplay:
    finished arcade frame and pulse/cutout, Settings heading, four tabs at left
    68/top 233, the 887-pixel panel below the tab strip, complete active panel
    contents, and Return at its source position. No dialog or dropdown is open.
    Every visible component includes the paint, icons, effects, scaling, and
    behavior completed earlier.

    **Exercise.** Visit all four tabs with animated panel replacement; edit values,
    return to panels and verify retained state; open help, erase confirmation, and
    rebinding. Erase confirms and closes without deleting anything. Return emits a
    main-route request to the harness. Reset restores Gameplay and every default.

    **Deferred.** Return need not reveal a real main menu here: that composition
    is Task 39 and integrated routing Task 40. No settings chrome or static paint
    is deferred beyond this page.

39. **MainMenu**

    "MainMenu composes background, header, buttons, music, and exit behavior;
    the complete router remains unasserted."

    **Visible result.** The complete main menu shows the arcade frame and
    pulse, attract background, CHESS CHESS REVOLUTION heading, Play, Settings,
    About, Quit in their source stack, and the music recommendation/indicator.
    At 100%, buttons occupy left 132, width 760, starting top 476, with 140-pixel
    heights and 24-pixel gaps. Playback and heartbeat follow the audio contract.
    No settings panel is shown initially.

    **Exercise.** About leaves the view unchanged. Settings emits a navigation
    request. Play and Quit each run the same exit sequence and finish on black;
    neither starts gameplay nor shuts down the host. Exercise sound mute/enable.
    Reset restores the complete menu, initial state, and playback lifecycle.

    **Deferred.** Settings-to-main route integration and first-navigation
    transition policy are Task 40. No main-menu paint or exit behavior is deferred.

40. **ArcadeScreenRouter**

    "ArcadeScreenRouter composes every accessible mockup behavior; no
    player-visible behavior remains outside this page's scope."

    **Visible result.** The gallery initially shows only this page's heading,
    caption, and launcher. Launching displays the entire source-matching app in a
    full-screen layer with no gallery navigation, counters, specimen selectors,
    clock controls, or other review UI. It opens on the complete main menu.

    **Exercise.** Settings opens the complete Gameplay settings screen; switch
    all tabs, edit controls, open each dialog, change text size, and Return to main.
    Preserve the source first-navigation rule and animate later route replacements.
    Play/Quit finish on black. An otherwise unconsumed Escape/controller Cancel
    closes the app layer and restores launcher focus. Relaunch/reselect resets all
    values, bindings, scroll, overlays, route state, effects, and audio lifecycle.

    **Deferred.** Nothing player-visible remains deferred. Only the documented
    platform substitutions, corrected Return cutout, and intentional prototype
    behaviors are exceptions to source parity. No browser render-mode diagnostics
    or sample controls appear inside the app.

    Before this candidate, run the project's single permitted independent
    review and final source-coverage audit over the complete port. After
    promotion, run the ordinary required port-ergonomics reviewer. If that
    review produces a Reactant follow-up, refresh every affected source-coverage
    and correspondence entry before the follow-up candidate. The port is not
    complete until the final promoted follow-up, or the no-follow-up rationale,
    retains a complete terminal audit.

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
scrollable, and scrolls the focused control fully into view. Each focusable
control retains an `ElementRef`; its focus handler calls the containing
ScrollView ref's explicit `scroll_to` action. The focus coordinator does not
perform automatic reveal.

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

## Automated Validation

Tests should describe player-visible behavior rather than private Rust
structure. A **fake host** is Reactant's deterministic non-Unity renderer used
to inspect committed UI output. Add focused unit tests only for genuinely
complex algorithms.

Pages 2--4, 10, 23, and 24 are static for capture purposes. Every other page is
interactive or time-varying and supplies named initial, changed, and reset
states. All pages receive Ditto smoke and reset coverage; pages with a
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
- Unity-backend assertions for roles, names, states, relationships, listbox and
  table semantics, landmarks, current-page state, dialog isolation, live
  announcements, and external links
- At Task 40, a VoiceOver path through the macOS player and a TalkBack path
  through an Android player: launch, open Settings, change one tab and control,
  open and close each dialog, activate Return, and dismiss the full app
- A Task 40 audit assigning every source line a terminal disposition; each
  resulting Reactant follow-up refreshes its affected audit entries before
  candidate submission
- A complete architectural challenge over the assembled application, including
  earlier components; prior per-page approval does not exempt accumulated glue,
  repeated associations, or inconsistent control APIs from redesign

The per-page smoke check opens the registered entry, finds its semantic target,
and asserts that no error or warning was emitted. The reset check mutates every
state domain owned by that page, reselects the same entry, and asserts the
documented default values, zero scroll, closed overlays, reset clock and audio,
and expected focus target.

Task 40 receives the project's single independent-review pass before candidate
submission. The required post-promotion port-ergonomics reviewer remains a
separate review and may produce one final Reactant follow-up. That follow-up
cannot promote until its affected source-coverage and correspondence entries
are current and the complete audit remains terminal.

## Manual QA

1. Launch `chess-ui`. Count 40 entries, read every description, navigate the
   full list with pointer, keyboard, and controller, and reselect entries. Pass
   when the named navigation and region, current-page state, selection, focus,
   explicit scrolling, and every reset value match the gallery contract.
2. Compare every visually applicable initial, changed, and reset state with its
   unchanged source crop at 1024x1536. Then capture the 2560x1440 integration
   view. Pass when geometry and pixel evidence meet the documented tolerances.
3. Exercise hover, press, release, pointer cancellation, focus-visible changes,
   D-pad, left stick, Submit, ignored shoulder buttons, and Cancel. Pass when
   controller and keyboard actions match and pointer focus never gains a
   keyboard-only ring.
4. Tab and Shift-Tab across all four settings tabs, then use every arrow,
   Home, and End. Test dropdown outside-click dismissal, Escape, typeahead, and
   focus restoration. Pass when focus and selection follow the defined order
   and the dropdown publishes one listbox with the expected options.
5. Drag sliders and use arrows, Page Up, Page Down, Home, and End. Capture a new
   shortcut, reject a conflict, cancel capture, and reset the binding. Pass when
   values, announcements, modal state, and display-only controller cells match
   the source.
6. Switch among 100%, 150%, and 200% text. Scroll the sticky input table and
   focus its final row. Pass when rows reflow, headings remain visible, focused
   content is revealed, and no text or control is clipped.
7. With VoiceOver and TalkBack, follow the Task 40 accessibility path. Activate
   Privacy Policy through a test host. Pass when roles, names, states,
   relationships, listbox and table semantics, link activation, dialog
   isolation, announcements, exact URL, and focus restoration are correct.
8. Observe each animation normally, with reduced motion, and while interrupted.
   Toggle sound, simulate unavailable playback, change visibility, background
   mute, and volume. Pass when timing follows the ledger and heartbeat, mute,
   enable, zero-volume restoration, and reset follow the audio contract.
9. Open the complete router. Verify every setting, Return, inert About, erase
   no-op, state-only controls, and identical Play and Quit exits. Pass when
   neither exit invokes gameplay or host shutdown and both finish on black.
10. Confirm the full-screen app contains no gallery chrome. After dismissal,
    press an otherwise unhandled Escape or controller Cancel. Pass when the
    app layer makes gallery content inert without publishing a dialog, the
    launcher regains focus, the review shell returns with Page 40 reset, and no
    player-visible sample control was added.
