# Reactant Accessibility Implementation Plan

This plan delivers the contract in the normative
[Reactant accessibility design](accessibility-technical-design.md). The design
wins if this plan appears to disagree with it. Each task leaves the repository
compiling and proves an observable boundary before the next task begins.

The entire [focus and navigation implementation
plan](focus-and-navigation-implementation-plan.md), including tasks F01-F21 and
its release evidence, is a hard prerequisite for every task in this plan.
Accessibility implementation does not begin while focus work is incomplete.

This plan must not add or replace:

- `FocusProps`, `FocusScope`, `NavigationNeighbors`, or roving focus types;
- the focus plan, focus snapshot, focus reports, or reconnect bookmarks;
- UI Toolkit focus ownership, focus-ring order, or input modality tracking;
- modal scope activation, effective inertness, opener restoration, or fallback;
  or
- Tab, directional, Home/End, controller, or roving focus movement.

Accessibility composes those completed APIs and consumes settled focus state.
It owns semantic projection, accessibility focus, accessibility actions,
controlled proposals, Unity accessibility mapping, and pattern semantics.

## Related information

- [Reactant accessibility](accessibility-technical-design.md) is the normative
  semantic and assistive-technology contract.
- [Focus and navigation](focus-and-navigation.md) is the completed input-focus
  and navigation dependency.
- [Events and default actions](events-and-default-actions.md) defines the
  synchronous disposition and safe response gate.
- [Reconciliation, events, and portals](reconciliation-events-and-portals.md)
  defines logical identity, portal ancestry, and transaction ordering.
- [Animations](animations.md) defines logical removal before physical presence
  exit.
- [Battlement UI technical design](../battlement-ui-technical-design.md) defines
  the shared snapshot, command, event, and reconnect transport.
- [Ditto technical design](../ditto-technical-design.md) defines production
  input, observable state, and scenario reports.

## Delivery rules

Every task includes its public documentation, shared fixtures, diagnostics, and
black-box proof. Tasks remain unreleased until all native behavior required by
their public API exists and the sample and feature ledger teach that behavior.

Implementation follows these rules:

- Query focus eligibility, active scope, effective inertness, focused host,
  modality, and focus-visible state only through the completed coordinator.
- Request input focus only through the coordinator operation used by
  `ElementRef::focus_with`; never call `VisualElement.Focus()` directly.
- Keep semantic reading order independent from the native physical Tab ring.
- Keep active-descendant item navigation in accessibility interaction policy;
  it may change semantic active descendant but never native input focus.
- Keep accessibility focus independent from input focus except where
  `InputFocusCorrelation` explicitly requests coordination.
- Treat invalid roles, names, relationships, actions, semantic topology, and
  focus-host bindings as developer errors before commit.
- Add no protocol version, compatibility adapter, native accessibility plugin,
  WebGL DOM mirror, or inferred semantic fallback.
- Prefer public Rust, Unity, and Ditto behavior over private manager snapshots.
- Keep source files near 500 lines and split cohesive responsibilities before a
  file approaches 1,000 lines.
- Stage all intended changes before running `./scripts/ci.py`.

## Dependency overview

| Phase | Tasks | Observable result |
|---|---|---|
| 1. Platform evidence | A01-A02 | Unity behavior and generated capability data are pinned |
| 2. Canonical model | A03-A05 | Semantic declarations project and validate deterministically |
| 3. Protocol and mirror | A06-A08 | Snapshots, sparse commits, inspector, and reconnect work |
| 4. Actions and focus | A09-A11 | Accessibility focus and controlled actions are safe |
| 5. Pattern hooks | A12-A15 | Core controls and overlays compose completed focus APIs |
| 6. Collections | A16-A18 | Virtualized and composite structures remain keyed and bounded |
| 7. Unity backend | A19-A20 | Mobile mapping, notifications, and diagnostics work |
| 8. Release proof | A21-A23 | Sample adoption, Ditto, mobile QA, and budgets pass |

Dependencies are linear by phase. Tasks within a phase may be developed in
parallel only after every earlier phase is complete and only when they do not
encode unfinished behavior from another task in that phase.

## Phase 1: platform evidence

This phase verifies the pinned Unity accessibility surface after the focus
project has shipped. It adds no public Reactant accessibility API.

### Task A01 - Pin Unity accessibility fixtures

**Prerequisites:** Focus tasks F01-F21 complete; pinned Unity and Input System
versions installed.

**API slice:** Add minimal native fixtures for every documented
`AccessibilityHierarchy`, `AccessibilityNode`, `AssistiveSupport`, focus,
notification, geometry, and action callback used by the design.

**Proof:** EditMode tests record exact callback order and fields. Small targeted
iOS and Android probes pin the assumed screen-reader focus, invoke, value,
dismiss, scroll, and announcement boundary. The checked-in evidence names
engine, OS, device, and assistive-technology versions. This is assumption
evidence, not backend conformance or final release QA.

**Completion condition:** Every backend assumption is either demonstrated or
classified as unsupported before product code depends on it.

### Task A02 - Generate the capability and fallback tables

**Prerequisites:** A01.

**API slice:** Generate the closed capability vocabulary and pure fallback
classifier shared by Rust and C#.

**Proof:** Golden fixtures cover every canonical role, state, relation, action,
notification, and fallback class. Rust and C# classify them identically.

**Completion condition:** Platform degradation is deterministic data rather
than scattered conditionals.

## Phase 2: canonical semantic model

This phase builds the platform-neutral Rust model without publishing Unity
accessibility nodes.

### Task A03 - Add semantic identity and declarations

**Prerequisites:** Phase 1 and existing logical host identity.

**API slice:** Add `AccessibilityId`, slots, keyed semantic collections,
incarnations, typed refs, `SemanticProps`, `AccessibilityKey`, virtual semantic
nodes, and host composition.

**Proof:** Black-box tests preserve keyed identities through reorder and
reconnect, replace incarnations after remount, and reject collisions, forward
parents, cycles, and foreign-runtime refs.

**Completion condition:** Host and virtual semantics have stable logical
identity independent from native objects and render position.

### Task A04 - Project names, relationships, and reading order

**Prerequisites:** A03.

**API slice:** Implement semantic transparency, canonical forest projection,
name and description computation, typed relationships, content roles, and
direct-child semantic reading order.

**Proof:** Rust tests cover portals, referenced hidden labels, name cycles,
logical parentage, canonical ordering, and the invariant that semantic reading
order never rewrites the native Tab ring.

**Completion condition:** A complete canonical semantic forest can be inspected
without Unity and without focus-policy duplication.

### Task A05 - Validate roles, state, actions, and coverage

**Prerequisites:** A03-A04.

**API slice:** Add closed role families, state/value/action validation,
collection metadata, coverage modes, aliases, fallback policy, and actionable
focus-host validation.

**Proof:** Invalid combinations panic before commit with stable diagnostics.
Coverage detects focusable or actionable hosts missing explicit semantics while
allowing deliberate decoration and transparent layout.

**Completion condition:** An invalid semantic candidate cannot reach protocol
lowering.

## Phase 3: protocol and Unity mirror

This phase transports the canonical model and reconstructs it in Unity without
publishing an assistive-technology hierarchy.

### Task A06 - Add the accessibility wire contract

**Prerequisites:** Phase 2 and generated Rust/C# fixtures.

**API slice:** Add complete accessibility snapshots, sparse complete-node
upserts, removals, relationship sources, action policies, presentation roots,
accessibility-focus commands, proposals, announcements, status, and receipts.
Every batch carries optional `required_focus_generation`: `Some` must equal the
settled installed plan, while `None` requires the candidate's absent focus
section and implicit coordinator root. It carries no focus plan, scope stack,
input focus, modality, or roving bookmark.

**Proof:** Shared fixtures cover every union, limit, ordering rule, `Some` and
`None` focus dependencies, stale generation, incarnation, and malformed
reference. An accessibility-only update does not emit focus-plan traffic.

**Completion condition:** Rust and C# consume the same semantic protocol and
cannot confuse it with the completed focus protocol.

### Task A07 - Build the canonical Unity mirror and inspector

**Prerequisites:** A06.

**API slice:** Add the canonical Unity mirror, inspector backend, indexes,
transaction preflight, presentation exposure, fallback diagnostics, and
consumption of the read-only coordinator integration delivered by focus F04 and
F10.

**Proof:** Unity tests apply complete and sparse semantic commits, accept a
static semantic tree with no focus plan, reject stale or incorrectly absent
focus generations, and derive modal exclusion only from the coordinator's
settled effective-inert state.

**Completion condition:** The Editor exposes the canonical and active semantic
forests without creating `AccessibilityNode` objects.

### Task A08 - Integrate commit ordering and reconnect

**Prerequisites:** A07 and the existing transaction/reconnect executor.

**API slice:** Stage semantic deactivation, host mutation, semantic mirror
updates, focus finalization, presentation derivation, notifications, and
presence destruction in the normative order. Rebuild semantic IDs and
accessibility focus on reconnect.

**Proof:** Injected preflight failure leaves both visual and semantic commits
unchanged. Post-mutation failure keeps input gated and requests a full snapshot.
Reconnect restores input focus only through the focus snapshot and restores or
clears accessibility focus independently.

**Completion condition:** No input window exposes mismatched visual, focus, and
semantic generations.

## Phase 4: actions and accessibility focus

This phase adds the synchronous behavior unique to assistive technology while
delegating input-focus movement to the completed coordinator.

### Task A09 - Implement accessibility focus and reveal

**Prerequisites:** Phase 3.

**API slice:** Add `InputFocusCorrelation`, accessibility-focus state,
generation/incarnation validation, bounded reveal routes, request
supersession, confirmation, timeout, and focus diagnostics.

**Proof:** Static text can receive accessibility focus without input focus. An
interactive node requests input focus only through the coordinator when its
correlation policy says so. Clipped, stale, inert, and removed nodes follow the
documented failure results.

**Completion condition:** Accessibility focus never calls or mirrors UI Toolkit
focus directly.

### Task A10 - Add action routing and controlled proposals

**Prerequisites:** A09 and the synchronous event gate.

**API slice:** Add action capture/target/bubble routing, admission-backed
actions, controlled proposals and resolutions, action-kind/value validation,
one in-flight proposal per target/action kind, and local-draft restoration.

**Proof:** Rust, Unity, and Ditto cover accept, reject, runtime failure,
disconnect, stale generation, duplicate resolution, and re-entrant callback
rejection through the production action path.

**Completion condition:** Handled status and authoritative state are
unambiguous before a Unity callback returns.

### Task A11 - Add announcements and canonical backend state

**Prerequisites:** A09-A10.

**API slice:** Add live regions, imperative announcements, deduplication,
acknowledgement, backend-generation records, capability replacement, and
canonical-tree replay against the inspector backend. Actual screen-reader
activation and Unity hierarchy lifecycle remain in A19.

**Proof:** Inspector-backed tests simulate activation off/on, duplicate
suppression, assertive reconnect replay, dropped notifications, and allowed
versus forbidden fallback without publishing `AccessibilityNode` objects.

**Completion condition:** Backend activation changes do not require an
application rerender and do not replay stale callbacks.

## Phase 5: pattern hooks

Pattern hooks now compose semantics and actions with focus types that already
ship. They do not extend the focus coordinator.

### Task A12 - Add press, button, link, and toggle patterns

**Prerequisites:** Phase 4.

**API slice:** Add state adapters, `use_press`, `use_repeat_press`, `use_hover`,
`use_long_press`, button, link, toggle button, checkbox, and switch hooks
returning `SemanticProps`, existing `FocusProps`, `InteractionProps`, and
interaction state.

**Proof:** Native and custom visuals produce equivalent semantic snapshots and
one logical action for pointer, keyboard, controller, and accessibility input.

**Completion condition:** Core activation and toggle controls are fully
composable without choosing a host type or style.

### Task A13 - Add range and text patterns

**Prerequisites:** A12.

**API slice:** Add `use_drag`, slider/thumb, progress, text field, search field,
validation, and text-edit proposal hooks.

**Proof:** Tests cover range bounds, multi-thumb constraints, UTF-16/scalar
conversion, selection, read-only behavior, native draft restoration, and
validation relationships.

**Completion condition:** Range and text controls expose authoritative state
without replacing native editing or dragging behavior.

### Task A14 - Add tabs and radio patterns

**Prerequisites:** A12-A13 and focus roving tasks F16-F18.

**API slice:** Add tab list/tab/panel and radio group/radio hooks. Compose the
existing tabs and radio roving presets and handle their ordinary settled
`UiRovingSelectionRequested` application events. Accessibility proposals remain
reserved for assistive-technology actions.

**Proof:** Automatic and manual tabs, radio selection, disabled items, portals,
and keyed removal preserve one native Tab stop and matching semantic state.

**Completion condition:** Selection semantics and focus movement agree without
accessibility owning roving position.

### Task A15 - Add dialog, menu, disclosure, and tooltip patterns

**Prerequisites:** A12-A14 and completed focus-scope behavior F10-F12.

**API slice:** Add overlay state, dialog, alert dialog, menu trigger/menu/item,
submenu, disclosure, tooltip, and input-rebinding hooks. Return existing
`FocusProps`, `FocusScope`, roving declarations, directional neighbors, and
`InputCapturePolicy` where needed.

**Proof:** Nested portaled overlays derive active presentation from the focus
coordinator, return a focusable scope anchor, mirror submenu Left/Right
neighbors in RTL, route rebind capture without a second navigation listener,
dismiss only the active scope, remove semantics before exit, and leave opener
restoration and fallback to the coordinator.

**Completion condition:** Overlay semantics cannot create a second scope stack
or restoration algorithm.

## Phase 6: collections and structural patterns

This phase adds bounded keyed structures and richer patterns without changing
focus ownership.

### Task A16 - Add keyed collections and virtualization

**Prerequisites:** Phase 5.

**API slice:** Add collection state, keyed semantic collections, materialized
windows, positions, sizes, continuation actions, and focus-key resolution.

**Proof:** A 10,000-item logical collection publishes only the visible window,
preserves keyed IDs through scroll/reorder, and rejects mismatched continuation
resolutions.

**Completion condition:** Semantic work is bounded by materialized content.

### Task A17 - Add listbox, combobox, and typeahead

**Prerequisites:** A16.

**API slice:** Add options, single/multiple selection,
`ActiveDescendantPolicy`, typed active-descendant navigation requests,
combobox composition, and typeahead. Roving listboxes use the completed preset;
active-descendant comboboxes keep input focus on the text host while the
accessibility interaction policy changes only semantic active descendant.

**Proof:** Keyboard/controller focus, accessibility focus, text editing, open
state, selected keys, and semantic active descendant remain independently
observable and controlled. A handled active-descendant move prevents only the
declared owner input, emits the typed event, keeps native focus on the text host,
and waits for the authoritative semantic response.

**Completion condition:** Typeahead requests focus only through predeclared
focus hosts and the coordinator; active-descendant navigation changes no native
focus state.

### Task A18 - Add table, grid, tree, and structural hooks

**Prerequisites:** A16-A17.

**API slice:** Add list, table, grid, tree, row/cell/header, heading, landmark,
group, separator, image, status, log, timer, marquee, and scroll-area hooks.
Grids use stable explicit directional neighbors; one-dimensional trees use the
completed roving mechanics.

**Proof:** Tests cover keyed hierarchy, row/column headers, collection levels,
expanded state, right-to-left direction, and rejection when a pattern cannot
express input focus using the completed focus APIs.

**Completion condition:** Structural semantics are complete without a private
accessibility navigation engine.

## Phase 7: Unity backend

This phase publishes the already-tested canonical mirror through the pinned
Unity accessibility API.

### Task A19 - Implement Unity accessibility mapping

**Prerequisites:** Phase 6 and A01-A02 evidence.

**API slice:** Add `UnityAccessibilityBackend`, screen-reader activation,
hierarchy lifecycle, exact and adapted role/state/value/action mapping,
geometry, focus callbacks, notifications, and canonical-tree replay.

**Proof:** EditMode fixtures compare mapping plans with the pure classifier.
Targeted iOS and Android conformance tests exercise the A01 fixtures through the
implemented backend and confirm the pinned boundary behavior.

**Completion condition:** Unity publishes no role, state, action, or relation
that the canonical model did not declare.

### Task A20 - Complete diagnostics and performance instrumentation

**Prerequisites:** A19.

**API slice:** Add hierarchy inspectors, aliases, fallback reports, backend
health, proposal/focus traces, frame dirtiness, counters, and bounded logs that
exclude private application text.

**Proof:** Diagnostics explain every degraded mapping and focus/action failure.
Geometry animation coalesces notifications and creates no per-frame semantic
transport.

**Completion condition:** Release evidence can distinguish semantic defects,
backend limitations, and focus-coordinator rejections.

## Phase 8: release proof

The final phase adopts the public APIs and records durable player evidence.

### Task A21 - Extend the Reactant specimen

**Prerequisites:** Phases 1-7.

**API slice:** Extend the existing focus specimen with explicit semantics and
behavior-hook sections for forms, tabs, sliders, dialogs, menus, validation, key
rebinding, virtualized content, announcements, portals, reconnect, and presence
exit. Do not imply those sections existed in focus task F19.

**Proof:** The sample contains no sample-specific native accessibility or focus
code and returns each interaction to a deterministic initial state.

**Completion condition:** A reviewer can exercise every acceptance family using
only public APIs and ordinary production input.

### Task A22 - Complete Ditto and mobile scenarios

**Prerequisites:** A21.

**API slice:** Add semantic tree, accessibility focus, action, proposal,
announcement, fallback, and backend-health observations. Add no command that
mutates private manager or coordinator state.

**Proof:** Ditto runs the complete inspector-backed suite. Final targeted
VoiceOver and TalkBack release spot-checks cover traversal, activation, values,
dialog changes, announcements, and reconnect on the pinned matrix after A19's
backend conformance is green.

**Completion condition:** Automated tests prove canonical behavior and targeted
mobile checks prove the actual platform boundary without conflating the two.

### Task A23 - Record release performance and failure evidence

**Prerequisites:** A21-A22 and all preceding suites green.

**API slice:** No new authoring API. Check in environment records, benchmark
data, failure fixtures, and completion evidence.

**Proof:** Measure 100,000 canonical nodes, 10,000 changed nodes, 1,000 visible
Unity nodes, one-node sparse updates, action callbacks, accessibility focus,
reconnect, virtual windows, and animated geometry. Exercise every hard limit,
stale generation, callback lifetime, fallback rejection, and unavailable
backend path.

**Completion condition:** A clean checkout reproduces functional, diagnostic,
platform, payload, allocation, latency, and reconnect evidence.

## Performance budgets

These are release gates:

- semantic projection is linear in visited logical nodes and relationships;
- keyed collection work is linear in materialized entries, not total size;
- one changed node emits one complete-node upsert, not a full snapshot;
- action dispatch and direct lookup are O(1) after validation;
- accessibility sends no per-frame semantic traffic;
- geometry dirtiness is coalesced per presentation root per frame;
- steady-state callbacks allocate zero managed bytes after warm-up;
- one accessibility commit consumes exactly one settled focus generation; and
- no accessibility task changes focus-plan payload size or focus input latency.

On the pinned Apple Silicon CI host, use the normative gates from the design:
`20 ms` for 100,000-node Rust projection, `8 ms` for a 10,000-node semantic
diff, `12 ms` for a 1,000-visible-node Unity batch, `0.5 ms` direct action
dispatch at the 99th percentile, `16 ms` semantic reconnect finalization at
10,000 nodes, and zero managed allocation after 100 warm callbacks.

## Completion criteria mapped to tasks

| Criterion | Required tasks |
|---|---|
| Focus/navigation remains one completed dependency | A06-A10, A14-A18, A23 |
| Canonical identity, projection, validation, and reconnect work | A03-A08 |
| Accessibility focus is independent and safely correlated | A09, A22-A23 |
| Actions and controlled values resolve synchronously | A10, A12-A18, A22 |
| Core controls and overlays compose existing focus APIs | A12-A15, A21-A22 |
| Collections and structures remain keyed and bounded | A16-A18, A23 |
| Unity mapping and fallback match generated evidence | A01-A02, A19-A20 |
| Ditto and mobile boundaries are both covered | A21-A23 |
| Payload, allocation, latency, and reconnect budgets pass | A06-A11, A16, A20-A23 |

Completion also requires public API documentation, shared fixtures, inspector
output, sample coverage, Ditto reports, mobile evidence, and performance records
to be checked in. There must be no second focus bundle, scope protocol,
navigation engine, modality tracker, restoration stack, or direct native focus
call in the accessibility subsystem.
