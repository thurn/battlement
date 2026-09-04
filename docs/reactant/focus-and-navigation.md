# Reactant Focus and Navigation

Reactant provides familiar focus authoring, reliable modal containment, and
keyboard or controller focus presentation without replacing Unity UI Toolkit's
focus engine.

The design is intentionally narrow. UI Toolkit already supplies one focused
element per panel, a native focus ring, directional navigation, focus events,
and control-specific default actions. Reactant exposes those capabilities and
adds only policy that must span reconciliation, portals, overlays, and Motion
presence.

The central rule is:

- Rust declares focus properties and queues ordinary ref actions.
- UI Toolkit decides whether a live element can receive focus.
- UI Toolkit performs focus changes and emits focus events.
- One Unity-side coordinator enforces active-modal exclusion and records local
  focus-visible state.
- No focus plan, focus-state mirror, navigation graph, or reconnect bookmark is
  exchanged with Rust.

This boundary supplies the input-focus behavior required by
[Reactant accessibility](accessibility-technical-design.md). Accessibility
reads the coordinator's settled active modal and effective inertness. It does
not require generic roving groups, explicit directional neighbors, or a focus
resume protocol.

## Related Information

- [Battlement Reactant technical design](reactant-technical-design.md) defines
  runtime ownership, sessions, commits, desired trees, and reconnects.
- [Reconciliation, events, and portals](reconciliation-events-and-portals.md)
  defines host identity, logical event routes, and physical portal placement.
- [Events and default actions](events-and-default-actions.md) defines native
  default precedence and the synchronous event disposition.
- [Reactant animations](animations.md) defines Motion gestures, presence, and
  physical exit retention.
- [Reactant accessibility](accessibility-technical-design.md) consumes modal
  activity and effective inertness from this design.
- [Refs and geometry](refs-geometry-and-floating-ui.md) defines `ElementRef`,
  queued host actions, and explicit scrolling actions.
- [Host facades](host-facades.md) defines composable host properties.
- [Battlement UI technical design](../battlement-ui-technical-design.md) defines
  the snapshot, command, event, and Unity host contract.
- Unity's [focus order][unity-focus-order], [focus events][unity-focus-events],
  and [navigation events][unity-navigation-events] define the native behavior
  preserved here.

[unity-focus-order]: https://docs.unity3d.com/6000.0/Documentation/Manual/UIE-focus-order.html
[unity-focus-events]: https://docs.unity3d.com/6000.0/Documentation/Manual/UIE-Focus-Events.html
[unity-navigation-events]: https://docs.unity3d.com/6000.0/Documentation/Manual/UIE-Navigation-Events.html

## Goals and Constraints

The design covers the focus behavior needed by ordinary application screens and
modal settings flows:

- native focusability and sequential order;
- mount-time and programmatic focus;
- native focus events through Reactant logical ancestry;
- stable focus for surviving keyed hosts;
- active-modal initial focus, outside exclusion, Tab containment, and
  restoration;
- nested modal overlays on one panel;
- hidden, inert, and exiting content leaving focus participation;
- keyboard and controller focus-visible styling; and
- black-box observation of native focus and effective inertness.

The following constraints keep the implementation bounded:

- Unity `6000.5.8f1` and Input System `1.20.0` are the host baseline.
- UI Toolkit owns one `FocusController` per panel.
- Native control editing, activation, and navigation defaults have first
  refusal.
- A modal and its logical descendants must remain on one panel.
- Controller movement uses the existing normalized UI Toolkit navigation
  events and the current gamepad selected by the Input System bridge.
- Input remains disabled while a snapshot or reconnect is installed.
- Backward compatibility and protocol version negotiation are not required.

This contract contains no generic non-modal focus scope, roving focus group,
explicit directional-neighbor graph, custom spatial-navigation algorithm,
automatic focus scrolling, or exact focus restoration across reconnects.
Native UI Toolkit controls retain their built-in composite behavior. Custom
composites use ordinary focusable hosts and activation until a demonstrated
product need justifies a separate navigation design.

## Ownership Boundary

UI Toolkit remains the only native focus authority. Reactant coordinates
policy around it but never stores a competing focused-element field.

UI Toolkit owns:

- `FocusController.focusedElement`;
- `VisualElement.focusable`, `tabIndex`, `delegatesFocus`, and
  `canGrabFocus`;
- native sequential and directional navigation;
- `FocusInEvent`, `FocusEvent`, `FocusOutEvent`, and `BlurEvent`;
- built-in editing, selection, submit, and cancel behavior; and
- the final live eligibility check immediately before `Focus()`.

Rust owns:

- authored focus properties;
- stable `ElementRef` identity;
- modal initial and explicit restoration references;
- logical ancestry through portals; and
- application event handlers.

The Unity coordinator owns:

- which authored modal is active on each panel;
- captured modal openers and the last focused modal descendant;
- temporary outside focus and picking exclusion;
- commit-time repair when an active modal would otherwise become focusless;
- each panel's last qualifying physical input family for focus-visible styling;
  and
- a read-only settled view for the accessibility manager.

Coordinator state is process-local and disposable. Rust does not acknowledge,
resume, or reconcile it.

## Public Rust API

Reactant keeps direct host builders and adds one small composable bundle for
behavior hooks.

### Focus properties

`FocusProps` contains only persistent host-level declarations:

```rust
pub struct FocusProps;

impl FocusProps {
    pub fn new() -> Self;
    pub fn focusable(self, value: bool) -> Self;
    pub fn tab_index(self, value: i32) -> Self;
    pub fn delegates_focus(self, value: bool) -> Self;
    pub fn auto_focus(self, value: bool) -> Self;
    pub fn inert(self, value: bool) -> Self;
}
```

Every compatible host facade accepts `.focus_props(FocusProps)`. Existing
single-purpose builders remain equivalent convenience forms:

```rust
Button::new(trox::tx("Save", "User-facing copy in this example."))
    .focusable(true)
    .tab_index(0)
    .auto_focus(true);
```

Two composed bundles assigning different values to one property are a developer
error. Focus properties contain no semantic role, accessible name, interaction
callback, directional neighbor, or scroll policy.

### Programmatic focus

The existing ref actions remain the complete programmatic API:

```rust
save_button.focus();
save_button.blur();
```

An action called during event handling is queued with the resulting render
transaction and executes after that transaction's host mutations. A ref that
detaches before command generation produces no action. A live target that
becomes invalid during Unity execution fails that transaction with a named
developer diagnostic.

There is no focus request ID or asynchronous result. Application behavior that
depends on the result observes native `Focus`, `Blur`, `FocusIn`, or `FocusOut`
events.

### Modal overlays

Modal focus policy remains part of the existing overlay abstraction:

```rust
Overlay::modal(overlay_root)
    .initial_focus(cancel_button)
    .restore_focus(settings_button)
    .child(dialog_contents)
```

`initial_focus` and `restore_focus` are optional. The coordinator captures the
actual focused opener when a first modal becomes active, so explicit restoration
is needed only when the desired return target differs from that opener.

An **overlay host** is the root `OverlayHost` that owns one portal target. A
**modal wrapper** is the focusable host created by one `Overlay::modal` call.
One overlay host may contain several ordered modal wrappers, but one native
panel may contain only one overlay host.

There is no general `FocusScope` type. Ordinary popovers do not trap focus or
exclude outside content. They may use explicit ref focus and restoration when
their interaction requires it.

## Focusability and Sequential Order

Reactant exposes UI Toolkit's native focus properties without another focus
ring:

- `focusable = true` permits programmatic focus when `canGrabFocus` also
  succeeds.
- `tab_index < 0` removes a host from sequential Tab navigation while retaining
  programmatic focusability.
- `tab_index == 0` participates in ordinary native order.
- `tab_index > 0` retains UI Toolkit's positive-index ordering.
- `delegates_focus = true` lets UI Toolkit choose the focused descendant.

Sequential order follows the physical Unity tree. Portals may therefore change
Tab order while preserving Reactant component ancestry, matching the distinction
between React component ancestry and physical DOM order.

A host is focus eligible when:

- it exists on the expected panel;
- UI Toolkit reports that it can grab focus;
- neither it nor a logical ancestor is authored inert;
- it is not retained as hidden Suspense content;
- it is not retained solely for Motion exit; and
- it is inside the active modal, when the panel has one.

Opacity alone does not change eligibility. Detachment, `display: none`, hidden
visibility, disabled hierarchy, and effective inertness do.

## Mount-time Focus

`auto_focus` is a one-shot request associated with host appearance, not a
persistent demand. It runs after the complete commit is installed.

The rules are:

- A newly mounted keyed host with `auto_focus = true` is a candidate.
- An initial session may apply the candidate after every document is attached.
- Re-rendering the same keyed host does not steal focus.
- Reconnect may apply the current candidate because every native host was
  recreated and no focus bookmark exists.
- An active modal's initial-focus rules take priority over a root candidate.
- The final Unity `canGrabFocus` check decides whether the request succeeds.

At most one candidate may be declared in one runtime's desired tree. Rust
rejects duplicates before emitting commands. This deliberately conservative
rule avoids introducing shared-panel identity solely for auto-focus and does
not create a persistent focus plan.

## Modal Coordination

The coordinator treats the last logically mounted modal wrapper in the overlay
host's resolved overlay order as the active modal. Rust already lowers that
order into existing overlay placement. A wrapper retained only for Motion exit
is logically absent and cannot remain active. No separate modal rank crosses
the wire.

### Activation

When a modal becomes active, the coordinator:

1. Captures the panel's actual focused element as the opener when no modal was
   previously active.
2. Makes hosts outside the active modal effectively inert.
3. Retains the previous modal's last focused descendant when nesting.
4. Focuses the explicit initial target when it is eligible.
5. Otherwise focuses the first eligible sequential descendant.
6. Otherwise focuses the modal wrapper, which is always programmatically
   focusable and has `tabIndex = -1`.

The wrapper fallback means an active modal never exposes input while focusless.
Failure to focus that validated wrapper invalidates the session and keeps input
disabled.

`Overlay::modal` reserves the wrapper's effective `focusable = true`,
`tabIndex = -1`, and `inert = false` values. The wrapper must also be attached,
visible, and enabled when active. A conflicting authored value or an ineligible
active wrapper is a developer error rejected before input resumes.

### Outside exclusion

Effective modal inertness applies three temporary layers together:

- programmatic and sequential focus eligibility;
- pointer picking; and
- Reactant input subscriptions.

The coordinator stores authored values before applying temporary overrides.
An application commit made while the modal is active updates the stored authored
value. Closing the modal restores the newest authored value rather than the
value captured when the modal opened.

The accessibility manager reads the same effective-inert decision. It does not
compute modal exclusion independently.

### Tab containment

Native controls receive `KeyDownEvent` before the modal boundary handler. A
control that consumes or prevents Tab keeps its native behavior.

For an unconsumed Tab, the root bubble handler builds the active modal's
eligible sequential members from current public UI Toolkit properties. It
orders positive `tabIndex` values first and then zero-valued members in
physical traversal order.

- Tab from the final member focuses the first member.
- Shift+Tab from the first member focuses the final member.
- Tab within the range uses UI Toolkit's ordinary destination.
- A modal with no sequential member retains focus on its wrapper.

When the handler supplies a boundary destination, it focuses that element and
calls `PreventDefault()` before UI Toolkit's later default action. The handler
does not replace the panel's ordinary focus ring outside a modal.

### Directional containment

Arrow keys, D-pad, and stick movement remain native
`NavigationMoveEvent` behavior. Outside hosts are not eligible candidates while
a modal is active.

A root `FocusInEvent` guard handles unexpected focus from external Unity code or
an unusual native control. If the new target is outside the active modal, the
coordinator immediately restores the modal's last eligible focused descendant
or runs the activation fallback. The only promised state is the settled focused
element after event dispatch; Reactant does not define a custom spatial
candidate or geometry score.

### Restoration and nesting

When the active modal closes, the coordinator chooses:

1. the previous modal's retained descendant when a nested modal remains;
2. the explicit `restore_focus` target when eligible;
3. the captured opener when eligible; or
4. no focused Reactant host.

The captured opener may be a Reactant-owned host or an external Unity
`VisualElement`. An external opener is restorable only when it remains attached
to the same panel, visible, enabled, able to grab focus, and permitted by the
new active-modal state. Otherwise the coordinator skips it.

Closing a non-active modal does not move focus. Closing an outer modal discards
restoration state for every nested modal removed with it. A later Motion exit
completion cannot restore focus again.

Modal wrappers on different panels are independent because their panels have
different focus controllers.

## Authored Inertness

`inert = true` removes one logical subtree from user interaction. It affects:

- programmatic and sequential focus;
- pointer picking;
- Reactant input subscriptions; and
- accessibility presentation through the coordinator's settled effective
  state.

Authored inertness composes with active-modal exclusion. Removing either source
does not make a host interactive while the other source remains.

An inert container need not be focusable. Unity indexes the logical ancestry
already supplied by Reactant reconciliation and applies effective state to its
current host descendants. No focus node is serialized for otherwise ordinary
hosts.

## Reconciliation and Presence

Stable focus follows stable keyed host identity.

When a keyed host survives a commit:

- Reactant retains its `ObjectId` and native `VisualElement`;
- property changes do not refocus it;
- sibling reordering preserves focus when UI Toolkit preserves the element;
- same-panel physical reparenting restores that same element only when Unity
  transiently blurs it; and
- changing the key creates a different focus target.

When the focused host is removed or becomes ineligible:

- an active modal runs its simple initial, first-member, wrapper fallback;
- a surviving outer modal restores its retained descendant; and
- an ordinary non-modal panel accepts UI Toolkit's resulting blur.

Reactant does not capture the old native ring to choose next or previous list
items after removal.

Before structural mutation, the panel coordinator captures the focused
Reactant-owned `ObjectId`. After mutation, it refocuses that same surviving
native element only when it remains attached and eligible and UI Toolkit lost
focus during reparenting. UI Toolkit still emits its native blur and focus
events; Reactant does not synthesize or suppress them.

Suspense-hidden and Motion-exiting content become effectively inert before
input resumes. A Motion exit may keep hosts visible, but those hosts cannot
receive focus, pointer input, or Reactant events. Physical exit completion has
no focus effect.

## Focus Events and Portals

UI Toolkit remains the source of all focus events. Reactant changes their
application route, not their native occurrence or order.

- `FocusOutEvent` maps to bubbling and capturing `FocusOut`.
- `FocusInEvent` maps to bubbling and capturing `FocusIn`.
- `BlurEvent` maps to target-only `Blur`.
- `FocusEvent` maps to target-only `Focus`.
- `related_target_id` identifies the nearest Reactant-owned host when one
  exists.
- native direction data remains unchanged.

A portaled target routes through its logical Reactant ancestors. Physical portal
containers do not receive Reactant handlers unless they are also logical
ancestors.

Focus events do not carry coordinator reason, active modal, modality, or
focus-visible fields. Those values are local presentation and diagnostics, not
application focus state.

## Keyboard and Controller Input

The design preserves normalized UI Toolkit input instead of adding a navigation
engine.

- Tab and Shift+Tab use the native focus ring except at an active-modal
  boundary.
- Arrow keys, D-pad, and stick input use native directional navigation.
- Native text, range, radio, tab, and list controls keep their own editing or
  selection behavior.
- Controller submit retains the native submit-to-click path.
- Unconsumed cancel follows the normal Reactant `NavigationCancel` route.
- Cancel never closes a modal without an application handler rendering it
  closed.

The existing Input System bridge owns dead zones, dominant-axis resolution,
repeat cadence, current-gamepad selection, and held-state synchronization.
Those behaviors are not duplicated in the focus coordinator.

Custom radio groups, tabs, menus, listboxes, toolbars, and trees receive no
special roving policy. Authors may:

- use the corresponding native UI Toolkit control;
- make each custom item an ordinary sequential focus target; or
- use application handlers and queued ref focus for a specialized interaction.

Accessibility pattern hooks follow the same rule. They provide semantics and
ordinary focus properties without creating private navigation state.

## Focus-visible Presentation

Each panel coordinator maintains its own focus-visible Boolean used only for
visual presentation and testing.

- A primary pointer down delivered within that panel's visual tree sets pointer
  modality and hides focus-visible styling.
- Tab, Shift+Tab, arrows, D-pad, or stick input sets keyboard or controller
  modality for the receiving panel and shows styling on its focused host.
- Pointer movement and hover do not change the decision.
- Programmatic focus retains the preceding decision.
- Reconnect starts hidden until new physical input establishes modality.

Motion exposes one gesture beside exact native focus:

```rust
Button::new(trox::tx("Play", "User-facing copy in this example."))
    .while_focus(focused_target)
    .while_focus_visible(keyboard_target)
```

`while_focus` reacts to exact native focus from any source.
`while_focus_visible` also requires the coordinator's local Boolean. Unity
updates both without a Rust render.

There is no `use_focus_visible` Rust hook, focus-visible wire field, or promise
to match every browser `:focus-visible` exception.

## Reconnect Behavior

Reconnect reconstructs current declarations rather than restoring ephemeral
focus history.

Unity installs hosts, overlays, focus properties, inertness, Motion state, and
accessibility semantics while input is disabled. It then:

1. selects the final active modal;
2. applies effective outside inertness;
3. runs that modal's initial-focus fallback, when present;
4. otherwise applies the panel's current `auto_focus` candidate; and
5. enables input only after focus and accessibility presentation settle.

Reconnect does not preserve the previously focused host, modal opener, local
focus-visible state, or a custom composite position. Rust sends no focus resume
section. Surviving logical hosts may retain their `ObjectId`, but newly created
native elements do not pretend to retain native focus.

## Accessibility Integration

The accessibility manager consumes a small read-only coordinator view after
each admitted response:

```text
active modal wrapper per panel
effective inertness for a host
```

Accessibility uses the active modal wrapper to publish the associated dialog as
the active presentation root. It uses effective inertness to omit excluded
hosts. It does not call `VisualElement.Focus()`, restore input focus, interpret
Tab, or maintain roving state.

When a runtime enables accessibility declarations, every authored modal wrapper
must carry exactly one exposed dialog semantic bundle on that same host. A
dialog on any other host or a modal wrapper without that bundle rejects the
combined visual and semantic candidate.

Focus settles before accessibility derives active presentation. On reconnect,
accessibility waits for the newly selected active modal and initial focus; it
does not wait for a focus bookmark.

## Commit and Failure Behavior

Focus properties and overlay placement use the existing ordered visual commit.
They do not introduce a second application transaction.

Before mutation, Unity validates:

- modal and referenced focus hosts exist in the prospective object set;
- modal initial focus belongs to its logical subtree when present;
- modal and focus references resolve to one panel;
- authored inert ancestry is structurally valid; and
- at most one overlay host owns modal coordination on a panel.

After mutation, Unity settles focus and effective inertness before accessibility
presentation and before input resumes.

Rust rejects desired-tree errors it can prove, including:

- a foreign-runtime `ElementRef`;
- more than one `auto_focus` candidate in the runtime's desired tree;
- an initial target outside its modal's logical subtree; and
- conflicting composed `FocusProps`.

An unexpected Unity exception after mutation invalidates the session, keeps
input disabled, and requests a complete snapshot. Reactant does not claim to
roll back destroyed native objects or previously emitted native focus events.

Diagnostics identify the panel, focused host before and after settlement,
active modal, rejected candidate, effective-inert source, and current physical
input family. Production diagnostics use object IDs rather than displayed text.

## Performance Requirements

Focus coordination adds no Rust call beyond existing subscribed application
events and sends no focus-specific transport traffic.

Steady-state requirements are:

- no managed allocation for ordinary Tab or directional movement outside a
  modal;
- no per-frame focus message or geometry sampling;
- no geometry scan for directional navigation;
- modal boundary work proportional to the active modal's current sequential
  members; and
- effective-inert updates proportional to hosts whose state changes.

CI records representative modal activation, Tab-boundary, inert-update, and
reconnect timings. The design does not impose synthetic 100,000-node focus-plan
or 16,384-candidate spatial-navigation gates because neither structure exists.

## Behavioral Acceptance Scenarios

The following scenarios define observable behavior without inspecting private
coordinator collections.

### Ordinary form

- Render two text fields, a toggle, a slider, and a submit button.
- Tab and Shift+Tab follow native physical order.
- Native text and range controls consume their editing keys.
- Controller directional movement uses native geometry.
- Controller submit activates the focused button once.
- A queued ref action focuses an eligible host after its commit.

### Modal overlay

- Open a portaled modal from a focused button.
- Initial focus reaches the declared cancel button.
- Outside hosts cannot be focused, picked, or invoked through Reactant.
- Tab and Shift+Tab remain inside the modal.
- Directional navigation cannot settle outside the modal.
- Closing restores the explicit target or captured opener when eligible.
- A pointer-opened modal hides focus-visible styling; a keyboard- or
  controller-opened modal shows it after navigation input.

### Nested modal

- Open an outer modal and move focus within it.
- Open and close an inner modal.
- The outer modal regains its retained focused descendant.
- Removing that descendant makes the outer modal use its simple fallback.
- Closing the outer modal restores its original eligible opener.

### Reconciliation and presence

- Reorder a keyed focused host without replacing it and retain focus.
- Remove the focused host outside a modal and observe native blur.
- Remove a focused modal descendant and observe modal fallback.
- Remove focused Motion content and observe focus leave before exit retention.
- Confirm the visible exiting host cannot receive input.

### Reconnect

- Reconnect with a modal open and an initial target declared.
- Recreate hosts while input remains disabled.
- Apply modal exclusion and initial focus before input resumes.
- Confirm that previous focus and focus-visible state are not restored.

## Automated Validation

Tests prove behavior through public state and native effects.

Rust coverage includes:

- `FocusProps` composition and direct-builder equivalence;
- one-shot `auto_focus` lowering and duplicate rejection;
- queued focus and blur ordering;
- modal reference validation through same-panel portals;
- inert ancestry and presence-exit lowering; and
- reconnect emitting no focus bookmark protocol.

Unity EditMode coverage includes:

- `focusController.focusedElement` remaining authoritative;
- native focus-event order and control default precedence;
- modal initial focus, outside exclusion, Tab boundary looping, and restoration;
- nested modal activation and removal fallback;
- authored values surviving temporary effective-inert overrides;
- keyboard, controller, and pointer focus-visible transitions;
- Motion exit becoming ineligible before physical destruction; and
- reconnect settling modal focus before input and accessibility publication.

Ditto coverage observes:

- actual focused host;
- focus-visible presentation;
- effective inertness and activation counters;
- keyboard and controller navigation through production input; and
- reconnect resetting focus to declared initial behavior.

No test command mutates private coordinator state.

## Alternatives Considered

### Complete focus policy protocol

A complete plan with generations, sparse updates, state reports,
acknowledgements, request outcomes, and reconnect bookmarks can reproduce exact
ephemeral focus state. It also creates a second distributed state machine for
behavior UI Toolkit already owns. The retained product flows need deterministic
modal initialization more than lossless reconnect history.

### Generic roving and directional navigation

A generic engine can unify tabs, menus, radio groups, toolbars, listboxes, and
trees. It requires item identity, orientation, disabled-item skipping,
selection requests, layout direction, geometry fallback, and reconnect state.
Native controls and ordinary focus targets cover the current settings-screen
requirements without that machinery.

### Old-ring removal fallback

Capturing the complete native ring before every structural commit can choose a
next or previous survivor after focused removal. Outside a modal, native blur is
an acceptable and observable result. Inside a modal, the modal's simple fallback
preserves the required containment invariant.

### Automatic reveal

Automatic nested scrolling after every non-pointer focus change needs layout
gates, cancellation, and scroll-policy APIs. Reactant already exposes explicit
scroll actions, and the retained screens do not require a second automatic
policy.

## Completion Criteria

The implementation is complete when:

- UI Toolkit remains the sole native focus authority;
- ordinary forms retain native focus and control behavior;
- public focus properties, refs, events, and mount-time focus work;
- keyed reconciliation preserves surviving focus;
- active modals initialize, exclude outside content, contain focus, and restore
  an eligible target;
- nested modal and Motion exit scenarios settle before input resumes;
- keyboard and controller focus-visible styling updates locally;
- reconnect applies current modal and initial declarations without a bookmark;
- accessibility derives active presentation from the same modal and inertness
  decisions; and
- Rust, Unity, and Ditto black-box coverage passes.

## Manual QA

Use the Reactant focus specimen in a packaged macOS player and desktop WebGL
build. Start from a fresh engine session and use only visible controls, keyboard
input, a controller, and the specimen's reconnect action.

1. Exercise the ordinary form with Tab, Shift+Tab, arrows, controller movement,
   submit, pointer focus, and ref-triggered programmatic focus. Confirm native
   controls retain their editing and activation behavior.
2. Open the modal from pointer, keyboard, and controller. Confirm initial focus,
   outside exclusion, Tab containment, directional containment, restoration,
   and the expected focus-visible presentation.
3. Open a nested modal, move focus in each level, remove the retained outer
   target, and close in reverse order. Confirm simple fallback and opener
   restoration.
4. Reorder and remove keyed focused hosts. Confirm surviving identity retains
   focus, ordinary removal blurs, and modal removal uses modal fallback.
5. Remove a focused Motion child. Confirm it remains visible during exit but is
   immediately unavailable to focus, pointer, and controller input.
6. Reconnect with a modal open. Confirm input stays disabled until current
   modal exclusion and initial focus settle, and confirm previous ephemeral
   focus history is not restored.
7. Run the retained Ditto accessibility assertions against the released-player
   mirror. Confirm its active modal and effective-inert hosts agree with visible
   focus and picking behavior.
