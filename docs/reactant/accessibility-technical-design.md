# Reactant Accessibility

Status: Proposed

## Decision

Reactant will provide a deliberately small accessibility layer for common game
settings and control screens. Rust behavior hooks declare accessible meaning and
compose the existing focus APIs. Unity mirrors those declarations through
UnityEngine.Accessibility on supported mobile players.

The public API follows the lower-level React Aria model:

- behavior hooks return semantic, focus, and interaction properties;
- developers choose host types, children, classes, and styles;
- state remains application-owned; and
- the same hook works with native UI Toolkit controls or custom visuals.

Version one supports host-backed buttons, checkboxes, switches, radio groups,
single-thumb sliders, progress indicators, tabs, dialogs, disclosures, headings,
images, static text, groups, and scroll areas. It also supports one-shot
announcements.

The project intentionally stops at that boundary. It does not implement virtual
semantic nodes, programmatic accessibility focus, accessibility text editing,
menus, listboxes, comboboxes, tooltips, virtualized collections, tables, grids,
trees, custom actions, live regions, or a general capability-policy system.
Adding one of those areas requires a separate design based on a demonstrated
product need.

The completed [focus and navigation design](focus-and-navigation.md) is a
prerequisite. UI Toolkit remains the only owner of native input focus and
navigation. Reactant's coordinator adds modal containment, restoration,
effective inertness, and local focus-visible presentation. Accessibility
composes the reduced `FocusProps` API and reads active-modal and inertness
state; it never creates matching focus or navigation state.

## Related information

- [Reactant technical design](reactant-technical-design.md) defines synchronous
  rendering and the Rust-to-Unity runtime boundary.
- [Component authoring](component-authoring.md) defines how hooks and host
  properties compose.
- [Hooks and effects](hooks-and-effects.md) defines positional hook ownership.
- [Reconciliation, events, and portals][reconciliation-design] defines logical
  ancestry and event routing.
- [Animations and presence](animations.md) defines retained physical hosts
  after logical removal.
- [Focus and navigation](focus-and-navigation.md) exposes ordinary focus
  authoring and owns modal focus containment used here.
- [Accessibility implementation plan](accessibility-implementation-plan.md)
  breaks this design into reviewable tasks.
- [Unity mobile accessibility][unity-mobile] documents the only version-one
  assistive-technology integration.
- [WAI-ARIA Authoring Practices][apg-patterns] supplies common widget behavior
  conventions without defining Reactant's wire format.

[reconciliation-design]: reconciliation-events-and-portals.md
[unity-mobile]: https://docs.unity3d.com/6000.5/Documentation/Manual/mobile-accessibility.html
[apg-patterns]: https://www.w3.org/WAI/ARIA/apg/patterns/

WAI-ARIA is a vocabulary and behavior reference. Reactant does not create DOM
nodes, serialize ARIA attributes, or call native platform accessibility APIs
directly.

## Responsibility boundary

Each overlapping behavior has one owner.

| Area | Focus and navigation | Accessibility |
| --- | --- | --- |
| Input focus | Exposes native focus and restores modals | Composes properties |
| Navigation | Preserves native UI Toolkit behavior | Adds no navigation |
| Modals | Owns inertness and restoration | Publishes active dialogs |
| Semantic tree | Owns no semantics | Owns the resolved host tree |
| Actions | Owns no callbacks | Routes Unity actions |
| Accessibility focus | No ownership | Observes platform focus diagnostics |

Accessibility does not call `VisualElement.Focus()`. An accessibility action
may cause ordinary application state to queue an existing `ElementRef::focus`
action, but accessibility has no private input-focus path.

## Existing runtime contracts

Reactant renders synchronously on the engine thread. It reconciles a candidate
tree, validates it, and commits one ordered response. Portals preserve logical
ancestry while changing physical UI Toolkit placement.

The design uses these repository contracts:

- **ObjectId** is the stable logical identity of a reconciled host. A surviving
  keyed host retains it through reorder and reconnect.
- The **safe response gate** admits a complete Unity response only after the
  current UI Toolkit event stack unwinds and before the next repaint.
- **Effective inertness** is the focus coordinator's settled decision that a
  host cannot receive input because of the active modal or an inert ancestor.
- A **modal wrapper** is the host created by `Overlay::modal`; a dialog attaches
  its semantic declaration to that same stable `ObjectId`.
- A **backend generation** identifies one live Unity accessibility hierarchy.
  Reconnect and screen-reader reactivation create a new generation.
- **Presence exit** is the interval in which a logically removed host remains
  physically mounted only to finish animation.

Unity receives native events before Rust can change their default behavior.
This design avoids a new controlled-value proposal system. AccessibilityNode
callbacks report direct actions and do not mutate UI Toolkit control state
locally. Rust handles an action synchronously and returns the authoritative
rendered state in the normal runtime response.

Unity owns:

- live VisualElement objects and their ObjectId index;
- geometry and clipping;
- the safe response gate used after event propagation;
- the focus coordinator; and
- the active AccessibilityHierarchy.

AnimatePresence may retain a removed VisualElement while it exits. Semantic
lifetime follows logical lifetime instead: removed content leaves the semantic
snapshot before its visual exit starts.

These contracts establish five rules:

1. Each semantic node belongs to one live logical host.
2. Logical ancestry determines semantic ancestry, including across portals.
3. Rust resolves names and validates semantics before transport.
4. Unity callbacks dispatch only actions already declared on the current node.
5. Visual, focus, and semantic changes are admitted as one complete response.

## Public authoring model

The authoring API keeps accessible behavior separate from rendering and styling.
It does not introduce accessibility-specific state containers.

### Shared hook result

Behavior hooks return one bundle for each concern:

~~~rust
pub struct AccessibleBehavior<G, S> {
    pub semantic: SemanticProps,
    pub focus: FocusProps,
    pub interaction: InteractionProps<G>,
    pub motion: MotionProps,
    pub state: S,
}
~~~

FocusProps is the existing focus type. InteractionProps contains ordinary Rust
handlers and the small Unity-local press or range policy already needed by the
visual interaction. Accessibility adds no general interaction-policy registry.

Each `ActionSet` bit is derived from a matching target-default handler in
`InteractionProps`; applications do not set action bits independently. Rust
projection rejects a declared action without its handler. Unity dispatch uses
the synchronous disposition contract from
[Events and default actions](events-and-default-actions.md): the matching
handler returns `Handled` or `Unhandled`, and observers do not determine that
result.

Hosts attach the complete bundle atomically:

~~~rust
VisualElement::new().behavior(button)
~~~

Semantic properties do not make a host focusable. Focus properties do not expose
a host to assistive technology. Interaction properties do not imply either.
The atomic attachment prevents a control from accidentally dropping the native
motion subscriptions that maintain hover, press, and focus-visible state.

Every public accessibility function named `use_*` is a positional Reactant
hook, including patterns whose current result is derived entirely from their
arguments. They require component render context, must be called
unconditionally, and must keep the same ordering and identity across renders.
This matches React Aria's composition model and lets a pattern acquire internal
state later without silently changing its public calling contract.

### Semantic properties

Every semantic-capable host accepts at most one resolved semantic bundle.

~~~rust
pub struct SemanticProps {
    pub role: SemanticRole,
    pub name: Option<AccessibleName>,
    pub description: Option<AccessibleDescription>,
    pub state: SemanticState,
    pub value: Option<RangeValue>,
    pub visibility: SemanticVisibility,
    pub actions: ActionSet,
}
~~~

The supported roles are:

- button, checkbox, switch, radio, and radio group;
- slider, progress indicator, disclosure, and scroll area;
- tab, tab list, and tab panel;
- dialog;
- heading, image, static text, and group.

The supported state is intentionally small:

~~~rust
pub struct SemanticState {
    pub disabled: bool,
    pub checked: Option<CheckedState>,
    pub selected: Option<bool>,
    pub expanded: Option<bool>,
    pub busy: bool,
}
~~~

CheckedState is False, True, or Mixed. A switch rejects Mixed. RangeValue
contains current, minimum, maximum, and optional localized display text.
These are canonical Reactant states; the Unity mapping below identifies which
ones the pinned mobile API can publish.

ActionSet records the callbacks Unity may invoke directly:

~~~rust
pub struct ActionSet {
    pub activate: bool,
    pub increment: bool,
    pub decrement: bool,
    pub dismiss: bool,
    pub scroll: BTreeSet<ScrollDirection>,
}
~~~

ScrollDirection is Forward or Backward. The five normalized action variants
are:

- activate;
- increment;
- decrement;
- dismiss; and
- Scroll with one ScrollDirection payload.

A role may declare only actions and state relevant to that role. For example, a
button may activate but may not expose a numeric range, while a slider may
increment and decrement but may not expose checked state.

The complete validation matrix is:

| Role | Required declaration | Optional declaration | Actions |
| --- | --- | --- | --- |
| Button | Name | Disabled | Activate required |
| Checkbox | Name and checked | Disabled | Activate required |
| Switch | Name and boolean checked | Disabled | Activate required |
| Radio | Name, group, and selected | Disabled | Activate required |
| Radio group | Name | None | None |
| Slider | Name and finite range | Disabled | Increment and decrement required |
| Progress | Name and busy or range | None | None |
| Tab | Name, tab list, and selected | Disabled | Activate required |
| Tab list | Name and at least one tab | None | None |
| Tab panel | Live tabs handle | Optional description | None |
| Dialog | Modal-wrapper name | Optional dismiss handler | Dismiss if present |
| Disclosure | Name and expanded | Disabled | Activate required |
| Heading | Text name and level | None | None |
| Image | Text name | None | None |
| Static text | Text name | None | None |
| Group | None | Name | None |
| Scroll area | Axis | Name, available directions | Declared directions only |

Heading level is carried by the Heading role variant. An informative image
requires a name; a decorative image has no semantic declaration. A progress
range contains current, minimum, maximum, and resolved display text. Radio uses
selected state, not checked state.

Required state fields must be present even when their value is false. Progress
requires exactly one of busy true or a range value. State and actions not listed
for a role are invalid. Disabled interactive nodes retain their declared actions
in the canonical snapshot, but Unity rejects those callbacks while disabled.

### Names and descriptions

Names and descriptions are resolved in Rust and cross the wire as strings.
Unity never receives semantic relationship IDs.

`SemanticVisibility` controls participation before names are resolved:

- `Exposed` publishes the host as a semantic node and permits contents-derived
  text.
- `NameSourceOnly` contributes text through explicit `LabelledBy` or
  `DescribedBy` references and eligible `StaticText` descendants in `Contents`
  names; it is never published.
- `Hidden` contributes neither a node nor text and prunes its complete logical
  subtree from the canonical semantic snapshot.

~~~rust
pub enum AccessibleName {
    Text(LocalizedString),
    LabelledBy(ElementRef),
    Contents,
}
~~~

AccessibleDescription supports explicit text or one DescribedBy ElementRef.
LocalizedString remains unresolved until Reactant builds the semantic snapshot.
Reactant sends only resolved application text to Unity.

Label and description references are limited:

- the target must be a live host in the same runtime;
- the target may contribute text without being published;
- cycles and empty required names are invalid; and
- the resolved text is flattened before transport.

Name resolution builds a dependency graph before the semantic tree is emitted.
Text is resolved as follows:

1. Text returns its already-resolved string.
2. LabelledBy resolves the target using the same rules and records the
   dependency for cycle detection.
3. Contents walks logical descendants in depth-first child order.
4. The walk appends the Text name from Exposed or NameSourceOnly StaticText
   declarations.
5. The walk skips Hidden declarations and does not enter an actionable
   descendant's subtree.

Multiple fragments join with one space. Whitespace is collapsed and trimmed
after the complete walk. DescribedBy uses the same text resolution but cannot
reference the node currently being named or described.

Heading, Image, and StaticText require AccessibleName::Text. Their resolved text
crosses the wire in label; there is no separate semantic text field. Group and
ScrollArea encode an absent optional name as name None and wire label None.

Text resolution collapses whitespace and trims the result. Reactant does not
append role words such as "button" because Unity and the operating system own
those phrases.

### Host composition

One host owns at most one semantic node. A hook that needs an independently
focusable or actionable part requires an independently rendered host.

This rule applies directly to composite visuals:

- a native slider uses its public slider host for semantics and focus;
- a custom slider uses an author-owned thumb or proxy host for semantics and
  focus;
- each tab is a host with tab semantics;
- each radio is a host with radio semantics; and
- a dialog container is a host with dialog semantics.

The semantic identity is the host's ObjectId. A surviving keyed host retains
its identity through reorder and reconnect. A remounted host receives a new
ObjectId, so a callback for the removed host cannot reach its replacement.

The design does not allocate semantic slots beneath one host. Applications that
draw many logical controls into one canvas must create host proxies or remain
outside version-one accessibility.

## Pattern hooks

The retained patterns cover ordinary settings screens while preserving custom
rendering.

### Button

~~~rust
let button = use_button(ButtonOptions {
    name: trox::tx("Save changes", "Accessible name for the save button."),
    description: None,
    is_disabled: saving,
    on_press: callback(|app: &mut App| app.save()),
});
~~~

use_button returns:

- the button role and accessible name;
- disabled state;
- activate action;
- existing focus properties;
- press interaction; and
- pressed styling state.

Pointer, touch, Enter, Space, controller submit, and accessibility activation
produce one logical press. The hook does not require a native Button host.

### Checkbox and switch

use_checkbox accepts the current checked state and an on-change callback. It
returns checkbox semantics, existing focus properties, and activation behavior.
Accessibility activation requests the next checked state through the ordinary
Rust callback.

use_switch has the same mechanics but:

- exposes the switch role;
- accepts only true or false; and
- represents an immediate setting rather than a form choice.

Disabled controls remain readable and reject activation.

### Radio groups

`use_radio_group` returns group semantics and a runtime-local membership handle.
`use_radio` returns one host's radio semantics, ordinary focus properties, and a
selection callback.

Native `RadioButtonGroup` controls retain their built-in arrow and controller
behavior. A custom radio host is an ordinary sequential focus target and changes
selection only when activated. The accessibility hook does not install roving
position or selection-follows-focus state. Its returned `FocusProps` set
`focusable = true` and `tab_index = 0` while the radio is enabled.

Each radio must:

- have a nonempty name;
- belong to one live radio group;
- expose selected state; and
- render on its own host.

use_radio_group returns a runtime-local handle naming the group host's ObjectId.
use_radio stores that handle in its Rust declaration. Projection requires the
group to be the radio's nearest semantic radio-group ancestor. The handle is
never serialized.

A radio group may contain zero or one selected radio. Multiple selected radios
are invalid. When none is selected, every enabled custom radio remains an
ordinary focus and activation target.

### Slider and progress

Version one supports a single-thumb slider. A native slider uses its public
control host as the semantic and focus host. A custom slider uses one
author-owned thumb or proxy host. That host contains the range, localized
display text, and increment and decrement actions.

~~~rust
let slider = use_slider(SliderOptions {
    name: trox::tx("Music volume", "Accessible name for the volume slider."),
    value: volume,
    range: 0.0..=100.0,
    step: 5.0,
    on_change: set_volume,
});
~~~

Keyboard, controller, pointer, and touch interaction continue to use the
existing focus and interaction systems. Unity accessibility increment and
decrement callbacks invoke the same Rust change callback. Rust clamps the
result to the declared range.

Multi-thumb sliders and accessibility set-value actions are not part of this
contract.

use_progress publishes a named determinate range or busy state. It has no
actions and does not announce every value change.

### Tabs

`use_tabs` returns tab-list semantics and a runtime-local membership handle.
Each `use_tab` call returns tab semantics, selected state, ordinary focus
properties, and activation. `use_tab_panel` returns the panel's semantics and
visibility state.

~~~rust
let tabs = use_tabs(trox::tx("Settings", "Accessible name for the settings tabs."));
let tab = use_tab(&tabs, trox::tx("Audio", "Accessible name for the audio tab."), selected, select_audio);
let panel = use_tab_panel(&tabs, selected);
~~~

Only the selected panel is exposed. A deselected panel becomes semantically
hidden before any exit animation. Tab-to-panel relationship IDs are not
transported in version one; logical adjacency and names provide the published
structure.

use_tabs returns a runtime-local handle naming the tab-list host. Each tab must
be a logical descendant whose nearest tab-list ancestor matches that handle. A
tab panel may be a sibling or portaled descendant, but it must reference a live
handle in the same semantic root. The hook copies the selected tab's name to
the panel. These membership handles are validated in Rust and are not
serialized.

A nonempty tab list has exactly one selected, enabled tab and exactly one
Exposed panel for its handle. Multiple selected tabs or panels, a missing
selected panel, and a panel with a stale or cross-root handle are invalid.

Native `TabView` controls retain their built-in navigation. Every enabled custom
tab is an ordinary sequential focus target and changes selection only when
activated. Its returned `FocusProps` set `focusable = true` and `tab_index = 0`.
The hook does not add arrow-key roving behavior.

### Dialog and disclosure

`use_dialog` returns dialog semantics and dismiss interaction for an existing
modal overlay wrapper.

~~~rust
let dialog = use_dialog(DialogOptions {
    name: trox::tx("Settings", "Accessible name for the settings dialog."),
    on_dismiss: Some(close_settings),
});

Overlay::modal(overlay_root)
    .initial_focus(cancel_button)
    .restore_focus(settings_button)
    .behavior(dialog)
    .child(dialog_contents)
~~~

The focus coordinator owns initial focus, containment, outside inertness,
restoration, and fallback. Accessibility:

- requires the dialog semantic host to be the modal wrapper itself;
- publishes only the active modal subtree;
- exposes Unity dismiss when the active dialog declares it; and
- sends one screen-change notification when presentation changes.

`on_dismiss` is optional. When absent, the semantic node declares no Dismiss
action and Unity dismissal returns unhandled. The hook does not create a
backdrop, title, close button, portal, layout, or style.

`use_disclosure` returns button-like activation plus expanded state. If focus is
inside content being collapsed, the application focuses the disclosure trigger
through the existing queued ref action before or with the closing render.

### Structural hooks

The small structural set contains:

- use_heading with levels one through six;
- use_image for informative images with explicit names;
- use_static_text;
- use_group; and
- use_scroll_area.

use_scroll_area declares one axis, whether each direction is currently
available, and an application callback for forward or backward movement.
Unavailable directions are absent from ActionSet. The application owns the
scroll amount and renders updated availability after the callback.

Forward increases the logical scroll offset on the declared axis; Backward
decreases it. This meaning does not reverse in a right-to-left layout. Unity's
signed scroll callback is normalized onto the declared horizontal or vertical
axis. A callback on the other axis is not handled.

Decorative images have no semantic bundle. Groups are used only when grouping
improves navigation; layout containers remain semantically transparent.

### Announcements

use_announce returns an imperative handle that submits one resolved string in
the current successful commit.

~~~rust
let announce = use_announce();
announce.send(trox::tx("Changes saved", "Announcement after saving succeeds."));
~~~

Unity calls SendAnnouncement once after the commit is admitted. Announcements:

- have no politeness setting;
- are not deduplicated or acknowledged;
- are not stored in the semantic snapshot; and
- are not replayed after reconnect.

Empty messages are ignored. Failed or unsupported submission produces a
diagnostic but does not reject the visual commit.

The runtime call owns the announcement queue. send may run only during an
active event/render transaction. A development call outside that transaction
panics. Messages retain call order, including duplicates.

Any response that fails rendering, preflight, or safe-gate admission discards
its messages. An admitted response drains them after hierarchy publication even
when its semantic snapshot is unchanged. An unexpected post-mutation failure
discards messages not yet submitted; they are never retried or reconnected.

## Semantic tree

The semantic tree is a projection of live logical hosts, not the physical UI
Toolkit hierarchy.

### Tree membership and identity

A canonical semantic host contributes:

- its ObjectId;
- its nearest exposed logical semantic ancestor;
- ordered exposed children;
- resolved role, name, description, state, value, and actions;
- its physical host for geometry; and
- whether the host is the wrapper of an authored modal overlay.

Components, fragments, and hosts without SemanticProps are transparent.
NameSourceOnly declarations participate only in name resolution. Hidden
declarations prune themselves and their logical descendants. The canonical
snapshot otherwise retains Exposed declarations even when their VisualElement
is detached, render-hidden, effectively inert, or outside the active modal.
Unity owns those runtime presentation filters.

A **canonical semantic root** is an Exposed host with no Exposed logical
semantic ancestor after transparent and hidden hosts are resolved. Radio and
tab membership handles must remain within one canonical root. Promoting a modal
to an active presentation root does not change this validation boundary.

Default reading order is depth-first logical child order. The design has no
authored reading-order override. Reading order never controls Tab order.

### Portals

A portal changes physical placement but not:

- semantic parentage;
- logical event routing;
- name-source resolution; or
- dialog ownership.

Geometry still comes from the physical VisualElement. Changing a portal target
follows Reactant's existing remount behavior and therefore creates new ObjectId
values. Reconnect recreates Unity objects while preserving surviving logical
ObjectId values.

### Visibility, inertness, and presence

A canonical semantic node is not published by Unity when it or a retained
logical ancestor:

- has display none;
- has UI Toolkit visibility hidden;
- is detached;
- is effectively inert according to the focus coordinator; or
- is outside the active modal subtree.

Opacity alone does not hide semantics. Disabled is also distinct from hidden:
disabled controls remain readable.

Logical removal removes semantics in the same response that begins presence
exit. The retained VisualElement may continue drawing, but it has no
AccessibilityNode callback route.

When a modal overlay is active, Unity promotes the dialog declaration on that
same wrapper host to the only active root. It omits the dialog's canonical
parent edge and filters every node outside the dialog subtree. Descendants keep
their canonical parent edges. Closing the modal restores the canonical roots
after the focus coordinator settles restoration and effective inertness.

In a runtime with accessibility declarations enabled, every authored modal
wrapper has exactly one Exposed dialog declaration on the wrapper itself. A
missing declaration, a dialog on a non-modal host, or more than one semantic
bundle on the host rejects the candidate before Unity derives presentation.

### Geometry

Every exposed node uses its host's current screen-space worldBound. Unity
converts panel coordinates into the platform coordinate convention expected by
AccessibilityNode.

Clipping intersects the frame with physical clipping ancestors. A fully clipped
node is omitted from the active Unity hierarchy until it becomes visible. The
design does not scroll or reveal a node in response to accessibility focus.

Motion may change geometry without a Rust render. The Unity manager uses a frame
getter or refreshes the current host bounds when Unity requests the frame.
Geometry changes do not send semantic protocol traffic.

## Actions and focus

Unity accessibility callbacks are direct inputs. They do not create a separate
controlled-value transaction.

### Action dispatch

The Unity manager handles a callback in this order:

1. Validate the active backend generation.
2. Resolve the target ObjectId in the current semantic mirror.
3. Confirm that the node is published, enabled, and declares the callback.
4. Dispatch the normalized action to the owning logical host.
5. Run the synchronous Rust render.
6. Admit the complete response through the existing safe response gate.

The target-default action handler returns ActionDisposition::Handled or
ActionDisposition::Unhandled. Pattern-hook callbacks returning unit are wrapped
as Handled after the callback completes. Capture and bubble observers may stop
further propagation but do not make an unhandled target action handled.

The callback returns handled when the current target-default handler returns
Handled and the complete runtime response passes synchronous admission
validation. Admission may queue mutation until the UI Toolkit event stack
unwinds; the callback does not wait for mutation. A handled callback may render
no state change.

If the safe response gate cannot accept the response, the callback returns not
handled. Runtime failure, stale identity, hidden state, missing action, or
Unhandled also returns not handled and leaves the previous committed state
active.

Accessibility events use the existing logical host event route, including
capture and bubble behavior already supplied by Reactant. There is no second
semantic event graph.

### Direct action behavior

The normalized actions behave as follows:

| Unity callback | Allowed semantic use | Rust result |
| --- | --- | --- |
| Invoke | Button and choice controls | Ordinary activation |
| Increment | Slider | Clamped value change |
| Decrement | Slider | Clamped value change |
| Dismiss | Active dialog | Ordinary close intent |
| Scroll | Available scroll direction | Ordinary scroll intent |

The design has no text, selection, arbitrary range, custom, or collection
payloads. AccessibilityNode does not expose a local draft that Rust must roll
back.

### Focus behavior

Reactant distinguishes input focus from accessibility focus:

- UI Toolkit owns input focus and the coordinator enforces modal policy;
- the operating system and Unity own accessibility focus; and
- the accessibility manager observes platform accessibility-focus callbacks for
  diagnostics only.

Version one does not expose a Rust command for moving accessibility focus. It
does not reveal clipped nodes, await focus confirmation, or correlate
accessibility focus with input focus.

Dialog screen changes use Unity notifications to encourage appropriate
assistive-technology navigation without claiming a confirmed focus result.

## Projection, transport, and commit

Rust sends an optional complete semantic snapshot. It is present when resolved
canonical semantics change and absent from visual- or focus-only responses. The
protocol does not contain sparse semantic mutations.

### Rust projection

Projection runs after visual reconciliation identifies stable logical hosts and
before either tree commits:

1. Collect each host's semantic declaration.
2. Validate local role, state, value, action, and live handle declarations.
3. Resolve logical semantic parentage through transparent hosts and portals.
4. Validate radio/tab ancestry, tab-panel roots, and modal dialog hosts.
5. Resolve names and descriptions and reject dependency cycles.
6. Prune Hidden subtrees and omit NameSourceOnly declarations from the canonical
   snapshot.
7. Validate parent, child, and host references.
8. Build a complete ordered canonical snapshot.

Any error rejects the complete visual and semantic candidate. Unity keeps the
previous complete response.

Render visibility, attachment, effective inertness, and active-modal filtering
are not Rust projection steps. Unity derives them from the post-response host
index and the settled focus coordinator.

### Wire snapshot

The wire contract contains resolved values only:

~~~rust
pub struct AccessibilitySnapshot {
    pub commit_sequence: u64,
    pub roots: Vec<ObjectId>,
    pub nodes: Vec<AccessibilityNodeSnapshot>,
}
~~~

Each node contains:

~~~rust
pub struct AccessibilityNodeSnapshot {
    pub id: ObjectId,
    pub parent: Option<ObjectId>,
    pub children: Vec<ObjectId>,
    pub role: SemanticRole,
    pub label: Option<String>,
    pub hint: Option<String>,
    pub state: SemanticState,
    pub value: Option<RangeValue>,
    pub actions: ActionSet,
}
~~~

An ordinary runtime response may also contain zero or more one-shot
announcements. Announcements are not part of snapshots and never replay.

Unity-to-Rust accessibility events contain:

~~~rust
pub struct AccessibilityEvent {
    pub backend_generation: u64,
    pub target: ObjectId,
    pub action: AccessibilityAction,
}
~~~

Unknown fields or enum variants fail the session handshake. Rust and C# protocol
fixtures change together. There is no schema negotiation or compatibility
adapter.

### Unity application order

Unity preflights the complete response against the post-response host index,
then applies it in this order:

1. Suspend hierarchy publication and reject all accessibility callbacks.
2. Apply visual host creation, movement, and updates.
3. Replace the canonical mirror when a snapshot is present; otherwise retain it.
4. Let the focus coordinator settle focus and effective inertness.
5. Derive the active semantic hierarchy from that settled state.
6. Reconcile AccessibilityNode objects by ObjectId.
7. Send screen, layout, and one-shot announcement notifications.
8. Destroy visual hosts no longer retained by presence.

The existing safe response gate prevents mutation during UI Toolkit event
propagation. A failed preflight applies none of these stages. Presentation is
re-derived even when the response has no semantic snapshot because visual
visibility, attachment, or active-modal state may have changed.

Preflight makes the remaining semantic operations non-failing. An unexpected
post-mutation exception deactivates the hierarchy, keeps input gated, and asks
Rust for a complete reconnect snapshot. Unity never republishes a partially
updated hierarchy.

### Reconnect

Reconnect sends the same complete visual and semantic snapshots used for
ordinary state reconstruction. Focus has no separate snapshot.

Unity:

- increments the backend generation;
- recreates visual hosts;
- lets the focus coordinator select the current active modal and run its
  initial-focus fallback;
- rebuilds the semantic mirror with surviving ObjectId values;
- derives active presentation after focus settles; and
- assigns one complete AccessibilityHierarchy.

Announcements do not reconnect. An accessibility callback from the disposed
generation is rejected before target lookup.

## Unity backend

Version one has one assistive-technology backend and one in-memory fallback.

### Manager ownership

One accessibility manager per Reactant runtime owns:

- the current resolved semantic snapshot;
- indexes by ObjectId;
- current active presentation after focus and visibility filtering;
- AccessibilityHierarchy and AccessibilityNode objects;
- direct action callbacks;
- geometry lookup;
- screen, layout, and announcement notifications;
- backend generation; and
- bounded diagnostics.

The manager depends on the live host index and focus coordinator. Application
code cannot mutate the manager.

### Unity mapping

The adapter maps the supported semantic roles exactly as follows:

| Reactant role | Unity representation |
| --- | --- |
| Button | Button |
| Checkbox, switch, radio | Toggle |
| Slider | Slider |
| Tab list | TabBar |
| Tab | TabButton |
| Heading | Header |
| Image | Image |
| Static text, progress | StaticText |
| Scroll area | ScrollView |
| Dialog, tab panel, radio group, group | Container |
| Disclosure | Button |

Name maps to label, description maps to hint, and formatted range text maps to
value. Disabled maps to Disabled. Expanded maps to Expanded. Checkbox, switch,
and radio true state and tab selection map to Selected. Checkbox Mixed and
progress busy remain canonical mirror states but have no Unity state mapping.
They are omitted with a diagnostic; Reactant does not invent a localized
phrase.

The mapping must never add an action that Reactant did not declare. When an
exact state is unavailable, the adapter emits a bounded diagnostic and uses the
mapping above. It does not reject the semantic commit or run a cross-language
capability classifier. If the pinned platform fixture disproves a mapping, the
design and mapping fixture must be amended before Unity backend implementation
begins.

### Notification policy

Notifications are deterministic and coalesced per successful response:

| Transition | Notification |
| --- | --- |
| Active root vector changes | One ScreenChanged after publication |
| Reconnect or screen reader turns on | One ScreenChanged after publication |
| Active node content or order changes | One LayoutChanged per root |
| Geometry changes without membership change | None; frame lookup stays live |
| One-shot announcement | One SendAnnouncement call per queued message |

ScreenChanged targets the first active root, or no node when no active root
exists. Closing a modal therefore targets the first restored canonical root.
LayoutChanged targets its affected active root. When a response changes both
the active root vector and descendant content, ScreenChanged subsumes every
LayoutChanged for that response. Duplicate notifications for the same root and
kind coalesce.

### Backend lifecycle

UnityAccessibilityBackend owns AssistiveSupport.activeHierarchy on iOS and
Android while a screen reader is active.

When screen-reader status turns off, Unity:

- clears the active hierarchy;
- increments the backend generation; and
- retains the in-memory semantic mirror.

When status turns on, Unity:

- increments the generation again;
- rebuilds nodes from the retained mirror;
- assigns one complete hierarchy; and
- sends one screen-change notification.

The Editor, unsupported players, and headless tests retain the in-memory mirror
without publishing assistive-technology nodes. There is no custom editor window,
plugin ABI, WebGL DOM tree, or direct native accessibility backend.

Supported assistive-technology players are iOS and Android. macOS, Windows,
Linux, consoles, and WebGL report the backend as unavailable while ordinary
keyboard, controller, pointer, touch, and semantic test behavior continue.

## Validation and diagnostics

Invalid semantic declarations are developer errors. Development and test builds
panic before commit. Production reports the failure and keeps the previous
complete visual and semantic response active.

Validation rejects:

- more than one semantic bundle on one host;
- an empty required name;
- a missing, removed, foreign-runtime, or cyclic name source;
- a role with unsupported state, value, or actions;
- Mixed checked state on a switch;
- a range with invalid bounds or a value outside its range;
- a radio outside a radio group;
- a tab outside a tab list;
- multiple selected radios in one group;
- a tab list without exactly one selected enabled tab and one Exposed panel;
- a stale or cross-root tab-panel handle;
- a Hidden or NameSourceOnly declaration with state, value, or actions;
- a dialog declaration on a host that is not an authored modal wrapper;
- an authored modal wrapper without exactly one Exposed dialog; and
- a snapshot whose parent, child, or host reference is inconsistent.

Warnings cover:

- a published mapping that loses an unsupported state;
- a fully clipped semantic node;
- a description that duplicates the name;
- an exposed zero-opacity duplicate; and
- an unsupported backend or failed announcement.

Each diagnostic includes ObjectId, logical host path, source location when
available, and a concrete correction. Localized application text is omitted
unless verbose accessibility diagnostics are enabled.

**Ditto** is the repository's production-path scenario runner. It can inspect
the production in-memory mirror. No separate editor inspector or diagnostic
semantic tree is created.

## Test architecture

Tests exercise behavior at the layer that owns it.

### Rust tests

Black-box hook tests render public examples and inspect the resolved semantic
snapshot. The focused suite covers:

- every retained hook's representative role, name, state, value, and actions;
- explicit, referenced, and contents-derived names;
- one invalid name cycle and representative role/state failures;
- logical parentage through a portal;
- stable ObjectId through reorder and reconnect;
- semantic removal before presence exit;
- direct action dispatch to the ordinary logical host route.

### Protocol and Unity tests

One shared Rust/C# fixture contains:

- a complete semantic snapshot;
- a direct action event;
- a reconnect generation; and
- a one-shot announcement.

Unity Editor tests cover normative role mapping, hierarchy replacement,
screen-reader off/on reconstruction, stale-generation rejection, host geometry,
modal filtering from settled focus-coordinator state, notification coalescing,
dialog presentation, and unsupported-platform selection.

### Ditto tests

Ditto exposes only the primitives needed by the retained scope:

~~~yaml
- accessibility_assert:
    target: { role: slider, name: Music volume }
    role: slider
    name: Music volume
- accessibility_action:
    target: { role: slider, name: Music volume }
    action: increment
~~~

The action enters through the production Unity callback adapter. It never calls
an application handler directly.

The focused suite covers one settings screen with a button, toggle, radio group,
slider, tabs, dialog, disclosure, progress indicator, announcement, portal,
reconnect, and presence exit. It does not duplicate every behavior at every
layer.

Routine CI does not build mobile players, attach a screen reader, or run large
semantic benchmarks. Accessibility joins existing Rust and Unity test
invocations and should not add another Unity startup.

## Acceptance scenarios

The following scenarios define the complete version-one behavior.

### Common settings controls

A custom-painted Save button exposes one button named "Save changes." Pointer,
touch, Enter, Space, controller submit, and accessibility invoke each produce
one logical press. Disabled state keeps the button readable and suppresses
every activation path.

A checkbox and switch expose their current checked state. Accessibility invoke
requests one Rust state change and the later committed snapshot contains the
authoritative result.

### Slider and progress

A custom music-volume slider exposes its numeric range and localized percent
text. Accessibility increment and decrement use the declared step and clamp to
the range. Pointer and controller changes use the ordinary interaction system
and produce the same committed semantic value.

A progress indicator exposes a determinate value or busy state and never
announces every frame.

### Tabs and radio groups

A native settings tab list and radio group retain their UI Toolkit composite
navigation. Custom tabs and radios are ordinary sequential focus targets and
change selection only when activated. Accessibility adds no navigation state in
either case. Settled application selection updates semantic selected state.

Only the selected tab panel is published. A deselected panel leaves the semantic
tree before its exit animation.

### Dialogs, portals, and presence

A portaled settings dialog retains logical semantic ancestry while using its
physical host for geometry. Opening it publishes only the active modal subtree
and sends one screen-change notification.

Escape, controller cancel, an authored close button, and Unity dismiss request
the same Rust close intent through their owning systems. The modal coordinator
owns initial focus, containment, and restoration.

Closing removes dialog semantics before its retained visual hosts animate out.

### Reconnect and backend activation

Reconnect recreates Unity objects, preserves surviving ObjectId values, rejects
callbacks from the old backend generation, and assigns one complete hierarchy.
It does not replay announcements.

Turning the screen reader off clears the active hierarchy. Turning it on
rebuilds from the retained semantic mirror without requiring a Rust render.

## Completion criteria

The subsystem is complete when:

- the retained hook examples compile and preserve rendering independence;
- each exposed semantic node belongs to exactly one live host;
- portals preserve logical semantic ancestry;
- names and descriptions resolve in Rust before transport;
- invalid semantic candidates cannot partially commit;
- Unity publishes normative roles, labels, hints, values, states, and direct
  actions on iOS and Android and diagnoses canonical-only Mixed and busy state;
- every accessibility callback targets the current ObjectId and backend
  generation;
- accessibility uses no second input-focus, modal, restoration, navigation, or
  focus-visible system;
- presence removes semantics at logical exit;
- reconnect restores one complete hierarchy without replaying announcements;
- Ditto inspects and acts through the production semantic mirror;
- focused Rust, protocol, and Unity Editor tests pass; and
- one initial VoiceOver and TalkBack settings-screen check succeeds.

## Rejected alternatives

### Infer semantics from UI Toolkit types

Native control types and visible text are rendering choices. Explicit behavior
hooks keep custom and native visuals consistent and prevent misleading inferred
names.

### Build a universal semantic model now

Relationships, virtual nodes, rich collections, text editing, custom actions,
and future platform mappings each introduce substantial identity, protocol,
validation, and lifecycle work. The retained mobile settings patterns do not
need that infrastructure.

### Add a controlled accessibility proposal protocol

The supported AccessibilityNode callbacks do not mutate a local UI Toolkit value
that must be rolled back. Direct synchronous actions already return the
authoritative Rust render and handled result.

### Programmatically manage accessibility focus

Unity's notification APIs do not provide a reliable focus completion result.
Observing platform focus and sending screen-change notifications avoids a
timeout and reveal state machine that the retained patterns do not require.

### Reduce semantics to focus order

Static text, headings, groups, images, and disabled controls can remain readable
without entering the Tab sequence. Semantic reading order and input focus remain
separate even in the smaller model.

## Manual QA

Manual device QA is a short boundary check before the first accessibility
release and after changing the pinned Unity version or mapping code.

Use one current iOS device with VoiceOver and one supported Android device with
TalkBack. On each device:

1. Open the representative settings screen.
2. Confirm the screen reader finds a heading, button, toggle, radio group,
   slider, tabs, selected tab panel, disclosure, informative image, static text,
   scroll area, and progress indicator in plausible logical order. Confirm the
   group improves structure without creating an extra action.
3. Activate the button and toggle, select a radio and tab, and adjust the
   slider. For custom radios and tabs, confirm each item is an ordinary Tab
   target and selection changes only on activation. Expand the disclosure and
   scroll the scroll area in both available directions. Confirm each action
   changes application state exactly once.
4. Open the portaled dialog. Confirm background nodes disappear from traversal,
   dismiss works, and input focus remains contained by the focus coordinator.
5. Close the dialog while its exit animation runs. Confirm it is no longer
   discoverable or actionable.
6. Trigger one announcement and confirm it is submitted once.
7. Turn the screen reader off and on. Confirm one hierarchy returns without
   duplicate nodes.
8. Reconnect the runtime. Confirm the current settings return and a stale
   callback cannot change state.

Record the commit, Unity version, device and OS, screen-reader version, and any
unexpected behavior. Add a focused automated regression at the lowest useful
layer for each discovered defect.
