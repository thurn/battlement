# Battlement UI implementation plan

When a task is complete, append `[DONE]` to its task heading.

Status: implementation companion to `docs/battlement-ui-technical-design.md`

This plan implements the approved Battlement UI contract without expanding or
revising its scope. The technical design remains normative. If this plan and
the technical design disagree, the technical design wins.

## Decisions and starting point

No Battlement UI implementation exists yet. The repository contains the
completed core Rust crates, Unity package, JSON protocol, fake client, sample
workflow, and visual-capture infrastructure on which the UI work builds.

The following decisions were resolved while preparing this plan:

- `samples/ui` is a new standalone Unity project and Rust rules crate. It is an
  interactive UI lab rather than an extension of another sample.
- The lab uses a persistent navigation column, active specimen canvas, and
  state/event/command inspector. Its visual language is a dark Battlement
  command deck with restrained cyan and amber accents.
- Every public capability has a visible lab specimen: all 23 elements, all 86
  styles, every typed part slot, document mode, action, event family, asset
  source, and controlled-state behavior. A checked coverage ledger maps each
  capability to its implementation task, test family, and sample specimen.
- Sample-specific navigation, state, event handling, mutations, and diagnostics
  live in Rust. The sample contains no game-specific C#. Reusable package and
  external capture-harness C# remain allowed.
- Sample apps contain no inline Rust unit tests. All Rust test coverage is
  expressed through the `battlement-fake` crate and exercises samples
  exclusively through public black-box behavior.
- The sample uses one integrated scene. Its screen-space document is the lab
  shell; the render-modes page also presents a target-texture document on an
  in-world monitor and a separate interactive world-space console.
- The sample receives a small original asset kit covering texture, sprite,
  vector image, render texture, cursor, legacy font, and UI font sources. It
  does not import a third-party UI pack.
- User-relevant rejection, rollback, input gating, and lifecycle behavior are
  visible. Malformed protocol, missing asset, partial mutation, and injected
  Unity failure cases remain automated-test fixtures rather than sample controls.
- Every task produces a browser-playable WebGL build, verifies it through
  browser QA, and includes a direct deployed Web demo link in its review
  handoff. One final packaged macOS Release player additionally proves the
  standalone native result.

## Task and testing conventions

Implementation is a mostly linear stack. Each task depends on the preceding
task unless its prerequisites say otherwise, leaves the workspace compiling,
and adds a runnable, reviewable increment to `samples/ui`.

Task numbers are coordination metadata used only inside this implementation
plan. Never put them in product or sample assets, filenames, source comments,
or ancillary Markdown documentation. Name durable repository artifacts after
their behavior, domain, or scenario instead.

Stable identities written into sample code must use independently generated
UUIDs. Never use illustrative sequences, zero-padded counters, or IDs chosen to
encode ordering; screenshots and inspector output must display the same real
sample identities.

Public UI components and their properties require durable, user-oriented API
documentation comparable in substance to Unity's UI Toolkit documentation.
Component documentation must explain purpose, behavior, layout role, important
distinctions from related components, and appropriate usage. Property fields
and builder methods must explain their practical effect, units, inheritance or
container relationships, and important interactions with related properties;
never merely restate the identifier. Comments must describe the lasting API
contract and must not refer to implementation phases, labs, milestones, tasks,
or why a field happened to be introduced.

Inline Rust unit tests in sample source are strictly forbidden. Delete every
existing Rust `#[cfg(test)]` module from sample apps. All Rust tests must be
black-box integration tests expressed through the `battlement-fake` crate,
which drives the real sample through public builders, serialization,
validation, routing, transport, and observable client behavior.

- Rust tests may import only public sample and Battlement APIs and may assert
  only externally observable state or protocol output. Sample code exposes no
  Rust test-only hooks.
- Unity EditMode tests reference only public package assemblies. They submit
  JSON or public host operations and inspect public `UIDocument`,
  `VisualElement`, control, hierarchy, event, transport, log, and resource
  behavior. Tests receive no friend access and do not reflect into executors.
- C# compilation itself is an API gate: `Battlement.UI` may not use reflection,
  `InternalsVisibleTo`, or internal UI Toolkit members. Each control-family
  task rechecks its implementation route against the audited Unity 6000.5.8f1
  source instead of deferring API correctness to a final review.
- Black-box tests assert client-visible state and protocol boundaries, not
  private registries, converters, factories, or command executors.
- Repetitive catalogs use one authoritative mapping plus structural
  completeness tests. Behavioral tests cover each distinct conversion and
  state machine rather than duplicating the catalog as assertions.
- Protocol validation changes add paired acceptance and rejection coverage:
  Rust coverage uses `battlement-fake`, and C# coverage uses public Unity
  EditMode tests. A DTO field must work in the task that adds it, or that task
  must explicitly reject it until its owning task; decoders and executors may
  never silently ignore a declared field.

`./scripts/ci.py` is the repository validation entry point. It discovers every
standalone Cargo workspace below `samples` by its own `Cargo.toml` `[workspace]`
marker, excluding generated/build directories, then formats, lints, and tests
each project independently of the root workspace while rejecting inline Rust
tests in sample source. Every discovered sample Rust test must use
`battlement-fake` as a black-box integration boundary. The same command
validates declared Unity samples (`samples/*/sample.toml`), their Input System
backend, the committed runtime `PanelSettings`/theme assets, required assembly
edges, Unity EditMode tests, and the remaining repository checks.

Every task supplies one or more scenario-named 1280x720 PNGs through
`./scripts/capture-sample-visual-evidence.py` using the task's real
`--sample-project`, `--cargo-manifest`, `--scenario`, and `--scene` values.
Run the same command with `--smoke` first, then without `--smoke` and with
`--capture png` or `--capture both`. Screen-space evidence uses
`PanelScaleMode::ConstantPixelSize` so pixel scaling is deterministic. Passive
scenarios must publish the runner's Ready state with a harmless interaction
before `SignalPassed`; on any timeout, use the retained run/player-log paths
and the player exception block printed by the runner. Screenshots and logs stay
under the ignored evidence root and are not committed. After review fixes,
restage and recapture evidence from the final staged tree rather than retaining
pre-review media. Task 28 also captures the packaged native player.

Every task also builds the staged sample for WebGL, deploys that exact build to
a reviewable Web endpoint, and verifies the direct scenario URL in a fresh
browser session. The review handoff must include that Web demo link and a short
reproducible walkthrough; a local-only URL is not sufficient. Keep the deployed
demo available through review and replace it after review fixes so the link
always represents the final staged tree.

The completion workflow for every task is: stage intended changes; run
`./scripts/ci.py`; perform and fix the repository-mandated single independent
review when required; restage; recapture affected evidence; run final
`./scripts/ci.py`; create one Conventional Commit; and immediately submit the
exact commit with `tg candidate HEAD` through the repository Tollgate workflow.

## Dependency overview

| Wave | Tasks | Result |
|---|---|---|
| 1 | 01–03 | First end-to-end screen document, commands, fake, and hierarchy |
| 2 | 04–09 | Complete assets and 86-property outer-style surface |
| 3 | 10–19 | Complete 23-element catalog and controlled control families |
| 4 | 20–25 | Typed parts, full events, actions, and input/lifecycle behavior |
| 5 | 26–28 | Target-texture/world documents, complete coverage, and native proof |

## Wave 1: end-to-end UI foundation

### Task 01 — Render the first screen-space UI lab slice [DONE]

**Prerequisites:** none.

Perform the behavior-preserving `battlement-types` extraction and C# protocol
assembly split. Add the corrected panel DTOs, `UiDocument`, recursive
`UiElement` tagged union, and concrete `VisualElement`, `Box`, and `Label`
builders through Rust validation, JSON, C# mirrors, snapshot application,
global identity reservation, and teardown. Use the technical design's private
declarative macro for common visual-element builder methods from the first
element types onward.

Create the `samples/ui` project, native Rust engine, original asset directory,
manifest, bootstrap scene, and first screen-space document. Render the static
navigation/canvas/inspector command-deck shell using only public Battlement Rust
APIs.

**Black-box acceptance:** existing core serialization remains byte-compatible;
paired `battlement-fake` and public C# tests accept the supported panel/document
shapes and reject unsupported fields; `battlement-fake` passes the real
`samples/ui` engine's snapshot contract with an explicit camera and the sample
uses the Input System backend; a public Unity test renders a Battlement-owned
root from the committed runtime panel/theme assets while leaving a
project-authored document and panel settings untouched; connect and teardown
leak no identity or runtime panel settings. Global identity reservation is
exercised through the real runtime
registry with cross-domain ID reuse—snapshot validation alone is not sufficient.

**Screenshots:** run `./scripts/capture-sample-visual-evidence.py
--sample-project samples/ui --cargo-manifest samples/ui/rules/Cargo.toml --task
ui-foundation --scenario ui-sample --scene Assets/Scenes/UiLab.unity --dimensions
1280x720 --capture png` after its matching `--smoke` run. Retain the complete
command-deck shell and the inspector identifying the document root and first
Rust-authored label.

### Task 02 — Add UI commands, click dispatch, and the fake foundation

**Prerequisites:** Task 01.

Add `Button`, the four UI command cases, aggregate common patches sufficient for
the shell, create/update/destroy/placement execution, minimal Click forwarding,
and the late UI-dispatch gate. Add the initial `battlement-ui-fake` `UiWorld`
and compose its command dispatch into `battlement-fake`.

Any DTO field introduced here must be executable in this task's Rust fake and
Unity runtime, or receive paired rejection coverage through `battlement-fake`
and public C# tests until its owning task. Do not deserialize and ignore future
command or patch fields.

Make lab navigation operate through a synchronous Rust Click action. Add a
specimen that creates, updates, reparents, and destroys a status card so the
first command surface is visible rather than test-only.

**Black-box acceptance:** one native click produces one action; its response is
decoded during dispatch but all mutations occur after propagation; target
destruction causes no UI Toolkit exception; fake and Unity reach the same
logical result and journal the same command family.

**Screenshots:** overview before navigation; selected page after a Rust-handled
click with event and command inspector entries.

### Task 03 — Complete common state, hierarchy, and identity behavior

**Prerequisites:** Task 02.

Complete shared element fields, logical hierarchy validation, cross-domain
identity checks, detached construction and attachment, recursive destruction,
and placement-driven reorder/reparent. Extend the existing private declarative
builder-method macro as common fields are added. Always use logical
`Add`/`Insert` and public child APIs so control content containers remain
authoritative.

Add a hierarchy explorer page for name, enabled state, picking, language
direction, focusability, tab order, delegated focus, classes, usage hints, and
logical child ordering.

**Black-box acceptance:** public logical children have declared order; duplicate,
wrong-kind, cross-document, cycle, depth, and index failures mutate nothing;
detached failure attaches no child; recursive removal clears identities and fake
state.

**Screenshots:** nested hierarchy explorer; reordered and disabled hierarchy
with the inspector showing final common state.

## Wave 2: inline styles and asset surface

### Task 04 — Add UI assets, Image, and usage leases

**Prerequisites:** Task 03.

Add the new UI address and prepared-asset cases, `Image`, its exclusive source
union, source rectangle, tint, scale mode, UV behavior, and document/element
usage leases. Register the sample's texture, sprite, vector image, render
texture, cursor, legacy font, and UI font through the normal Addressables and
generated-address workflow.

Stage replacement leases before native setters, retain old leases through
successful application, and release displaced leases only after commit.

**Black-box acceptance:** each source resolves to its exact Unity type; setting
one source clears the other native source properties; sprite/source-rectangle
and numeric validation fail before mutation; replacement, destruction,
snapshot replacement, and teardown have correct lease counts in fake and Unity.

**Screenshots:** addressed asset-source gallery; one Image switched between two
source kinds with the active address in the inspector.

### Task 05 — Implement flex, dimensions, spacing, and positioning

**Prerequisites:** Task 04.

Implement length/auto/percentage values and the layout style families:
alignment, flex direction/grow/shrink/wrap/basis, width/height/min/max,
position/offsets, aspect ratio, rows/columns gaps, margins, padding, and
four-sided shorthands. Extend the authoritative Rust and C# style catalogs
together.

Add an adjustable layout playground that exposes every enum and value family,
including row/column reversal, wrapping, percentages, absolute positioning,
and shorthand-expanded spacing.

**Black-box acceptance:** representative conversions cover each value family;
all numeric bounds and clear/unset distinctions are enforced; a structural
catalog check proves every field in this task has one Rust and C# mapping.

**Screenshots:** wrapped row layout; resized column and absolute-position layout.

### Task 06 — Implement color, borders, radii, clipping, and visibility

**Prerequisites:** Task 05.

Implement color, the four border widths/colors/radii and shorthands, opacity,
display, visibility, overflow, overflow clip box, slice values/type/scale, and
background tint. Preserve explicit defaults in updates while omitting create
defaults.

Extend the styling page with layered cards, border/radius comparisons,
nine-slice presentation, opacity, hidden versus display-none, and overflow
clipping specimens.

**Black-box acceptance:** style clears assign `StyleKeyword.Null`; invalid
colors, negative widths/radii/slices, and invalid scale fail before native
mutation; tests inspect public inline style state rather than converter helpers.

**Screenshots:** border and radius matrix; clipping, opacity, hidden, and
display-none comparison.

### Task 07 — Implement backgrounds, gradients, repeats, and cursor

**Prerequisites:** Task 06.

Implement asset-backed and gradient backgrounds, linear/radial gradient fields,
positions, x/y repeat, size, background tint interaction, cursor texture and
hotspot, and associated leases. Use no arbitrary style-property or source
escape hatch.

Add a background laboratory covering every source kind, repeat mode, size
mode, radial extent/shape, mixed stop units, and the custom cursor.

**Black-box acceptance:** preserve gradient stop order; reject invalid stop
counts, fractions, centers, axes, and hotspots; test source compatibility and
old/staged lease ordering; prove cursor restoration and teardown release.

**Screenshots:** gradient and asset-source grid; repeat/position/size comparison
with cursor state visible in the inspector.

### Task 08 — Implement transforms and transitions

**Prerequisites:** Task 07.

Implement rotate, scale, translate, transform origin, transition property,
duration, delay, timing-function lists, and the typed conversion catalogs.
Retain UI Toolkit list repetition semantics and patch clearing.

Add transform-origin and transition specimens with deterministic controls for
the initial and settled states.

**Black-box acceptance:** reject nonfinite values, zero rotation axes, negative
durations, and unsupported properties; test all timing-function cases and list
repetition; public native transition events report supported property names.

**Screenshots:** transform-origin comparison; settled transition endpoint with
the transition payload in the inspector.

### Task 09 — Complete typography and text styling

**Prerequisites:** Task 08.

Complete `TextElement` and `Label` properties plus font size/source,
style/weight, alignment, auto-size, outline, shadow, paragraph/letter/word
spacing, whitespace, overflow, and overflow position. Apply text through a
public `INotifyValueChanged<string>` cast and selection preferences through
`ITextSelection`.

Add a typography page covering both addressed font kinds, every text style,
rich text, emoji fallback, escape parsing, elision, and selectable text.

**Black-box acceptance:** Rust writes emit no value event; leases distinguish
legacy and UI fonts; UTF-16 selection bounds and text numeric limits are
validated; catalog checks close the remaining outer-style fields.

**Screenshots:** typography and font matrix; selectable rich-text specimen with
selection indices in the inspector.

## Wave 3: element catalog and controlled controls

### Task 10 — Complete Button and RepeatButton

**Prerequisites:** Task 09.

Complete Button text/icon properties, pointer/navigation Click precedence,
RepeatButton typestate, fixed forwarding, and timing updates. Construct and
update RepeatButton only through public constructor/`SetAction(Action,long,long)`
routes; timing replacement must reinstall the same fixed callback exactly once.

Add ordinary, icon, disabled, navigation, and repeating command controls to the
lab.

**Black-box acceptance:** pointer and navigation never double-submit; one press
and hold has exact repeat counts; release contributes no root Click; timing
replacement preserves one callback; fake and Unity agree.

**Screenshots:** button state and icon gallery; repeat counter after a held
activation.

### Task 11 — Add GroupBox and PopupWindow

**Prerequisites:** Task 10.

Implement both containers, their content-container routing, title/text
properties, rich links where applicable, and conditional internal title and
content parts.

Add grouped settings and popup-card specimens that exercise populated, empty,
and dynamically titled states.

**Black-box acceptance:** logical order uses public content APIs; GroupBox
rejects Rust-owned RadioButton descendants; conditional title creation and
removal retain correct state and leases.

**Screenshots:** titled and untitled groups; populated PopupWindow specimen.

### Task 12 — Add ScrollView and Scroller

**Prerequisites:** Task 11.

Implement ScrollView modes, nested interaction, scroller visibility, offset,
page sizes, wheel size, touch behavior, deceleration, elasticity, interval,
and ScrollChanged/ScrollSettled. Implement controlled Scroller direction,
limits, page size, ValueChanging, and ValueCommitted.

Observe the public horizontal and vertical scroller callbacks under a
command-origin suppression guard. Implement the exact 100 ms manual-clock
settlement rule in Unity and fake.

Add nested scrolling, horizontal gallery, controlled scroller, and settlement
diagnostics to the lab.

**Black-box acceptance:** emit one combined offset action per logical change;
capture, continued motion, command writes, disable, detach, and teardown behave
at the exact settlement boundary; fake manual-clock scenarios match Unity.

**Screenshots:** nested two-axis scrolling; terminal settled offset and
controlled Scroller value in the inspector.

### Task 13 — Add Tab and TabView

**Prerequisites:** Task 12.

Implement constrained Tab children, text/icon/closeable state, TabView
selection, reorder, header scrolling, close veto, and scoped command-origin
suppression around selected-tab, insertion, removal, and reorder calls.

Add a reorderable and closeable workspace page with accepted and rejected
close requests.

**Black-box acceptance:** command-origin operations do not echo; close always
vetoes native removal and succeeds only when Rust returns Tab destruction;
selection and proposed indices remain valid after insert/remove/reorder; fake
and native order agree.

**Screenshots:** multi-tab workspace; reordered and closed result with the
event inspector.

### Task 14 — Add TextField drafts, commits, and selection

**Prerequisites:** Task 13.

Implement TextField label/value, multiline, password/read-only behavior,
placeholder and hide-placeholder through `ITextEdition`, selection fields,
Input, ValueCommitted, and SelectionChanged. Coalesce cursor and selection
callbacks into one logical selection mutation.

Add accepted, normalized, rejected, multiline, password, and read-only input
specimens.

**Black-box acceptance:** typing sends no traffic without Input subscription;
Enter and focus loss make one proposal; Escape restores silently; rejection and
accepted Rust writes occur before repaint; one selection change produces one
action; fake behavior matches.

**Screenshots:** active local draft beside its committed inspector value;
accepted and rejected terminal fields.

### Task 15 — Add Toggle and RadioButton

**Prerequisites:** Task 14.

Implement controlled Boolean values, labels, text, and complete common native
part capture for Toggle and standalone RadioButton.

Add settings toggles and standalone radio specimens with accepted and rejected
proposals.

**Black-box acceptance:** each interaction submits one proposed Boolean,
restores without notification, and changes only through returned Rust state;
disabled controls and global input gating submit nothing.

**Screenshots:** mixed toggle/radio states; rejected proposal with event and
committed-value history.

### Task 16 — Add RadioButtonGroup and ToggleButtonGroup

**Prerequisites:** Task 15.

Implement radio choices and selected index plus mask-based single/multiple
ToggleButtonGroup selection. Construct public
`ToggleButtonGroupState(mask, childCount)`, write with
`SetValueWithoutNotify`, and use Unity's public `isMultipleSelection` property.

Add formation-choice and multi-filter specimens.

**Black-box acceptance:** validate choice bounds, sorted unique indices,
single-selection masks, the 64-button limit, and constrained Button children;
Rust writes never echo and fake/native results agree.

**Screenshots:** exclusive radio group; multi-selection ToggleButtonGroup with
selected indices in the inspector.

### Task 17 — Add DropdownField

**Prerequisites:** Task 16.

Implement choices, selected index/value coherence, labels, empty selection,
controlled commit behavior, and public native parts.

Add theme and loadout selectors with accepted, rejected, and cleared states.

**Black-box acceptance:** a choice contains two matching `Some` values or two
`None` values; invalid indices and mismatches fail before mutation; rollback
and acceptance are silent; fake and Unity agree.

**Screenshots:** open dropdown; committed and cleared selector states.

### Task 18 — Add Slider and SliderInt

**Prerequisites:** Task 17.

Implement selected values, limits, direction, page size, inversion, optional
text input/fill, ValueChanging during capture, and one ValueCommitted on
release. Capture conditional public parts when materialized.

Add continuous and stepped tuning controls in horizontal, vertical, and
inverted configurations.

**Black-box acceptance:** local capture values remain transient; release sends
one final proposal; live traffic requires subscription; clamping and integer
semantics are correct; command writes and rollback do not echo.

**Screenshots:** filled horizontal Slider; vertical inverted SliderInt with its
final value.

### Task 19 — Add MinMaxSlider and ProgressBar

**Prerequisites:** Task 18.

Implement bounded and unbounded MinMaxSlider limits, ordered dual-thumb values,
ValueChanging/ValueCommitted, and ProgressBar low/high/value/title state.

Add resource-range and staged-progress specimens.

**Black-box acceptance:** unbounded limits map to native extrema without
putting them on the wire; finite limits and selected values are ordered and
clamped; release emits one final range; ProgressBar remains output-only.

**Screenshots:** active min/max resource range; progress variants at distinct
completion states.

## Wave 4: parts, events, actions, and lifecycle

### Task 20 — Implement simple private-part styling

**Prerequisites:** Task 19.

Add typed part keys, owner-scoped audited lookup, unique-match failure,
part-style patch/clear semantics, and asset leases. Cover the simple
Button, GroupBox, PopupWindow, Toggle, RadioButton, DropdownField, and
ProgressBar part catalogs.

Prefer direct public references. Otherwise query only below the owning control
with public `Q<T>` and audited public USS class-name constants. Never perform a
global query.

Add a part-anatomy overlay and custom simple-control skins to the lab.

**Black-box acceptance:** every valid part state resolves exactly one native
element; zero or multiple matches fail; clear and asset replacement preserve
unrelated part style; destruction releases part leases.

**Screenshots:** labeled simple-control anatomy; customized Button, Toggle,
DropdownField, and ProgressBar parts.

### Task 21 — Implement complex and conditional part styling

**Prerequisites:** Task 20.

Complete parts for ScrollView, Scroller, Tab, TabView, TextField,
RadioButtonGroup indexed options, ToggleButtonGroup, Slider, SliderInt,
MinMaxSlider, and remaining conditional slots. Apply property changes before
part styles and validate the aggregate final state.

Add complex-control anatomy, indexed-option overrides, and controls that toggle
icons, titles, fill, text input, and multiline scroll parts.

**Black-box acceptance:** `AllOptions` applies before indexed overrides;
conditional create/remove requires matching style set/clear; missing or
ambiguous audited parts fail rather than selecting another descendant; no stale
lease or style remains.

**Screenshots:** labeled slider, scroll, and tab anatomy; conditional parts
before and after activation.

### Task 22 — Complete pointer, wheel, capture, and routed phases

**Prerequisites:** Task 21.

Implement all pointer payloads, boundary/crossing events, Wheel, related-target
mapping, Trickle/Target/Bubble subscriptions, deterministic Rust routing, and
pointer capture events. Root observation maps Unity-created targets to the
nearest Rust-owned logical ancestor.

Add a nested event-routing visualizer and pointer-capture specimen.

**Black-box acceptance:** one native event creates one Rust action regardless
of subscribed ancestors; route order is deterministic; target-only events do
not propagate; omitted defaults encode exactly; unsubscribed high-frequency
events allocate no message; native/fake routes agree.

**Screenshots:** highlighted routed ancestor path; captured pointer and complete
payload in the inspector.

### Task 23 — Complete keyboard, navigation, focus, and activation

**Prerequisites:** Task 22.

Implement physical-key mapping, text, modifiers, native repeat, navigation
move/submit/cancel, focus relations and direction, and Button navigation Click
precedence. Preserve the separation between UI focus routing and global core
keyboard selection.

Add a keyboard/gamepad navigation page with visible focus rings and activation
diagnostics.

**Black-box acceptance:** mapped and unmapped keys have exact payloads; focus
routes through public UI Toolkit focus APIs; route-wide Click precedence avoids
double activation; phase order and fake routing agree.

**Screenshots:** keyboard-focused navigation grid; navigation activation and
focus relation in the inspector.

### Task 24 — Complete lifecycle, geometry, link, selection, and transition events

**Prerequisites:** Task 23.

Implement GeometryChanged, AttachToPanel, DetachFromPanel, transition events,
text selection, and rich-link enter/leave/down/up. Use the experimental public
link-tag event classes. Maintain link identity per `(ObjectId, pointer_id)`
because native link-out lacks full identity.

Add an exhaustive event timeline page for every remaining event kind.

**Black-box acceptance:** selection callbacks coalesce; link leave uses its
matching cached identity; unmatched leave is dropped; multiple pointers remain
independent; detach, destruction, disable, replacement, and teardown clear the
cache; transition lists are nonempty and supported.

**Screenshots:** rich-link interaction timeline; geometry, transition, and
lifecycle timeline.

### Task 25 — Complete actions, controlled-state hardening, and input gating

**Prerequisites:** Task 24.

Implement Focus, Blur, CapturePointer, ReleasePointer, ScrollTo, and SelectText
through public UI Toolkit APIs. Complete shared accepted/rejected controlled
semantics, response deferral, `input_disabled` cleanup, draft/drag restoration,
and first-eligible-frame feedback.

Add an action console plus visible rejection, rollback, and disabled-input
scenarios.

**Black-box acceptance:** every action validates its target and preconditions;
ScrollTo requires a logical descendant; selection uses UTF-16 indices; input
disable restores drafts and drags, releases capture/focus, and emits nothing;
deferred target destruction completes safely; fake/native behavior agrees.

**Screenshots:** action console after ScrollTo and SelectText; disabled-input
cleanup beside an accepted/rejected controlled-value comparison.

## Wave 5: document modes and release hardening

### Task 26 — Complete panel settings and target-texture rendering

**Prerequisites:** Task 25.

Implement the complete corrected panel scale, target-display, clearing,
dynamic-atlas, and target-texture contract. Apply settings only through public
`PanelSettings` setters and retain the target texture with a document lease.
Keep screen-space evidence on `ConstantPixelSize`; add other scale modes as
explicit specimens rather than changing the capture baseline.

Add a render-modes page that keeps the screen-space lab shell visible while an
in-world monitor displays a target-texture document.

**Black-box acceptance:** exercise every scale mode and its applicable fields;
reject invalid cross-mode configuration; validate dynamic-atlas powers and
ordering; target texture has the exact type and lifetime; target-texture panels
do not claim automatic pointer mapping; authored documents remain untouched.

**Screenshots:** screen-space scale-mode comparison; rendered target-texture
monitor with its document settings in the inspector.

### Task 27 — Add world-space documents and process-wide input

**Prerequisites:** Task 26.

Implement world-space document position, size, pivot, transform, public
`PanelInputConfiguration` setup, camera selection, interaction layers,
unbounded/finite distance, redirection, Unity-default collider behavior,
duplicate world-action suppression, and cleanup.

Reject an active project-authored input configuration before mutation; never
adopt, disable, or restore it. Add an interactive world-space console beside
the target-texture monitor in the integrated scene.

**Black-box acceptance:** cover EventSystem and camera requirements, main versus
explicit camera selection, exact infinity mapping, authored configuration
conflict, pointer interaction, generated-collider exclusion, and cleanup after
the final world document.

**Screenshots:** integrated screen/target/world three-mode scene; hovered and
activated world-space control with exactly one UI action recorded.

### Task 28 — Prove complete coverage, replacement, and release behavior

**Prerequisites:** Task 27.

Complete authoritative snapshot replacement, session-fatal Unity-exception
cleanup, protocol limits, serialization matrices, package assembly checks,
coverage metadata, and representative performance instrumentation. Add a lab
coverage dashboard showing every element, outer style, part, event family,
action, asset source, and document mode mapped to a live specimen and automated
test family.

Run the complete sample engine through the `battlement-fake` black-box, Unity
EditMode tests, and protocol fixtures with staged `./scripts/ci.py`; its
sample-workspace checks and prohibition on inline Rust sample tests are required
in addition to root `cargo test --workspace`. Run
`./scripts/capture-sample-visual-evidence.py --sample-project samples/ui
--cargo-manifest samples/ui/rules/Cargo.toml --task ui-release --scenario ui-sample
--scene Assets/Scenes/UiLab.unity --dimensions 1280x720 --smoke`, then the same
command with final media options to build and run one non-Development macOS
Release player with native transport. Also produce the final WebGL build,
deploy it, verify the complete lab in a fresh browser session, and include its
direct Web demo link in the task review handoff.

**Black-box acceptance:** replacement while focus, capture, draft, scroll, and
leases are active leaves only authoritative new state; injected post-mutation
Unity failure closes the session and attempts all cleanup; count/depth/string
and payload limits match Rust/C#; the coverage ledger has no missing or duplicate
entry; representative unsubscribed and controlled events meet the established
traffic/timing checks.

**Screenshots:** complete coverage dashboard with all categories green; polished
native-player overview; integrated render-modes page from the packaged player.

## Completion criteria

- All 28 tasks are individually committed, reviewed, and promoted in order.
- The coverage ledger has one implementation, test family, and sample specimen
  for every public UI capability in the technical design.
- `samples/ui` runs through the ordinary native sample workflow, contains no
  game-specific C#, and presents the complete command-deck lab in one scene.
- Staged `./scripts/ci.py` passes its root workspace, discovered standalone
  sample workspaces, `battlement-fake` black-box suite, Unity EditMode tests,
  protocol fixtures, and sample preflight contracts; the final native
  Release-player smoke also passes.
- Required screenshots and logs exist under the ignored evidence root for every
  task; no media or capture-only sample C# enters Git.
- Every task has a verified, directly accessible WebGL demo URL in its review
  handoff.
- No implementation uses reflection, internal UI Toolkit APIs, custom-command
  fallbacks, optional UI packaging, or protocol versioning.
