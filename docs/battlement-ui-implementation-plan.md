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

- `UiNode` owns stable identity and logical children. `UiElement` owns only
  concrete visual state, and controls compose the shared public
  `VisualElement` fields.
- Creation and property updates reuse the same element structs. An omitted
  optional field preserves Unity behavior during creation and leaves the live
  value unchanged during an update. Updates serialize only supplied fields;
  resetting an assigned property to Unity's default is not supported.
- Parent changes and sibling-index changes are independent update operations.
  There is no aggregate placement value.
- Element-kind derivation uses `enum-kinds`. Shared enum behavior uses
  `enum_dispatch` after composition has removed unnecessary forwarding APIs.
  Do not maintain handwritten kind enums or repetitive forwarding matches.
- Element data is public. Do not add private copies of protocol fields or
  zero-argument getters that merely return public fields. Builder conveniences
  are reserved for meaningful construction, validation, or collection work.
- Every type that composes `VisualElement` names that public field `element`.
  `VisualElementProperties` exposes it uniformly through `visual_element()` and
  `visual_element_mut()`.
- Each element-specific file owns the complete element type: its `pub struct`,
  constructors, builders, `VisualElementProperties` implementation, and update
  logic. Do not leave an element's struct or ordinary implementation in
  `elements/mod.rs` and move only its update method. `Style`, its builders, and
  its supporting style enums likewise live in `elements/style.rs`. `Style` is
  an explicit exception to the normal source-file size limit and may exceed
  1,000 lines so its fields and directly written builder methods remain
  together. Do not macro-generate the `Style` builders.
  `UiElement` performs kind dispatch only; it does not accumulate property
  application logic as controls are added. Element modules that invoke the
  shared visual-element builder macro import every type referenced by the
  generated method signatures, including `Style`, because those names resolve
  at the macro invocation site. Use a descriptive module name such as
  `box_element.rs` when the element name is a Rust keyword; do not use raw
  identifier module syntax.
- **Every documentation comment for a Unity-backed API MUST satisfy the
  documentation quality gate below.** Unity Manual and Scripting API review for
  the targeted editor version is mandatory authoring and review work, not an
  optional source of inspiration.
- The protocol uses update terminology exclusively. Do not introduce parallel
  per-control update structs, change-delta structs, reset lists, or reset
  operations.
- Applying an update to an incompatible element is a developer invariant
  violation and panics. Do not add a marker error type for it.
- UI event coordinates use `PanelPoint`. `ClickEvent` remains the public event
  name and its Unity-derived cases are `Pointer`, `NavigationSubmit`, and
  `Repeat`.
- UI command helpers generate command IDs. Specialized callers that require an
  explicit ID use `Command::new`; do not add UI-specific `_with_id` variants.
- Immediate UI operations are not described as blocking. The scheduler's
  general blocking behavior remains unchanged.
- The C# null-guard helper is named `Preconditions`; `Battlement.Errors` remains
  the diagnostic namespace.

- `samples/ui` is a new standalone Unity project and Rust rules crate. It is an
  interactive UI lab rather than an extension of another sample.
- The lab uses a persistent navigation column and an active specimen canvas.
  Its visual language is a dark Battlement command deck with restrained cyan
  and amber accents. Internal names, IDs, property dumps, event payloads, and
  command logs belong in automated evidence, not permanent sample-screen text.
- Every public capability has a visible lab specimen: all 23 elements, all 88
  current styles, every typed part slot, document mode, action, event family, asset
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
  vector image, render texture, cursor, and UI font sources. It
  does not import a third-party UI pack.
- User-relevant rejection, rollback, input gating, and lifecycle behavior are
  visible. Malformed protocol, missing asset, partial mutation, and injected
  Unity failure cases remain automated-test fixtures rather than sample controls.
- Every task produces a browser-playable WebGL build, verifies it through
  browser QA, and includes a direct deployed Web demo link in its review
  handoff. One final packaged macOS Release player additionally proves the
  standalone native result.
- During WebGL browser QA, allow the canvas to finish a frame before capturing a
  screenshot and run screenshot capture separately from console or interaction
  queries. If the Playwright screenshot call reaches its five-second timeout
  while WebGL is repainting, wait for another settled frame and retry the
  screenshot; do not launch a second browser or replace the configured
  Playwright service.

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
encode ordering. Do not render those internal identities on sample screens.

### Mandatory sample-screen design rules

These rules are acceptance requirements for every `samples/ui` task. They take
precedence over any later specimen or screenshot wording that could be read as
requesting extra explanatory copy, debug output, or smaller text.

- **Use the established visual language.** Every visible sample element MUST
  use an authored `samples/ui` style. `design_system.rs` owns only shared
  tokens, utilities, and styles reused across screens; screen-specific styles
  MUST live in page-specific style modules, including variants used by page
  interactions. Do not improvise one-off inline colors, type sizes, spacing,
  or control treatments in a specimen.
- **Use dark surfaces with deliberate contrast.** White and light backgrounds
  are forbidden throughout `samples/ui`. Every visible container and control
  MUST explicitly select a shared dark-surface role instead of inheriting a
  light Unity default. Ordinary text and actionable controls MUST retain at
  least a 4.5:1 contrast ratio in every reachable state; demonstrations of
  opacity or visibility must keep their identifying labels outside the faded
  or hidden subtree. Screenshot review MUST reject washed-out, white-on-white,
  or otherwise illegible specimens even when the underlying property works.
- **Never render body text below 24 px.** The 24 px “Hello from Rust” value is
  the absolute minimum size for labels, buttons, values, captions, statuses,
  and any other body copy. The 28 px “Label component” style remains the
  minimum specimen-heading size. Titles remain 44 px. This floor is mandatory,
  not a suggestion, and applies to every current and future page and every
  interaction state. Tests MUST reject sample text below its applicable floor.
- **Use the fewest words that can demonstrate the behavior.** Screens 01 and 02
  set the visual standard: one short title, one focused specimen, and only the
  control text needed to operate it. Do not add descriptions, instructions,
  category prose, internal names such as “root” or “logical element explorer,”
  numbered property labels, or sentences that narrate state already shown by
  layout, styling, or control state. Raw state strings such as
  `enabled=true`, `picking=position`, class lists, or child order dumps are
  forbidden in the rendered sample.
- **Set and test a visible-word budget before implementing each page.** Count
  every whitespace-delimited token rendered in the specimen canvas, including
  headings, controls, status text, values, and text introduced by an
  interaction. The persistent left navigation is excluded. Every reachable
  state MUST stay within the page's budget; punctuation, casing, and repeated
  words do not reduce the count. Prefer showing behavior visually over raising
  the budget.
- **Every interaction MUST be reversible on the same screen.** A control that
  changes the sample MUST become, or be paired with, an obvious control that
  restores the exact initial state. Restore hierarchy, order, parentage,
  enabled and picking state, classes, focus behavior, displayed copy, and any
  other mutation. Navigating away, reconnecting, or restarting the sample does
  not count as reversal. Black-box acceptance MUST exercise the complete
  initial → changed → initial round trip.

Public UI components and their properties require durable, user-oriented API
documentation grounded in the corresponding Unity Manual and Scripting API
pages. The author and reviewer MUST open and review those Unity pages for every
documented Unity-backed type, property, and method; memory, inference, and
copying a neighboring Battlement comment are not acceptable substitutes.
Component documentation must explain purpose, behavior, layout role, important
distinctions from related components, and appropriate usage. Property fields
and builder methods must explain their practical effect, units, inheritance or
container relationships, and important interactions with related properties;
never merely restate the identifier. Comments must describe only lasting
behavior in the current implementation. Source comments and public API
documentation must not mention tasks, phases, slices, milestones, planned work,
deferred implementation, future support, or when functionality will be
introduced. Review must remove such comments rather than rewording roadmap
status into durable source documentation.

### Documentation quality gate

Every Battlement UI task that adds or changes a public type, field, variant,
trait, function, or builder method MUST update its Rust documentation in the
same change. Documentation is part of the feature's definition of done and is
reviewed with the same rigor as serialization, validation, fake-client, and
native behavior.

Before writing documentation, the author MUST inventory every public API in the
task and open the corresponding Unity 6000.5 Manual and Scripting API pages.
Search-result excerpts, memory, generated C# summaries, and neighboring
Battlement comments do not count as review. At minimum, use the following pages
as the quality exemplars for element documentation:

- Unity [VisualElement](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-VisualElement.html)
  and [Box](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-Box.html)
  documentation for container purpose, layout role, inherited behavior,
  styling, and creation examples.
- Unity [Label](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-Label.html)
  documentation for text purpose, styling, inherited text behavior, and
  appropriate use.
- Unity [Button](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-Button.html)
  documentation for activation behavior, internal content, interaction states,
  styling, and creation examples.
- The exact Unity Scripting API pages for each mapped class, property, event,
  and method, plus the [USS properties
  reference](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-USS-Properties-Reference.html)
  for every supported style property.

If the project editor version changes, these links are starting points only;
the author MUST switch to pages for the new exact documentation stream and
reconcile semantic differences before changing Battlement documentation.

Public type and variant documentation MUST explain all of the following that
apply:

- what the API represents and what native Unity type or behavior it maps to;
- when a caller should use it and how it differs from related Battlement UI
  APIs;
- its layout, hierarchy, rendering, input, event, or lifecycle behavior;
- supported composition, inherited behavior, important defaults, and explicit
  Battlement limitations relative to Unity; and
- a compiling Rust example for primary user-facing types when construction or
  composition is not obvious from the signature.

Public field and builder-method documentation MUST explain the property's
observable effect, units and coordinate system, inheritance, valid range or
ordering, mode dependencies, container or sibling relationships, and
interactions with related properties whenever applicable. A comment that only
expands the identifier—for example, “sets the width” or “the button text”—does
not pass review. `Option`, collection, and builder signatures already express
presence and cardinality; documentation must spend its words on behavior.

Public fallible functions MUST include an `Errors` section describing rejection
conditions. Public functions that can panic on caller input MUST include a
`Panics` section describing the violated invariant. Examples MUST use only
public APIs and run as doctests unless they genuinely require Unity runtime
state, in which case the comment must state the runtime precondition rather
than publishing a non-running pseudo-example.

The reviewer MUST independently open the same Unity pages and compare every
changed comment against them. Review MUST reject inaccurate terminology,
missing units or inheritance, undocumented native/Battlement differences,
tautological comments, roadmap language, and examples that do not compile.
Before completion, run `cargo test -p battlement-ui --doc` and
`RUSTDOCFLAGS="-D warnings" cargo doc -p battlement-ui --no-deps` in addition to
the repository-wide staged `./scripts/ci.py` gate.

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

Run the aggregate command directly for normal validation. If isolating one of
its Python functions for diagnosis, include `scripts` on `PYTHONPATH`, for
example `PYTHONPATH=scripts python3 -c "import runpy; ns =
runpy.run_path('scripts/ci.py'); ns['run_unity_edit_mode_tests']()"`; importing
the file without that path fails to resolve its sibling validation modules.

If the CI C# style-diagnostics step reports only that its restore operation
failed, run `dotnet restore battlement-ci.slnx` from the repository root and
then rerun `./scripts/ci.py`. Do not treat that workspace restore failure as a
source diagnostic or skip the full rerun.

Standalone sample workspaces cannot be added as development dependencies of a
root-workspace crate: Cargo rejects that graph as multiple workspace roots. Put
sample black-box integration tests in the sample workspace's `tests/`
directory, add `battlement-fake` as that sample's development dependency, and
drive only the sample's public engine plus public fake APIs. Keep reusable fake
execution and assertions in `battlement-ui-fake` and `battlement-fake`; do not
duplicate them in the sample.

If Unity exits after reporting script compiler errors, it may leave the
generated `Temp/UnityLockfile` behind even though no Unity process owns the
project. Before retrying, confirm no process has the lock open (for example
with `lsof Temp/UnityLockfile`); only then remove that exact stale file. Rerun
Unity with an explicit retained `-logFile` path to recover the compiler
diagnostics, because the CI wrapper's temporary log is cleaned up when its
lock-wait step fails.

When sample capture reports a Ready-signal timeout, read the retained
`*-player.log` before changing scenario timing. Snapshot validation failures
occur before the scenario can publish Ready and are reported there with the
rejected protocol type. A direct `BattlementUiDocuments` EditMode test bypasses
snapshot validation, so every newly supported element kind also needs coverage
through `BattlementSnapshotValidator` or a packaged-player smoke.

The sample-capture wrapper builds the copied sample's committed Addressables
catalog; it does not infer Rust asset addresses from the engine. Each sample
must therefore retain its `Assets/AddressableAssetsData` configuration (force
add it because the generated-directory pattern is ignored) and map every
prepared address such as `ui/content` to the correct typed asset before running
the packaged smoke.

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

When a capture scenario targets an element created by a UI command, wait until
its `worldBound` produces finite, in-range normalized coordinates before
requesting pointer input. The element can be queryable one frame before UI
Toolkit completes layout; targeting it immediately causes the capture runner to
reject non-finite pointer coordinates.

Every task also builds the staged sample for WebGL and exposes that exact build
through a Cloudflare Quick Tunnel. `scripts/deploy.py` is reserved for release
deployment: it requires a clean `master` checkout and must not be used from a
task worktree. From the staged task worktree, build with `cargo run --quiet -p
battlement-cli -- sample build ui --web --release`, use `scripts.deploy`'s
`assemble_site(["ui"], revision)` and `validate_site(["ui"])` functions to
prepare `Build/cloudflare`, serve that directory with local Wrangler so its
Unity `_headers` rules apply, and point `cloudflared tunnel --no-autoupdate
--url http://127.0.0.1:<port>` at the verified-free Wrangler port. Verify the
generated HTTPS `/ui/` URL in a fresh browser session. The review handoff must
include that direct Web demo link and a short reproducible walkthrough; a
localhost-only URL is not sufficient. Keep the named Wrangler and tunnel
services available through review and replace them after review fixes so the
link always represents the final staged tree.

If the WebGL build reports `FROZEN_CACHE is set, but cache file is missing`,
the matching Unity editor's WebGL Build Support module is incomplete. Do not
generate individual cache artifacts; reinstall that module through Unity Hub,
then rerun the staged sample build.

The completion workflow for every task is: stage intended changes; run
`./scripts/ci.py`; perform and fix the repository-mandated single independent
review when required; restage; recapture affected evidence; run final
`./scripts/ci.py`; create one Conventional Commit; and immediately submit the
exact commit with `tg candidate HEAD` through the repository Tollgate workflow.

### Unity 6000.5.8f1 API audit for tasks 08–28

The remaining tasks were audited against the public metadata shipped in
`UnityEngine.UIElementsModule.dll` by Unity 6000.5.8f1 revision
`5cb7df797b7d` and the matching Unity 6000.5 Manual and Scripting API. This
table is an implementation constraint. A name in the Battlement event or
protocol vocabulary is not evidence that Unity provides a class, event, or
setter with that name; the implementation route must use the native surfaces
listed here.

| Tasks | Audited public Unity surface and limits |
|---|---|
| 08 | `IStyle.rotate`, `scale`, `translate`, `transformOrigin`, `filter`, and the four `transition*` lists; `FilterFunctionType` supplies `Tint`, `Opacity`, `Invert`, `Grayscale`, `Sepia`, `Blur`, `Contrast`, and `HueRotate`; transition callbacks are `TransitionRunEvent`, `TransitionStartEvent`, `TransitionEndEvent`, and `TransitionCancelEvent`. |
| 09 | Text styles are the public `IStyle` members `unityFont`, `unityFontDefinition`, `unityFontStyleAndWeight`, `unityTextAlign`, `unityTextAutoSize`, `unityEditorTextRenderingMode`, `unityTextGenerator`, outline, shadow, and spacing/overflow fields. `TextElement` implements `INotifyValueChanged<string>`, `ITextSelection`, and `ITextEdition`; `unityFont` consumes `UnityEngine.Font`. |
| 10–11 | `Button.iconImage` and `text`; `RepeatButton(Action,long,long)` and `SetAction(Action,long,long)`; `GroupBox.text`; and inherited `TextElement` behavior plus `PopupWindow.contentContainer`. `GroupBox` is not a `TextElement`; rich links apply to `PopupWindow`, not `GroupBox`. |
| 12 | `ScrollView` exposes its mode, offset, page sizes, wheel/touch/deceleration/elasticity settings, `elasticAnimationIntervalMs`, both public scrollers, and `ScrollTo`. `Scroller` exposes limits, direction, value, `valueChanged`, its `slider`, and its repeat buttons. It has no direct page-size member and no `SetValueWithoutNotify`; Battlement writes through `Scroller.slider.SetValueWithoutNotify`. `ScrollChanged`, `ScrollSettled`, `ValueChanging`, and `ValueCommitted` are Battlement events derived from these callbacks, not Unity event classes. |
| 13–19 | `Tab` exposes label, icon, closeable state, `selected`, `closing`, and `closed`; `TabView` exposes active/selected tab state, reorder, and callbacks. Text and choice controls use their public value interfaces and `SetValueWithoutNotify`; `ToggleButtonGroupState(ulong,int)` and `ToggleButtonGroup.isMultipleSelection` are public. `BaseSlider<T>.pageSize` and `SliderInt.pageSize` are `float`; live/final event names remain Battlement adapter events. |
| 20–21 | Part access may use a direct public control reference or an owner-scoped `Q<T>` query keyed by that control's public USS class-name constants. There is no native typed-part API and no permission to use internal fields. |
| 22–25 | Pointer, wheel, keyboard, navigation, focus, lifecycle, geometry, capture, transition, and input classes named by the technical design are public. `PointerOverEvent` and `PointerOutEvent` do not expose `relatedTarget`; `KeyDownEvent` and `KeyUpEvent` do not expose a repeat flag. Rich-link mapping uses the experimental `PointerOverLinkTagEvent`, `PointerOutLinkTagEvent`, `PointerDownLinkTagEvent`, and `PointerUpLinkTagEvent`; the out event lacks link identity. Actions use `Focus`, `Blur`, `PointerCaptureHelper`, `ScrollView.ScrollTo`, and `ITextSelection.SelectRange`. |
| 26–27 | `PanelSettings` publicly exposes the specified render, scale, target, clear, display, and dynamic-atlas setters. `UIDocument` exposes position, world-space size mode and size, pivot reference and pivot, sorting order, panel settings, and its read-only root. `PanelInputConfiguration` publicly exposes world-space processing, layers, maximum distance, main/explicit cameras, redirection, and automatic panel-component creation. Collider policy setters are not public and must not be invented. |
| 28 | Release coverage may exercise only the audited surfaces above; it adds no additional Unity API route. |

## Dependency overview

| Wave | Tasks | Result |
|---|---|---|
| 1 | 01–03 | First end-to-end screen document, commands, fake, and hierarchy |
| 2 | 04–09 | Complete assets and the current outer-style surface |
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
navigation and command-deck canvas using only public Battlement Rust APIs.

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
command-deck shell and first Rust-authored label without internal identifiers or
diagnostic copy.

### Task 02 — Add UI commands, click dispatch, and the fake foundation [DONE]

**Prerequisites:** Task 01.

Add `Button`, the four UI command cases, sparse element updates sufficient for
the shell, create/update/destroy/parent/index execution, minimal Click
forwarding, and the late UI-dispatch gate. Add the initial
`battlement-ui-fake` `UiWorld` and compose its command dispatch into
`battlement-fake`.

Any DTO field introduced here must be executable in this task's Rust fake and
Unity runtime, or receive paired rejection coverage through `battlement-fake`
and public C# tests until its owning task. Do not deserialize and ignore future
command or update fields.

Make lab navigation operate through a synchronous Rust Click action. Add a
specimen that creates, updates, reparents, and destroys a status card so the
first command surface is visible rather than test-only.

**Black-box acceptance:** one native click produces one action; its response is
decoded during dispatch but all mutations occur after propagation; target
destruction causes no UI Toolkit exception; fake and Unity reach the same
logical result and journal the same command family.

**Screenshots:** overview before navigation; selected page after a Rust-handled
click, with the result communicated by the specimen itself.

### Task 03 — Complete common state, hierarchy, and identity behavior [DONE]

**Prerequisites:** Task 02.

Complete shared element fields, logical hierarchy validation, cross-domain
identity checks, detached construction and attachment, recursive destruction,
and independent reorder/reparent operations. Extend the shared public element
composition as common fields are added. Always use logical
`Add`/`Insert` and public child APIs so control content containers remain
authoritative.

Add a hierarchy page for name, enabled state, picking, language direction,
focusability, tab order, delegated focus, classes, usage hints, and logical
child ordering. The page MUST contain no more than eight visible words total in
every state, excluding the persistent left navigation. Use exactly one short
page title, one-word node labels, and the two-word `Reorder children` action;
do not render a state inspector, property names, internal identifiers,
numbering, or explanatory prose. The action MUST toggle to `Reset` after
applying the mutations and restore the exact initial hierarchy and common state
without leaving the page.

**Black-box acceptance:** public logical children have declared order; duplicate,
wrong-kind, cross-document, cycle, depth, and index failures mutate nothing;
detached failure attaches no child; recursive removal clears identities and fake
state. The sample test counts at most eight visible words, verifies that every
visible sample label and control is at least 24 px, applies the hierarchy change,
and toggles it back to the exact initial order, parentage, enabled state, picking
mode, classes, delegated-focus state, and action label.

**Screenshots:** clean initial hierarchy; reordered and disabled hierarchy with
the same minimal control changed to its reset action. Neither capture may add a
state dump or exceed eight visible words outside navigation.

## Wave 2: inline styles and asset surface

### Task 04 — Add UI assets, Image, and usage leases [DONE]

**Prerequisites:** Task 03.

Add the new UI address and prepared-asset cases, `Image`, its exclusive source
union, source rectangle, tint, scale mode, UV behavior, and document/element
usage leases. Register the sample's texture, sprite, vector image, render
texture, cursor, and UI font through the normal Addressables and
generated-address workflow.

Stage replacement leases before native setters, retain old leases through
successful application, and release displaced leases only after commit.

**Black-box acceptance:** each source resolves to its exact Unity type; setting
one source clears the other native source properties; sprite/source-rectangle
and numeric validation fail before mutation; replacement, destruction,
snapshot replacement, and teardown have correct lease counts in fake and Unity.

**Screenshots:** addressed asset-source gallery; one Image switched between two
source kinds with the active address in the inspector.

### Task 05 — Implement flex, dimensions, spacing, and positioning [DONE]

**Prerequisites:** Task 04.

Implement length/auto/percentage values and the layout style families:
alignment, flex direction/grow/shrink/wrap/basis, width/height/min/max,
position/offsets, aspect ratio, margins, padding, and
four-sided shorthands. Extend the authoritative Rust and C# style catalogs
together.

Add an adjustable layout playground that exposes every enum and value family,
including row/column reversal, wrapping, percentages, absolute positioning,
and shorthand-expanded spacing.

**Black-box acceptance:** representative conversions cover each value family;
all numeric bounds and clear/unset distinctions are enforced; a structural
catalog check proves every field in this task has one Rust and C# mapping.

**Screenshots:** wrapped row layout; resized column and absolute-position layout.

### Task 06 [DONE] — Implement color, borders, radii, clipping, and visibility

**Prerequisites:** Task 05.

Implement color, the four border widths/colors/radii and shorthands, opacity,
display, visibility, overflow, overflow clip box, slice values/type/scale, and
background tint. Add custom UI material assignment and its prepared-asset
lease. Omitted fields preserve Unity defaults during creation and leave current
values unchanged during updates.

Extend the styling page with layered cards, border/radius comparisons,
nine-slice presentation backed by a real prepared image, opacity, hidden versus
display-none, and overflow clipping specimens. A custom material is not a
substitute for the nine-slice image. Any material shown in the sample must be
compatible with the sample's render pipeline and must never render Unity's
magenta error surface.

**Black-box acceptance:** invalid colors, negative widths/radii/slices, and
invalid scale fail before native
mutation; tests inspect public inline style state rather than converter helpers.
All specimen containers resolve to explicit dark design-system surfaces, text
and controls meet the contrast requirement in both visibility states, and the
nine-slice specimen visibly renders its prepared image without an error color.

**Screenshots:** border and radius matrix; clipping, opacity, hidden, and
display-none comparison.

### Task 07 [DONE] — Implement background placement, repetition, sizing, and cursor

**Prerequisites:** Task 06.

Extend Task 06's prepared asset-backed background with independent x/y
positions, x/y repeat, size, background tint interaction, cursor texture and
hotspot, and the cursor's associated lease. Retain the existing background
lease ordering and apply the same stage-before-mutation ordering independently
to cursor assets. Import the sample cursor with Unity's Cursor texture defaults,
and have the Unity host reject a hotspot outside the acquired texture's pixel
bounds before any native property changes. Use no arbitrary style-property or
source escape hatch.

Unity 6.5 exposes no public linear or radial background-gradient value through
`Background`, `StyleBackground`, or `IStyle`. Do not add gradient DTOs,
generated textures, custom mesh rendering, a material convention, or gradient
sample claims. The technical design's background capability audit is
normative.

Add a background laboratory covering every asset source kind, position keyword
and offset family, x/y repeat mode, automatic/cover/contain/explicit size mode,
background tint, and the custom cursor. The page MUST contain no more than 28
visible words in every reachable state, excluding the persistent left
navigation. Any control that changes the specimen must restore the exact
initial source, placement, repeat, size, tint, and cursor state on the same
screen.

**Black-box acceptance:** preserve independent x/y values; reject nonfinite
offsets, wrong-axis position keywords, negative or nonfinite explicit sizes,
wrong cursor asset types, unreadable cursor textures, and hotspots that are
nonfinite, negative, or outside the acquired texture. Test all four background
source types, every repeat and size mode, tint interaction, independent
background and cursor old/staged lease ordering, exact reset behavior, cursor
restoration after hover, and teardown release. Tests inspect public inline
style state rather than converter helpers.

**Screenshots:** asset-source grid; repeat/position/size comparison with the
cursor texture preview and hover target visible.

### Task 08 [DONE] — Implement transforms and transitions

**Prerequisites:** Task 07.

Implement rotate, scale, translate, transform origin, standard filter functions,
transition property, duration, delay, timing-function lists, and the typed
conversion catalogs. Retain UI Toolkit list repetition semantics and sparse
update behavior.

Add transform-origin and transition specimens with deterministic controls for
the initial and settled states.

**Black-box acceptance:** reject nonfinite values, zero rotation axes, negative
durations, and unsupported properties; test all timing-function cases and list
repetition; public native transition events report supported property names.

**Screenshots:** transform-origin comparison; settled transition endpoint with
the transition payload in the inspector.

### Task 09 — Complete typography and text styling

**Prerequisites:** Task 08.

Complete `TextElement` and `Label` properties plus both public font-source
styles, style/weight, alignment, auto-size, outline, shadow,
paragraph/letter/word spacing, whitespace, overflow, overflow position, editor
rendering mode, and text-generator selection. Add the typed
`UnityEngine.Font` prepared-asset case required by `unityFont`. Apply text
through a public `INotifyValueChanged<string>` cast and selection preferences
through `ITextSelection`.

Add a typography page covering the addressed UI font, every text style,
rich text, emoji fallback, escape parsing, elision, and selectable text.

**Black-box acceptance:** Rust writes emit no value event; UI font leases,
UTF-16 selection bounds, and text numeric limits are
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
and Battlement `ScrollChanged`/`ScrollSettled` events. Implement controlled
Scroller direction, limits, `ValueChanging`, and `ValueCommitted`; do not add a
Scroller page-size field because Unity exposes page size only on its public
child `slider` and the normative Scroller contract excludes it.

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
sparse part-style updates, and asset leases. Cover the simple
Button, GroupBox, PopupWindow, Toggle, RadioButton, DropdownField, and
ProgressBar part catalogs.

Prefer direct public references. Otherwise query only below the owning control
with public `Q<T>` and audited public USS class-name constants. Never perform a
global query.

Add a part-anatomy overlay and custom simple-control skins to the lab.

**Black-box acceptance:** every valid part state resolves exactly one native
element; zero or multiple matches fail; asset replacement preserves unrelated
part style; destruction releases part leases.

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
conditional create/remove keeps matching authored style state; missing or
ambiguous audited parts fail rather than selecting another descendant; no stale
lease or style remains.

**Screenshots:** labeled slider, scroll, and tab anatomy; conditional parts
before and after activation.

### Task 22 — Complete pointer, wheel, capture, and routed phases

**Prerequisites:** Task 21.

Implement all pointer payloads, boundary/crossing events, Wheel, related-target
mapping for focus events, Trickle/Target/Bubble subscriptions, deterministic
Rust routing, and pointer capture events. Pointer crossing payloads contain no
related target because Unity's public `PointerOverEvent` and `PointerOutEvent`
surface does not provide one. Root observation maps Unity-created targets to
the nearest Rust-owned logical ancestor.

Add a nested event-routing visualizer and pointer-capture specimen.

**Black-box acceptance:** one native event creates one Rust action regardless
of subscribed ancestors; route order is deterministic; target-only events do
not propagate; omitted defaults encode exactly; unsubscribed high-frequency
events allocate no message; native/fake routes agree.

**Screenshots:** highlighted routed ancestor path; captured pointer and complete
payload in the inspector.

### Task 23 — Complete keyboard, navigation, focus, and activation

**Prerequisites:** Task 22.

Implement physical-key mapping from public `KeyDownEvent`/`KeyUpEvent`
`keyCode`, text, modifiers, navigation move/submit/cancel, focus relations and
direction, and Button navigation Click precedence. Do not add a UI key-repeat
field: Unity 6000.5.8f1 exposes no public repeat value on its UI Toolkit key
events. Preserve the separation between UI focus routing and global core
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
text selection, and rich-link enter/leave/down/up. Map those semantic link
events to the experimental public `PointerOverLinkTagEvent`,
`PointerOutLinkTagEvent`, `PointerDownLinkTagEvent`, and
`PointerUpLinkTagEvent`. Maintain link identity per `(ObjectId, pointer_id)`
because `PointerOutLinkTagEvent` lacks link ID and text.

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
- Every sample screen uses only shared design-system roles, renders body text at
  24 px or larger, stays within its tested visible-word budget, and restores its
  exact initial state after every interaction without requiring navigation or a
  restart.
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
