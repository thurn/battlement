# Reactant Accessibility Implementation Plan

Status: Proposed

## Purpose

This plan delivers the host-backed accessibility design for common mobile
settings and control screens. It preserves composable behavior hooks while
avoiding a universal semantic platform.

The plan contains twelve tasks. It does not schedule work for virtual semantic
nodes, programmatic accessibility focus, controlled accessibility proposals,
text editing, menus, listboxes, comboboxes, tooltips, virtualized collections,
tables, grids, trees, custom actions, live regions, or capability policies.

## Related information

- [Accessibility technical design](accessibility-technical-design.md) is the
  normative behavior and ownership contract.
- [Focus and navigation](focus-and-navigation.md) supplies ordinary focus
  authoring plus modal containment, restoration, and inertness.
- [Focus and navigation implementation
  plan](focus-and-navigation-implementation-plan.md) must be complete before
  accessibility implementation begins.
- [Reconciliation, events, and
  portals](reconciliation-events-and-portals.md) supplies logical host identity
  and event routing.
- [Animations and presence](animations.md) supplies the physical-host retention
  lifecycle.

## Delivery rules

Each task must leave the repository in a valid state and prove its observable
result through public or production-backed behavior.

- Implement against the pinned Unity version.
- Use the completed reduced focus APIs without adding accessibility-owned input
  focus or navigation.
- Keep one semantic node per live logical host.
- Keep Rust and C# protocol fixtures synchronized.
- Test behavior at the lowest layer that owns it.
- Avoid a new Unity startup or mobile-player build in routine CI.
- Treat invalid semantic declarations as developer errors before commit.
- Keep all mobile assumptions in the initial platform fixture and final device
  check rather than spreading device-specific behavior through hook tests.

## Dependency overview

| Phase | Tasks | Observable result |
| --- | --- | --- |
| 1. Platform boundary | A01 | Required Unity behavior is pinned |
| 2. Rust semantics | A02-A03 | Host declarations resolve and validate |
| 3. Transport and mirror | A04-A05 | Complete snapshots commit and reconnect |
| 4. Actions and hooks | A06-A09 | Common controls expose reusable behavior |
| 5. Mobile delivery | A10-A12 | Unity, Ditto, and device evidence pass |

Dependencies are linear by phase. Tasks within a phase may proceed in parallel
only when their interfaces are already fixed by an earlier completed task.

## Phase 1: platform boundary

### Task A01 - Pin the minimal Unity accessibility surface

**Prerequisites:** Complete reduced focus and navigation implementation; pinned
Unity version installed.

**API slice:** Add small fixtures for AccessibilityHierarchy,
AccessibilityNode, AssistiveSupport, host geometry, screen-reader status, direct
invoke/increment/decrement/dismiss/scroll callbacks, screen/layout
notifications, and SendAnnouncement.

**Proof:**

- EditMode fixtures record available fields, states, roles, callback signatures,
  and hierarchy lifecycle.
- One targeted iOS probe and one Android probe confirm traversal, invoke,
  increment/decrement, dismiss, scroll, screen/layout notifications,
  announcement, and screen-reader off/on behavior.
- Unsupported or unreliable behavior is removed from the design rather than
  gaining a fallback subsystem.

**Completion condition:** Every Unity API used by later tasks has pinned
evidence and a normative documented mapping.

## Phase 2: Rust semantics

### Task A02 - Add host semantic declarations

**Prerequisites:** A01 and existing stable ObjectId host identity.

**API slice:** Add SemanticProps, the retained role/state/value/action enums,
SemanticVisibility, AccessibleName, AccessibleDescription, host semantic
composition, source locations, action-handler validation, and modal-wrapper
validation for dialogs.

Dialog semantics may be authored only on the host created by
`Overlay::modal`. No separate focus-scope or modal-association identifier is
added to the semantic protocol.

One host accepts at most one semantic bundle. The host ObjectId is the semantic
identity; there are no slots, virtual nodes, semantic keys, or incarnations.

**Proof:**

- Public examples attach semantics to native and custom hosts.
- Duplicate semantic bundles panic before commit.
- A keyed host retains semantic identity through reorder and reconnect.
- Remount creates a new identity and invalidates the old target.

**Completion condition:** A live host can declare every semantic field needed by
the retained patterns without allocating an independent semantic identity.

### Task A03 - Project names, ancestry, and visibility

**Prerequisites:** A02.

**API slice:** Validate local declarations and live membership handles; project
exposed hosts through transparent logical ancestors; preserve logical ancestry
across portals; validate radio/tab ancestry, panel roots, selection cardinality,
and modal dialog hosts; resolve explicit, content, and reference names and
descriptions; omit Hidden and NameSourceOnly declarations from the canonical
snapshot, pruning complete Hidden subtrees; and build the ordered canonical
host tree. Unity consumes settled effective inertness when it derives active
presentation.

Default reading order is logical child order. No reading-order override or
relationship graph is introduced.

**Proof:**

- Rust black-box tests cover explicit, referenced, and contents-derived names.
- Tests reject an empty required name, one name cycle, a foreign reference, and
  representative invalid role/state/value/action pairs.
- Tests reject invalid radio/tab ancestry, duplicate selection, stale or
  cross-root panel handles, a dialog on a non-modal host, and a modal without a
  dialog.
- Portals retain logical semantic parentage.
- Hidden declarations are absent from the canonical snapshot.
- Detached, inactive-modal, inert, and presence-exiting hosts are absent from
  Unity's active presentation.

**Completion condition:** Rust can produce one complete validated semantic tree
whose nodes all resolve to live hosts.

## Phase 3: transport and mirror

### Task A04 - Add the complete semantic snapshot protocol

**Prerequisites:** A03 and the repository's Rust/C# generation path.

**API slice:** Add optional AccessibilitySnapshot, AccessibilityNodeSnapshot,
the five direct action variants, backend generation, and one-shot announcement
records. A response contains a complete snapshot when canonical semantics
change and no snapshot for visual- or focus-only work. The protocol contains no
semantic upserts, removals, relationship sources, proposals, receipts, or
capability reports.

**Proof:**

- One shared Rust/C# fixture round-trips a representative complete snapshot,
  direct action, reconnect generation, and announcement.
- Unknown required fields or enum values fail loudly.
- Announcement records are absent from reconnect snapshots.

**Completion condition:** Rust and Unity agree on one small resolved protocol
without a sparse mutation grammar.

### Task A05 - Build the Unity mirror and atomic lifecycle

**Prerequisites:** A04 and the existing transaction and reconnect executor.

**API slice:** Add the in-memory semantic mirror, ObjectId index, complete
snapshot preflight, active presentation derived from visibility and settled
focus state, host geometry lookup, generation replacement, and reconnect.

Apply each accepted response in the design's order:

1. suspend hierarchy publication and reject accessibility callbacks;
2. apply visual host mutations;
3. replace the semantic mirror only when a snapshot is present;
4. settle focus and effective inertness;
5. derive active presentation;
6. reconcile backend nodes;
7. send notifications and announcements; and
8. destroy hosts no longer retained by presence.

Active presentation is re-derived even when the response has no semantic
snapshot.

**Proof:**

- An injected preflight failure leaves visual and semantic state unchanged.
- Unity active presentation publishes only the active modal subtree.
- Presence removal disables semantic callbacks before physical destruction.
- Reconnect preserves surviving ObjectId values and rejects an old generation.

**Completion condition:** No callback window exposes mismatched host, focus, and
semantic state.

## Phase 4: actions and hooks

### Task A06 - Route direct accessibility actions

**Prerequisites:** A05 and the existing synchronous event gate.

**API slice:** Validate generation, ObjectId, active exposure, disabled state,
and declared action; then route activate, increment, decrement, dismiss, or
scroll through the owning logical host's ordinary event path.

Target-default handlers return Handled or Unhandled. Unit-returning pattern
callbacks are wrapped as Handled after successful completion. Capture and bubble
observers do not determine the Unity handled result. Unity returns handled after
the response passes synchronous safe-gate admission validation; mutation may
remain queued until the current event stack unwinds.

The task adds no semantic event tree, ProposalId, ControlValue, local draft,
text payload, custom action, or in-flight proposal registry.

**Proof:**

- Accepted actions run one logical callback and admit its normal Rust render.
- Stale, hidden, disabled, unsubscribed, and role-invalid actions run nothing.
- Runtime failure preserves the previous committed semantic and visual state.
- A portaled target uses logical capture and bubble ancestry.

**Completion condition:** Unity handled status is determined synchronously from
the current host action without a controlled proposal protocol.

### Task A07 - Add button, checkbox, switch, and radio hooks

**Prerequisites:** A06 and completed reduced `FocusProps` authoring.

**API slice:** Add use_button, use_checkbox, use_switch, use_radio_group, and
use_radio. Return SemanticProps, existing FocusProps, ordinary
InteractionProps, and styling state.

**Proof:**

- Native and custom visuals produce equivalent semantic snapshots.
- Pointer, touch, keyboard, controller, and accessibility activation produce
  one logical intent.
- Disabled controls remain readable and reject actions.
- Switch rejects Mixed; native radio controls retain native movement, while
  custom radios are ordinary focus and activation targets.

**Completion condition:** Core activation and choice controls are accessible
without choosing a host type or style.

### Task A08 - Add slider, progress, and tabs

**Prerequisites:** A07 and completed reduced `FocusProps` authoring.

**API slice:** Add a single-thumb use_slider, use_progress, use_tabs, use_tab,
and use_tab_panel. Slider exposes increment/decrement only; it has no set-value
accessibility action or multi-thumb semantic representation.

**Proof:**

- Slider values clamp to the declared range and use localized value text.
- Accessibility and ordinary input changes produce the same authoritative
  semantic value.
- Progress exposes determinate value or busy state with no action.
- Native tabs preserve native navigation; custom tabs are ordinary focus and
  activation targets; both publish only the selected panel.
- Deselection hides a panel before presence exit.

**Completion condition:** Range and selection patterns compose existing
interaction and focus behavior without accessibility-owned navigation.

### Task A09 - Add dialog, disclosure, structure, and announcements

**Prerequisites:** A07-A08 and completed modal overlay behavior.

**API slice:** Add use_dialog, use_disclosure, use_heading, use_image,
use_static_text, use_group, use_scroll_area, and use_announce.

Dialog semantics must be authored on an existing `Overlay::modal` wrapper.
Announcements are one-shot strings with no politeness, deduplication,
acknowledgement, or replay.
The current runtime transaction owns their ordered queue and discards it when
rendering, preflight, or safe-gate admission fails. Unsubmitted messages are
also discarded after an unexpected application failure.

**Proof:**

- A portaled modal derives active presentation from the focus coordinator.
- Unity dismiss reaches the active dialog's ordinary close callback once.
- Closing removes semantics before presence exit and leaves restoration to the
  focus coordinator.
- Structural hooks add no Tab stops.
- One accepted commit submits one announcement; reconnect submits none.
- Every non-admitted response and post-mutation failure submits no announcement.

**Completion condition:** Common settings structure and modal behavior are
complete without menus, tooltip state, input capture, or live regions.

## Phase 5: mobile delivery

### Task A10 - Publish the Unity accessibility hierarchy

**Prerequisites:** A01 and A05-A09.

**API slice:** Implement the normative mapping table, AccessibilityHierarchy
ownership, node reconciliation by ObjectId, current host frames, direct
callbacks, screen/layout notifications, SendAnnouncement, and screen-reader
status reconstruction.

The backend has only available and unavailable status. Unsupported fields emit
bounded diagnostics; they do not run a generated classifier or reject an
otherwise valid semantic commit.

**Proof:**

- EditMode tests cover each retained role and direct action mapping.
- Hierarchy activation, replacement, and teardown preserve current nodes.
- Screen-reader off/on increments generation and rebuilds from the mirror.
- Unsupported players keep the mirror without assigning a hierarchy.
- Callback lookup never uses a disposed AccessibilityNode reference.
- Active-root changes emit one ScreenChanged; content/order changes emit
  coalesced LayoutChanged; ScreenChanged subsumes LayoutChanged.

**Completion condition:** iOS and Android publish no semantic field or action
that Rust did not declare.

### Task A11 - Add focused specimen and Ditto coverage

**Prerequisites:** A10.

**API slice:** Extend one existing settings specimen with a button, checkbox,
switch, radio group, slider, progress indicator, tabs, dialog, disclosure,
heading, image, static text, scroll area, announcement, portal, reconnect, and
presence exit.

Add Ditto accessibility assertions and the five direct actions through the
production callback adapter. Do not add commands that mutate private manager or
focus state.

**Proof:**

- The specimen uses only public hooks and ordinary application state.
- Ditto inspects the production in-memory mirror.
- Ditto actions enter through the same adapter as AccessibilityNode callbacks.
- One focused scenario covers each acceptance family without duplicating all
  hook tests.

**Completion condition:** A reviewer can exercise the complete retained scope
without sample-specific accessibility code.

### Task A12 - Complete release validation

**Prerequisites:** A11 and all focused automated suites green.

**API slice:** Add no new public API. Run the representative settings specimen
on one current iOS device with VoiceOver and one supported Android device with
TalkBack.

**Proof:**

- Traversal finds the retained roles in plausible logical order.
- Invoke, toggle, select, increment/decrement, dismiss, and scroll change
  application state once.
- The selected tab panel, disclosure content, informative image, static text,
  group, and scroll area are discoverable without unintended Tab stops.
- Opening a modal removes background traversal and closing removes the dialog
  before visual exit completes.
- Scroll and modal open/close produce the pinned direct callback and
  screen/layout notification behavior.
- One announcement submits once.
- Screen-reader off/on and runtime reconnect rebuild one hierarchy without
  duplicate nodes or stale actions.
- The evidence records commit, Unity version, device, OS, and screen-reader
  version.

**Completion condition:** Automated tests prove Reactant-owned behavior and the
small mobile check proves the actual Unity boundary.

## Routine validation budget

Accessibility joins existing test processes and should add no more than five
seconds to a warm routine CI run.

Routine CI includes:

- focused Rust hook, projection, validation, and action tests;
- one Rust/C# protocol fixture;
- one Unity EditMode fixture in the existing Unity invocation; and
- no mobile player, screen reader, emulator, large semantic benchmark, or
  repeated cross-layer scenario matrix.

The implementation traverses the complete semantic snapshot when semantics
change. No release gate targets 100,000 semantic nodes, sparse diff latency,
zero-allocation callbacks, or virtualized collection complexity.

## Completion criteria mapped to tasks

| Design outcome | Task |
| --- | --- |
| Pinned Unity boundary | A01 |
| Host-owned semantic declarations | A02 |
| Names, logical ancestry, visibility, and validation | A03 |
| Complete snapshot and direct event protocol | A04 |
| Atomic mirror, presentation, presence, and reconnect | A05 |
| Direct accessibility actions | A06 |
| Button, toggle, switch, and radio hooks | A07 |
| Slider, progress, and tabs | A08 |
| Dialog, disclosure, structure, and announcements | A09 |
| iOS and Android Unity hierarchy | A10 |
| Public specimen and production-backed Ditto tests | A11 |
| VoiceOver and TalkBack release evidence | A12 |

The project is complete only when A01 through A12 are complete. Work outside
those tasks requires a new design rather than an informal extension of this
plan.
