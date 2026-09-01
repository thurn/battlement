# Reactant Events and Default Actions

Reactant needs event handling that feels familiar to a Rust author porting an
ordinary React interface. It must also tell the truth about where decisions are
made. Unity UI Toolkit dispatches input and runs many default actions while its
event callback is active. A Rust callback may run across an asynchronous
transport and cannot retroactively cancel that work.

This design splits the system at that timing boundary:

- Unity makes every event-time decision synchronously from state installed by
  the last Reactant commit.
- Rust performs logical capture and bubble propagation after delivery.
- Reactant does not add a browser-shaped `prevent_default()` method.
- An author who must prevent a native default declares a closed, serializable
  policy before the input occurs.
- Native controls and Motion keep their higher-level Unity-local state
  machines. Reactant does not suppress and later replay their behavior.

The result is portable across in-process and asynchronous transports. The same
component cannot accidentally depend on a synchronous Rust callback that only
some hosts can provide.

## Related information

- [Reactant technical design](reactant-technical-design.md) defines sessions,
  commits, host façades, and the Rust-to-Unity boundary.
- [Reconciliation, events, and portals](reconciliation-events-and-portals.md)
  defines committed identity and the
  existing logical propagation API. This document supersedes its event timing
  and stale-event rules.
- [Hooks and effects](hooks-and-effects.md) defines state batching and callback
  snapshots.
- [Refs and geometry](refs-geometry-and-floating-ui.md) defines delayed host
  actions, including focus and pointer-capture requests.
- [Reactant Animations](animations.md) defines Motion gestures, dragging,
  velocity, constraints, momentum, and coalesced samples.
- [Battlement UI technical design](../battlement-ui-technical-design.md) defines
  the Unity host, wire values, controlled controls, and fake client. This
  document supersedes its assumption that Rust event handling is always
  synchronous.
- [Controller input](../controller-input.md) defines the existing global
  gameplay controller actions. Captured UI input remains separate from them.
- [Ditto technical design](../ditto-technical-design.md) defines production
  input injection and observable player evidence.
- [Unity event handling][unity-events] defines target selection, trickle-down,
  bubble-up, default-action ordering, propagation stopping, and cancellation.
- [Unity pointer capture][unity-capture] defines native pointer ownership and
  capture transition events.
- The settings mockup in `~/Documents/mockups` at commit
  `2451ea9cc6f76b356b1102ee37b82c478853122a` is behavioral evidence only. Its
  browser event calls and DOM structure are not architecture.

[unity-events]: https://docs.unity3d.com/Manual/UIE-Events-Handling.html
[unity-capture]: https://docs.unity3d.com/Manual/UIE-capture-the-pointer.html

## Design goals and constraints

The system must let ordinary components predict capture, target, and bubble
ordering without knowing their physical Unity placement. It must preserve that
logical behavior through portals and must make event metadata sufficient for a
bug report after the native callback has finished.

The system must also preserve UI Toolkit behavior that Reactant cannot safely
reimplement: text composition, selection, native control manipulation, pointer
tracking, focus movement, and scroll inertia. Prevention is available only
where Unity can decide synchronously from a finite descriptor.

The fixed constraints are:

- Rust event callbacks may cross an asynchronous boundary.
- A committed host state is the only Rust-authored state Unity may consult
  during native dispatch.
- One native event never waits for a Rust callback.
- One accepted event runs all of its Rust handlers before reconciliation.
- A session has one total FIFO order for reliable input boundaries.
- Physical UI Toolkit listeners outside Reactant remain native listeners and
  are not controlled by Rust logical propagation.
- Policies are data. They never contain Rust closures or arbitrary scripts.
- The Rust and C# wire models change atomically. No compatibility layer or
  protocol version is introduced.

## Core concepts

A **logical route** is the Reactant host path from the logical root to the
event target, inclusive. Reactant authors this parentage. A portal changes a
host's physical Unity parent but not its logical route.

A **native default** is UI Toolkit or control-adapter work that occurs because
of input, such as editing text, moving a slider, focusing a button, or scrolling
a view. A **normalized default** is Reactant-owned native work that makes one
behavior consistent across controls, such as label activation or modal focus
containment.

A **native event policy** is committed, serializable data that matches a closed
set of input properties and selects a synchronous native disposition. Policies
can prevent a default, stop native propagation, or do both. They do not invoke
application code.

A **reliable boundary** changes ownership or semantic state. Pointer down, up,
cancel, capture changes, focus changes, key transitions, activation, value
commit, and captured-input transitions are reliable boundaries. They are never
coalesced or dropped.

A **replaceable sample** reports an intermediate state that a later sample
fully supersedes. Pointer moves, live scroll changes, Motion drag samples, and
other explicitly marked samples may be coalesced.

## Ownership matrix

| Behavior | Synchronous owner |
|---|---|
| Text editing, IME, caret, selection | UI Toolkit control |
| Native focus-ring movement | UI Toolkit control |
| Native control activation | UI Toolkit control |
| Native slider tracking | UI Toolkit control |
| Control-internal pointer capture | UI Toolkit control |
| Wheel and touch scrolling, inertia | UI Toolkit control |
| Logical click activation | Reactant host adapter |
| Bubbling focus and blur | Reactant host adapter |
| Pointer enter and leave crossing | Reactant host adapter |
| Controlled value proposals | Reactant host adapter |
| Label focus, activation, naming | Reactant host adapter |
| Modal focus containment | Reactant host adapter |
| Keyboard and controller capture | Reactant host adapter |
| Disabled and hidden gating | Reactant host adapter |
| Portal-aware logical routing | Reactant host adapter |
| Gesture recognition and drag | Motion in Unity |
| Drag velocity, constraints, momentum | Motion in Unity |

UI Toolkit retains the complete native state machine for its controls. A text
field consumes editing keys and navigation needed for caret movement. A slider
tracks its value and pointer. A scroll view owns wheel, touch scrolling, and
inertia. Their adapters emit typed proposals and notifications at the defined
boundaries.

Reactant normalizes behavior that depends on logical structure or portable
semantics. The Unity host performs the synchronous half from committed
descriptors. Rust later receives the corresponding logical event and updates
application state.

Motion retains Unity-local recognition, pointer capture, translation, velocity,
constraints, and momentum. Rust receives reliable start, end, cancel, and
completion boundaries plus coalesced samples. A Rust callback cannot cancel a
drag that Unity has already recognized.

## Public Rust event API

`ReactantEvent<E>` remains an immutable view over one shared event. Its only
mutating operation is logical `stop_propagation()`.

```rust
pub enum EventOrigin {
    Native,
    NativeNormalized,
    NativeCleanup,
    RustSynthetic,
}

pub enum NativeControlKind {
    Button,
    Toggle,
    Radio,
    Dropdown,
    Slider,
    TextField,
    ScrollView,
    TabView,
}

pub enum DefaultActionState {
    NotCancelable,
    Allowed,
    PreventedByPolicy {
        policy_owner: ElementTarget,
    },
    HandledByNativeControl {
        control: NativeControlKind,
    },
}

pub enum NativeInputOwner {
    None,
    NativeControl(NativeControlKind),
    Motion,
    InputCapture(ElementTarget),
}

pub enum NativeBubbleDisposition {
    Continued,
    StoppedByPolicy {
        policy_owner: ElementTarget,
    },
    StoppedByNativeControl {
        control: NativeControlKind,
    },
}

pub enum LogicalBubbleDisposition {
    Allowed,
    SuppressedByNativeControl(NativeControlKind),
}

impl<E> ReactantEvent<E> {
    pub fn payload(&self) -> &E;
    pub fn target(&self) -> ElementTarget;
    pub fn current_target(&self) -> ElementTarget;
    pub fn phase(&self) -> EventPhase;
    pub fn cancelable(&self) -> bool;
    pub fn default_action_state(&self) -> DefaultActionState;
    pub fn origin(&self) -> EventOrigin;
    pub fn native_input_owner(&self) -> NativeInputOwner;
    pub fn native_bubble_disposition(&self) -> NativeBubbleDisposition;
    pub fn native_propagation_stopped(&self) -> bool;
    pub fn logical_bubble_disposition(&self) -> LogicalBubbleDisposition;
    pub fn stop_propagation(&self);
}
```

`cancelable()` describes whether the native event admitted prevention. It does
not imply that Rust can prevent it. `default_action_state()` records the result
already chosen by Unity. The native-input owner, physical native propagation,
and logical bubble disposition are independent read-only dimensions.
`native_propagation_stopped()` is shorthand for testing whether
`native_bubble_disposition()` is not `Continued`.

`stop_propagation()` affects only later Reactant callbacks for the current
logical dispatch. It cannot stop UI Toolkit callbacks, physical ancestors,
native defaults, another already queued event, or application code outside
Reactant. Reactant does not add `prevent_default()`.

`DefaultActionState::HandledByNativeControl` means an intrinsic control ran its
default. `NativeInputOwner::NativeControl` can still classify an input when a
policy prevents that default. Ownership prevents generic ancestor behavior
from reinterpreting the input, even if the control did not mutate a visible
value. For example, a text field can own Left Arrow at the beginning of its
text.

The default-state choice is deterministic. `NotCancelable` wins for a
non-cancelable event. Otherwise matched prevention yields
`PreventedByPolicy`; otherwise an intrinsic default that ran yields
`HandledByNativeControl`; all other cases yield `Allowed`. Independent owner
and policy fields retain the facts not represented by that mutually exclusive
result.

For physical propagation, policy stopping is reported as
`StoppedByPolicy` when any policy requested it; otherwise a control stop is
`StoppedByNativeControl`; otherwise the result is `Continued`. The separate
logical-bubble value never changes this physical result.

## Declarative native policies

Policies use typed selectors and a shared disposition. The public shape is
closed so the Rust lowering and Unity evaluator can validate identical rules.

```rust
pub enum NativeEventDisposition {
    PreventDefault,
    StopNativePropagation,
    PreventDefaultAndStopNativePropagation,
}

pub enum ModifierMatch {
    Any,
    None,
    Exactly(KeyModifiers),
}

pub enum RepeatMatch {
    FirstOnly,
    RepeatsOnly,
    FirstAndRepeats,
}

pub struct KeyDownPolicy {
    pub keys: KeySet,
    pub modifiers: ModifierMatch,
    pub repeat: RepeatMatch,
    pub disposition: NativeEventDisposition,
}

pub enum NavigationIntent {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    Submit,
    Cancel,
    Next,
    Previous,
}

pub struct NavigationPolicy {
    pub intents: NavigationIntentSet,
    pub source: NavigationSourceMatch,
    pub disposition: NativeEventDisposition,
}

pub enum WheelAxisMatch {
    Horizontal,
    Vertical,
    Either,
}

pub struct WheelPolicy {
    pub axis: WheelAxisMatch,
    pub modifiers: ModifierMatch,
    pub disposition: NativeEventDisposition,
}

pub enum PointerPolicyPhase {
    Down,
    Move,
    Up,
    Cancel,
}

pub struct PointerPolicy {
    pub phases: PointerPolicyPhaseSet,
    pub buttons: PointerButtonSet,
    pub pointer_types: PointerTypeSet,
    pub disposition: NativeEventDisposition,
}

pub struct NativeEventPolicy {
    pub key_down: Vec<KeyDownPolicy>,
    pub navigation: Vec<NavigationPolicy>,
    pub wheel: Vec<WheelPolicy>,
    pub pointer: Vec<PointerPolicy>,
}
```

`KeySet`, the set wrappers, and modifier values serialize as canonical sorted
sets. Duplicate members, empty selectors, unsupported keys, and overlapping
same-node entries with different dispositions are developer errors rejected
before commit. Canonical values make Rust and C# fixtures byte-for-byte stable.

Pointer button matching is phase-specific. Down and up match the button that
changed. Move matches when any selected button is currently pressed. Cancel
matches the initiating button of the capture or gesture being canceled; a
cancel with no initiating button matches only `PointerButtonSet::any()`.
Pointer state in the payload includes both the optional changed button and the
complete pressed-button set so Rust, Unity, and the fake evaluator agree.

Every host façade that can receive the relevant input provides:

```rust
impl<G> View<G> {
    pub fn native_event_policy(
        self,
        policy: NativeEventPolicy,
    ) -> Self;
}
```

The method represents the common façade capability; concrete host façades
return their own builder type. Multiple builder calls replace the same policy
slot. Authors compose entries in one `NativeEventPolicy` rather than relying on
builder order.

Policies match the event-time logical route from target toward root. Prevention
is additive: if any matched entry requests prevention, the default is
prevented. Native propagation stopping is also additive. For diagnostics, the
**policy winner** for each disposition bit is the nearest matching logical
node. If entries on that node are equivalent, the canonical kind and selector
order breaks the tie. This order is deterministic and has no behavioral effect
beyond attribution because matched bits are combined with logical OR.

Policies cannot suppress delivery to Rust. A prevented key event still enters
logical capture and target handling unless an intrinsic native-control rule
marks logical bubble as suppressed. This lets a component consume a native
default and still update application state later.

Prevention against a non-cancelable event has no effect on its default and
records `NotCancelable`. A native-propagation bit in the same policy still
applies where UI Toolkit permits. The inspector records the ineffective
prevention match so the declaration is not silently misleading.

## Pointer capture policy

Portable custom drags need capture during the initiating native callback. A
delayed `ElementRef::capture_pointer()` request cannot promise that timing.

```rust
pub enum PointerCapturePolicy {
    None,
    OnPointerDown {
        buttons: PointerButtonSet,
        pointer_types: PointerTypeSet,
    },
}

impl<G> View<G> {
    pub fn pointer_capture_policy(
        self,
        policy: PointerCapturePolicy,
    ) -> Self;
}
```

Unity evaluates capture claimants along the logical target-to-root route. The
nearest matching node wins. At the same node, an intrinsic control claimant
wins over Motion, and Motion wins over generic capture. A host cannot declare
both Motion pointer initiation and generic capture for the same input;
canonical validation rejects that same-priority conflict before commit.

A matching `PointerPolicy` is evaluated before an intrinsic pointer default.
Its prevention bit can block control activation or tracking. An explicitly
declared Motion or generic capture claimant is not a default and still captures;
authors commonly combine prevention with custom capture. Native propagation
stopping remains independent. The inspector records both the selected capture
claimant and each policy outcome.

Capture begins before the pointer-down callback returns. Subsequent pointer
events target the capturing host while preserving the original pointer ID.
Unity releases capture on matching up, cancel, explicit release, host disable,
`display: none`, detach, removal, document blur, or reconnect. It emits one
reliable capture-loss boundary where the session still exists.

`ElementRef::capture_pointer()` remains available for imperative work. A
request made from Rust runs in a later Unity commit and carries no guarantee
for the first move or up event. Diagnostics identify it as delayed capture.
Motion uses its own control-specific capture contract and does not lower to
this generic policy.

## Focus scopes

A focus scope is logical state lowered to Unity. A modal scope owns containment
and restoration without waiting for Rust.

```rust
pub enum InitialFocus {
    FirstFocusable,
    Control(ControlRefId),
    Container,
}

pub enum FocusRestore {
    PreviouslyFocused,
    Control(ControlRefId),
    None,
}

pub struct FocusScope {
    private: FocusScopeState,
}

impl FocusScope {
    pub fn modal() -> Self;

    pub fn initial_focus(self, value: InitialFocus) -> Self;

    pub fn restore(self, value: FocusRestore) -> Self;
}

impl<G> View<G> {
    pub fn focus_scope(self, scope: FocusScope) -> Self;
}
```

`FocusScope::modal()` defaults to `FirstFocusable` and
`PreviouslyFocused`. When installed, Unity records the currently focused
eligible host, focuses the declared initial target after attachment, and limits
native keyboard and controller focus movement to eligible logical descendants.
Portaled descendants count; unrelated physical descendants do not.

V1 exposes only modal scopes. `FocusScope` is public but opaque; authors cannot
construct a non-modal state or mutate its fields. A requested initial control
falls back to the first eligible focusable descendant, then an eligible scope
container, then clear focus. `InitialFocus::Container` falls back to the first
eligible focusable descendant and then clear focus.

Cancel input such as Escape remains an application decision, but a modal scope
prevents it from escaping to a lower focus scope or gameplay. The modal's
`NavigationIntent::Cancel` event is delivered to Rust. Removing the modal later
restores its recorded focus target if that target is still attached, visible,
enabled, and focusable. Otherwise Unity chooses the nearest eligible logical
ancestor and then the first focusable host in the newly active scope. Failure
to find one leaves focus clear and records a diagnostic.

The modal descriptor lowers a mandatory cancel-prevention and native-stop rule
owned by the scope root. It is installed with the focus state and appears as a
policy winner in event metadata. Authors may handle cancel but cannot let it
fall through while the modal is active.

For `FocusRestore::Control`, Unity first tries the requested control, then its
nearest eligible logical ancestor, then the first focusable host in the newly
active scope. `FocusRestore::None` clears focus unless the newly active modal
scope must establish its own initial focus.

Nested modal scopes form a stack in commit order. Only the topmost attached
modal is active. Equal-depth portaled scopes are ordered by logical root
registration and source-tree order, the same stable order used for portal
ranges. Moving a scope retains its restoration record; removing and recreating
it creates a new record.

The active modal scope also limits accessibility traversal to its logical
descendants and exposes native modal semantics on its container. Background
content remains rendered but is inaccessible to focus, activation, and
assistive navigation until the modal leaves. Portaled descendants retain their
source-side accessible ownership.

## Keyboard and controller input capture

Input rebinding is a specialized exclusive mode, not a generic event handler.
It must claim an input before a focused button, UI navigation, or gameplay can
use it.

```rust
pub enum CapturedPhysicalInput {
    Key(PhysicalKey),
    ControllerButton {
        controller: ControllerId,
        button: ControllerButton,
    },
}

pub enum CapturedInputPhase {
    Pressed,
    Released,
}

pub struct CapturedInputEvent {
    pub input: CapturedPhysicalInput,
    pub phase: CapturedInputPhase,
    pub modifiers: KeyModifiers,
    pub repeat: bool,
}

pub enum InputCaptureKeys {
    NonModifierKeys,
    Explicit(KeySet),
}

pub struct InputCapture {
    pub keys: InputCaptureKeys,
    pub controller_buttons: ControllerButtonSet,
}

impl InputCapture {
    pub fn rebinding() -> Self;
}

impl<G> View<G> {
    pub fn input_capture(self, capture: InputCapture) -> Self;

    pub fn on_captured_input_event(
        self,
        handler: impl Fn(&mut G, ReactantEvent<CapturedInputEvent>) + 'static,
    ) -> Self;
}
```

`InputCapture::rebinding()` matches every non-modifier physical keyboard key
and every supported controller button. Modifier-only transitions update the
modifiers included with a later key but do not complete a capture.

An input-capture descriptor is active only when its host is eligible and is a
logical ancestor of the focused host in the topmost active modal scope. With no
modal, it must be an ancestor of the focused host in that root. The nearest
matching ancestor wins. If focus is clear, only the active modal root may own
capture. A host has one replaceable capture slot, so equal-depth candidates
cannot exist on one focus path. Cross-root candidates are inactive because one
panel has only one focused Reactant host.

Unity marks a matched press as captured before intrinsic control handling. It
prevents default behavior, stops native UI propagation, and withholds the input
from global controller actions. The corresponding release remains suppressed
even if Rust removes the capture owner after receiving the press. Unity retains
a small per-device release latch until every captured control is released or
the session ends. The release boundary is always retained and delivered on the
wire. Rust dispatches its handler only if the original owner remains eligible;
otherwise it records `CapturedOwnerIneligible` in the inspector without
invoking a logical callback. Suppression never depends on handler delivery.

A claimed press produces `CapturedInput` instead of ordinary key, navigation,
activation, or controller-action envelopes. Its claimed release likewise
produces only `CapturedInput`. This replacement prevents two Rust handler
families from interpreting one physical transition.

An active input capture lowers an implicit mandatory native event policy at
its owner. The event therefore reports `PreventedByPolicy` and
`StoppedByPolicy` with that host as both policy winners, while
`NativeInputOwner::InputCapture` identifies why the policy existed. No extra
default or propagation variant is required.

Captured controller payloads are a new UI event family. They do not reuse or
merge with existing global `ControllerAction` messages. This prevents a bind
operation from also navigating a menu or triggering gameplay.

## Labels and native controls

Label behavior is a declared relationship rather than an ordinary nested click
handler. It provides synchronous focus, activation, and accessible naming.

```rust
pub trait LabelControl: private::Sealed {}

pub struct ControlRef<C: LabelControl> {
    private: ControlRefState,
    marker: PhantomData<C>,
}

impl<C: LabelControl> Label<C> {
    pub fn for_control(
        text: impl Into<String>,
        control: ControlRef<C>,
    ) -> Self;
}
```

Each compatible native façade accepts a matching `ControlRef<C>`. V1
implements `LabelControl` for buttons, toggles, radio controls, dropdowns,
sliders, and text inputs. It is absent for plain views and output-only text.
Cross-session and cross-runtime relationships are developer errors.

On primary activation, Unity focuses the related control and invokes its native
activation exactly once. The label then emits one normalized logical click
whose target is the label. The control emits its ordinary value proposal or
activation event. Nesting the control physically inside the label does not add
a second activation.

Unity assigns the label click the first normalized sequence after its raw
activation boundary and names that raw sequence as its source. Focus-out and
focus-in envelopes caused by the relationship follow the label click. The
control activation or value proposal follows those focus envelopes and names
the label-click sequence as its source. Unity has completed the synchronous
default before delivery, but Rust always observes this normalized semantic
order. A control that was already focused simply omits the focus envelopes.

The relationship supplies the control's accessible name when the control has
no explicit accessible label. An explicit control name wins. The label remains
an accessible relationship even when a portal separates the physical hosts,
provided both are in the same panel and session.

A disabled, hidden, detached, or incompatible control makes label activation a
no-op. The label may still receive its own non-activation pointer events when
it is otherwise eligible. Validation rejects relationship cycles and more than
one primary label for the same control. Additional descriptive text must use a
separate accessibility-description relationship in a future design.

## Native control precedence

Unity evaluates input in this order:

1. Reject ineligible sessions, targets, and routes.
2. Apply an active `InputCapture` claim.
3. Classify intrinsic control ownership without mutating control state.
4. Resolve intrinsic, Motion, and generic pointer-capture claimants.
5. Evaluate every matching policy along the logical route.
6. Run each allowed intrinsic or normalized default.
7. Allow generic focus, scrolling, or activation if still unclaimed.

Input capture is intentionally first because it is an explicit exclusive mode.
Outside that mode, native-control behavior takes precedence over generic
ancestor behavior. A text field's caret navigation wins over an ancestor's
menu navigation. A focused slider's arrow handling wins over a scroll view.

Declarative prevention is additive after ownership classification and before
mutation. A matching policy can prevent an intrinsic or generic default and
can stop native propagation. The control remains the input owner, so prevention
does not let a generic ancestor reinterpret the input. No policy runs after a
control mutation and no state machine is partially rewound.

A native control may return `suppress_logical_bubble`. Reactant still runs
logical capture, target capture, and target bubble handlers, then stops before
the target parent. This is reserved for inputs the control semantically owns,
including text editing and caret movement. It is not a general control setting.
The event records `LogicalBubbleDisposition::SuppressedByNativeControl` so the
missing ancestor callback is explainable. This is independent of physical
`NativeBubbleDisposition`.

### Intrinsic control profiles

Intrinsic ownership is a closed adapter contract, not an arbitrary C# callback.
Each host state carries one profile:

```rust
pub enum UiIntrinsicInputProfile {
    None,
    Button,
    Toggle,
    Radio,
    Dropdown,
    Slider,
    TextField,
    ScrollView,
    TabView,
}
```

Unity combines the profile with native state that the adapter exclusively
owns, such as an open dropdown, text composition, slider drag, or remaining
scroll range. The fake client models the same state transitions. Shared outcome
fixtures enumerate every profile, input family, and relevant state flag.

All profiles first apply disabled, hidden, and detached gating. A policy may
prevent an owned cancelable default before mutation; ownership still blocks
generic reinterpretation. The exhaustive profile rules are:

- `Button` owns keyboard or controller Submit and a valid primary-pointer
  click. Its default emits one activation. Submit key events stop physical
  native propagation and suppress logical ancestor bubble. The normalized
  click itself logically bubbles.
- `Toggle` and `Radio` use the button rules, then emit one committed value
  proposal. A radio already selected still owns activation but emits no value
  change.
- `Dropdown` uses button rules while closed. While open, it owns Up, Down,
  Home, End, Submit, and Cancel; updates highlight locally; commits once on
  Submit or pointer selection; and closes on Cancel. Owned open-state key and
  navigation events stop physical propagation and suppress logical bubble.
- `Slider` owns arrows, Page Up, Page Down, Home, End, and primary-pointer
  tracking. Each discrete key changes by exactly one declared step or endpoint.
  Owned keys stop physical propagation and suppress logical bubble. Pointer
  tracking uses intrinsic capture and emits typed live and commit proposals.
- `TextField` owns character input, composition, clipboard editing, deletion,
  caret arrows, Home, End, and selection-modified forms. It owns Enter as commit
  for single-line text or newline for multiline text. Tab is generic focus
  navigation unless the native `accept_tab` property is set. Escape is owned
  only while canceling composition. Owned editing keys stop physical
  propagation and suppress logical bubble.
- `ScrollView` owns wheel or touch movement only while it can move on the
  requested axis. At an edge, the unconsumed axis chains to an ancestor. A
  focused scroll view also owns Page Up, Page Down, Home, and End when movement
  is possible. Owned keys suppress physical and logical ancestor propagation;
  consumed wheel and touch use UI Toolkit's native chaining disposition.
- `TabView` owns arrows, Home, and End while its tab strip is focused. It moves
  selection once, stops physical propagation, suppresses logical bubble, and
  emits one tab-selection proposal.
- `None` owns no input and contributes no default, capture, or suppression.

Primary pointer down, move, and up events still logically propagate unless a
declared policy stops them. Intrinsic capture changes their target but does not
by itself suppress their logical bubble. Control-generated value, selection,
and commit proposals remain target-only under their existing typed contract.

The shared fixtures treat every input not listed above as unowned. Adding an
owned input or state flag requires changing this enum, the Rust and C# profile
tables, and their fixtures atomically.

## Text editing and controlled values

`Input` is a post-edit notification. Its payload contains the control draft
after UI Toolkit has applied the edit. No Rust callback can reject that edit
before it appears.

Editing restrictions must be native declarations. Existing properties such as
single-line mode, maximum length, read-only state, and allowed character class
remain control properties. A closed `TextEditPolicy` may add portable filters
that Unity can enforce before mutation:

```rust
pub struct TextEditPolicy {
    pub character_filter: CharacterFilter,
    pub max_utf16_length: Option<u32>,
    pub newline: NewlinePolicy,
    pub paste: PastePolicy,
}

impl<G> TextField<G> {
    pub fn text_edit_policy(self, policy: TextEditPolicy) -> Self;
}
```

`CharacterFilter` is `Any`, `AsciiDigits`, `AsciiHex`, or a canonical set of
Unicode general categories. `NewlinePolicy` is `Allow`, `Reject`, or
`Commit`. `PastePolicy` is `Allow`, `Reject`, or `Filter`. None accepts a
regular expression or Rust callback. IME composition remains allowed only for
`Any` or a category set that accepts every code point in the completed
composition. ASCII filters reject IME enablement before commit. A finite
maximum must be greater than zero.

Controlled text retains the adapter-owned proposal and restore behavior. The
adapter owns the draft, committed value, selection, composition, and exact
restore point, so it can safely propose and restore. That control-specific
state machine is not generic suppression and replay.

`ValueChanging`, `ValueCommitted`, and `ScrollChanged` likewise describe state
already produced by a native control. Rust may accept, clamp, or replace the
next declarative value. It cannot claim that the earlier native action never
happened.

## Activation and navigation

Reactant defines one logical activation from pointer click, keyboard submit,
controller submit, or label activation. Unity emits the normalized activation
only after the intrinsic control determines that activation is valid. Disabled
or hidden targets cannot activate.

One physical input produces at most one activation per eligible control.
Pointer down and up remain separate events. Click is emitted after a valid
primary release. Keyboard or controller submit uses the target control's native
pressed-state timing. A label routes activation to its control and does not
manufacture a second control click.

Unity maintains one activation latch per primary pointer and target generation.
A valid primary down creates it. Default prevention on that down marks it
disarmed. A matching primary up consults its event-time policy; prevention on
the up also disarms it before click generation. Move prevention alone does not
disarm activation, although native movement slop or Motion drag recognition
may do so.

Up emits click only when the latch is still armed, target and generation still
match, the control remains eligible, and its native activation test succeeds.
Up, cancel, capture loss, disablement, hiding, removal, drag recognition, and
reconnect always clear the latch. Cancel never activates, regardless of whether
its own default was prevented. Native propagation stopping without prevention
does not disarm activation.

Pointer-policy changes after down do not rewrite the latched down result; the
up uses the policy installed when up occurs. Keyboard and controller Submit
have no cross-event latch: the first nonrepeat press is one activation decision,
and matching key or navigation prevention suppresses it before mutation.

Navigation has typed intents independent of raw keys. Keyboard arrows, Tab,
Escape, and controller controls may produce navigation intents. Raw `KeyDown`
remains available for key-specific behavior. A host may therefore receive a
raw arrow event and one normalized move event with consecutive dispatch
sequences. Policies can match either family, and the inspector links a
normalized event to its source sequence.

Unity classifies an optional navigation intent before evaluating policy. It
combines matching `KeyDownPolicy` and `NavigationPolicy` bits in one native
decision. The raw key envelope is queued first. If no intrinsic control owns
the intent, Unity then queues the normalized navigation envelope with the next
sequence and the raw sequence as `source_sequence`. Prevention suppresses the
native navigation default but not either Rust envelope. Native stopping applies
once to the physical key callback and both envelopes report that result.

An intrinsic control that consumes the raw input suppresses the corresponding
generic navigation event. This rule makes a slider change once without also
scrolling or moving focus. It also keeps a text field's arrow key from reaching
an ancestor navigation handler.

For arbitrary nested interactive controls, the deepest picked eligible control
is the intrinsic owner. An ancestor control never runs its own activation
default for the descendant's input. Ordinary ancestor logical click handlers
may still observe bubble unless propagation is stopped. A declared label
relationship is the only mechanism that forwards native activation to another
control.

An event policy never grants accessibility semantics. A custom slider, button,
or menu must declare its native role, name, value, enabled state, and supported
actions through the accessibility façade. Native controls contribute these
semantics from their adapter. Activation and navigation policies are validated
against the declared role where a mismatch would create unusable input.

## Physical target mapping and normalized crossings

Pointer capture selects its capturing Reactant host before hit testing. Without
capture, UI Toolkit picks a physical target and the coverage adapter walks its
physical ancestors to the nearest registered Reactant host in that event
island. An internal child created by a native control maps to the owning control
host. An unmanaged child under a Reactant host maps to its nearest registered
ancestor.

The nearest mapped host is authoritative. If it is disabled, hidden, detached,
or otherwise ineligible, Reactant does not skip it to target an eligible outer
host for activation. UI Toolkit overlap and picking order decide which physical
branch wins before Reactant mapping. If no registered eligible host exists, no
`UiEvent` is created and external Unity listeners continue normally.

Keyboard events map to the focused eligible Reactant host. With no such focus,
only an active `InputCapture` at the modal root may receive a key or controller
button; other UI input creates no Reactant event. Navigation uses that same
target. A portaled host maps in its physical event island, then resolves its
Reactant-authored logical route.

When focus moves, Unity settles containment and the native focus target first.
It then queues `FocusOut` for the old target followed by `FocusIn` for the new
target, each with the other host as related target and each with its own route
snapshot and consecutive sequence. Reactant exposes them as bubbling
`on_blur` and `on_focus`. Clearing or establishing focus omits the missing side.

For pointer crossing, Rust derives leave events from the old target upward to
but excluding the lowest common logical ancestor. It then derives enter events
from that ancestor's entering child down to the new target. Leave and enter
events use target phase, share the raw event's state batch, and finish before
the raw over or out propagation. Stopping one synthetic traversal does not stop
the raw event or its complementary crossing event.

## Event-time host state

Each successful Unity commit installs one immutable `ReactantEventHostState`
for the Reactant runtime across all of its panels. It contains:

- the session ID and event-route revision;
- each Reactant host's logical parent and source root;
- enabled, displayed, attached, and event-eligibility bits;
- native event and pointer-capture policies;
- focus-scope and input-capture descriptors;
- label-to-control relationships;
- control kind and intrinsic input capabilities; and
- the native subscription coverage required by logical handlers.

The shared wire model is concrete:

```rust
pub struct ReactantEventHostState {
    pub runtime_id: ReactantRuntimeId,
    pub session_id: SessionId,
    pub route_revision: u64,
    pub cleanup_token: Option<UiCleanupToken>,
    pub nodes: Vec<ReactantEventNode>,
    pub focus_scopes: Vec<UiFocusScope>,
    pub labels: Vec<UiLabelRelationship>,
}

pub struct ReactantEventNode {
    pub id: ObjectId,
    pub generation: u64,
    pub panel_id: UiPanelId,
    pub logical_parent: Option<ObjectId>,
    pub logical_root: ObjectId,
    pub eligibility: UiEventEligibility,
    pub native_control: Option<UiNativeControlKind>,
    pub intrinsic_input: UiIntrinsicInputProfile,
    pub policy: Option<UiNativeEventPolicy>,
    pub pointer_capture: UiPointerCapturePolicy,
    pub motion_pointer_claim: Option<UiMotionPointerClaim>,
    pub input_capture: Option<UiInputCapture>,
    pub subscriptions: UiEventKindSet,
}
```

Unity swaps the complete state only after validating it. A route revision
changes when logical parentage, host generation, eligibility, policy, scope,
capture, label, or control capability changes. A Rust handler-only replacement
does not change the Unity route revision.

The Unity UI manager validates the runtime-global state, builds one lookup shard
per physical panel, and swaps every shard while native event forwarding is
paused on the main thread. Failure leaves all old shards installed. A target
panel's shard includes the global node index and precomputed logical ancestor
indices needed by portals, so it evaluates source-panel policies without
walking or querying the source panel during dispatch.

One runtime has one Reactant focus coordinator above UI Toolkit's per-panel
focus controllers. It records at most one focused Reactant host as
`(panel_id, object_id, generation)`. Focusing a host in another panel clears
the old panel first, then focuses the new panel. Focus events retain their
respective old and new logical routes and use consecutive runtime-global
dispatch sequences.

A modal scope is runtime-global. Its portaled descendants may occupy another
panel, but all nonmodal Reactant panels are gated from focus, activation, input
capture, and accessibility traversal while it is active. Restoration records
the prior panel and host. The two-state focus transaction chooses the pending
panel and host before swapping shards, emits old-panel `FocusOut`, installs all
shards atomically, and emits new-panel `FocusIn`. External Unity focus and
listeners outside the runtime remain outside this contract.

Every policy lookup uses this immutable state. Event callbacks allocate no
route or policy collections. The commit precomputes compact ancestor indices,
canonical match tables, and capture claimants.

`subscriptions` is the canonical union of event kinds required by logical
handler coverage, intrinsic control profiles, Motion, focus scopes, input
capture, pointer capture, and label relationships. Rust computes it; Unity
validates the union and installs exactly that coverage. Unity never infers
subscriptions from a missing field or from current physical ancestry.

## Wire protocol

Rust and C# replace the current `UiEvent` model together:

```rust
pub struct UiEvent {
    pub session_id: SessionId,
    pub dispatch_sequence: u64,
    pub coalesced_sequences: Option<UiSequenceRange>,
    pub route_revision: u64,
    pub target_panel_id: UiPanelId,
    pub target_id: ObjectId,
    pub target_generation: u64,
    pub logical_path: Vec<UiLogicalPathEntry>,
    pub origin: UiEventOrigin,
    pub cancelability: UiEventCancelability,
    pub native_input_owner: UiNativeInputOwner,
    pub default_action: UiDefaultActionState,
    pub native_bubble: UiNativeBubbleDisposition,
    pub logical_bubble: UiLogicalBubbleDisposition,
    pub policy_outcome: UiPolicyOutcome,
    pub pointer_capture_owner: Option<ObjectId>,
    pub cleanup_token: Option<UiCleanupToken>,
    pub captured_delivery: Option<UiCapturedInputDelivery>,
    pub source_sequence: Option<u64>,
    pub body: UiEventBody,
}

pub struct UiLogicalPathEntry {
    pub id: ObjectId,
    pub generation: u64,
}

pub struct UiSequenceRange {
    pub first: u64,
    pub last: u64,
    pub count: u32,
}

pub struct UiPolicyOutcome {
    pub default_winner: Option<ObjectId>,
    pub native_stop_winner: Option<ObjectId>,
    pub ineffective_prevention: bool,
}

pub enum UiCapturedInputDelivery {
    Logical,
    OwnerIneligible,
}

pub enum UiEventStreamItem {
    Event(UiEvent),
    CleanupWatermark(UiCleanupWatermark),
}

pub struct UiCleanupWatermark {
    pub session_id: SessionId,
    pub cleanup_token: UiCleanupToken,
    pub final_dispatch_sequence: u64,
}

pub struct UiEventStreamAction {
    pub stream_item_id: u64,
    pub item: UiEventStreamItem,
}

pub struct UiEventStreamAck {
    pub session_id: SessionId,
    pub stream_item_id: u64,
}
```

`logical_path` is ordered root to target and includes both endpoints. Its final
entry repeats `target_id` and `target_generation`; disagreement rejects the
envelope. Generations make ID reuse detectable without retaining every normal
route revision.

`source_sequence` names the immediate causal envelope. A normalized event may
therefore point to a raw event or to another normalized event. Following the
chain reaches one raw native sequence. Raw and cleanup events use `None`. A
non-coalesced event has no sequence range. A coalesced event's range starts at
the first replaced sample, ends at `dispatch_sequence`, and has the exact
number of represented samples.

Host state adds wire descriptors for:

- Reactant-authored logical event parentage;
- closed native event policies;
- focus scopes and their ordered focusable members;
- keyboard and controller input capture;
- portable pointer capture; and
- label and compatible-control relationships.

`UiEventStreamItem` is FIFO. A cleanup watermark is not an application event
and consumes no dispatch sequence. It follows every cleanup event bearing its
token and records the greatest sequence assigned before it. A token with no
cleanup event still produces a watermark using the last assigned sequence.

### Transport delivery and acknowledgement

The wire-breaking change removes `ActionBody::VisualElement(UiEvent)` and adds
`ActionBody::UiEventStream(UiEventStreamAction)`. It also adds the
Rust-to-Unity command `CommandBody::AcknowledgeUiEvent(UiEventStreamAck)`.
`ActionBody::MotionEvents` remains for the Motion records retained below.

`IBattlementUiHost.SubmitUiEvent` is replaced by nonblocking
`EnqueueUiEvent`. The event bridge calls it from the native callback after
policy evaluation. `BattlementRunner.EmitUiEvent` is replaced by a queue pump;
no Rust transport call occurs while `uiDispatchDepth` is nonzero.

Unity permits exactly one stream item in flight. After the native callback
unwinds and `uiDispatchDepth` reaches zero, the runner submits the queue head
with a monotonically increasing `stream_item_id`. Submission does not remove
the item. An asynchronous transport may leave it in flight across frames while
native callbacks append later items behind it.

Rust handles one `UiEventStreamAction` as one engine action. An application
event produces its one Reactant dispatch and `ReactantCommit`. The response
contains that commit's ordered mutation and action groups, followed by a final
sequential group containing `AcknowledgeUiEvent`. A stale drop, inspector-only
release, or cleanup watermark returns only the acknowledgement when it creates
no other command.

The existing response stream applies every earlier response group on Unity's
main thread before applying the acknowledgement. The acknowledgement verifies
the active session and exact in-flight item ID, removes the queue head, and
schedules the pump for the next safe runner update. The next item cannot enter
Rust before the preceding response, commit, and acknowledgement finish. This
preserves `ReactantCommit`'s no-outstanding-receipt rule.

The Battlement transport either returns one response for the action or ends the
session; the event stream adds no independent retry or replay protocol. A
duplicate, missing, or out-of-order item ID is a framework failure. Disconnect
clears the in-flight item with the old-session queue. The fake host implements
the same submit, response-group, acknowledgement, and next-item ordering.

`UiEventBody` adds `CapturedInput`. Its controller button values remain distinct
from global controller actions. The input payloads used by policies and
normalized routing have these minimum wire fields:

```rust
pub struct UiKeyEvent {
    pub physical_key: PhysicalKey,
    pub logical_key: LogicalKey,
    pub modifiers: KeyModifiers,
    pub repeat: bool,
}

pub struct UiNavigationEvent {
    pub intent: NavigationIntent,
    pub source: NavigationSource,
}

pub struct UiPointerEvent {
    pub pointer_id: i32,
    pub pointer_type: PointerType,
    pub button: Option<PointerButton>,
    pub pressed_buttons: PointerButtonSet,
    pub position: UiPoint,
    pub delta: UiPoint,
    pub related_target_id: Option<ObjectId>,
}

pub struct UiFocusEvent {
    pub related_target_id: Option<ObjectId>,
}

pub struct UiActivationEvent {
    pub source: ActivationSource,
}
```

Value, text, selection, scroll, and Motion payloads remain the typed values
owned by their existing adapters. This design changes their envelope and
ordering, not their value representation.

The C# and Rust definitions use the same canonical validation fixtures. The
fake client validates and evaluates every closed descriptor, but does not claim
to reproduce UI Toolkit text, focus, gesture, or scroll internals.

This change is:

- **wire-breaking:** `UiEvent` and host state change shape;
- **source-additive:** new façade methods and event accessors are added; and
- **semantically breaking:** Rust callbacks no longer rely on a synchronous
  Unity call or current-tree route discovery.

All Rust fixtures, C# fixtures, fake behavior, and callers change in one commit
stack. There is no compatibility shim, negotiation field, protocol version, or
dual event path.

## Ordered event lifecycle

One native input follows this lifecycle:

1. UI Toolkit selects the physical native target.
2. The Reactant coverage listener maps it to one eligible Reactant target.
3. Unity snapshots the committed logical path and route revision.
4. Unity applies input capture, intrinsic-control rules, and matching policies.
5. UI Toolkit and control adapters perform allowed native defaults.
6. Unity records outcomes and classifies the draft as reliable or replaceable.
7. Unity admits the draft, then assigns its next per-session sequence.
8. Unity appends it FIFO or replaces only a matching queue-tail sample.
9. The transport delivers envelopes in sequence order to Rust.
10. Rust validates the session, target generation, and complete logical path.
11. Rust snapshots the accepted path and every applicable handler slot.
12. Rust runs logical capture, target, and bubble propagation.
13. Reactant reconciles all state and model changes once.
14. A later Unity commit installs mutations, actions, and new event host state.

Steps 1 through 8 complete without waiting for Rust. Steps 10 through 13 form
one active Reactant entry. No commit from that entry can affect the native
default that produced it.

Unity assigns one strictly increasing `dispatch_sequence` to every admitted
native and Unity-normalized event in a session. Sequence zero is invalid.
Events produced by a nested Unity callback are queued; the coverage listener
never reenters Reactant.

Coalescing is allowed only when the matching replaceable envelope is the queue
tail. The replacement stays in that position, takes the newest sequence and
payload, and extends `coalesced_sequences` from the first represented sequence
through the newest. If any reliable or nonmatching envelope is later in the
queue, Unity appends the sample when capacity exists. Rust accepts a sequence
gap only when the next envelope's exact coalesced range starts at the expected
sequence and ends at its own sequence.

Rust-derived pointer enter and leave events run as deterministic synthetic
subevents of their owning raw sequence. The inspector records a
`synthetic_ordinal`, starting at one. They finish before Rust starts the next
wire envelope and do not consume a Unity sequence.

Derived subevents snapshot all crossing routes and handlers when their raw
dispatch starts. They share the raw event's state batch and cause no separate
reconciliation. After the derived traversal, the raw event uses its already
snapshotted handlers. A synthetic `stop_propagation()` flag is local to that
traversal.

An application-injected synthetic event enters the same Rust queue behind the
active event. It receives a monotonically increasing Rust-local synthetic
sequence, takes a fresh route and handler snapshot when its own dispatch starts,
and causes its own single reconciliation. It has no Unity dispatch sequence or
native default. It cannot create native focus, editing, scrolling, capture, or
activation. Tests may inject it through the public test dispatcher, but
production behavior must be proven through native input.

## Rust propagation algorithm

At Rust dispatch start, Reactant performs these checks in order:

1. `session_id` equals the active session.
2. The sequence is next, or an exact coalesced range explains every gap.
3. An ineligible captured release is recorded and consumed without route
   dispatch.
4. A cleanup token resolves to its retained old-route tombstone.
5. Otherwise every path ID and generation matches current committed hosts.
6. The path matches committed logical parentage and route revision.
7. The target and route were eligible when Unity created the event.

Step 3 applies only to a captured release with
`UiCapturedInputDelivery::OwnerIneligible`. It produces the named inspector
reason, invokes no handler, and advances sequence state. It is neither a stale
drop nor a tombstone dispatch. `Logical` captured input follows the normal
checks. Step 4 compares every path generation against the tombstone captured
for that token; a missing, reused, or retired token is a framework failure.

A route revision remains compatible across handler-only commits. A structural,
eligibility, relationship, or policy commit makes an older route stale. Any
failed session, target, or route check drops the entire event and invokes no
handler. Reactant still advances sequence bookkeeping where safe so one stale
event cannot make all later valid events appear out of order.

After validation, Reactant snapshots the path and the capture and bubble
handler slots for every path member. It does not look up a slot again during
the dispatch. Handler replacement committed before dispatch is visible.
Handler or model changes requested during dispatch do not change the snapshot.

Propagation order is:

1. capture on strict ancestors, root to target parent;
2. target capture with `EventPhase::Target`;
3. target bubble with `EventPhase::Target`; and
4. bubble on strict ancestors, target parent to root.

`stop_propagation()` stops all later steps and nodes. It does not stop another
handler already executing. There is one capture slot and one bubble slot per
event kind per node, so there is no same-node immediate-propagation variant.

If Unity marked logical bubble as suppressed by a native control, steps 1
through 3 still run and step 4 is skipped. A Rust call to
`stop_propagation()` during capture can stop before the target as usual.

All callbacks share one mutable application-model borrow and one state batch.
Reactant reconciles roots once after propagation completes. Host mutations and
actions are emitted after that reconciliation in the ordering defined by the
main Reactant design.

## Portals and physical propagation

Unity policy evaluation and Rust propagation both use the event-time logical
path. A portaled overlay therefore sees policies, focus scopes, capture owners,
and handlers declared on its source ancestry.

The physical portal container and its unrelated Unity ancestors never become
Reactant ancestors. Reactant's coverage listener reports the event once. Any
external UI Toolkit listeners on the physical path still run according to UI
Toolkit rules. Rust `stop_propagation()` cannot affect them.

A native propagation-stopping policy is applied at the earliest Reactant
coverage callback available for the physical island. It stops later native
callbacks only where UI Toolkit permits. The event metadata reports that fact;
Reactant never claims it stopped callbacks that had already run.

Policy lookup across a portal is still O(logical depth). Commit-time parent
indices avoid walking Unity's physical tree. Portal target changes increment
the route revision and make already queued old-route events stale.

## Eligibility and lifecycle changes

A disabled subtree is inert for activation, focus movement, navigation, input
capture, label activation, and new drag starts. Descendants cannot opt back in
while an ancestor is disabled. Pointer and focus exit or cancellation events
needed for cleanup may still originate from a newly ineligible host.

A detached host or host under `display: none` cannot originate a new event.
`visibility: hidden` follows the existing UI Toolkit picking and focus rules;
Reactant lowers its effective event eligibility explicitly so Rust and Unity do
not disagree.

When a pending commit makes a host ineligible, Unity applies one two-state
transaction:

1. validate the complete pending host state and its cleanup token;
2. cancel activation latches, input capture, Motion, custom drags, and pointer
   capture against the old state;
3. select restored focus using old restoration records but pending scope,
   eligibility, and logical ancestry;
4. enqueue cleanup and `FocusOut` envelopes on old routes with the token;
5. install the pending host state and destroy removed native hosts;
6. enqueue `FocusIn` on the selected new route and revision; and
7. enqueue the cleanup watermark before accepting later native input.

Removal, disablement, and hiding use this same transaction. `FocusOut` can
therefore reach the old tombstone, while `FocusIn` always validates against the
new state. If no pending-state target is eligible, the transaction clears
focus and omits `FocusIn`.

Cleanup envelopes use `UiEventOrigin::NativeCleanup` and carry a commit-issued
cleanup token. Before emitting the commit, Reactant stores one tombstone for
the token containing every affected old route and a fallback copy of its
handler slots. Only an event with that token may use it. Ordinary old-revision
input remains stale and never gains tombstone access.

At cleanup dispatch, each old-path node first looks for the same host generation
in current committed state. A surviving host contributes its current handler
slot even when the cleanup-producing commit replaced that handler or moved the
host. A removed or replaced host contributes the tombstone fallback. Reactant
freezes those per-node choices once before capture starts. Thus a removed target
can receive its terminal callback, while a surviving ancestor obeys the normal
dispatch-time handler rule. The cleanup event has one state batch and one
reconciliation.

FIFO delivery guarantees that Rust processes every token-bearing event before
its `UiCleanupWatermark`. Processing the watermark retires the tombstone; no
additional acknowledgement is needed. The tombstone store contains 1,152
entries, one per possible queued or in-flight cleanup watermark. Cleanup items
join the sequence at the queue tail; they never pass input that Unity captured
before the commit.

Reactant never retains or defers a nonempty commit. Unity reserves 128 stream
slots beyond the 1,024 ordinary slots for commit-generated cleanup. If applying
a valid commit would exceed the total bound, Unity still applies the commit and
performs native cleanup, then enters input-fatal reconnect without promising
old-session Rust cleanup callbacks. The session reset clears every tombstone.
This failure path preserves Reactant's mandatory commit handoff and prevents
native and Rust trees from diverging.

`begin_session` or reconnect clears the old event queue, sequence state,
pointer capture, focus-scope stack, focus restoration records, control drafts,
activation latches, captured-release latches, Motion gestures, and policy state.
The new snapshot reconstructs only committed declarative state. An old-session
event is rejected without invoking handlers, even if its object IDs happen to
exist again.

Reconnect is the exception to Rust cleanup delivery. Unity releases capture
and cancels gestures locally, records one old-session terminal inspector entry,
and then discards the old queue. It does not deliver a cancellation callback to
the new session. The acceptance requirement is exactly one local release and
no old-session Rust callback, not a cross-session terminal event.

## Removal, handler updates, and reentrancy

The event-time route is preserved only while it remains a valid committed
route at Rust dispatch start. This prevents a queued event from targeting a
node that an earlier commit removed or reparented.

Once dispatch starts, the route and handler slots are frozen. If an early
handler changes application state so the target will disappear, later handlers
from the active snapshot still run. Reconciliation and removal occur only after
propagation finishes.

A later queued event for that target is checked against the resulting commit.
After the commit makes the target stale, the later event is dropped with
`reactant.event.stale_target` or `reactant.event.stale_route`. It does not fall
back to an ancestor.

Changing a handler without changing structure affects the next event whose Rust
dispatch has not started. An active dispatch keeps the prior snapshotted
closure. This rule applies equally to capture and bubble slots.

Rust event dispatch is non-reentrant. A synthetic event, engine callback, or
transport delivery attempted during an active dispatch is appended to the
session queue. It begins only after the active propagation and reconciliation
complete. Nested Unity events follow their native sequence and are likewise
delivered later.

## Scrolling and gesture arbitration

Wheel and touch scrolling remain UI Toolkit defaults. A nested scroll view
first receives an opportunity to consume movement on an axis where it can
scroll. Ancestor scrolling receives only unconsumed movement according to the
control adapter's native chaining rule.

A `WheelPolicy` can prevent or stop a selected wheel input synchronously. It is
appropriate for zoom surfaces or fixed interaction regions. Rust cannot inspect
the delta and then retroactively decide to scroll. A policy that needs a delta
threshold is unsupported until that threshold is a closed descriptor.

Motion contributes a capture claimant at its declaring host. Intrinsic control,
Motion, and generic capture use the precedence frozen in
[Pointer capture policy](#pointer-capture-policy). `PointerPolicy` prevention
and native stopping are independent of that claimant selection. Once Motion
drag begins, Unity cancels tap, captures the pointer, and emits `DragStart`.
Rust can change later declarative state but cannot cancel that already-started
drag.

For a non-Motion custom drag, `PointerCapturePolicy::OnPointerDown` establishes
ownership and Rust handlers interpret the ordered pointer events. The policy
does not calculate velocity, constraints, or momentum.

## Motion protocol boundary

This design supersedes the gesture portion of the Motion lifecycle protocol.
Every `MotionGestureEventKind`, including hover, tap, focus, pan, drag, scroll,
in-view, constraints, and momentum-complete records, moves from
`MotionEventBatch::gesture_events` to
`UiEventBody::MotionGesture(MotionGestureEvent)`. The
`gesture_events` field is removed atomically from Rust and C#.

Reliable gesture boundaries use the unified UI dispatch sequence, one-item
in-flight acknowledgement, route snapshot, and stale-event rules. Replaceable
Pan, Drag, and Scroll gesture samples use the unified queue-tail coalescing
rule. Input-caused gesture events name the immediate input envelope in
`source_sequence`; in-view and momentum events without one use `None`.

Animation lifecycle boundaries in `MotionEventBatch::events`, imperative
`playback_events`, presentation `samples`, and `value_samples` remain under the
Motion design. Its reliable Motion sequence, highest-contiguous acknowledgement,
timeout replay, logical-ID deduplication, and sample partitioning all survive.
Those records never consume UI dispatch sequences.

Gesture callbacks now reconcile once per unified stream event. A remaining
`MotionEventBatch` still batches its lifecycle callbacks and reconciles once per
batch as specified by the Motion design. The two sequence spaces and
acknowledgements are independent and are distinguished by their `ActionBody`
variants.

## Diagnostics and event inspection

Structured diagnostics use stable names and fields:

- `reactant.event.stale_session` records expected and received sessions.
- `reactant.event.invalid_sequence` records previous and received sequences.
- `reactant.event.stale_target` records target ID and route revision.
- `reactant.event.stale_route` records expected and received logical paths.
- `reactant.event.policy_conflict` records the node and conflicting selectors.
- `reactant.event.capture_conflict` records competing pointer claimants.
- `reactant.event.queue_overflow` records capacity and the blocked boundary.
- `reactant.event.sample_dropped` records a replaceable draft rejected before
  sequence assignment.
- `reactant.event.input_disabled` records the fatal overflow sequence.
- `reactant.event.capture_released` records pointer ID and release reason.
- `reactant.event.focus_restore_failed` records the removed scope and target.
- `reactant.event.cleanup_overflow` records exhausted reserved cleanup slots.
- `reactant.event.policy_non_cancelable` records ineffective prevention.
- `reactant.event.captured_owner_ineligible` records a retained release with no
  logical handler.
- `reactant.event.native_mismatch` records Rust and Unity outcome disagreement.

Expected stale events are warnings in development and structured trace entries
in production. Invalid descriptors, sequence regression, or Rust/C# outcome
disagreement are framework failures. They poison or disconnect the affected UI
session rather than continuing with ambiguous input state.

The development event inspector stores one bounded record per envelope:

```rust
pub enum EventInspectionOrder {
    Wire {
        dispatch_sequence: u64,
        synthetic_ordinal: Option<u16>,
    },
    RustSynthetic {
        synthetic_sequence: u64,
    },
    NativeDropped {
        attempted_after_sequence: u64,
    },
}

pub struct EventInspection {
    pub session_id: SessionId,
    pub order: EventInspectionOrder,
    pub target_id: ObjectId,
    pub target_generation: u64,
    pub logical_path: Vec<UiLogicalPathEntry>,
    pub route_revision: u64,
    pub origin: UiEventOrigin,
    pub default_policy_winner: Option<ObjectId>,
    pub native_stop_policy_winner: Option<ObjectId>,
    pub native_input_owner: UiNativeInputOwner,
    pub default_action: UiDefaultActionState,
    pub native_bubble: UiNativeBubbleDisposition,
    pub logical_bubble: UiLogicalBubbleDisposition,
    pub pointer_capture_owner: Option<ObjectId>,
    pub coalesced_samples: u32,
    pub drop_reason: Option<UiEventDropReason>,
    pub rust_queue_latency_us: Option<u64>,
    pub rust_handling_duration_us: Option<u64>,
    pub resulting_commit_id: Option<CommitId>,
}
```

A stale drop has no resulting commit. A handled event records the commit that
results from its one reconciliation, including a no-mutation commit marker. The
inspector never stores text contents, composed characters, or application
payload fields by default.

## Queueing, coalescing, and performance

The Unity session queue has 1,024 ordinary envelope slots plus 128 slots
reserved for commit-generated cleanup and watermarks. Native input cannot use
the reserve. Reliable boundaries retain exact order. A replaceable sample may
replace only the queue tail, and only when event kind, target, pointer or
device, capture generation, and rendered frame all match. Otherwise it appends
when ordinary capacity exists.

Pointer moves, scroll changes, and Motion samples may coalesce to one per
target, pointer, and rendered frame. Down, up, cancel, capture, focus, key,
activation, commit, and captured-input boundaries are never dropped.

No queued envelope is evicted. At ordinary capacity, a nonmatching replaceable
draft is discarded before sequence assignment and gets a local
`sample_dropped` inspection record. Because it had no sequence, it creates no
receiver gap. A reliable native draft at ordinary capacity records
`queue_overflow` outside the full queue, disables further input, and requests
reconnect. No later event from that session is delivered, so its unassigned
boundary cannot create a hidden gap. Rendering may continue while the session
is input-fatal.

The reference machine is the repository's `Mac17,6` with an Apple M5 Max. Tests
run the project's pinned Unity Editor natively on arm64 in batch mode, with Deep
Profile disabled and the release scripting defines enabled.

The policy workload has one depth-32 route. Every node carries one matching key
prevention rule, one matching navigation-stop rule, one nonmatching wheel rule,
and one nonmatching pointer rule: 128 entries and 64 matches per lookup. The
enqueue workload holds 768 of 1,024 slots, uses a non-coalescing reliable key
event, and dequeues one envelope after each measured enqueue to hold occupancy
constant.

On that fixed workload and hardware:

- policy lookup for a depth-32 logical route allocates nothing after
  installation and remains below 0.25 ms at p95;
- native event capture, outcome recording, and enqueue remain below 0.5 ms at
  p95, excluding transport and application code; and
- coalescing work is constant per replaceable queue key.

Unity EditMode performance tests warm each path for 2,000 iterations, measure
20,000 iterations, and calculate p95 with nearest-rank selection. They report
p50, p95, and maximum and fail the p95 gates. A profiler test asserts zero
managed allocations in the measured policy lookup. The retained result records
Unity version, build target, hardware, route depth, entry and match counts,
queue capacity and occupancy, warm-up count, and sample count.

## Migration examples

The examples translate behavior observed in the settings mockup. They do not
copy its DOM or browser event implementation.

### Slider arrows

The mockup prevents an arrow key so the slider changes without scrolling its
ancestor. A native Reactant slider needs no generic handler policy: the native
adapter owns arrows, applies one value change, suppresses generic navigation,
and emits one controlled proposal.

```rust
Slider::new(0.0..=100.0)
    .value(game.volume)
    .on_change(|game: &mut Game, value| {
        game.volume = value;
    })
```

A custom slider declares the prevention before input:

```rust
View::new()
    .role(AccessibilityRole::Slider)
    .native_event_policy(NativeEventPolicy::new().key_down(
        KeyDownPolicy::arrows(
            NativeEventDisposition::PreventDefault,
        ),
    ))
    .on_key_down_event(update_custom_slider)
```

### Modal Escape and focus

The mockup traps focus, closes on Escape, and restores prior focus. The scope
performs containment and synchronous cancel suppression; Rust handles closure.

```rust
View::new()
    .focus_scope(
        FocusScope::modal()
            .initial_focus(InitialFocus::Control(close_button.id()))
            .restore(FocusRestore::PreviouslyFocused),
    )
    .on_navigation_cancel(|game: &mut Game| {
        game.settings_open = false;
    })
    .child(settings_panel)
```

### Input rebinding

The mockup captures a physical key before the focused button can activate. The
Reactant capture descriptor makes that claim synchronous for keyboard and
controller input.

```rust
View::new()
    .input_capture(InputCapture::rebinding())
    .on_captured_input_event(|game: &mut Game, event| {
        if event.payload().phase == CapturedInputPhase::Pressed {
            game.bind(event.payload().input.clone());
        }
    })
```

### Dropdown navigation

A native dropdown owns Up, Down, Submit, and Cancel while open. It moves its
highlight locally and emits a committed selection once. An ancestor tab or
scroll handler never receives those owned navigation events. A custom popup
declares matching `NavigationPolicy` values and manages its logical highlight
from the later Rust callbacks.

### Nested label controls

The mockup uses nested labels and controls. Reactant declares the relationship
so one label activation focuses and activates the control exactly once.

```rust
let music = use_control_ref::<Toggle>();

View::new()
    .child(Label::for_control("Background music", music.clone()))
    .child(
        Toggle::new()
            .control_ref(music)
            .value(game.music_enabled)
            .on_change(set_music_enabled),
    )
```

### Pointer dragging

The mockup calls pointer capture during pointer down. A custom Reactant drag
declares that first-event requirement; a Motion drag uses `.drag(...)` instead.

```rust
View::new()
    .pointer_capture_policy(PointerCapturePolicy::OnPointerDown {
        buttons: PointerButtonSet::primary(),
        pointer_types: PointerTypeSet::direct_manipulation(),
    })
    .on_pointer_move_event(update_drag)
    .on_pointer_up_event(finish_drag)
```

## Acceptance scenarios

Each scenario specifies observable order. Unit tests may inspect event metadata,
but end-to-end tests use production input and visible outcomes.

### Slider consumes arrow input

1. Focus a slider inside a vertically scrollable ancestor.
2. Press Right Arrow once.
3. Unity classifies the slider as the intrinsic owner.
4. The slider changes by one step and emits one value proposal.
5. No generic navigation or ancestor scroll default runs.
6. Rust receives capture and target handling with
   `HandledByNativeControl` and suppressed logical bubble.
7. Rust accepts the proposal and the later commit preserves the new value.

A custom slider with an arrow policy follows the same visible result, but its
event records `PreventedByPolicy` and its Rust target handler computes the new
value.

### Modal handles Escape

1. Focus a control behind the modal, then mount the modal scope.
2. Unity records prior focus and focuses the declared initial modal control.
3. Press Escape.
4. Unity prevents lower-scope navigation and gameplay synchronously.
5. Rust receives modal capture, target, and bubble handlers in logical order.
6. The modal handler changes the model to closed.
7. Reactant reconciles once and a later commit removes the modal.
8. Unity restores the prior eligible control.

Focus never leaves the modal between steps 2 and 7, including controller
navigation and portaled descendants.

### Rebinding captures Space or a controller button

1. Focus a button inside an active rebinding scope.
2. Press Space or the controller South button.
3. Input capture claims the physical press before the button or navigation.
4. Unity prevents activation, navigation, and global gameplay delivery.
5. Rust receives one captured pressed event and records the binding.
6. Rust removes the rebinding scope in its later commit.
7. Release the same input.
8. Unity's release latch suppresses the release from UI and gameplay.

The focused button never enters its activation callback or visible pressed
commit because of the captured input.

### Nested click stops logical propagation

1. A child button is nested under a Reactant view with click handlers on both.
2. Unity performs any valid native button action and enqueues one click.
3. Reactant runs ancestor capture, child target capture, then child target
   bubble.
4. The child target bubble handler calls `stop_propagation()`.
5. The ancestor Reactant bubble handler does not run.
6. Physical UI Toolkit listeners run according to native propagation and are
   explicitly unaffected by the Rust call.

### Pointer capture sustains a drag

1. Pointer down matches `OnPointerDown` and Unity captures synchronously.
2. Rust later receives pointer down.
3. Move outside the original bounds; ordered moves still target the captor.
4. Pointer up releases capture and emits the final reliable boundary.
5. Repeat with cancel, disable, removal, and reconnect.
6. Every case ends capture exactly once and no later move targets the old
   captor.

Cancel, disable, and removal deliver their reliable cleanup event through the
old route tombstone. Reconnect records its terminal release only in Unity's
old-session inspector and invokes no Rust callback in the new session.

### Native text field owns navigation

1. Focus a text field inside an ancestor with a navigation-move handler.
2. Press Left Arrow while editing.
3. The text field moves or retains its caret according to native rules.
4. Unity classifies the event as handled by the native control.
5. Logical capture and target handlers may observe it.
6. Logical bubble to the ancestor is suppressed.
7. No ancestor focus move, scroll, or menu navigation occurs.

### Portaled overlay preserves logical ancestry

1. Render a modal overlay into an external physical portal container.
2. Activate a nested overlay control.
3. Unity snapshots the route through the source-side modal ancestry.
4. Modal policies and focus containment evaluate on that logical route.
5. Rust capture and bubble use the same source ancestry.
6. The unrelated physical container never becomes a Reactant current target.
7. External Unity listeners on the physical path remain unaffected.

### Nodes disappear during dispatch

1. Queue events A and B for one target in sequence order.
2. Event A begins Rust dispatch and snapshots its route and handlers.
3. An early A handler changes state so the target will be removed.
4. Every later handler in A's snapshot still runs unless propagation stops.
5. Reactant reconciles once and Unity commits the removal.
6. Event B reaches Rust after that commit and fails target or route validation.
7. Reactant invokes no B handler and records the structured stale reason.

## Automated validation

### Rust black-box tests

Tests drive the public dispatcher and observe application state, commits, and
inspector records. They cover:

- root-to-target capture, target capture, target bubble, and bubble-to-root;
- logical `stop_propagation()` at every phase;
- native-control bubble suppression without capture suppression;
- event-time route validation and dispatch-time handler snapshots;
- handler replacement before and during dispatch;
- portal routes that exclude physical ancestors;
- stale sessions, target and path generations, revisions, and parentage;
- nodes removed during an active dispatch and before a queued dispatch;
- cleanup-token tombstones, watermarks, retirement, and overflow reconnect;
- removed-target fallbacks with current surviving-ancestor handlers;
- ineligible captured releases that remain reliable but invoke no handler;
- one-item stream acknowledgement and mandatory commit handoff;
- synthetic pointer crossing and synthetic-event queue ordering;
- nested dispatch attempts and FIFO non-reentrancy;
- disabled, hidden, detached, and re-enabled nodes;
- reconnect clearing all session-owned event state;
- canonical policy lowering and invalid conflict rejection; and
- one reconciliation and one resulting commit record per accepted event.

These are black-box tests against public Reactant and fake-host behavior. They
do not assert private fiber, callback-erasure, or mutation-plan structure.

### Unity EditMode tests

Tests construct real UI Toolkit controls and drive their native callbacks. They
observe value, focus, propagation, capture, queues, and emitted envelopes. They
cover:

- synchronous key, navigation, wheel, and pointer prevention;
- native propagation stopping and accurate outcome metadata;
- every intrinsic profile, state flag, owned input, and unowned input;
- intrinsic native-control precedence over generic ancestors;
- phase-specific pointer-button policy matching;
- pointer activation-latch creation, disarming, and every cleanup path;
- text editing, IME-safe filters, caret movement, and post-edit `Input`;
- native and custom slider arbitration against scrolling;
- modal initial focus, containment, nesting, and restoration;
- label focus, activation, accessible naming, and disabled gating;
- portable and control-owned pointer capture with every release reason;
- keyboard and controller rebinding, including release latches;
- per-session sequencing, nested native callbacks, and FIFO delivery;
- asynchronous one-item pump, response ordering, acknowledgement, and dequeue;
- exact coalesced ranges, pre-sequence sample drops, and gap validation;
- exact raw, normalized, focus, label, and source-sequence ordering;
- cleanup watermark ordering, tombstone retirement, and reserve exhaustion;
- runtime-global shard installation and cross-panel modal focus restoration;
- unified Motion gesture versus retained lifecycle sequence separation;
- reliable-boundary retention;
- queue overflow entering input-fatal state; and
- disable, hide, detach, removal, document blur, and reconnect cleanup.

Rust and C# share serialized valid and invalid descriptor fixtures plus event
outcome fixtures. Any difference is a test failure.

### Ditto scenarios

Ditto adds a deterministic controller-button step that injects a complete
press or release through the production Input System. It never calls Reactant
handlers directly and does not repurpose global controller actions.

One retained suite exercises all eight acceptance scenarios. Assertions use
visible model state, focused object where the player exposes it, object
existence, visibility, enabled state, and screenshots. The suite checks that:

- the slider value changes once while scroll position stays fixed;
- Escape closes the modal and restores the prior visible focus state;
- Space and controller binding text change without button activation;
- a nested click changes only the child-observable state;
- dragging continues outside bounds and terminates on every cleanup path;
- text caret behavior does not move the ancestor selection;
- the portaled overlay closes through its logical modal handler; and
- a stale queued event produces no second visible model change.

Tests use `Controlled` Motion where time affects drag or focus visuals. They
retain screenshots only at stable boundaries.

## Failure modes and mitigations

- **A policy is installed one commit too late.** The old committed policy
  governs input until Unity installs the new state. Components render an input
  mode and its policy in the same commit. The inspector reports the route
  revision used by the event.
- **Rust is slow or disconnected.** Unity continues intrinsic control behavior
  and bounded enqueueing. Reliable overflow disables session input and requests
  reconnect instead of losing order.
- **A policy over-suppresses input.** Typed, narrow selectors and the
  inspector's winning node make the declaration visible. Canonical fixtures
  exercise each selector.
- **A native control and ancestor both claim navigation.** Intrinsic control
  ownership wins; only capture and target observation remain. Generic ancestor
  navigation does not run.
- **A modal restoration target disappears.** Unity applies the deterministic
  fallback order and records `focus_restore_failed` if no target exists.
- **Pointer ownership outlives a node.** Eligibility cleanup releases capture
  before destruction and emits one cancellation boundary.
- **A stale event names reused object IDs.** Session, route revision, and host
  generation validation prevent delivery to the replacement host.
- **Rust and Unity lower policies differently.** Shared canonical fixtures and
  outcome assertions fail the build; runtime mismatch ends the session.

## Alternatives considered

### Browser-style `prevent_default()`

Rejected. A Rust callback may run after UI Toolkit has edited text, moved
focus, captured a pointer, scrolled, or activated a control. A method with that
name would either lie on asynchronous hosts or create transport-dependent
behavior. Read-only cancelability and outcome metadata preserve useful
inspection without implying authority.

### Declarative default-action policies

Accepted for closed, timing-sensitive cases. Policies are portable because
Unity evaluates data already installed before dispatch. The limitation is
intentional: arbitrary application conditions must become declarative state in
an earlier commit or remain a later application reaction.

### Public synchronous Rust handler subsets

Rejected. They would work only for in-process transports, split the handler
model, complicate borrowing and reentrancy, and make a component behave
differently after transport changes. Unity-local closed descriptors cover the
legitimate synchronous cases.

### Speculative suppression and replay

Rejected. Focus, IME composition, caret selection, pointer capture, drag,
scroll chaining, and inertia cannot be reconstructed faithfully after Rust
answers. Replay would also reorder external Unity callbacks.

Control-specific proposal and restore remains allowed. A text field or slider
adapter may restore its own value because it owns the complete control state
machine and knows the exact native invariants. That is not a generic replay
facility.

### Higher-level native controls

Accepted as the default for complex behavior. Text fields, dropdowns, sliders,
scroll views, labels, and Motion gestures keep semantic adapters in Unity.
Rust configures them and receives typed outcomes. This produces fewer generic
escape hatches and preserves accessibility and input-device behavior.

## Phased implementation plan

Each phase is dependency-complete and ends with externally observable evidence.
Later phases may not invent new propagation, precedence, or cancellation rules.

### Phase 1: Shared protocol foundation

Implement the Rust and C# event enums, stream action and acknowledgement,
sequence fields, route revisions, logical paths, origins, cancelability,
default outcomes, native-bubble outcomes, policy descriptors, relationship
descriptors, and diagnostics. Add canonical validation to Rust, C#, and the
fake client.

Prerequisite tests:

- cross-language valid and invalid descriptor fixtures;
- exact `UiEvent` round trips for every new field and payload;
- stream-item, cleanup-watermark, and acknowledgement round trips;
- canonical set ordering and conflict rejection;
- fake-client policy matching and outcome fixtures; and
- structured diagnostic serialization.

Exit criteria:

- Rust and C# accept and reject the same fixtures;
- old `UiEvent` construction no longer compiles;
- the old per-event `VisualElement` action and `SubmitUiEvent` path are absent;
- fake behavior exposes sequence, route, policy, and outcome metadata; and
- the event inspector can record a fabricated complete lifecycle.

### Phase 2: Reactant logical dispatch

Update event ingestion, route validation, handler snapshotting, capture and
bubble propagation, one-event batching, stale drops, synthetic events,
non-reentrant queueing, and portal routing. Add the read-only public accessors.

Prerequisite tests:

- all Rust black-box ordering, stopping, snapshot, stale, portal, reconnect,
  synthetic, reentrancy, eligibility, and lowering cases; and
- one accepted event producing one reconciliation and commit record.

Exit criteria:

- event-time routes and dispatch-time handlers follow this document;
- removal during dispatch preserves the active snapshot;
- queued stale events invoke no callbacks and have named diagnostics;
- portals evaluate and propagate only through logical source ancestry; and
- cleanup dispatch uses old routes, removed-host fallbacks, and current
  surviving-host handlers.

### Phase 3: Unity synchronous event core

Install immutable event host state, the no-allocation policy evaluator,
intrinsic outcome classification, monotonic sequence queue, coalescing, reliable
overflow failure, one-item transport pumping, normalized-event links, and
reconnect cleanup.

Prerequisite tests:

- Unity prevention and propagation tests for every policy family;
- sequence, nested-callback, coalescing, overflow, and reconnect tests;
- async response application, acknowledgement, dequeue, and next-pump tests;
- native-control precedence fixtures; and
- depth-32 policy and enqueue performance harnesses.

Exit criteria:

- Unity makes no event-time Rust call;
- every emitted event contains a complete native outcome and route snapshot;
- reliable boundaries remain FIFO under nested callbacks and pressure;
- the next item cannot enter Rust before the prior commit and ack apply; and
- both p95 timing gates and the zero-allocation lookup gate pass.

### Phase 4: Interaction relationships

Implement modal focus scopes, initial focus and restoration, keyboard and
controller input capture, captured-release latches, label relationships,
portable pointer capture, disabled and hidden cleanup, and native control
ownership tables. Install runtime-global panel shards and cross-panel focus
coordination.

Prerequisite tests:

- Unity modal nesting, portal containment, and focus fallback tests;
- cross-panel portal policy, focus, accessibility, and atomic-shard tests;
- key and controller capture tests with focused native controls;
- label activation and accessibility relationship tests;
- pointer capture and all release-reason tests;
- pointer activation-latch tests for down and up prevention; and
- Rust lowering and conflict tests for every new façade builder.

Exit criteria:

- modal focus cannot escape through keyboard, controller, or portal placement;
- captured input cannot activate UI or gameplay before its release;
- labels activate compatible controls exactly once; and
- capture, focus, and release latches cannot survive ineligibility or reconnect.

### Phase 5: Native controls and Motion integration

Integrate Motion arbitration, migrate gesture events to the unified stream,
retain the independent lifecycle protocol, and add native text edit policies,
controlled text and range proposals, scroll chaining, slider key ownership,
dropdown navigation, and logical bubble suppression.

Prerequisite tests:

- Motion tap-to-drag ordering, capture loss, coalescing, and momentum tests;
- Motion gesture-stream and lifecycle-sequence separation tests;
- text editing, composition, selection, proposal, and restore tests;
- slider and text-field navigation arbitration tests; and
- scroll view chaining and wheel-policy tests.

Exit criteria:

- no generic policy duplicates a native control state machine;
- Motion retains Unity-local timing and emits ordered Rust boundaries;
- text `Input` is observably post-edit; and
- slider, dropdown, text, range, and scroll behavior obey the ownership matrix.

### Phase 6: Production evidence and migration

Add Ditto controller-button input, build the eight production-input scenarios,
retain performance evidence, migrate sample call sites, and update the
superseded event sections in related designs.

Prerequisite tests:

- all Rust and Unity suites from phases 1 through 5;
- Ditto adapter tests for deterministic controller press and release;
- the eight end-to-end scenarios on the required native player target; and
- retained p95 and allocation reports on reference hardware.

Exit criteria:

- every acceptance scenario passes without direct handler invocation;
- migration examples compile in the Reactant sample;
- related designs link here and contain no contradictory timing claim;
- event inspector evidence names every native decision and resulting commit;
  and
- implementation review can map every completion criterion to retained proof.

## Completion criteria

The event and default-action system is complete only when all of the following
are true:

- Unity performs every timing-sensitive decision from committed descriptors
  without waiting for Rust.
- Rust exposes no mutating default-prevention API.
- Capture, target, and bubble order matches the frozen algorithm.
- One acknowledged stream item is the only Reactant event in flight, and its
  commit applies before the next dispatch.
- Handler replacement and node removal follow the dispatch snapshot rules.
- Portals use source logical ancestry for policies, focus, and propagation
  across panels through one atomic runtime state.
- Intrinsic controls win over generic ancestor navigation and scrolling.
- Declarative prevention and native stopping report deterministic winners.
- Disabled, hidden, detached, removed, and reconnected hosts clean up focus,
  pointer, drag, draft, scope, and capture state deterministically.
- Text editing, selection, IME, slider tracking, scrolling, and Motion remain in
  their higher-level native adapters.
- Motion gestures use unified event ordering while animation lifecycle records
  retain their independent reliable protocol.
- Pointer prevention disarms activation through the declared latch rules.
- Modal Escape, focus containment, rebinding, labels, and pointer capture need
  no synchronous Rust callback.
- Reliable event boundaries cannot be coalesced or silently lost.
- Fatal overflow disables session input and emits the named diagnostic.
- Rust and C# fixtures, Rust black-box tests, Unity EditMode tests, Ditto
  scenarios, and performance gates all pass.
- The event inspector connects every accepted event to its route, policy
  result, Rust latency, and resulting commit.
- The eight acceptance scenarios have retained, externally observable evidence.

## Manual QA

1. Open the Reactant settings sample with a scrollable sound panel. Focus both
   a native and custom slider, press every arrow key, and verify one value step
   per press with no ancestor scroll.
2. Open the modal from a focused background button. Navigate by keyboard and
   controller, including through a portaled child. Verify focus stays inside.
   Press Escape, verify one close, and verify focus returns to the background
   button.
3. Start rebinding while a button is focused. Bind Space, then repeat with each
   supported controller button. Verify no focused-button activation, UI move,
   gameplay action, or release leak.
4. Activate a nested child whose target handler stops propagation. Verify the
   child state changes, the Reactant ancestor state does not, and an external
   Unity listener still follows its physical propagation contract.
5. Drag a custom captured host outside its bounds. End by up, cancel, disable,
   removal, document blur, and reconnect. Verify one terminal record and no
   stale capture in every case. Verify reconnect invokes no old-session Rust
   callback.
6. Edit a native text field with arrows, Home, End, selection modifiers, IME,
   paste, and controller navigation nearby. Verify text and caret behavior win
   without moving the ancestor menu.
7. Activate controls in a portaled overlay. Inspect the event record and verify
   source logical ancestry, modal policy attribution, physical-listener
   independence, and the resulting commit.
8. Pause Rust delivery, queue two events, and make the first remove the target.
   Resume delivery. Verify the active first dispatch finishes, the second event
   is diagnosed as stale, and no replacement or ancestor handler receives it.
9. Exercise a native label for every compatible control. Verify focus,
   activation, value proposal, accessible naming, disabled gating, and exactly
   one activation.
10. Run the depth-32 Unity profiler harness on reference hardware. Retain the
    zero-allocation policy lookup and p50, p95, and maximum timing report for
    both policy lookup and capture-plus-enqueue.
