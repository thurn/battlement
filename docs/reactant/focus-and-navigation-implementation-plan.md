# Reactant Focus and Navigation Implementation Plan

This plan delivers the contract in the normative
[Reactant focus and navigation design](focus-and-navigation.md). The design
wins if this plan appears to disagree with it.

The work extends existing focus properties, ref actions, focus events, and the
overlay coordinator. It does not introduce a focus-plan protocol, generic focus
scopes, roving composites, custom spatial navigation, automatic reveal, or
reconnect bookmarks.

All tasks finish before accessibility implementation begins. The resulting
coordinator exposes only the settled active modal and effective inertness needed
by [Reactant accessibility](accessibility-technical-design.md).

## Related Information

- [Reactant focus and navigation](focus-and-navigation.md) is normative.
- [Reactant accessibility](accessibility-technical-design.md) consumes the
  completed modal and inertness boundary.
- [Accessibility implementation plan](accessibility-implementation-plan.md)
  records that dependency.
- [Reactant technical design](reactant-technical-design.md) defines runtime,
  reconciliation, portal, ref, event, and Motion contracts.
- [Battlement UI technical design](../battlement-ui-technical-design.md) defines
  snapshots, commands, receipts, and reconnect.
- [Ditto technical design](../ditto-technical-design.md) defines production
  input and observable assertions.

## Delivery Rules

Each task leaves the repository compiling and proves a public or native result.

- Keep `FocusController.focusedElement` authoritative.
- Preserve native control defaults before generic modal handling.
- Reuse stable `ObjectId`, `ElementRef`, overlay placement, and logical portal
  ancestry.
- Add no focus-specific snapshot, update, report, or acknowledgement records.
- Add no compatibility layer or protocol version.
- Prefer black-box Rust, Unity, and Ditto tests.
- Keep source files near 500 lines and split cohesive responsibilities before
  any file approaches 1,000 lines.
- Stage every intended change before running `./scripts/ci.py`.

## Evidence Model

Tasks use the smallest layer that can prove the contract:

1. Rust tests prove authoring, desired-tree validation, and command ordering.
2. Unity EditMode tests prove actual focus, event precedence, picking, and
   effective inertness.
3. Ditto proves production keyboard, controller, modal, and reconnect behavior.
4. Representative measurements guard allocations and modal traversal cost.

The fake host records declarations and commands. It does not simulate UI
Toolkit focus-ring or directional-geometry decisions.

## Dependency Overview

| Phase | Tasks | Observable result |
| --- | --- | --- |
| 1. Authoring | F01-F02 | Focus properties and ref actions are complete |
| 2. Modal coordination | F03-F04 | Modals exclude, contain, and restore focus |
| 3. Lifecycle and presentation | F05 | Lifecycle and modality settle |
| 4. Release proof | F06 | Sample, Ditto, and performance evidence pass |

## Phase 1: Authoring

### Task F01 - Add composable focus and inert properties

**Prerequisites:** Existing host focus properties, host facades, desired-tree
validation, and visual-element property application.

**API slice:** Add the reduced `FocusProps`, `.focus_props`, `auto_focus`, and
`inert`. Preserve direct `focusable`, `tab_index`, and `delegates_focus`
builders as equivalent forms.

**Implementation notes:**

- Merge bundles during ordinary host authoring.
- Reject conflicting assignments as developer errors.
- Validate at most one `auto_focus` candidate per runtime desired tree.
- Apply authored inertness through focusability, picking, and Reactant input
  subscriptions while retaining the newest authored values underneath.
- Emit ordinary visual properties and commands rather than a focus plan.

**Proof:**

- Rust public tests compare bundle and direct-builder lowering.
- Duplicate auto-focus and conflicting bundles panic before commit.
- Unity tests show authored inertness disables all three input layers and
  restores updated authored values when removed.
- An unrelated visual render emits no focus-specific command.

**Completion condition:** Hooks can return one small focus bundle and inertness
has one observable cross-input meaning.

### Task F02 - Complete mount-time and ref focus behavior

**Prerequisites:** F01 and existing queued `ElementRef` actions.

**API slice:** Keep `focus()` and `blur()` as the complete programmatic API. Add
one-shot mount and session installation for `auto_focus`.

**Implementation notes:**

- Execute ref actions after their entry's host mutations.
- Ignore a ref detached during Rust lowering.
- Let ordinary Unity command failure diagnose a target that becomes invalid
  during execution.
- Apply auto-focus after all documents and overlays in the commit are attached.
- Let active-modal initialization take priority over root auto-focus.
- Add no request ID, result record, or focus-state report.

**Proof:**

- Rust journal tests show mutation-before-focus ordering.
- Keyed rerenders do not replay auto-focus.
- Unity observes actual `focusedElement` after mount and ref actions.
- Reconnect applies current declarations without a bookmark payload.

**Completion condition:** Initial and imperative focus work through existing
commands without introducing replicated state.

## Phase 2: Modal Coordination

### Task F03 - Harden active-modal exclusion and focus lifecycle

**Prerequisites:** F01-F02, existing `Overlay::modal`, portal membership, and
overlay ordering.

**API slice:** Preserve `Overlay::modal`, `initial_focus`, and `restore_focus`.
Expose no general `FocusScope` type.

**Implementation notes:**

- Select the active modal from final logical overlay order and exclude wrappers
  retained only for Motion exit.
- Reserve the active wrapper's focusable, tab-index, and non-inert values and
  reject conflicting authoring.
- Capture the opener when the first modal activates.
- Apply effective inertness to every outside Reactant host.
- Focus the explicit initial target, first eligible sequential descendant, or
  modal wrapper in that order.
- Retain the previous modal's last focused descendant while a nested modal is
  active.
- Restore the outer descendant, explicit target, or captured opener on close,
  including an eligible same-panel external Unity opener.
- Make active modal and effective inertness available through a Unity-internal
  read-only query used by accessibility.

**Proof:**

- Unity tests inspect actual focus, picking, and event subscription behavior.
- Portalled descendants retain logical modal membership on one panel.
- Nested modals reactivate the outer retained descendant.
- Removing a target uses the documented simple modal fallback.
- Cross-panel modal references fail before input resumes.

**Completion condition:** One active modal cannot expose focus or input outside
its logical subtree and always settles an eligible focus target.

### Task F04 - Preserve native navigation and contain modal boundaries

**Prerequisites:** F03 and existing event/default-action dispatch.

**API slice:** Add no navigation authoring API. Extend only the existing modal
coordinator callbacks.

**Implementation notes:**

- Register the Tab boundary handler after target controls receive first
  refusal.
- Query current public focusability and `tabIndex` values only for the active
  modal's sequential members.
- Intervene only when Tab or Shift+Tab would leave the modal.
- Leave arrows, D-pad, and stick movement to UI Toolkit.
- Keep outside hosts effectively ineligible while a modal is active.
- Redirect any unexpected outside `FocusInEvent` to the retained modal target
  or activation fallback.
- Preserve native submit-to-click and logical cancel behavior.

**Proof:**

- Native text, range, radio, tab, list, submit, and cancel fixtures retain their
  defaults.
- Tab and Shift+Tab loop only at modal boundaries.
- Directional input cannot settle focus outside the active modal.
- Ordinary panels match native UI Toolkit traversal with no coordinator move.
- Warm ordinary navigation allocates no coordinator memory.

**Completion condition:** Modal containment adds no general navigation engine
and does not steal control-specific defaults.

## Phase 3: Lifecycle and Presentation

### Task F05 - Integrate reconciliation, presence, and focus-visible state

**Prerequisites:** F03-F04, keyed reconciliation, Motion presence, Suspense
retention, and Motion gestures.

**API slice:** Add `while_focus_visible`. Add no Rust hook or focus-visible wire
field.

**Implementation notes:**

- Preserve the same native focused element for surviving keyed hosts.
- Repair a transient reparent blur only for that same surviving element.
- Let ordinary removal outside a modal produce native blur.
- Run modal fallback when focused modal content is removed or becomes inert.
- Make Suspense-hidden and Motion-exiting hosts effectively inert before input
  resumes.
- Track pointer versus keyboard/controller modality independently per panel.
- Reset focus-visible state on reconnect and rerun current modal or auto-focus
  initialization before accessibility publication.

**Proof:**

- Reorder preserves exact keyed focus.
- Ordinary removal blurs and modal removal falls back.
- Presence exit loses focus and input before physical removal.
- Pointer focus and keyboard/controller focus remain visually distinct.
- Reconnect contains no focus resume data and settles accessibility after modal
  initialization.

**Completion condition:** Lifecycle changes never leave an active modal
focusless or an exiting host interactive, and visual focus modality remains
Unity-local.

## Phase 4: Release Proof

### Task F06 - Add the focused specimen and retained evidence

**Prerequisites:** F01-F05 and the existing Ditto production input path.

**API slice:** Extend one Reactant specimen with an ordinary form, nested
portalled modals, authored inert content, keyed removal, Motion exit, and
reconnect. Add public Ditto observations for actual focus, focus-visible
presentation, and effective inertness if they do not already exist.

**Implementation notes:**

- Use only public Reactant authoring.
- Exercise native controls rather than custom roving widgets.
- Keep focus observation read-only.
- Record representative modal activation, Tab-boundary, inert-update, and
  reconnect timing.

**Proof:**

- Ditto covers the ordinary form, modal open and close, nesting, pointer versus
  keyboard/controller styling, keyed removal, presence exit, and reconnect.
- Unity allocation fixtures cover warm ordinary navigation and modal boundary
  handling.
- Rust/C# protocol fixtures prove that no focus-specific snapshot or report was
  added.
- Manual packaged-player checks follow the normative design.

**Completion condition:** Reviewers can exercise the complete retained focus
scope through production input and public state.

## Performance Budgets

The focus design has no large focus plan or geometry candidate cache. Release
evidence therefore targets actual retained work:

- ordinary Tab and directional navigation outside a modal allocate no managed
  memory in the coordinator;
- modal Tab-boundary handling completes in time proportional to the modal's
  current sequential members;
- changing one inert subtree touches only affected current hosts;
- no focus-specific message is sent per input or frame; and
- reconnect focus settlement adds no additional Rust exchange.

Performance fixtures record Unity version, platform, build type, modal member
count, affected inert host count, and warm-up count. Regressions fail through
the repository's ordinary performance comparison rather than synthetic limits
for data structures the design does not contain.

## Safeguards During Implementation

- Do not add a private native focus-ring reflection adapter.
- Do not infer a global sequential or directional graph in Rust or C#.
- Do not add focus request outcomes to make tests easier.
- Do not preserve focus-visible or focused-host history across reconnect.
- Do not let accessibility call `VisualElement.Focus()` or compute its own
  active modal.
- Do not make Motion-retained exiting hosts interactive.
- Do not intercept Tab before native target controls receive first refusal.
- Do not use displayed text as focus identity or diagnostics.

## Completion Criteria Mapped to Tasks

| Criterion | Tasks |
| --- | --- |
| UI Toolkit remains authoritative | F02-F06 |
| Focus properties, auto-focus, and refs work | F01-F02 |
| Authored and modal inertness agree across input layers | F01, F03, F06 |
| Modal initialization, containment, nesting, and restoration work | F03-F04 |
| Native controls and ordinary navigation retain defaults | F04, F06 |
| Keyed focus, removal, Suspense, and Motion exits settle safely | F05-F06 |
| Focus-visible presentation remains local | F05-F06 |
| Reconnect uses current declarations without bookmarks | F02, F05-F06 |
| Accessibility consumes active modal and inertness only | F03, F05-F06 |
| Rust, Unity, Ditto, and manual evidence pass | F06 |
