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
- Tasks target roughly 200–250 lines of non-test code. Prefer a smaller
  coherent slice to filler. A larger slice must remain below 350 lines and
  state why it cannot be divided without leaving an unusable public contract.

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
| 3 | 20–29 | Runtime, authoring, identity, reconciliation, and delivery |
| 4 | 30–39 | Events, state, refs, context, memoization, effects, and stores |
| 5 | 40–42 | Portals and recoverable render errors |
| 6 | 43–46 | Resources, Suspense, and deterministic async behavior |
| 7 | 47–53 | Refs, geometry, lifecycle hardening, and final evidence |

## Wave 1: resettable Battlement UI properties

### Task 01 — Add the resettable property wire shape [DONE]

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

### Task 02 — Apply `Prop<T>` to shared visual-element state [DONE]

**Prerequisites:** Task 01. **Target:** 200–250 non-test lines.

Convert the shared `VisualElement` fields that Reactant reconciliation must
remove as well as assign. Preserve ordinary UI builder ergonomics and document
the exact reset default for each converted property.

**Black-box acceptance:** exhaustive public fake tests cover every supported
omit/set/reset transition and compare the resulting visible shared state with
public Unity EditMode behavior.

**Visual evidence:** runtime-only journal for a multi-property update whose
reset returns the element to its creation state.

### Task 03 — Reset layout styles [DONE]

**Prerequisites:** Task 02. **Target:** 200–250 non-test lines.

Convert display, position, flex, alignment, size, min/max, margin, padding,
border width, and overflow styles to resettable updates in coherent groups.
Keep wire names and Unity units unchanged. Gap styles remain outside the V1
contract because pinned Unity 6000.5 exposes no public inline row-gap or
column-gap property; revisit them when the public host API can apply and
observe their state.

**Black-box acceptance:** table-driven fake and EditMode tests exercise one
nondefault value and reset for each distinct conversion/state family; omitted
fields retain live values.

**Visual evidence:** runtime-only public host-state comparison for a layout
tree before assignment, after assignment, and after reset.

### Task 04 — Reset color, background, border, opacity, and cursor styles [DONE]

**Prerequisites:** Task 03. **Target:** 200–250 non-test lines.

Complete the paint and interaction style families, including asset-backed
background and cursor values and lease changes caused by reset.

**Black-box acceptance:** fake and Unity reach the same paint values; resetting
an asset-backed value releases only its own usage lease and restores the native
default without disturbing sibling properties.

**Visual evidence:** runtime-only host-state and lease journal for set/reset.

### Task 05 — Reset transforms, filters, and transitions [DONE]

**Prerequisites:** Task 04. **Target:** 200–250 non-test lines.

Complete transform origin, translate, rotate, scale, filter, and transition
list resets using only audited public Unity APIs.

**Black-box acceptance:** public tests prove list replacement, empty-list set,
reset, units, and ordering. Invalid numeric values reject atomically.

**Visual evidence:** runtime-only journal plus public Unity state for one
transform and one transition round trip.

### Task 06 — Reset typography and text styles [DONE]

**Prerequisites:** Task 05. **Target:** 200–250 non-test lines.

Complete font, weight/style, alignment, sizing, whitespace, spacing, outline,
shadow, overflow, and rendering-option resets. Handle font asset leases through
the shared property path.

**Black-box acceptance:** fake and EditMode tests cover inherited and
noninherited behavior, unit conversions, asset release, omission, and reset.

**Visual evidence:** runtime-only public text-style comparison across the three
states.

### Task 07 — Reset Label, TextElement, Image, and Button properties [DONE]

**Prerequisites:** Task 06. **Target:** 200–250 non-test lines.

Apply reset semantics to the first leaf primitives needed by Reactant and their
asset-backed fields. Keep creation-required values distinct from updateable
values.

**Black-box acceptance:** public fake and Unity tests prove text, image source,
tint, icon, and button content can be removed or restored without remounting.

**Visual evidence:** runtime-only tree and lease journal with stable object IDs.

### Task 08 — Reset container and navigation properties [DONE]

**Prerequisites:** Task 07. **Target:** 200–250 non-test lines.

Cover Box-like containers, group/popup controls, tabs, and tab views. Preserve
typed part ownership and controlled selection semantics.

**Black-box acceptance:** reset leaves hierarchy and selected-tab identity
valid; fake and Unity agree after set/reset; invalid selection rejects without
partial change.

**Visual evidence:** runtime-only hierarchy journal proving no remount.

### Task 09 — Reset scrolling properties [DONE]

**Prerequisites:** Task 08. **Target:** 200–250 non-test lines.

Cover scroll views and scrollers, including mode, offsets, paging, wheel/touch
configuration, elasticity, and limits. Use the audited public slider route
where Unity lacks a direct setter.

**Black-box acceptance:** reset restores the documented native configuration;
controlled values do not emit authored change actions; fake and Unity agree.

**Visual evidence:** runtime-only scroll-state journal.

### Task 10 — Reset text input and choice-control properties [DONE]

**Prerequisites:** Task 09. **Target:** 200–250 non-test lines.

Cover text fields, toggles, radio controls, dropdowns, and toggle groups. Keep
controlled-value writes notification-free and preserve validation invariants.

**Black-box acceptance:** set/reset does not synthesize user events; invalid
choice values reject atomically; omitted fields preserve current state.

**Visual evidence:** runtime-only event and value journal.

### Task 11 — Reset range and progress-control properties [DONE]

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

### Task 12 — Finish reset coverage across fake, Unity, and UI sample [DONE]

**Prerequisites:** Task 11. **Target:** 150–225 non-test lines.

Remove legacy optional-update assumptions, update UI sample call sites, and add
one authoritative structural ledger for every resettable field. Keep Reactant
absent from this shared validation.

**Black-box acceptance:** all existing UI scenarios retain their behavior;
structural checks require a fake and Unity route for every field; malformed
resets mutate nothing.

**Visual evidence:** recapture the affected UI sample screens in their initial
and restored states and verify the deployed UI WebGL sample.

### Task 13 — Add pointer related-target data [DONE]

**Prerequisites:** Task 12. **Target:** 150–225 non-test lines.

Extend pointer crossing actions with the optional picked related target needed
for React-style enter/leave synthesis. Derive it from public picked paths; do
not claim Unity exposes native `relatedTarget` on over/out events.

**Black-box acceptance:** public Unity tests produce complementary over/out
pairs with reversed targets; fake serialization preserves absence and presence;
unrelated or intervening events remain distinct.

**Visual evidence:** runtime-only event journal for sibling crossing, ancestor
crossing, and leaving the document.

### Task 14 — Define and validate the geometry protocol [DONE]

**Prerequisites:** Task 13. **Target:** 200–250 non-test lines.

Add canonical observation IDs, targets, registry updates, batches, values,
unavailable reasons, spaces, and generations to the core protocol. Validate a
whole batch before accepting any value.

**Black-box acceptance:** Rust JSON and public C# mirrors round-trip every
variant; duplicate IDs, wrong value kinds, invalid numbers, and malformed
generations reject before partial submission.

**Visual evidence:** runtime-only accepted and rejected protocol transcript.

### Task 15 — Sample screen-space element and viewport geometry [DONE]

**Prerequisites:** Task 14. **Target:** 200–250 non-test lines.

Implement UI element and display viewport observations in one native sampling
pass, normalized into the specified upper-left viewport space. Report display
zero on single-display builds.

**Black-box acceptance:** public EditMode tests cover panel scale, safe area,
display mapping, unavailable targets, and unchanged-value omission; fake
application exposes the same values.

**Visual evidence:** host-only geometry fixture screenshot plus its public
observation batch; Reactant sample evidence begins later.

### Task 16 — Sample world-space UI and target-texture geometry [DONE]

**Prerequisites:** Task 15. **Target:** 200–250 non-test lines.

Extend element sampling across world-space panels and target textures using the
explicit camera and display mappings already defined by Battlement UI.

**Black-box acceptance:** tests cover visible, clipped, unavailable-camera, and
target-texture mappings without mixing coordinate spaces or partial batches.

**Visual evidence:** host fixture captures for world-space and target-texture
placements with their observation values.

### Task 17 — Sample world origins and named anchors [DONE]

**Prerequisites:** Task 16. **Target:** 200–250 non-test lines.

Add world-object origin and authored `BattlementGeometryAnchor` projection.
Prepared-asset validation rejects duplicate anchor names; a missing anchor on a
live object is a host contract failure.

**Black-box acceptance:** public tests cover root origin, named child, camera
selection, behind-camera status, duplicate anchors, and missing anchors.

**Visual evidence:** world fixture capture with projected origin and anchor
markers, backed by the public batch.

### Task 18 — Sample rendered world bounds [DONE]

**Prerequisites:** Task 17. **Target:** 150–225 non-test lines.

Project the combined bounds of enabled renderers beneath a world object and
report the defined unavailable and clipping states.

**Black-box acceptance:** tests cover multiple renderers, disabled renderers,
empty objects, partial clipping, camera changes, and deterministic bounds.

**Visual evidence:** world fixture before and after renderer visibility changes.

### Task 19 — Coalesce and route geometry in the runner [DONE]

**Prerequisites:** Task 18. **Target:** 200–250 non-test lines.

Retain at most one pending batch, merge by observation, drop returns to the
last submitted state, and send geometry instead of that frame's empty poll.
Immediate input events remain separate.

**Black-box acceptance:** complete engine tests prove one scheduled frame round
trip, newest-generation retention, stale-epoch rejection, unchanged omission,
and ordinary input ordering.

**Visual evidence:** runtime-only transport transcript showing several native
frames coalesced into one engine action.

## Wave 3: Reactant runtime and rendering foundation

### Task 20 — Establish the runtime, one root, and one host primitive [DONE]

**Prerequisites:** Tasks 12 and 19. **Target:** 250–350 non-test lines. The
runtime transaction and first public vertical slice are inseparable.

Add `battlement-reactant`, the sealed `Render` trait, `Spawner`, `SpawnedTask`,
the runtime state machine and stored executor, one childless document root, one
renderable primitive, `begin_session`, session conversion, and the minimal
opaque commit path. The executor remains idle until resources arrive. Declare
all public runtime entry methods with their baseline registering/active
legality, even when later tasks add their work.

**Black-box acceptance:** `FakeClient` begins a session whose public snapshot
contains one rendered primitive; a second root may be empty; duplicate or
pre-populated documents fail before mutation; every entry method has its
documented baseline result or panic.

**Visual evidence:** runtime-only full response and applied fake hierarchy.

### Task 21 — Add structural render values and erasure [DONE]

**Prerequisites:** Task 20. **Target:** 200–250 non-test lines.

Complete sealed render conversion for `()`, `Option`, tuples, arrays, vectors,
`Rc`, `Fragment`, `Either`, and `Node`. Preserve logical empty positions and
the wrapped concrete descriptor through erasure. Raw scalars remain rejected.

**Black-box acceptance:** public roots compose every structural form into the
expected fake hierarchy; changing `Option` or `Either` follows the specified
position and type identity; compile tests reject scalars and arbitrary
iterators.

**Visual evidence:** runtime-only hierarchy and stable-ID journal.

### Task 22 — Add component structs and pure nested rendering [DONE]

**Prerequisites:** Task 21. **Target:** 200–250 non-test lines.

Implement the `Component` adapter, nested component identity, hook-forbidden
row and render-prop closures, and pure abandoned rendering. Component
boundaries create no host elements.

**Black-box acceptance:** nested components and closure props render the same
public tree as equivalent primitives; an abandoned component render changes no
host, callback, or identity; doctests compile owned `'static` authoring forms.

**Visual evidence:** runtime-only nested hierarchy and abandoned-render journal.

### Task 23 — Complete primitive properties and child builders [DONE]

**Prerequisites:** Task 22. **Target:** 200–300 non-test lines.

Make every supported `battlement-ui` primitive a render value. Add complete
property, `.child`, and `.children` coverage while preserving legal native
children and the property-before-children portion of canonical builder order.

**Black-box acceptance:** a structural catalog renders every primitive and
legal child family; compile-fail cases reject illegal children and property
methods after children; authored native subscriptions fail transactionally.

**Visual evidence:** runtime-only catalog hierarchy and validation journal.

### Task 24 — Add required props and the focused prelude [DONE]

**Prerequisites:** Task 23. **Target:** 200–250 non-test lines.

Implement `Missing`, hand-written typestate support, `required_props!`, and the
focused authoring prelude. Record the prelude as the sole exception to the
repository's public re-export rule. Keep key, ref, and portal methods absent
until the tasks that make each adapter work end to end.

**Black-box acceptance:** required setters work in every order; incomplete
values and repeated children fail to compile; the prelude compiles the ordinary
terse authoring example without exposing runtime administration.

**Visual evidence:** runtime-only equivalent required-prop trees.

### Task 25 — Create the Reactant sample shell and Composition screen [DONE]

**Prerequisites:** Task 24. **Target:** 200–300 non-test lines.

Create `samples/reactant`, its Rust workspace, Unity project, scene, design
tokens, persistent navigation, and Composition screen without sample-specific
C#. Exercise components, structural values, required props, and primitive
children without depending on events.

**Black-box acceptance:** the sample begins through `FakeClient`, navigates to
Composition through its public initial state, and satisfies the screen's word,
text-size, and contrast checks.

**Visual evidence:** Composition initial capture and verified WebGL link.

### Task 26 — Preserve keyed and unkeyed identity [DONE]

**Prerequisites:** Task 25. **Target:** 200–250 non-test lines.

Implement the terminal `.key` adapter, typed keys, duplicate validation,
absolute unkeyed positions, fixed semantic wrapper markers, and keyed component
and fragment matching. Compile tests reject child or property methods after a
key.

**Black-box acceptance:** insertion, removal, and reorder preserve or replace
public host IDs exactly as specified; empty positions retain later unkeyed
identity; duplicate same-typed keys panic before commit.

**Visual evidence:** runtime-only ID journal; the interactive Composition
reorder is added after events exist.

### Task 27 — Reconcile host creation, properties, and removal [DONE]

**Prerequisites:** Task 26. **Target:** 200–250 non-test lines.

Implement host-kind replacement, maximal subtree creation, sparse property
diffing with `Prop<T>` resets, child-first destruction, and deterministic no-op
elimination.

**Black-box acceptance:** desired public fake trees match after add, change,
reset, remove, and host-kind replacement; identical rerender has an empty
journal; failed validation commits nothing.

**Visual evidence:** runtime-only public tree and command journal.

### Task 28 — Reconcile physical moves and portal-ready ranges [DONE]

**Prerequisites:** Task 27. **Target:** 200–250 non-test lines.

Implement longest-increasing-subsequence move selection, deterministic ties,
reparenting, flattened component/fragment host ranges, and physical-parent
child sequences suitable for later portals.

**Black-box acceptance:** randomized small trees match a desired-tree oracle;
move journals are minimal under the specified tie-break; zero-host and
multi-host logical children retain correct identity.

**Visual evidence:** runtime-only reorder, grouping, and restored journals.

### Task 29 — Complete commit ordering, receipts, and response helpers [DONE]

**Prerequisites:** Task 28. **Target:** 200–250 non-test lines.

Lower semantic mutations into deterministic dependency groups. Complete
`ReactantCommit`, its delivery receipt, `into_groups`, `into_batch`, and
`ResponseReactantExt`. An unconsumed nonempty commit or reentry with an
outstanding receipt panics; only an abandoned render transaction is retryable.

**Black-box acceptance:** dependent mutations retain barriers, independent
mutations share groups, every consumption path acknowledges the exact receipt,
and dropping a nonempty commit panics without describing it as a safe retry.

**Visual evidence:** runtime-only grouped journal and receipt diagnostics.

## Wave 4: events, hooks, and reactive state

### Task 30 — Add basic typed handlers and recognized dispatch [DONE]

**Prerequisites:** Task 29. **Target:** 200–250 non-test lines.

Implement one payload-free/event-aware handler slot per event kind and phase,
model-type validation, `ReactantEvent`, basic click subscription, callback
replacement, and recognized event dispatch. Event methods occupy the canonical
builder position between children and terminal adapters.

**Black-box acceptance:** a click reaches the last callback written through
either builder form and changes visible fake UI; replacing only the callback
emits no subscription command; unknown, unmounted, and unsubscribed targets
render only already-dirty work.

**Visual evidence:** Composition initial, reordered, and restored interaction
through the first verified WebGL event path.

### Task 31 — Complete primitive event builders and subscriptions [DONE]

**Prerequisites:** Task 30. **Target:** 200–300 non-test lines.

Add every approved payload-free and event-aware builder, capture availability,
control-specific `on_change` mapping, target-only subscriptions, and minimal
physical-island coverage. Reject authored native subscriptions on
Reactant-owned primitives.

**Black-box acceptance:** a structural event ledger maps every builder to its
typed payload, phase, control, and native subscription; same-slot forms replace
one another; unsupported capture forms do not compile; authored conflicts panic
transactionally.

**Visual evidence:** runtime-only subscription and typed-event journal.

### Task 32 — Add logical propagation and pointer crossing [DONE]

**Prerequisites:** Task 31. **Target:** 200–250 non-test lines.

Implement capture, target, and bubble paths, stop propagation, logical current
targets, focus bubbling, and complementary pointer over/out synthesis into
enter/leave. Any intervening event clears the crossing pair.

**Black-box acceptance:** public sequences prove path order, stopping, nested
targets, sibling and ancestor crossings, document exit, related targets, and no
false deduplication.

**Visual evidence:** add Events & Portals with a reversible non-portal event
interaction and verified WebGL behavior.

### Task 33 — Add hook context and state queues [DONE]

**Prerequisites:** Task 32. **Target:** 200–250 non-test lines.

Implement positional slots, `use_state`, `use_state_with`, stable
`StateSetter`, ordered replacement/updater queues, event batching, render-phase
updates and their retry limit, unmounted no-ops, and forbidden-context checks.

**Black-box acceptance:** clicks queue several updates but commit one final
visible value; lazy initialization is stable; keyed identity preserves state;
hook count, kind, type, cross-component render updates, and retry overflow panic
without a partial commit and poison later entries.

**Visual evidence:** add State & Identity; capture initial, updated, reordered,
and restored states.

### Task 34 — Add reducers and identity-driven reset [DONE]

**Prerequisites:** Task 33. **Target:** 200–250 non-test lines.

Implement `use_reducer`, `use_reducer_with`, stable `ReducerDispatch`,
current-render reducer closures, ordered action queues, batching, and remount
reset behavior. Reducers remain pure and hook-forbidden.

**Black-box acceptance:** public clicks produce one visible reduced state;
queued actions use the current render's reducer; reducer panic poisons; remount
resets state while keyed reorder does not.

**Visual evidence:** State & Identity reducer initial, changed, and restored.

### Task 35 — Add arbitrary refs and both context forms [DONE]

**Prerequisites:** Task 34. **Target:** 200–250 non-test lines.

Implement `use_ref`, `use_ref_with`, stable `Ref<T>` access, render-time access
rejection, `Context`, `RequiredContext`, providers, nearest lookup, and stable
nonzero static identity. Evaluate each default once per runtime.

**Black-box acceptance:** ref mutation in callbacks survives without scheduling
a render, every ref value operation during render panics, nested providers
affect only descendants, a missing required provider panics, and separate
same-typed contexts never alias.

**Visual evidence:** add Context & Memo and capture outer, overridden, and
restored themes.

### Task 36 — Add dependencies, memo values, callbacks, and bailout [DONE]

**Prerequisites:** Task 35. **Target:** 200–250 non-test lines.

Implement `Dependencies`, `use_memo`, `use_callback`, callback identity, and
`memo` component bailout with context and descendant-dirty invalidation. Memo
calculations are pure, hook-forbidden, and transactional.

**Black-box acceptance:** unrelated parent changes do not touch a memoized fake
subtree; props, dependency, context, and local work update it; callback identity
follows dependencies; panic poisons.

**Visual evidence:** Context & Memo initial, unrelated update, context update,
and restored captures.

### Task 37 [DONE] — Add passive effect variants and cleanup ordering

**Prerequisites:** Task 36. **Target:** 200–250 non-test lines.

Implement `use_effect`, `use_effect_always`, sealed cleanup conversion,
dependency replacement, child-before-parent cleanup, later-entry setup,
unmount cleanup, and synchronous shutdown cleanup. State queued by an effect
joins the next eligible render.

**Black-box acceptance:** public model and fake UI prove commit-before-effect
timing, `()` versus always semantics, cleanup/setup ordering, unmount,
reconnect deferral to the following non-session entry, and shutdown; effect
panic poisons without a partial commit.

**Visual evidence:** add Effects & Stores and capture disconnected, connected,
and restored states.

### Task 38 [DONE] — Add external stores and safe source swaps

**Prerequisites:** Task 37. **Target:** 200–250 non-test lines.

Implement `ExternalStore`, `use_external_store`, `StoreNotify`, `Subscription`,
snapshot reads, commit-time subscription, immediate recheck, retry limit, wake
coalescing, and overlapping source swaps with generation-safe retirement.

**Black-box acceptance:** a deliberately racy public store cannot miss an
update between render and subscribe; several wakes coalesce; stale old-source
wakes are ignored; unchanged sources reuse one subscription; every active entry
can consume a wake; retry exhaustion panics and poisons.

**Visual evidence:** Effects & Stores source swap, update, and restored captures
through verified WebGL behavior.

### Task 39 — Close hook scheduling and transactional failure coverage [DONE]

**Prerequisites:** Task 38. **Target:** 150–225 non-test lines.

Exercise every hook update source across `dispatch`, `refresh`, `poll`, and
`begin_session`; pin frozen-work acknowledgement, external-store stabilization
retries, memo dirty propagation, unconsumed-session poisoning, and callback
poisoning before portals and resources add more structural outcomes.

**Black-box acceptance:** one table-driven public suite covers every implemented
hook against each eligible entry; store recheck retries apply queued work once;
successful commits and session conversions acknowledge frozen work; an
unconsumed session and callback failures poison without a partial host commit.

**Visual evidence:** runtime-only lifecycle matrix and affected restored sample
captures.

## Wave 5: portals and recoverable render errors

### Task 40 — Add internal portals and logical ancestry [DONE]

**Prerequisites:** Task 39. **Target:** 200–250 non-test lines.

Implement `PortalTarget`, the terminal `.portal_target` adapter,
`create_portal`, one attached target per internal host, logical/physical
ancestry separation, source-ordered target ranges, context flow, and
event-island coverage across registered roots. Compile tests reject property,
child, or event methods after the adapter.

**Black-box acceptance:** fake Unity proves physical placement and logical
propagation simultaneously; same-target portals from several roots retain one
deterministic sequence; missing, duplicate, and cross-runtime targets fail
before mutation; target or key changes remount.

**Visual evidence:** Events & Portals inline card, portaled overlay, event
response, and restored captures through WebGL.

### Task 41 — Add external portals and reconnect rebinding [DONE]

**Prerequisites:** Task 40. **Target:** 200–250 non-test lines.

Implement external-container registration, caller-owned child prefixes,
post-snapshot portal commands, staged reconnect rebind, alias validation, and
outermost external event-island coverage.

**Black-box acceptance:** external commands validate against the supplied
snapshot; prefix children remain unchanged; missing and aliased targets fail
before mutation; a staged rebind applies only with successful session
conversion and preserves logical state.

**Visual evidence:** external-target runtime journal plus the unchanged
Events & Portals round trip.

### Task 42 — Add fallible rendering and error boundaries [DONE]

**Prerequisites:** Task 41. **Target:** 250–350 non-test lines.

Error traversal, latching, and post-commit reporting form one public
transaction.

Implement `Result<R, E>` render conversion, `RenderError`, nearest
`ErrorBoundary`, typed matching, latched fallback, `reset_on`, model-aware
`on_error`, and fallback escalation. Panics remain uncaught and poison. A
changed reset dependency type clears the latch without remounting the boundary;
several reports run in depth-first logical left-to-right catch order.

**Black-box acceptance:** nested boundaries show the nearest fallback; concrete
and boxed errors downcast correctly; value and dependency-type resets mount a
fresh primary; fallback errors reach the outer boundary; sibling reports mutate
the model in catch order; an escaped root error leaves Unity unchanged and a
corrected retry succeeds.

**Visual evidence:** add Resources & Boundaries error, reset, and restored
captures. Runtime evidence separately proves root `Err` retry.

## Wave 6: resources and Suspense

### Task 43 — Add resource identity, spawner, and cache entries [DONE]

**Prerequisites:** Task 42. **Target:** 200–250 non-test lines.

Implement `Resource::new`, `Resource::try_new`, process-unique identity, erased
typed buckets, runtime-wide task generations, resource use of the stored
`Spawner` and `SpawnedTask`, cross-thread completion queuing, and cache sharing
across roots.

**Black-box acceptance:** a deterministic executor proves one task per
resource/key generation, root sharing, synchronous completion queuing, stale
completion suppression, and current task panic delivery on the engine thread.

**Visual evidence:** runtime-only request/completion transcript.

### Task 44 — Add resource administration and cancellation [DONE]

**Prerequisites:** Task 43. **Target:** 200–250 non-test lines.

Implement `preload`, `invalidate`, `clear`, ready/failed retention, generation
replacement, best-effort cancellation, cancellation-panic handling, and
runtime destruction cleanup. Consumer registration and dirtying arrive with
resource reads in Task 45.

**Black-box acceptance:** administration is idempotent where specified;
invalidation and clear cannot accept stale results; every cancellation runs at
most once; administration with no mounted read changes no root.

**Visual evidence:** runtime-only administration, cancellation, and stale-task
journal.

### Task 45 — Add resource reads and initial Suspense [DONE]

**Prerequisites:** Task 44. **Target:** 200–300 non-test lines.

Implement `use_resource`, `ResourceRead::status`, `.then`, pending-token
collection, initial `Suspense` fallback, nested boundaries, status consumer
registration for pending, ready, and failed entries, and failed-resource
propagation. The hook and `.then` closure obey their forbidden contexts.
`RenderError::downcast_ref::<E>()` exposes `E`, never the private `Arc<E>`.
Missing Suspense panics and poisons.

**Black-box acceptance:** sibling reads start without waterfalls; shared reads
start one task; status consumers wake; failed reads reach the nearest error
boundary and downcast to `E`; missing boundaries and hook misuse poison without
partial commit. Status consumers register for pending, ready, and failed state;
invalidation dirties ready/failed consumers through enclosing memo boundaries.
Pending completion likewise defeats memo bailout for status consumers and
boundary retries.

**Visual evidence:** Resources & Boundaries pending capture through the manual
executor and verified WebGL link.

### Task 46 — Retain suspended trees and retry coherently [DONE]

**Prerequisites:** Task 45. **Target:** 200–300 non-test lines.

Preserve committed primary state and host identity while hidden, retain and
replace boundary waiter sets, retry dirty primary work transactionally, reveal
on the first successful entry, and preserve resource/cache semantics through
reconnect. Do not add transitions, deferred values, or timed reveal batching.

**Black-box acceptance:** initial and repeated suspension, hidden event
rejection, stale completion, key change, invalidation, unmount, reconnect,
queued state, and recovery all end in observable fake UI facts without an
intermediate partial tree.

**Visual evidence:** Resources & Boundaries initial, pending, ready, error, and
restored captures driven deterministically in WebGL.

## Wave 7: refs, geometry, and release proof

### Task 47 — Add element refs and queued host actions [DONE]

**Prerequisites:** Task 46. **Target:** 200–250 non-test lines.

Implement `use_element_ref`, attachment generations, the terminal
`.element_ref` adapter, committed attachment queries, supported one-shot host
actions, and action validation. Components and structural render values cannot
receive an element ref; compile tests reject earlier builder categories after
the adapter.

**Black-box acceptance:** attach, keyed move, remount, detach, reconnect, stale
action, duplicate attachment, render-time action, and cross-runtime action all
produce the specified public fake state or transactional panic.
`is_attached` panics during render and remains available from callbacks.

**Visual evidence:** add Refs & Geometry screen with focus/select action,
changed focus state, and restored state through WebGL.

### Task 48 — Add geometry targets, registry diffs, and base snapshots [DONE]

**Prerequisites:** Task 47. **Target:** 250–350 non-test lines. Target-set
registration needs one public hook and observable snapshot to be testable.

Implement element, viewport, and world refs, their exact identity, sealed
target shapes, tuples, arrays, vectors, deduplication, observation epochs, and
registry add/remove ordering. Add `Measurement`, `use_geometry`, base coherent
`GeometrySnapshot` values, complete-generation gating, and status changes. Only
`ElementRef` is runtime-owned.

**Black-box acceptance:** public tests cover target add, remove, reorder,
duplicates, equal reconstructed targets, and ref reattachment through
`use_geometry`; one render observes only complete current-session generations
and never inspects private registry storage.

**Visual evidence:** runtime-only registry and command-order journal.

### Task 49 — Complete geometry retention, cache reads, and conversions [DONE]

**Prerequisites:** Task 48. **Target:** 200–250 non-test lines.

Implement unavailable values, retained latest values, `ElementRef::geometry`,
same-display coordinate conversions, reconnect invalidation, and memo dirty
propagation. Reconstructed world and viewport refs have no direct cache read.

**Black-box acceptance:** public batches cover unchanged advancement,
unavailable values, retained stale values, reconnect invalidation, coherent
tuple/vector output, and round-trip or rejected cross-display conversions.
`ElementRef::geometry` panics during render, and changed geometry defeats an
otherwise eligible enclosing memo bailout. Reconnect replaces the registry,
retires the old epoch, retains last samples as waiting, and rejects stale old-
epoch observations.

**Visual evidence:** Refs & Geometry screen-space initial, moved, unavailable,
and restored captures using snapshot-derived placement.

### Task 50 — Add geometry effects and world projection sample [DONE]

**Prerequisites:** Task 49. **Target:** 200–250 non-test lines.

Implement `use_geometry_effect`, cleanup/setup ordering, coherent target
snapshots, dependency replacement, and model-aware callbacks. Complete the
sample screen with a projected world origin or anchor and a world-bounds
specimen using the shared host protocol.

**Black-box acceptance:** callbacks run only for coherent generations or
dependency change, clean up child-before-parent, queue state correctly, render
all roots after model mutation, and poison on panic.

**Visual evidence:** Refs & Geometry captures for screen-space UI, world point,
world bounds, unavailable state, and restored placement in WebGL.

### Task 51 — Harden reconnect, shutdown, and failures [DONE]

**Prerequisites:** Task 50. **Target:** 200–300 non-test lines.

Close lifecycle gaps across all entry points: reconnect retention and geometry
invalidation, staged portal rebind, failed session renders, unconsumed sessions,
empty successful commits, synchronous shutdown cleanup, dropped-runtime
diagnostics, explicit root errors, missing Suspense, and panic poisoning.

**Black-box acceptance:** one table-driven public suite exercises every runtime
state/entry-point combination; no failed entry emits a partial commit; explicit
errors retry; pre-entry guard panics do not poison; transaction and must-use
delivery panics do; failed shutdown returns no destruction commit and leaves a
poisoned runtime; frozen work is acknowledged only by a successful commit or
session conversion.

**Visual evidence:** runtime-only lifecycle matrix output. Recapture any sample
state affected by fixes.

### Task 52 — Expand randomized reconciliation and performance evidence

**Prerequisites:** Task 51. **Target:** 150–225 non-test lines.

Expand randomized desired-tree coverage across structural values, keys,
properties, moves, portals, suspension retention, and caught errors. Record
stable representative command counts and timings without making them public
performance guarantees.

**Black-box acceptance:** deterministic seeds produce final fake trees equal to
the simple oracle; no-op and reorder journals stay bounded; failures preserve
the last committed tree; every seed is printed for reproduction.

**Visual evidence:** runtime-only randomized seed log and command-count
baseline.

### Task 53 — Complete the sample, documentation, and release evidence

**Prerequisites:** Task 52. **Target:** 150–250 non-test lines.

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
