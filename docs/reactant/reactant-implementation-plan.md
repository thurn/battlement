# Battlement Reactant implementation plan

When a task is complete, append `[DONE]` to its task heading.

Status: implementation companion to
[`reactant-technical-design.md`](reactant-technical-design.md) and its
appendices.

This plan implements the approved Battlement Reactant contract without adding
features. The technical design and appendices are normative. If this plan and
the design disagree, the design wins.

## Decisions and starting point

Battlement Reactant does not exist yet. The repository already contains the
core protocol, Rust engine, Unity host, fake client, Battlement UI protocol,
UI fake, sample workflow, and visual-capture infrastructure on which Reactant
builds.

The following decisions were resolved while preparing this plan:

- Authoring optimizes first for brief, attractive UI expressions, then React
  parity, type safety, performance, and other concerns in that order.
- Primitive builder chains use one canonical order: primitive properties,
  children, events, then identity and placement adapters such as key, element
  ref, and portal. Wrapper types expose only the next legal methods.
- Ordinary application code may import a focused
  `battlement_reactant::prelude::*`. It includes authoring essentials, not the
  complete crate surface.
- Rust strings, characters, numbers, and booleans are not render values.
  Authors use a text primitive or a component, preserving deliberate UI code
  and avoiding surprising implicit formatting.
- V1 has no Strict Mode and no `use_id`. The latter remains reserved until the
  host can preserve React's accessibility-relationship purpose.
- Explicit render errors that escape a root return `Err(RenderError)` before
  commit and do not poison the runtime. Actual Rust panics and a missing
  Suspense boundary are developer failures that poison it.
- `ErrorBoundary::on_error` is model-aware:
  `Fn(&mut G, &RenderError)`. After it runs, every root factory is eligible to
  render because arbitrary application state may have changed.
- An external-store source swap overlaps the old and new subscriptions until
  commit. A generation token ignores stale notifications from the retiring
  source.
- `ElementRef::geometry` is the only direct ref-cache read. Reconstructible
  `WorldRef` and `ViewportRef` values are read through captured coherent
  snapshots.
- The host work required for `Prop<T>`, complete property resetting, and the
  geometry observation protocol is part of this plan. Reactant must not build
  private substitutes for those shared capabilities.
- `samples/reactant` is a focused teaching lab, not an exhaustive Battlement UI
  catalog. Its screens demonstrate composition, state and identity, context
  and memoization, effects and stores, events and portals, resources and
  boundaries, and refs and geometry.
- The Reactant lab uses the UI sample's dark Battlement visual language while
  owning Reactant-specific screens and styles. It includes both screen-space
  UI and world projection on its geometry screen, and contains no
  sample-specific C#.
- Tasks target roughly 200–250 lines of non-test code. A smaller coherent slice
  is preferred to filler; a larger slice must remain below 350 lines and state
  why it cannot be divided without leaving an unusable public contract.

## Task and testing conventions

Implementation is a mostly linear stack. Each task depends on the previous
task unless its prerequisites say otherwise, leaves all workspaces compiling,
and exposes only APIs whose behavior works end to end.

Task numbers are coordination metadata used only in this plan. Never put them
in product or sample assets, filenames, source comments, public documentation,
or diagnostics. Durable artifacts are named after behavior or scenario.

Every public API added or changed in a task receives concise user-facing
documentation in the same change. Doctests cover authoring shape and compile
fail tests cover typestate or hook restrictions where those are part of the
contract. Documentation must state deliberate React differences directly and
must not promise unsupported timing, batching, concurrency, accessibility, or
recovery behavior.

Black-box Rust tests use public APIs and finish in a visible fake-Unity fact or
an executed-command journal fact. They must continue to pass if virtual-tree,
hook-slot, cache, or mutation-planning internals are replaced. C# EditMode
tests use public package assemblies and inspect public Unity state. Tests may
not use reflection, friend assemblies, private registries, test-only sample
hooks, or implementation snapshots.

Sample code contains no inline Rust unit tests. Its integration tests live in
the sample workspace and drive its public engine through `battlement-fake`.
Reusable host behavior belongs in `battlement-ui-fake` or `battlement-fake`,
not copied into the sample.

Before repository validation, stage every intended change and run
`./scripts/ci.py`. Tasks that change public Reactant APIs also run Reactant
doctests and build documentation with warnings denied. Tasks that change the
Unity host run the relevant public EditMode suite. Major changes receive the
repository-mandated independent review once.

### Visual evidence contract

Every task that leaves a runnable sample slice captures visual evidence from
the final staged tree. Use the repository capture harness for deterministic
1280x720 initial, changed, and restored PNGs, as applicable. Each interaction
must be reversible without navigation or reconnect, and the restored capture
must match the initial behavior and layout.

Every runnable slice also builds the staged `reactant` WebGL sample, serves it
on a verified-free non-default port, exposes it through a named Cloudflare
Quick Tunnel, and verifies the direct HTTPS sample URL with the configured
Playwright MCP service. Keep the exact server and tunnel alive through review;
stop only those recorded services immediately before promotion. Final sample
acceptance additionally captures the packaged macOS Release player.

Runtime-only tasks that cannot yet affect a sample screen still produce
reviewable evidence: attach the named black-box test output and the public fake
command journal showing the before, action, and after states. A journal is not
a substitute once a visible sample slice exists.

During browser QA, use accessibility and DOM inspection for navigation and
state, and screenshots only for rendered appearance. Wait for a settled WebGL
frame before capture. Do not launch another browser process or replace the
configured shared Playwright service.

### Reactant sample design rules

- Use one persistent navigation column and one specimen canvas. Share the UI
  sample's dark surface, typography, spacing, cyan action, and amber warning
  tokens, while keeping Reactant screen styles in `samples/reactant`.
- Body copy and controls are at least 24 px, specimen headings at least 28 px,
  and page titles 44 px. Every reachable state retains at least 4.5:1 contrast.
- Each screen has one short title and only the words required to operate and
  recognize the behavior. It must not display object IDs, hook slots, command
  logs, property dumps, internal names, or explanatory paragraphs.
- Establish a visible-word budget in the screen test before implementation.
  Navigation is excluded. The budget applies to every reachable state.
- Every state-changing interaction restores the exact initial visible and
  behavioral state on the same screen. Black-box acceptance exercises the full
  initial → changed → initial round trip.
- Async demonstrations use a deterministic manual executor controlled by Rust.
  They do not depend on network access, wall-clock races, or sample-specific
  native code.

## Dependency overview

| Wave | Tasks | Result |
|---|---|---|
| 1 | 01–11 | Complete resettable Battlement UI property surface |
| 2 | 12–19 | Shared geometry protocol and runner integration |
| 3 | 20–25 | Reactant roots, components, identity, and reconciliation |
| 4 | 26–31 | State, reducers, refs, context, memoization, effects, stores |
| 5 | 32–35 | Typed events, propagation, portals, and render errors |
| 6 | 36–38 | Resources, Suspense, and deterministic async sample |
| 7 | 39–43 | Refs, coherent geometry, lifecycle hardening, final evidence |

## Wave 1: resettable Battlement UI properties

### Task 01 — Add the resettable property wire shape

**Prerequisites:** none. **Target:** 200–250 non-test lines.

Add the shared `Prop<T>` protocol representation with omitted, set, and reset
states. Prove JSON and C# DTO parity on one existing scalar visual property,
including strict rejection of malformed or ambiguous shapes. Do not expose it
on the remaining catalog yet.

**Black-box acceptance:** Rust JSON, public C# deserialization, Unity execution,
and `UiWorld` agree on omit/set/reset; resetting restores the documented Unity
default; rejection changes neither fake nor Unity state.

**Visual evidence:** runtime-only journal showing create, set, reset, and an
omitted no-op against the same public element.

### Task 02 — Apply `Prop<T>` to shared visual-element state

**Prerequisites:** Task 01. **Target:** 200–250 non-test lines.

Convert the shared `VisualElement` fields that Reactant reconciliation must
remove as well as assign. Preserve ordinary UI builder ergonomics and document
the exact reset default for each converted property.

**Black-box acceptance:** exhaustive public fake tests cover every supported
omit/set/reset transition and compare the resulting visible shared state with
public Unity EditMode behavior.

**Visual evidence:** runtime-only journal for a multi-property update whose
reset returns the element to its creation state.

### Task 03 — Reset layout styles

**Prerequisites:** Task 02. **Target:** 200–250 non-test lines.

Convert display, position, flex, alignment, size, min/max, margin, padding,
border width, overflow, and gap styles to resettable updates in coherent
groups. Keep wire names and Unity units unchanged.

**Black-box acceptance:** table-driven fake and EditMode tests exercise one
nondefault value and reset for each distinct conversion/state family; omitted
fields retain live values.

**Visual evidence:** runtime-only public host-state comparison for a layout
tree before assignment, after assignment, and after reset.

### Task 04 — Reset color, background, border, opacity, and cursor styles

**Prerequisites:** Task 03. **Target:** 200–250 non-test lines.

Complete the paint and interaction style families, including asset-backed
background and cursor values and lease changes caused by reset.

**Black-box acceptance:** fake and Unity reach the same paint values; resetting
an asset-backed value releases only its own usage lease and restores the native
default without disturbing sibling properties.

**Visual evidence:** runtime-only host-state and lease journal for set/reset.

### Task 05 — Reset transforms, filters, and transitions

**Prerequisites:** Task 04. **Target:** 200–250 non-test lines.

Complete transform origin, translate, rotate, scale, filter, and transition
list resets using only audited public Unity APIs.

**Black-box acceptance:** public tests prove list replacement, empty-list set,
reset, units, and ordering. Invalid numeric values reject atomically.

**Visual evidence:** runtime-only journal plus public Unity state for one
transform and one transition round trip.

### Task 06 — Reset typography and text styles

**Prerequisites:** Task 05. **Target:** 200–250 non-test lines.

Complete font, weight/style, alignment, sizing, whitespace, spacing, outline,
shadow, overflow, and rendering-option resets. Handle font asset leases through
the shared property path.

**Black-box acceptance:** fake and EditMode tests cover inherited and
noninherited behavior, unit conversions, asset release, omission, and reset.

**Visual evidence:** runtime-only public text-style comparison across the three
states.

### Task 07 — Reset Label, TextElement, Image, and Button properties

**Prerequisites:** Task 06. **Target:** 200–250 non-test lines.

Apply reset semantics to the first leaf primitives needed by Reactant and their
asset-backed fields. Keep creation-required values distinct from updateable
values.

**Black-box acceptance:** public fake and Unity tests prove text, image source,
tint, icon, and button content can be removed or restored without remounting.

**Visual evidence:** runtime-only tree and lease journal with stable object IDs.

### Task 08 — Reset container and navigation properties

**Prerequisites:** Task 07. **Target:** 200–250 non-test lines.

Cover Box-like containers, group/popup controls, tabs, and tab views. Preserve
typed part ownership and controlled selection semantics.

**Black-box acceptance:** reset leaves hierarchy and selected-tab identity
valid; fake and Unity agree after set/reset; invalid selection rejects without
partial change.

**Visual evidence:** runtime-only hierarchy journal proving no remount.

### Task 09 — Reset scrolling properties

**Prerequisites:** Task 08. **Target:** 200–250 non-test lines.

Cover scroll views and scrollers, including mode, offsets, paging, wheel/touch
configuration, elasticity, and limits. Use the audited public slider route
where Unity lacks a direct setter.

**Black-box acceptance:** reset restores the documented native configuration;
controlled values do not emit authored change actions; fake and Unity agree.

**Visual evidence:** runtime-only scroll-state journal.

### Task 10 — Reset text input and choice-control properties

**Prerequisites:** Task 09. **Target:** 200–250 non-test lines.

Cover text fields, toggles, radio controls, dropdowns, and toggle groups. Keep
controlled-value writes notification-free and preserve validation invariants.

**Black-box acceptance:** set/reset does not synthesize user events; invalid
choice values reject atomically; omitted fields preserve current state.

**Visual evidence:** runtime-only event and value journal.

### Task 11 — Reset range and progress-control properties

**Prerequisites:** Task 10. **Target:** 200–250 non-test lines.

Complete sliders, integer sliders, min/max sliders, progress bars, and remaining
element fields. Add one structural completeness check tying every updateable
protocol field to a set/reset executor.

**Black-box acceptance:** exhaustive catalog coverage finds no silently ignored
field; range invariants reject atomically; fake and Unity match for each
distinct conversion.

**Visual evidence:** runtime-only journal for controlled value, range, and reset
without generated input events.

## Wave 2: shared geometry and runner prerequisites

### Task 12 — Finish reset coverage across fake, Unity, and UI sample

**Prerequisites:** Task 11. **Target:** 150–225 non-test lines.

Remove legacy optional-update assumptions, update UI sample call sites, and add
one authoritative structural ledger for every resettable field. Keep Reactant
absent from this shared validation.

**Black-box acceptance:** all existing UI scenarios retain their behavior;
structural checks require a fake and Unity route for every field; malformed
resets mutate nothing.

**Visual evidence:** recapture the affected UI sample screens in their initial
and restored states and verify the deployed UI WebGL sample.

### Task 13 — Add pointer related-target data

**Prerequisites:** Task 12. **Target:** 150–225 non-test lines.

Extend pointer crossing actions with the optional picked related target needed
for React-style enter/leave synthesis. Derive it from public picked paths; do
not claim Unity exposes native `relatedTarget` on over/out events.

**Black-box acceptance:** public Unity tests produce complementary over/out
pairs with reversed targets; fake serialization preserves absence and presence;
unrelated or intervening events remain distinct.

**Visual evidence:** runtime-only event journal for sibling crossing, ancestor
crossing, and leaving the document.

### Task 14 — Define and validate the geometry protocol

**Prerequisites:** Task 13. **Target:** 200–250 non-test lines.

Add canonical observation IDs, targets, registry updates, batches, values,
unavailable reasons, spaces, and generations to the core protocol. Validate a
whole batch before accepting any value.

**Black-box acceptance:** Rust JSON and public C# mirrors round-trip every
variant; duplicate IDs, wrong value kinds, invalid numbers, and malformed
generations reject before partial submission.

**Visual evidence:** runtime-only accepted and rejected protocol transcript.

### Task 15 — Sample screen-space element and viewport geometry

**Prerequisites:** Task 14. **Target:** 200–250 non-test lines.

Implement UI element and display viewport observations in one native sampling
pass, normalized into the specified upper-left viewport space. Report display
zero on single-display builds.

**Black-box acceptance:** public EditMode tests cover panel scale, safe area,
display mapping, unavailable targets, and unchanged-value omission; fake
application exposes the same values.

**Visual evidence:** host-only geometry fixture screenshot plus its public
observation batch; Reactant sample evidence begins later.

### Task 16 — Sample world-space UI and target-texture geometry

**Prerequisites:** Task 15. **Target:** 200–250 non-test lines.

Extend element sampling across world-space panels and target textures using the
explicit camera and display mappings already defined by Battlement UI.

**Black-box acceptance:** tests cover visible, clipped, unavailable-camera, and
target-texture mappings without mixing coordinate spaces or partial batches.

**Visual evidence:** host fixture captures for world-space and target-texture
placements with their observation values.

### Task 17 — Sample world origins and named anchors

**Prerequisites:** Task 16. **Target:** 200–250 non-test lines.

Add world-object origin and authored `BattlementGeometryAnchor` projection.
Prepared-asset validation rejects duplicate anchor names; a missing anchor on a
live object is a host contract failure.

**Black-box acceptance:** public tests cover root origin, named child, camera
selection, behind-camera status, duplicate anchors, and missing anchors.

**Visual evidence:** world fixture capture with projected origin and anchor
markers, backed by the public batch.

### Task 18 — Sample rendered world bounds

**Prerequisites:** Task 17. **Target:** 150–225 non-test lines.

Project the combined bounds of enabled renderers beneath a world object and
report the defined unavailable and clipping states.

**Black-box acceptance:** tests cover multiple renderers, disabled renderers,
empty objects, partial clipping, camera changes, and deterministic bounds.

**Visual evidence:** world fixture before and after renderer visibility changes.

### Task 19 — Coalesce and route geometry in the runner

**Prerequisites:** Task 18. **Target:** 200–250 non-test lines.

Retain at most one pending batch, merge by observation, drop returns to the
last submitted state, and send geometry instead of that frame's empty poll.
Immediate input events remain separate.

**Black-box acceptance:** complete engine tests prove one scheduled frame round
trip, newest-generation retention, stale-epoch rejection, unchanged omission,
and ordinary input ordering.

**Visual evidence:** runtime-only transport transcript showing several native
frames coalesced into one engine action.

## Wave 3: Reactant rendering foundation

### Task 20 — Create the crate, prelude, and first render value

**Prerequisites:** Tasks 12 and 19. **Target:** 200–250 non-test lines.

Add `battlement-reactant`, its focused prelude, sealed render conversion, empty
rendering, one primitive, and canonical primitive builder order. Add compile
tests proving raw scalars do not render and out-of-order adapters are absent.

**Black-box acceptance:** a public root renders one primitive into `UiWorld`;
an identical render emits no command; doctests demonstrate the intended terse
authoring expression.

**Visual evidence:** runtime-only fake tree and empty second-commit journal.

### Task 21 — Add roots, sessions, and a static document

**Prerequisites:** Task 20. **Target:** 200–250 non-test lines.

Implement root registration, stable document ownership, `begin_session`,
`SessionUi::into_response`, registration closure, and static child creation.
Reject pre-populated or duplicate documents before runtime mutation.

**Black-box acceptance:** `FakeClient` begins a session with two roots, including
an empty one; IDs remain stable; abandoned conversion changes nothing; late
registration and duplicate ownership panic transactionally.

**Visual evidence:** runtime-only full response and applied fake hierarchy.

### Task 22 — Add components and the Reactant sample shell

**Prerequisites:** Task 21. **Target:** 200–250 non-test lines.

Implement component identity and nested rendering sufficient for pure
composition. Create `samples/reactant`, its Rust rules workspace, Unity project,
scene, design tokens, navigation, and a Composition screen built without
sample-specific C#.

**Black-box acceptance:** the sample engine begins through `FakeClient`; nested
components render the same public tree as equivalent primitives; component
boundaries add no host element.

**Visual evidence:** Composition initial capture and verified WebGL link. The
screen shows nested reusable cards with no implementation terminology.

### Task 23 — Preserve identity with fragments and keys

**Prerequisites:** Task 22. **Target:** 200–250 non-test lines.

Add fragments, iterable children, typed keys, sibling duplicate validation,
and keyed/unkeyed identity matching. Do not expose internal IDs in sample UI.

**Black-box acceptance:** reorder, insertion, and removal preserve or replace
public host IDs exactly as specified; duplicate same-typed keys panic before
commit; abandoned work leaves Unity unchanged.

**Visual evidence:** Composition screen before reorder, changed order, and
restored order with stable visible row state.

### Task 24 — Reconcile creation, destruction, properties, and no-ops

**Prerequisites:** Task 23. **Target:** 200–250 non-test lines.

Implement host-kind replacement, sparse property diffing with `Prop<T>` resets,
subtree creation/destruction, and deterministic no-op elimination.

**Black-box acceptance:** desired public fake trees match after add/change/reset/
remove; replacement destroys and recreates; identical rerender has an empty
journal; validation failure commits nothing.

**Visual evidence:** Composition screen changes one card style/content and
restores it without moving unaffected siblings.

### Task 25 — Reconcile moves, groups, and receipts

**Prerequisites:** Task 24. **Target:** 200–250 non-test lines.

Plan reparent and sibling moves with dependency barriers, parallel groups, and
receipt acknowledgement. Preserve logical ordering across content containers
and discard tentative plans on failure.

**Black-box acceptance:** randomized small-tree tests compare final `UiWorld`
with a simple desired-tree oracle; journals contain only required operations
and preserve barriers; failed or abandoned receipts retry safely.

**Visual evidence:** Composition screen initial, moved grouping, and restored;
the WebGL interaction proves the same public result.

## Wave 4: hooks and reactive state

### Task 26 — Add hook context and state queues

**Prerequisites:** Task 25. **Target:** 200–250 non-test lines.

Implement positional hook slots, `use_state`, lazy initialization, stable
setters, queued value/updater application, batching, and forbidden-context
checks. Actual panics poison the runtime.

**Black-box acceptance:** clicks queue multiple updates but commit one final
visible value; lazy initialization is stable; keyed identity preserves state;
hook count/kind/type mismatches panic without a partial commit and poison later
entries.

**Visual evidence:** add the State & Identity screen; capture initial, updated,
reordered, and restored states with a verified WebGL interaction.

### Task 27 — Add reducers and state reset behavior

**Prerequisites:** Task 26. **Target:** 200–250 non-test lines.

Implement `use_reducer`, lazy initialization, stable dispatch, ordered action
queues, and state replacement through component identity. Reducers remain pure
and hook-forbidden.

**Black-box acceptance:** a sequence of public actions produces one visible
reduced state; reducer panic poisons; remount resets state while keyed reorder
does not.

**Visual evidence:** State & Identity reducer interaction initial, changed,
and restored.

### Task 28 — Add arbitrary refs and context propagation

**Prerequisites:** Task 27. **Target:** 200–250 non-test lines.

Implement `use_ref`, context definitions/default factories, providers, nearest
lookup, and provider identity. Evaluate each default once per runtime.

**Black-box acceptance:** nested providers change only descendant visible
content; ref mutation survives rerender without scheduling one; changing
provider value updates consumers; reconnect retains logical state.

**Visual evidence:** add Context & Memo screen and capture outer, overridden,
and restored themes.

### Task 29 — Add memo values, callbacks, and component bailout

**Prerequisites:** Task 28. **Target:** 200–250 non-test lines.

Implement dependency tuples, `use_memo`, `use_callback`, and `memo` component
bailout with context and local-dirty invalidation. Memo calculations are pure,
hook-forbidden, and may be repeated after abandoned renders.

**Black-box acceptance:** journals prove unrelated parent changes do not touch a
memoized public subtree; dependency or context changes update it; callback
identity follows dependencies; panic poisons.

**Visual evidence:** Context & Memo initial, unrelated update, context update,
and restored captures; visible output demonstrates which card changed without
showing counters or logs.

### Task 30 — Add passive effects and cleanup ordering

**Prerequisites:** Task 29. **Target:** 200–250 non-test lines.

Implement passive effect registration, dependency replacement, child-before-
parent cleanup, later-entry setup, unmount cleanup, and synchronous shutdown
cleanup. State queued by an effect joins the next eligible render.

**Black-box acceptance:** public model state and fake UI prove commit-before-
effect timing, cleanup/setup order, dependency stability, unmount, and shutdown;
effect panic poisons and emits no partial commit.

**Visual evidence:** add Effects & Stores screen; capture disconnected,
connected, and restored states through deterministic entries.

### Task 31 — Add external stores and generation-safe source swaps

**Prerequisites:** Task 30. **Target:** 200–250 non-test lines.

Implement snapshot reads, commit-time subscription, immediate recheck, retry
limit, wake coalescing, and overlapping source swaps. Generation tokens suppress
notifications from a retired source after the new subscription commits.

**Black-box acceptance:** a deliberately racy public store cannot miss an
update between render and subscribe; several wakes coalesce; stale old-source
wakes are ignored; retry exhaustion panics and poisons.

**Visual evidence:** Effects & Stores initial, source swap/update, and restored
captures plus verified WebGL behavior.

## Wave 5: events, portals, and render errors

### Task 32 — Add typed event adapters and host subscriptions

**Prerequisites:** Task 31. **Target:** 200–250 non-test lines.

Implement typed handler builders, model-type validation, subscription diffing,
builder-order handler storage, and minimal host coverage subscriptions. Reject
authored native subscriptions on Reactant-owned primitives.

**Black-box acceptance:** a user action reaches the typed Rust handler and
changes visible fake UI; adding/removing/replacing a handler produces only the
required subscription commands; wrong model types and authored conflicts panic
transactionally.

**Visual evidence:** add Events & Portals screen with one reversible typed
interaction and verified WebGL link.

### Task 33 — Add logical propagation and pointer crossing

**Prerequisites:** Task 32. **Target:** 200–250 non-test lines.

Implement capture and bubble paths, stop propagation semantics, current target,
logical ancestry through components, and exact complementary pointer over/out
deduplication into enter/leave. Any intervening event clears the pair.

**Black-box acceptance:** public action sequences prove path order, stopping,
nested targets, sibling crossing, document exit, related target, and no false
deduplication.

**Visual evidence:** Events & Portals capture showing pointer transition and
restored state; screen copy remains user-facing rather than an event log.

### Task 34 — Add internal and external portals

**Prerequisites:** Task 33. **Target:** 200–250 non-test lines.

Implement portal targets, logical/physical ancestry separation, external
container registration and staged reconnect rebind, event-island coverage, and
portal remount on target/key change.

**Black-box acceptance:** fake Unity proves physical placement and logical
propagation simultaneously; external commands validate against caller
snapshots; missing/cross-runtime targets fail before mutation; reconnect uses
the staged binding.

**Visual evidence:** Events & Portals initial inline card, portaled overlay,
event response, and restored captures through WebGL.

### Task 35 — Add fallible rendering and error boundaries

**Prerequisites:** Task 34. **Target:** 200–250 non-test lines.

Implement `Result<R,E>` render conversion, nearest `ErrorBoundary`, typed error
matching, latched fallback, `reset_on`, model-aware `on_error`, and fallback
escalation. An error escaping a root returns `Err(RenderError)` without commit
or poisoning; actual panics remain uncaught and poison.

**Black-box acceptance:** nested boundaries show the nearest public fallback;
reset remounts primary state; fallback errors reach the outer boundary;
`on_error` mutates the model and causes all root factories to run; a root error
leaves Unity unchanged and a corrected retry succeeds.

**Visual evidence:** add Resources & Boundaries error state, reset, and restored
captures. Runtime-only evidence separately proves the root `Err` retry because
that failure intentionally has no visual commit.

## Wave 6: resources and Suspense

### Task 36 — Add resource identity, cache, and spawner integration

**Prerequisites:** Task 35. **Target:** 200–250 non-test lines.

Implement typed resources, erased cache identity, keyed generations, preload,
invalidate, clear, shared tasks, and the injected spawner. Task completion only
wakes the engine thread; it never mutates render state off-thread.

**Black-box acceptance:** a deterministic executor proves request deduplication,
generation replacement, stale completion suppression, cache sharing across
roots, and administration-driven dirty work.

**Visual evidence:** runtime-only resource request/completion transcript.

### Task 37 — Add reads and initial Suspense fallback

**Prerequisites:** Task 36. **Target:** 200–250 non-test lines.

Implement `use_resource`, `ResourceRead::then`, pending tokens, nearest
Suspense collection, fallback rendering, and fallible-resource propagation.
The `.then` closure is hook-forbidden. Missing Suspense is a panic that poisons.

**Black-box acceptance:** pending content shows fallback; shared pending reads
start one task; failure reaches the nearest error boundary; missing boundary and
hook misuse panic without partial commit and poison.

**Visual evidence:** Resources & Boundaries pending capture through the manual
executor and verified WebGL link.

### Task 38 — Retain suspended trees and reveal ready content

**Prerequisites:** Task 37. **Target:** 200–250 non-test lines.

Preserve previously committed primary state while suspended, manage waiter
lifetimes, retry on completion, and reveal on the first successful Reactant
entry. Do not add timed batching, transitions, or deferred values.

**Black-box acceptance:** initial fallback, retained update suspension, stale
completion, key change, unmount, retry, and state preservation all end in
observable fake UI facts with no intermediate partial tree.

**Visual evidence:** Resources & Boundaries initial, pending, ready, error, and
restored captures driven deterministically in WebGL.

## Wave 7: refs, geometry, and release proof

### Task 39 — Add element refs and queued host actions

**Prerequisites:** Task 38. **Target:** 200–250 non-test lines.

Implement `use_element_ref`, attachment generations, `.element_ref`, committed
attachment queries, supported one-shot host actions, and action validation.
Components and structural render values cannot receive an element ref.

**Black-box acceptance:** attach, keyed move, remount, detach, reconnect, stale
action, duplicate attachment, render-time action, and cross-runtime action all
produce the specified public fake state or transactional panic.

**Visual evidence:** add Refs & Geometry screen with focus/select action,
changed focus state, and restored state through WebGL.

### Task 40 — Add geometry targets, snapshots, and conversions

**Prerequisites:** Task 39. **Target:** 200–250 non-test lines.

Implement element, viewport, and world refs; sealed target shapes; tuples,
arrays, and vectors; observation registry diffs; coherent snapshots; measurement
status; and coordinate conversions. Only `ElementRef` exposes `geometry()`.

**Black-box acceptance:** public tests cover target add/remove/reorder,
duplicates, unavailable values, retained stale values, reconnect invalidation,
complete-generation gating, and round-trip conversions without inspecting the
registry.

**Visual evidence:** Refs & Geometry screen-space placement initial, moved,
and restored captures, with snapshot-derived placement rather than hardcoded
coordinates.

### Task 41 — Add geometry effects and world projection sample

**Prerequisites:** Task 40. **Target:** 200–250 non-test lines.

Implement `use_geometry_effect`, cleanup/setup ordering, coherent target
snapshots, dependency replacement, and model-aware callbacks. Complete the
sample screen with a projected world origin or anchor and a world-bounds
specimen using the shared host protocol.

**Black-box acceptance:** callbacks run only for coherent generations or
dependency change, clean up child-before-parent, queue state correctly, render
all roots after model mutation, and poison on panic.

**Visual evidence:** Refs & Geometry captures for screen-space UI, world point,
world bounds, unavailable state, and restored placement in WebGL.

### Task 42 — Harden reconnect, shutdown, failures, and reconciliation

**Prerequisites:** Task 41. **Target:** 200–300 non-test lines; the combined
lifecycle matrix is one inseparable public contract.

Close lifecycle gaps across all entry points: reconnect retention and geometry
invalidation, staged portal rebind, abandoned sessions, empty successful
commits, synchronous shutdown cleanup, dropped-runtime diagnostics, explicit
root errors, missing Suspense, and panic poisoning. Expand randomized desired-
tree testing and record a stable performance baseline without making it a
public guarantee.

**Black-box acceptance:** one table-driven public suite exercises every runtime
state/entry-point combination; no failed entry emits a partial commit; explicit
errors retry; every actual panic poisons; randomized final fake trees match the
oracle; representative no-op and reorder journals stay bounded.

**Visual evidence:** runtime-only lifecycle matrix output, randomized seed log,
and command-count baseline. Recapture any sample state affected by fixes.

### Task 43 — Complete the sample, documentation, and release evidence

**Prerequisites:** Task 42. **Target:** 150–250 non-test lines.

Polish the seven focused screens, remove temporary fixtures, complete public
documentation and compile tests, and add a checked feature-to-screen/test
ledger. The ledger maps only the approved V1 surface and explicitly marks
reserved React APIs such as `useId` and `useLayoutEffect` as unsupported rather
than planned sample features.

**Black-box acceptance:** `FakeClient` navigates every screen and performs each
initial → changed → initial interaction; word budgets, text-size floors,
contrast roles, deterministic async behavior, and absence of sample-specific C#
are structurally checked; all doctests and `./scripts/ci.py` pass from the final
staged tree.

**Visual evidence:** capture every screen at 1280x720 in its initial state and
every meaningful changed state, then capture the restored states. Verify the
final direct WebGL URL with Playwright and capture the packaged macOS Release
player. Retain the evidence manifest, browser walkthrough, and exact service
identities for review.

## Completion criteria

Reactant is complete when all tasks are marked done, the normative design and
public documentation describe the implemented behavior without contradictions,
all public black-box and Unity tests pass, the feature ledger has no uncovered
V1 capability, and the final staged sample evidence demonstrates every focused
screen in initial, changed, and restored states.
