# Reactant Focus and Navigation Implementation Plan

This plan delivers the contract in the normative
[Reactant focus and navigation design](focus-and-navigation.md). The design
wins if this plan appears to disagree with it. Each task below leaves the
repository compiling and proves a user-observable boundary before the next
task begins.

This plan does not relax Reactant's central ownership rule. Unity UI Toolkit
remains the low-level focus engine. Rust authors policy, Unity installs that
policy before input begins, and Unity reports the result after native focus has
settled. No task may introduce a Rust round trip into an input event.

## Related information

- [Reactant technical design](reactant-technical-design.md) defines the runtime,
  host façade, hook, reconciliation, event, portal, ref, and Motion contracts.
- [Battlement UI technical design](../battlement-ui-technical-design.md) defines
  the shared snapshot, command, event, receipt, and reconnect transport.
- [Ditto technical design](../ditto-technical-design.md) defines scenario
  authoring, production input injection, observations, and reports.
- [Reactant implementation plan](reactant-implementation-plan.md) records the
  completed foundation on which these tasks build.
- [Feature ledger](feature-ledger.md) maps shipped Reactant modules to their
  sample and black-box proof.
- [Unity focus order][unity-focus-order], [navigation events][unity-navigation],
  and [runtime input][unity-runtime-input] define the native baseline.

[unity-focus-order]: https://docs.unity3d.com/6000.0/Documentation/Manual/UIE-focus-order.html
[unity-navigation]: https://docs.unity3d.com/6000.0/Documentation/Manual/UIE-Navigation-Events.html
[unity-runtime-input]: https://docs.unity3d.com/6000.0/Documentation/Manual/UIE-faq-event-and-input-system.html

## Delivery rules

Every task is an independently reviewable change. A task includes its public
documentation, shared fixtures, diagnostics, and black-box tests. Authoring and
wire tasks may land before later interaction tasks, but the feature remains
explicitly unreleased and absent from the sample and feature ledger until all
native behavior for that API is present. No release may cut between those
dependencies.

Implementation work follows these rules:

- Keep native `FocusController`, `focusedElement`, focus ring, focus events,
  control default actions, and navigation events authoritative.
- Install synchronous decisions in Unity before enabling input. Rust callbacks
  may observe outcomes but cannot retroactively prevent native defaults.
- Use stable `ObjectId` and desired-tree `ElementRef` identity. Never derive
  focus identity from render position or native instance IDs.
- Treat invalid scope, group, relationship, and panel topology as developer
  errors. Do not recover by creating a second focus engine.
- Add no protocol version, compatibility adapter, legacy branch, or migration
  path.
- Prefer public black-box tests. Unity tests inspect actual UI Toolkit focus,
  event order, control state, geometry, and scroll offsets.
- Keep source files near 500 lines. Split by cohesive responsibility before a
  file approaches 1,000 lines.
- Stage all intended changes before running `./scripts/ci.py`.

Tasks that alter the Unity host run its relevant EditMode suite. Tasks that
alter public Rust APIs run Reactant doctests and build documentation with
warnings denied. A task is not complete when only private coordinator state
has the expected value.

## Evidence model

Each task records four kinds of evidence when applicable:

1. A Rust black-box test showing the public authoring or transport result.
2. A Unity EditMode test showing native focus or navigation behavior.
3. A Ditto scenario step showing the behavior through production input.
4. A measurement showing payload, allocation, latency, or reconnect cost.

Early infrastructure tasks may use a public fake-client journal instead of a
Ditto scenario. Once the sample can render the behavior, later tasks must use
the real specimen and production input path.

Performance evidence uses release-like builds with diagnostics disabled. It
must distinguish cold plan installation from steady-state input. Allocation
and latency measurements name their Unity version, Input System version,
platform, panel size, and candidate count.

## Dependency overview

| Phase | Tasks | Observable result |
|---|---|---|
| 1. Protocol foundation | F01-F03 | Complete plans, sparse updates, and reports round-trip |
| 2. Unity coordinator | F04-F06 | Native focus survives commits and falls back predictably |
| 3. Reactant authoring | F07-F09 | Public focus APIs lower, validate, queue, and reconnect |
| 4. Scopes and overlays | F10-F12 | Portalled scopes, nesting, exclusion, and exits work |
| 5. Navigation behavior | F13-F15 | Tab, directional input, scrolling, and modality work |
| 6. Roving composites | F16-F18 | Composite focus and accessibility presets work |
| 7. Release proof | F19-F21 | Sample, Ditto, and measured release evidence are complete |

The dependency graph is mostly linear. Within a phase, tasks may be developed
in parallel only when their prerequisites are present and their fixtures do
not encode unfinished behavior.

## Phase 1: protocol foundation

This phase creates the complete transport vocabulary and fake-host model before
Reactant exposes authoring APIs. It makes malformed and stale focus policy
observable without requiring Unity behavior.

### Task F01 - Add the complete focus wire contract

**Prerequisites:** Existing Battlement snapshots, ordered command batches,
events, `ObjectId`, and Rust/C# fixture generation.

**API slice:**

- Add `UiFocusPlan`, `UiFocusScope`, `UiFocusNode`, `UiRovingGroup`, and their
  supporting enums and relationship records.
- Add `UiFocusPlanUpdate`, `UiFocusPlanChange`, `UiFocusState`,
  `UiFocusSnapshot`, `UiFocusResume`, `UiFocusRequest`, request results,
  panel identity, roving positions, acknowledgements, `InputModality`, and
  `FocusReason`.
- Extend focus-event payloads with modality and reason while preserving native
  related-target and navigation-direction fields.
- Add the typed accessibility role, state, and relationship fields required by
  the normative design.
- Embed one complete plan in a UI snapshot and sparse updates in live commit
  batches.

**Implementation notes:** Keep the schema closed and generated like the
existing UI protocol. Define hard limits for plan nodes, scopes, groups,
neighbors, and accessibility relationships. Reject duplicate IDs and dangling
hard references during decode or validation. Preserve dangling initial,
fallback, and neighbor soft references for their normative live fallback. Add
no protocol version field.

**Rust proof:** Round-trip every enum and optional field, reject each invalid
reference class, prove omission when no focus metadata or resume state exists,
and prove explicit plan presence when metadata exists without a focusable host.

**Unity proof:** Deserialize the shared fixtures and verify exact field values,
including absent versus explicit `None` policy.

**Ditto proof:** Not yet applicable. Store fixture names in the ordinary shared
protocol report.

**Performance proof:** Record encoded byte size for empty, 100-node, and
10,000-node plans and for one-node sparse updates.

**Completion condition:** Rust and C# consume the same fixtures, all hard-limit
failures are named diagnostics, and a plan can travel through a snapshot and a
live commit without affecting focus.

### Task F02 - Model focus policy in the public fake client

**Prerequisites:** F01.

**API slice:** Add fake-client state for the active plan generation, focus
state, logical policy reports, and a journal of plan installation and focus
requests. Add panel-keyed public observations for focused object, scope stack,
modality, focus-visible state, reason, request results, and effective roving
positions.

**Implementation notes:** The fake proves transport and Reactant lowering. It
must not pretend to implement UI Toolkit geometry or control default actions.
For unsupported native decisions it records the command and accepts an
explicit test-supplied focus result.

**Rust proof:** Install a snapshot plan, apply a sparse update, reject a stale
generation, record a supplied native result, and coalesce duplicate state
reports.

**Unity proof:** Not applicable.

**Ditto proof:** Not applicable. The fake command journal is the reviewable
artifact.

**Performance proof:** Prove one sparse change does not clone or reserialize
the complete plan in the fake transport.

**Completion condition:** A black-box client test can distinguish desired
policy, the action sent to Unity, and the actual state reported by Unity.

### Task F03 - Stage focus plans transactionally with host mutations

**Prerequisites:** F01-F02 and the existing ordered batch transaction model.

**API slice:** Add focus-plan snapshot and update placement to the shared batch
coordinator. Add generation acknowledgement and batch-failure diagnostics.

**Implementation notes:** A focus update belongs to the same atomic UI commit
as the structural and property changes it describes. Snapshot reconstruction
installs one complete generation. Live commits carry only generation-checked
changes. A preflight-rejected batch leaves the previous plan and focus active.
An unexpected post-mutation Unity failure leaves input disabled, invalidates
the session, and requests a full snapshot without claiming native rollback.

**Rust proof:** Journal ordering shows structure, properties, focus policy,
focus finalization, and input exposure in that order. Stale reports are ignored
without mutating the reconnect bookmark.

**Unity proof:** A preflight-invalid mutation leaves the preceding plan and
focused element intact. An injected post-mutation exception keeps input gated,
invalidates the session, suppresses partial events, and requests a resnapshot.

**Ditto proof:** Not yet applicable.

**Performance proof:** An unrelated visual-property commit emits no focus-plan
traffic.

**Completion condition:** Focus policy is transactionally coupled to the host
tree it references, and failure cannot expose input against a half-installed
plan.

## Phase 2: Unity focus coordinator

This phase installs a panel-local coordinator around UI Toolkit. It preserves
native behavior and adds only the policy UI Toolkit cannot infer by itself.

### Task F04 - Install panel-local plans and report native focus

**Prerequisites:** Phase 1.

**API slice:** Add the Unity focus coordinator, O(1) object/scope/group lookup
tables, plan validation, panel ownership, input gating, state-report
coalescing, coordinator diagnostics, and Ditto's public `focused` object-state
observation.

**Implementation notes:** Register native focus and navigation callbacks on the
panel without synthesizing focus events. Read `FocusController.focusedElement`
after native dispatch settles. The coordinator owns modality, scope stack,
roving position, opener stack, and reconnect bookmark only as Unity-local
ephemeral state. Add the one startup-bound `NativeFocusRingAdapter`; fail host
startup when its baseline binding or event-order conformance check fails.

**Rust proof:** The fake client accepts final focus reports independently of
application event subscriptions.

**Unity proof:** Focusing a `TextField`, `Button`, and `Toggle` reports their
actual IDs, native related targets, modality, and reason. Removing all Rust
focus handlers does not suppress the policy report. The conformance fixture
proves target default action, root bubble coordination, later default action,
native focus events, and settled report ordering.

**Ditto proof:** A host fixture focuses an ordinary button and Ditto observes
`focused` without invoking a coordinator command.

**Performance proof:** Repeated focus changes allocate no managed memory after
warm-up and produce no report when state is unchanged.

**Completion condition:** Unity is the sole source of truth for the reported
focused object, and installing an inert plan does not alter ordinary control
behavior.

### Task F05 - Preserve focus and resolve deterministic commit fallback

**Prerequisites:** F04 and transactional staging from F03.

**API slice:** Implement batch capture and finalization, retained keyed-host
focus, explicit fallback, next/previous pre-mutation ring fallback, scope
anchor fallback, and final clear behavior.

**Implementation notes:** Capture old focus and native ring order before any
mutation. Apply all host work with input disabled. If the keyed native element
survives, restore it before exposing the commit when native reparenting caused
transient focus loss. Otherwise apply the normative fallback ladder exactly
once. Report only the settled outcome.

**Rust proof:** A keyed move lowers to retained host identity, while replacement
lowers to a new host and an explicit fallback request.

**Unity proof:** Move a focused keyed control, remove the focused middle member,
remove the last member, invalidate an explicit fallback, and assert actual
`focusedElement`, native event order, and next/previous choice.

**Ditto proof:** Not yet applicable.

**Performance proof:** Finalization scans at most the captured ring and does
not rebuild rings for unaffected panels.

**Completion condition:** No committed frame exposes an avoidable focus loss,
and every focused-node removal has one deterministic, tested outcome.

### Task F06 - Preserve native control defaults and event precedence

**Prerequisites:** F04-F05.

**API slice:** Add coordinator handling for unconsumed navigation events, the
rule that `PreventDefault` is called only when a predeclared policy move is
actually applied, and Ditto's controller-navigation step through Unity's
virtual Input System.

**Implementation notes:** Native text editing, selection, submit, cancel,
toggle, and control-specific directional behavior receive first refusal. Do
not add general key handlers to controls. Rust handler results never change the
already-completed native default.

**Rust proof:** Event types expose observation but no browser-style synchronous
`prevent_default` API.

**Unity proof:** Arrow keys edit or move within text and native composite
controls before generic navigation. Enter, Space, submit, and cancel retain
native behavior. The conformance fixture covers bubble `KeyDownEvent` for Tab,
Shift+Tab, Home, and End plus bubble `NavigationMoveEvent` for directional
input. An unconsumed event with no policy remains untouched.

**Ditto proof:** A minimal production-input scenario edits a field and toggles
a control without coordinator-specific calls.

**Performance proof:** The no-policy path performs O(1) lookup and no
allocation.

**Completion condition:** Ordinary UI Toolkit controls behave identically with
the coordinator installed and with an empty focus plan.

## Phase 3: Reactant focus authoring

This phase exposes declarative focus policy only after the transport and native
engine can honor it. It also connects focus state to sessions and queued refs.

### Task F07 - Add focus, scope, inert, and accessibility authoring

**Prerequisites:** Phase 2 and the existing opaque host-façade builders.

**API slice:** Add `FocusScopeMode`, `FocusContainment`, `FocusRestore`,
`FocusScope`, and the typed accessibility semantics from the design. Add façade
builders for `focus_scope`, `auto_focus`, and `inert`.

**Implementation notes:** Lower authoring through the desired tree into the
complete plan. Preserve native positive, zero, and negative `tab_index`
semantics. Do not invent a Rust numeric-ordering algorithm. Relationship
builders resolve stable `ElementRef` identities after the desired tree exists.

**Rust proof:** Public black-box tests cover every builder in arbitrary call
order, absent/default fields, accessibility relationships, and exact plan
lowering. Compile-fail tests reject builders on unsupported façades.

**Unity proof:** A lowered ordinary form follows the same native focus ring as
an equivalent hand-authored UI Toolkit form.

**Ditto proof:** Not yet required; the form slice may still use a focused host
fixture.

**Performance proof:** Unchanged subtrees reuse lowered policy, and an unrelated
render emits no focus update.

**Completion condition:** Authors can fully declare ordinary focusability,
initial focus, one scope, inertness, and relationships without accessing wire
types.

### Task F08 - Validate desired-tree focus topology

**Prerequisites:** F07 and existing portal binding validation.

**API slice:** Add validation for duplicate scope/group membership, incompatible
nested groups, multiple eligible `auto_focus` descendants, dangling refs,
invalid active items, and cross-panel scopes or groups.

**Implementation notes:** Logical ancestry determines scope and group
membership, including same-panel portals. Physical panel ownership determines
whether that topology is legal. Developer errors must identify all involved
logical IDs and panel targets without leaking native instance IDs.

**Rust proof:** Validate same-panel portals, reject cross-panel portals,
duplicate membership, multiple `auto_focus` nodes, a scope with no possible
anchor, a modal anchor without authored `focusable(true)`, and relationships
to destroyed refs.

**Unity proof:** Reject a malformed externally supplied plan before it can
alter active focus.

**Ditto proof:** Not applicable; invalid authoring is a build/runtime developer
failure rather than player behavior.

**Performance proof:** Validation is linear in changed policy and reuses panel
and ancestry indexes from reconciliation.

**Completion condition:** Every foundational topology invariant fails before
Unity input can observe an invalid plan.

### Task F09 - Add queued focus actions and reconnect bookmarks

**Prerequisites:** F07-F08 and existing `ElementRef` queued actions, session
receipts, shutdown, and reconnect lifecycle.

**API slice:** Preserve `ElementRef::focus()` and `blur()`. Add
`FocusVisibility`, `FocusScroll`, `FocusOptions`,
`focus_with(FocusOptions)`, queued programmatic intent, request-result
reporting, focus-state receipts, and per-session focus bookmarks.

**Implementation notes:** Queue focus for the next eligible commit. Multiple
requests in one entry use documented last-writer behavior. Unity determines the
actual result and reports it. Shutdown clears native-only handles but retains
the last acknowledged per-panel logical bookmarks and session modality needed
for reconnect.

**Rust proof:** Queue before attachment, replace an earlier request, reject a
stale result, distinguish `NotFocused` and `NativeRejected`, preserve current
per-panel bookmarks through shutdown, and rebuild after portal bindings exist.

**Unity proof:** Programmatic focus honors explicit visibility and scroll
intent, rejects ineligible targets, and reports the actual fallback.

**Ditto proof:** A reconnect fixture requests focus, disconnects, rebuilds, and
observes the restored target without private coordinator access.

**Performance proof:** No polling or per-frame bookmark traffic occurs.

**Completion condition:** Programmatic focus and reconnect use acknowledged
Unity outcomes rather than optimistic Rust state.

## Phase 4: scopes, portals, overlays, and Motion

This phase adds containment and restoration while preserving logical Reactant
event ancestry and physical Unity stacking.

### Task F10 - Implement modal and non-modal scope lifecycle

**Prerequisites:** Phase 3.

**API slice:** Implement initial-focus selection, `NonModal` and `Modal`
activation, `None`/`Trap`/`Loop` containment, scope anchors, outside exclusion,
and `None`/`Opener` restoration.

**Implementation notes:** Initial focus tries retained focus, explicit target,
one eligible `auto_focus`, first native ring member, and the focusable anchor.
Modal defaults trap and loop sequential navigation, contain directional input,
exclude outside picking and accessibility, and restore the opener. Non-modal
scopes do none of those by default and restore only when focus remains inside
at close. Use the startup-bound native focus-ring adapter and root bubble event
ordering defined by the design. Reject a modal whose prospective anchor cannot
provide the mandatory live fallback.

**Rust proof:** Lower all default and explicit combinations and validate that
outside exclusion is modal-only.

**Unity proof:** Open and close modal and non-modal fixtures; assert initial
focus, actual ring traversal, picking, accessibility exclusion, and restoration.

**Ditto proof:** Production pointer and keyboard input opens and closes one
modal, loops at both Tab boundaries, and cannot activate outside content.

**Performance proof:** Activating a scope updates affected lookup state without
rebuilding unrelated panels.

**Completion condition:** One scope behaves completely according to its mode,
containment, and restoration settings through public input.

### Task F11 - Integrate portals, stacking, and nested overlays

**Prerequisites:** F10 and existing portal/stacking contracts.

**API slice:** Implement the Dormant, Available, Occluded, and Active scope
states, derive the active modal from the complete physical stacking key,
maintain a native LIFO opener stack, reactivate outer scopes, and apply logical
ancestry through same-panel portals.

**Implementation notes:** Portal placement changes physical sequential order
but never Reactant event ancestry. Nested modal and non-modal overlays may
interleave. Reject cross-panel or physically interleaved modal fragments. When
an inner opener disappears, use deterministic fallback in the now-active outer
scope. Never restore through an inactive higher modal.

**Rust proof:** Logical focus-in/out routes cross the portal boundary through
the committed logical tree. Scope membership survives keyed physical moves.

**Unity proof:** Open an outer modal, an inner portalled modal, and a non-modal
overlay. Close them in multiple orders and invalidate the inner opener.

**Ditto proof:** A nested-overlay scenario observes scope activation and final
focus entirely through player-visible state.

**Performance proof:** Active-modal selection uses maintained stacking indexes,
not a full visual-tree scan on each input event.

**Completion condition:** Nested overlays restore or fall back within the
correct active physical stack while logical events retain Reactant ancestry.

### Task F12 - Exclude hidden, suspended, inert, and exiting content

**Prerequisites:** F10-F11, Suspense retained-content behavior, and Motion
presence lifecycle.

**API slice:** Unify eligibility for `display: none`, hidden visibility,
disabled hierarchy, Suspense-retained hidden content, explicit inertness, and
presence exits.

**Implementation notes:** Opacity alone remains eligible. At exit start, remove
logical membership from focus and navigation, resolve fallback, then begin
physical retention. Physical removal completion performs no focus restoration.
The same immediate rule applies when a focused subtree becomes inert or hidden.

**Rust proof:** Presence lowering marks exclusion in the same commit that starts
exit. Suspense and inert transitions use the shared eligibility model.

**Unity proof:** Focus an exiting item, start a retained animation, and assert
focus has already moved while the host remains visible. Completion emits no
second focus change.

**Ditto proof:** The player observes the animation and can prove the retained
host is no longer keyboard or pointer reachable.

**Performance proof:** Eligibility changes invalidate only affected scope,
group, and geometry indexes.

**Completion condition:** No logically hidden or exiting element can retain or
receive focus, and opacity-only animation remains unaffected.

## Phase 5: navigation and focus-visible behavior

This phase adds declarative overrides around native Tab and navigation events.
The no-policy path remains entirely native.

### Task F13 - Add scope-boundary Tab and explicit neighbors

**Prerequisites:** Phase 4 and native precedence from F06.

**API slice:** Add public `NavigationNeighbors` and the
`navigation_neighbors` façade builder, then implement explicit neighbors
resolved by stable desired-tree refs. Re-run the scope-boundary conformance
from F10 with the combined policy.

**Implementation notes:** Ordinary Tab and Shift+Tab stay in the native ring.
Only a declared trap or loop handles the boundary. Explicit directional
neighbors apply only when eligible in the governing scope. Missing or invalid
neighbors defer to native automatic navigation.

**Rust proof:** Lower ref identities through keyed moves, deletion, portals, and
replacement. A missing neighbor emits absence, not a positional guess.

**Unity proof:** Exercise both Tab boundaries, four directional neighbors,
disabled targets, and native-control consumption before neighbor handling.
Prove Next/Previous through the native ring adapter and root `KeyDownEvent`.

**Ditto proof:** Keyboard and controller steps traverse the same authored
neighbor graph where the device exposes equivalent directions.

**Performance proof:** Valid explicit moves use O(1) lookups and allocate
nothing after warm-up.

**Completion condition:** Declarative boundary and neighbor rules work without
changing ordinary native traversal.

### Task F14 - Add scope-filtered automatic directional navigation

**Prerequisites:** F13 and Unity layout notification infrastructure.

**API slice:** Cache physical spatial candidates after layout and add filtered
automatic fallback for governing scopes.

**Implementation notes:** Native automatic navigation wins when no Reactant
filter is needed. When containment excludes a native candidate, use the exact
clipped-world-bound tuple from the normative design: cross-axis overlap,
primary gap, cross-axis center distance, squared center distance, paint order,
then `ObjectId`. Apply the documented opposite-edge loop rule. Invalidate only
affected panel geometry and rebuild lazily on navigation.

**Rust proof:** Plans declare filtering policy but contain no computed geometry
or chosen automatic destination.

**Unity proof:** Use controlled rectangles to prove each tie-break, invalidation
after layout, bounded scans, and fallback to no move when no candidate exists.

**Ditto proof:** Controller navigation crosses an asymmetric specimen and stays
inside the active modal.

**Performance proof:** Record warm navigation latency at 100, 1,000, and the
hard-limit candidate count. No per-frame work occurs without dirty geometry.

**Completion condition:** Directional automatic movement is deterministic,
panel-local, and synchronous without asking Rust for geometry.

### Task F15 - Add modality, focus-visible Motion, and scroll reveal

**Prerequisites:** F13-F14 and existing Motion gestures and geometry hooks.

**API slice:** Add the `scroll_on_focus` façade builder, Unity-local modality
state, `while_focus_visible`, reported `focus_visible`, and
`FocusScroll::Nearest` through nested physical `ScrollView` ancestors. Add
Ditto's public `focus-visible` object-state observation.

**Implementation notes:** Pointer input hides focus-visible. Tab, keyboard
directional input, and controller navigation show it. Programmatic focus uses
`FocusVisibility`, with `Auto` following current modality. Reconnect retains the
last acknowledged modality. Apply the normative scroll matrix for keyboard,
controller, initial, restoration, fallback, reconnect, plain programmatic, and
explicit programmatic focus. Pointer focus does not auto-scroll. Cancel a
queued reveal when its object, focus sequence, or session becomes stale.

**Rust proof:** Motion lowering includes the focus-visible target without a
state render. Focus options encode visibility and scroll intent exactly.

**Unity proof:** Alternate pointer, keyboard, controller, and programmatic
focus. Assert style state without a Rust commit. Reveal a target through two
nested scroll views and inspect native offsets.

**Ditto proof:** Observe `focused` and `focus-visible` separately and prove the
pointer-to-keyboard transition plus nested scroll reveal.

**Performance proof:** Modality changes update affected style/motion state with
no managed allocation, plan traffic, or Rust render.

**Completion condition:** Focus-visible styling and reveal behavior follow the
documented modality heuristic through production input.

## Phase 6: roving composites and accessibility

This phase adds Unity-owned ephemeral position for composite widgets while
keeping application selection in Rust.

### Task F16 - Add generic roving group and item authoring

**Prerequisites:** Phase 5.

**API slice:** Add `RovingFocusGroup`, `RovingFocusItem`, `RovingKind`,
orientation, looping, activation, active-item seeding, and host-façade
builders.

**Implementation notes:** Require exactly one active eligible item whenever a
nonempty group mounts. Unity synchronously moves the active roving position and
effective `tabIndex`, then reports focus. Rust may update application selection
in the later response. Implement `seed_revision` so unrelated Rust renders
cannot overwrite a newer native position, and report changed positions for
unfocused groups until acknowledged. Reject duplicate, incompatible nested,
and cross-panel membership.

**Rust proof:** Lower group and item policy through keyed reorder, same-panel
portal placement, active-item removal, and disabled membership. Prove stale,
equal, and newer seed revisions, acknowledgement, deliberate `focus_with`
reset, and reconnect with a position outside the focused group.

**Unity proof:** Assert one sequential stop, synchronous effective `tabIndex`,
Home/End, orientation filtering, looping, disabled-item skipping, and removal
fallback. Prove `First`/`Last` selection-event directions and default
prevention only when the group handles the key.

**Ditto proof:** A generic specimen proves only one item is reached by Tab and
directional movement stays inside the group.

**Performance proof:** A roving move updates only the old and new effective
items and emits one coalesced state report.

**Completion condition:** Generic roving focus is synchronous, panel-local,
and independent of a Rust selection render.

### Task F17 - Add tabs, menus, toolbars, and listboxes

**Prerequisites:** F16 and typed accessibility semantics from F07.

**API slice:** Implement exact presets for tabs, menus, toolbars, and listboxes,
including roles, selected state, controls relationships, active descendant,
orientation, Home/End, looping, and optional automatic tab activation.

**Implementation notes:** Tabs default horizontal. Menus and listboxes default
vertical. Toolbars require authored orientation. Type-ahead remains outside
this design. Automatic tab activation reports an activation request after the
native focus move; Rust owns the selected panel content.

**Rust proof:** Validate each preset's derived role and dynamic active
descendant, manual versus automatic tabs, item removal, and relationship
resolution. Reject a tab without explicit `controls`, missing selected state,
and conflicts with derived semantics.

**Unity proof:** Exercise all keys and controller directions, inspect the
production `AccessibilityHierarchy` projection and retained typed
relationships, and prove manual tabs do not activate on focus.

**Ditto proof:** A tablist scenario covers arrows, Home/End, looping, automatic
activation, selected panel content, and one sequential stop.

**Performance proof:** A move in a 1,000-item listbox stays within the bounded
candidate scan and emits no complete-plan update.

**Completion condition:** Each preset has exact keyboard/controller behavior
and matching typed accessibility semantics.

### Task F18 - Integrate radio groups and selection reporting

**Prerequisites:** F16-F17.

**API slice:** Preserve native `RadioButtonGroup` behavior and add the composed
radio-item roving preset with checked semantics and selection requests.

**Implementation notes:** Native groups receive no duplicate roving key logic.
Composed groups use Unity-owned focus position and Rust-owned checked state.
Selection reports use the normative `UiRovingSelectionRequested` schema,
including plan generation, event sequence, previous and proposed items,
direction, modality, and reason.

**Rust proof:** Lower native and composed variants distinctly, validate one
checked and one active eligible item as separate authored facts, and ignore
stale selection reports by generation.

**Unity proof:** Compare native `RadioButtonGroup` interaction with the same
control outside Reactant. For composed radios, prove arrow movement, checked
request ordering, disabled skipping, and one Tab stop.

**Ditto proof:** Controller and keyboard select a composed radio while a native
radio fixture retains native behavior.

**Performance proof:** Selection reporting is coalesced with final focus state
when both arise from one native navigation event.

**Completion condition:** Native radios remain native, composed radios satisfy
the roving contract, and Rust receives an unambiguous selection request.

## Phase 7: sample, Ditto, and release proof

This phase turns the completed contract into durable teaching and release
evidence. It adds no new foundational focus semantics.

### Task F19 - Add the focused Reactant specimen

**Prerequisites:** Phases 1-6.

**API slice:** Add one sample screen or tightly related screen set covering an
ordinary form, modal and nested overlays, a same-panel portal, a tablist,
controller navigation, presence exit, nested scrolling, reconnect controls,
and pointer-versus-keyboard focus-visible styling.

**Implementation notes:** Follow the Reactant sample's existing visual language
and restoration rules. The specimen displays user-facing state, never object
IDs, plan generations, private coordinator fields, or command logs. Every
interaction returns to its exact initial state without navigating away.

**Rust proof:** Sample integration tests drive the public engine through the
fake client and verify initial, changed, and restored behavior.

**Unity proof:** Build and load the sample without sample-specific native focus
code.

**Ditto proof:** Reserved for F20.

**Performance proof:** Record the specimen's complete initial plan size and the
largest sparse update produced by an interaction.

**Completion condition:** A reviewer can exercise every acceptance family in
the sample using only public Reactant APIs and ordinary input.

### Task F20 - Complete Ditto focus scenarios and observations

**Prerequisites:** F19 and the existing Ditto production-input path.

**API slice:** Reuse the `focused`, `focus-visible`, and controller primitives
from F04, F06, and F15. Add production accessibility and scroll-offset
observations needed by the complete acceptance suite. Add no private focus
coordinator commands.

**Implementation notes:** Observations read public rendered/native state after
settling. Keyboard, pointer, and controller actions use the same production
input path as the player. Scenario failures report the expected object, actual
focused object, modality, reason, and governing scope stack.

**Rust proof:** Parse, validate, and report the new scenario vocabulary with
clear failures for missing or ambiguous objects.

**Unity proof:** Virtual controller input produces the same navigation event
path as a real controller and never calls the coordinator directly.

**Ditto proof:** Add scenarios for ordinary forms, modal open/close, nested
overlays, tabs, directional navigation, focused removal, exit animation,
portal reconnect, modality styling, nested scrolling, and reconnect inside a
nested modal with an active roving group.

**Performance proof:** Scenario observation adds no per-frame focus polling; it
samples only at explicit settle points.

**Completion condition:** All normative acceptance scenarios run through their
assigned public oracle. Ditto proves production input and player-visible state;
Unity proves native event order, accessibility, picking, offsets, and input
gating; Rust proves logical routes and validation failures.

### Task F21 - Record release performance and failure evidence

**Prerequisites:** F19-F20 and all preceding task suites green.

**API slice:** No new authoring API. Add durable benchmark/report fixtures and
release documentation for the implemented limits and diagnostics.

**Implementation notes:** Measure cold snapshot installation, one-node sparse
updates, native Tab, explicit neighbors, filtered automatic navigation,
roving moves, modal activation, focus removal, nested scroll reveal, and
reconnect. Exercise hard-limit rejection, stale reports, batch failure,
destroyed openers, and stale reconnect bookmarks.

**Rust proof:** Full black-box suites cover lowering, validation, logical event
routes, portal membership, keyed reconciliation, presence, fallback, queued ref
actions, stale rejection, receipts, shutdown, and reconnect.

**Unity proof:** Full EditMode suites inspect actual `focusedElement`, native
event order, ring traversal, built-in controls, scopes, geometry, scroll
offsets, modality state, and allocation counts.

**Ditto proof:** The complete player suite passes in a clean packaged build with
the pinned Unity and Input System baselines.

**Performance proof:** Publish the raw environment and results for all budgets
defined below. A failed budget blocks completion or requires an explicit design
revision; it cannot be waived in this plan.

**Completion condition:** A clean checkout reproduces the functional,
reconnect, diagnostic, allocation, payload, and latency evidence.

## Performance budgets

These are release gates, not aspirational targets. F21 records exact values and
environment details.

- No Rust request or response is required to finish one input event.
- Warm focus, Tab, explicit-neighbor, and roving dispatch allocate zero managed
  bytes in Unity.
- Focus policy emits no per-frame traffic and no repeated unchanged state
  report.
- An unrelated Reactant commit emits no focus-plan update.
- A one-node policy change is sparse and does not serialize the complete plan.
- Plan installation and validation are linear in changed plan size.
- Explicit neighbors and object/scope/group lookup are O(1).
- Automatic directional scans are bounded by the active panel's validated
  candidate limit and use cached post-layout geometry.
- Eligibility and layout invalidation touch only affected panel indexes.
- Reconnect installs one complete plan, performs at most one restoration or
  initial-focus resolution, and emits one settled state report.

On the pinned Apple Silicon CI host, use the normative release gates: `0.25 ms`
99th-percentile direct dispatch, `4 ms` 99th-percentile automatic dispatch at
16,384 candidates, `50 ms` complete-plan validation at 100,000 nodes, `16 ms`
focus-only reconnect finalization at 10,000 nodes, and `16 MiB` encoded focus
data subject to lower transport limits. Measure zero allocation after 100 warm
events. A hardware-baseline change is approved and recorded before results are
collected; observed implementation performance cannot choose its own gate.

## Safeguards during implementation

The following hazards receive tests and named diagnostics in the task that
first encounters them:

- Unity 6000.5.8f1 may not expose every focus-ring or accessibility operation
  through a stable public API. Prefer documented APIs, isolate the smallest
  required adapter, and fail explicitly if the native behavior cannot be
  preserved.
- Structural reparenting may emit transient native focus events. Gate input and
  application delivery during the transaction, retain native events for host
  bookkeeping, and report only the settled logical outcome.
- Cross-panel scopes and roving groups cannot share a `FocusController`. Reject
  them during Reactant lowering and Unity plan validation.
- A reconnect bookmark may name a removed, hidden, inert, or now-inactive
  object. Revalidate it against the installed generation and then use initial
  focus fallback.
- Large plans can cost memory and install time. Enforce hard limits, sparse
  updates, indexed lookup, and measured cold-plan budgets.
- Application selection can lag Unity-owned roving position by one exchange.
  Keep focused/active styling Unity-local and treat Rust selection as a later
  semantic state update.
- Outside-content exclusion spans focus, picking, and accessibility. Test all
  three surfaces so a visual modal cannot leak interaction through one of them.

## Completion criteria mapped to tasks

The focus and navigation system is complete only when every criterion below is
observable in a clean checkout.

| Criterion | Required tasks |
|---|---|
| UI Toolkit remains authoritative and ordinary controls keep defaults | F04-F06, F21 |
| Complete plans, sparse updates, reports, limits, and stale handling work | F01-F05, F21 |
| Public Rust focus, scope, ref, relationship, and accessibility APIs work | F07-F09 |
| Keyed focus retention and deterministic removal fallback work | F05, F09, F20 |
| Logical events and same-panel portals preserve scope membership | F08, F11, F20 |
| Modal, non-modal, nested, exclusion, and restoration rules work | F10-F12, F20 |
| Hidden, Suspense-retained, inert, and exiting elements are ineligible | F12, F20 |
| Tab, keyboard, controller, explicit, and automatic navigation work | F06, F13-F15, F20 |
| Roving tabs, menus, radios, toolbars, and listboxes work | F16-F18, F20 |
| Focus-visible styling and nested scroll reveal work without Rust renders | F15, F20-F21 |
| Reconnect restores a valid bookmark or applies deterministic initial focus | F09, F11, F20-F21 |
| Rust, Unity, and Ditto provide black-box release evidence | F19-F21 |
| Allocation, payload, latency, and bounded-scan budgets pass | F01-F06, F13-F16, F19-F21 |

Completion also requires all public API documentation, shared fixtures,
diagnostics, sample restoration flows, EditMode tests, Ditto reports, and
performance records to be checked in. There must be no compatibility shim,
protocol version, per-control key-handler workaround, synthetic focus engine,
or input-event round trip.
