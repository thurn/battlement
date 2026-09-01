# Reactant Events and Default Actions

Reactant invokes every subscribed UI Toolkit event handler synchronously in
Rust while the originating Unity event callback is still active. A handler may
therefore call `prevent_default()` and have Unity observe that decision before
it performs the event's remaining default actions.

Only the default-action decision returns through the active callback. Reactant
still defers every Unity mutation produced by reconciliation until normal
response processing resumes after UI Toolkit dispatch. This boundary gives
application code useful event-time authority without permitting arbitrary tree
mutation during native event propagation.

The design makes synchronous UI event submission part of the core Battlement
engine contract. It does not support an asynchronous Reactant UI transport or a
second, policy-based cancellation model.

The two values returned by Rust have different lifetimes:

- The **event disposition** is the immediate decision Unity consumes before
  the current callback returns. It is either `Continue` or `PreventDefault`.
- The **deferred response** is the ordinary Battlement `Response` containing
  commands produced by Reactant reconciliation. Unity queues it and applies it
  only at a normal response-drain point.

## Related Information

- [Battlement Reactant technical design](reactant-technical-design.md) defines
  sessions, roots, commits, and the Rust component runtime.
- [Reconciliation, events, and portals](reconciliation-events-and-portals.md)
  defines host identity, logical ancestry, handler storage, and physical portal
  placement.
- [Hooks and effects](hooks-and-effects.md) defines event-time state batching,
  reconciliation, effects, and cleanup.
- [Focus and navigation](focus-and-navigation.md) defines native focus
  ownership, scopes, restoration, controller navigation, and portal behavior.
- [Refs, geometry, and floating UI](refs-geometry-and-floating-ui.md) defines
  deferred host actions such as focus, scrolling, and pointer capture.
- [Reactant animations](animations.md) defines Unity-owned Motion gestures,
  capture, samples, velocity, constraints, and momentum.
- [Battlement UI technical design](../battlement-ui-technical-design.md) defines
  UI documents, event payloads, native controls, and command application.
- [Battlement technical design](../technical-design.md) defines the engine,
  native ABI, transport, sessions, response ordering, and input gating.
- [Unity event handling][unity-events] defines target selection,
  trickle-down, bubble-up, default actions, and cancellation.
- [Unity event dispatch][unity-dispatch] defines dispatch ordering and event
  lifetime.
- [Unity pointer capture][unity-capture] defines native pointer ownership and
  capture transition events.

[unity-events]: https://docs.unity3d.com/6000.0/Documentation/Manual/UIE-Events-Handling.html
[unity-dispatch]: https://docs.unity3d.com/6000.0/Documentation/Manual/UIE-Events-Dispatching.html
[unity-capture]: https://docs.unity3d.com/6000.0/Documentation/Manual/UIE-capture-the-pointer.html

## Required Behavior

This design establishes one event model for every Battlement engine and every
Reactant host. The same component must not change behavior according to which
transport happens to host it.

The fixed requirements are:

- Every `UiEvent` submitted by Unity enters Rust before the originating UI
  Toolkit callback returns.
- Every Battlement engine implements the synchronous UI event operation.
- Every subscribed UI Toolkit event uses that operation, including events that
  cannot prevent a default action.
- One UI event runs all applicable logical handlers before Reactant reconciles.
- Rust returns one disposition and one ordinary `Response` from the operation.
- Unity consumes the disposition immediately and queues the response without
  applying it.
- Unity applies no Reactant command while a callback forwarded through the
  Reactant bridge is active.
- `prevent_default()` affects only the current Unity event's remaining default
  actions.
- `stop_propagation()` affects only later Reactant handlers in the current
  logical dispatch.
- Native controls continue to own editing, selection, control tracking,
  scrolling, focus internals, and other higher-level state machines.
- Motion continues to own gesture recognition and continuous gesture state in
  Unity.
- Logical routing follows Reactant ancestry, including through portals.
- Physical UI Toolkit listeners outside Reactant remain native listeners.
- A panic, engine error, or malformed native result never prevents the Unity
  default action.

The synchronous contract is intentionally limited. It permits an immediate
default-action decision, not an immediate Unity mutation channel.

## Architectural Consequences

The synchronous requirement removes machinery whose only purpose would be to
make a delayed event appear current. Reactant does not need a separate UI-event
delivery queue, retained logical routes, route revisions, event
acknowledgements, or a C# evaluator for Rust-authored default-action policies.
Rust evaluates the current handler tree while the native event still exists.

It also keeps the UI-specific result out of ordinary Battlement actions.
`UiEventDisposition` belongs only to `submit_ui_event`; `Response`, custom
actions, geometry observation, Motion traffic, and polling do not acquire an
`ActionOutcome` concept.

The decision does not eliminate every timing mechanism. Unity must still defer
the ordinary response, bound and order queued responses, recover if Rust
commits state but a response is lost, and retain specialized native ownership
for controls, focus, input capture, pointer capture, and Motion.

## Core Engine Contract

UI is a core Battlement subsystem, even when a particular snapshot contains no
UI documents. Synchronous UI submission therefore belongs directly on
`Engine`; it is not an optional extension trait.

The engine contract adds one operation:

```rust
pub trait Engine {
    type ActionPayload: DeserializeOwned;
    type ErrorCode: DeserializeOwned;
    type Command: Serialize;

    fn submit_ui_event(
        &mut self,
        action: UiEventAction,
    ) -> Result<UiEventResponse<Self::Command>, EngineError>;
}
```

The existing `connect`, `submit`, and `poll` operations retain their current
signatures and semantics. The complete conceptual contract is:

- `connect` establishes or replaces a session.
- `submit` processes an ordinary built-in or game-owned action.
- `submit_ui_event` processes an event before Unity may continue dispatching
  it.
- `poll` retrieves independently produced work without blocking.

`ActionBody::VisualElement(UiEvent)` is removed. Keeping it would create two UI
event paths with different timing, cancellation, and failure semantics.

### UI event action

`UiEventAction` gives a UI event the same causal and session identity that an
ordinary action currently carries without wrapping it in `ActionBody`.

```rust
pub struct UiEventAction {
    pub action_id: ActionId,
    pub session_id: SessionId,
    pub event: UiEvent,
}
```

The fields have these meanings:

- `action_id` is unique within the session and identifies command batches
  caused by this event.
- `session_id` selects the active engine and Unity session.
- `event` contains the Reactant target and typed UI payload.

The existing `UiEvent` wire value adds the native cancellation fact captured by
the active Unity callback:

```rust
pub struct UiEvent {
    pub target_id: ObjectId,
    pub cancelable: bool,
    pub default_prevented: bool,
    pub body: UiEventBody,
}
```

`cancelable` and `default_prevented` must come from the actual `EventBase`.
Rust does not infer either value from `UiEventKind`. The second field preserves
prevention requested by an earlier native callback. Native adapter events with
no corresponding `EventBase` set both fields to `false`.

`default_prevented == true` requires `cancelable == true`. The native adapter
copies Unity's effective values, and Rust rejects a malformed action that
violates this invariant.

The response may use `action_id` as `Batch::caused_by_action_id`, exactly as a
batch caused by an ordinary submitted action does today.

### UI event response

`UiEventResponse` keeps the immediate decision structurally separate from the
ordinary response protocol.

```rust
pub struct UiEventResponse<C> {
    pub disposition: UiEventDisposition,
    pub response: Response<C>,
}
```

The disposition has a deliberately small public shape:

```rust
#[repr(u32)]
pub enum UiEventDisposition {
    Continue = 0,
    PreventDefault = 1,
}
```

The response is still required when it contains no commands. An empty response
preserves the session and causal identity of a successfully completed event.

The disposition does not contain focus, pointer capture, scrolling, element
mutation, or arbitrary commands. Those operations remain in the response and
therefore remain deferred.

## Native ABI

The native ABI exposes a dedicated operation because UI event submission has a
different completion contract from ordinary action submission.

```c
int battlement_submit_ui_event(
    void* engine,
    const uint8_t* request,
    uint64_t request_length,
    uint32_t* out_disposition,
    BattlementBuffer* out_payload
);
```

The ABI operation performs all of the following before returning success:

1. Reject null engine and output pointers and validate the live engine handle.
2. Initialize both outputs to their failure-safe values.
3. Validate the request pointer, length, and configured message-size limit.
4. Decode exactly one `UiEventAction` with no trailing JSON value.
5. Invoke `Engine::submit_ui_event` serially and non-reentrantly.
6. Validate that the returned response belongs to the submitted session.
7. Serialize the complete ordinary response into `out_payload`.
8. Write the validated disposition into `out_disposition`.

The output rules prevent a partial result from canceling native behavior:

- The adapter initializes `out_disposition` to `Continue` before invoking the
  engine.
- The adapter initializes `out_payload` to the canonical empty buffer.
- The adapter writes `PreventDefault` only after response serialization
  succeeds.
- The Rust enum cannot contain an invalid disposition. C# nevertheless
  validates the returned `uint32_t`; an unknown value is an ABI failure, not
  `PreventDefault`.
- A panic poisons the engine through the existing native panic boundary.
- Any non-success status leaves the effective disposition as `Continue`.
- C# frees `out_payload` through the existing buffer-free operation after it
  has copied or consumed the payload according to current transport ownership
  rules.

A null request pointer is valid only when `request_length` is zero. Both output
pointers must be non-null, writable, and correctly aligned as required by the
unsafe C contract. The adapter can check null and the registered live handle;
writability and alignment are caller preconditions that the ABI cannot prove.
If response allocation or serialization fails, the adapter frees any partial
response allocation before producing the failure result.

As with `battlement_submit`, `out_payload` has status-dependent contents. On
success it owns the serialized `Response`. On failure it owns the bounded UTF-8
diagnostic, or the canonical empty buffer when no diagnostic can be allocated.
A failure payload is never admitted to the response queue.

The status code, handle validation, size limit, panic boundary, buffer shape,
and free operation are the existing Battlement native transport contract. This
operation adds no second ownership model.

The normal `battlement_submit` ABI remains unchanged. It never accepts a
`UiEventAction` and never returns an event disposition.

### Exported engine surface

Every engine export contains `battlement_submit_ui_event`. There is no runtime
capability negotiation and no fallback to ordinary `submit`.

The engine export macro emits the complete fixed symbol set. An exported engine
that omits the UI symbol is incompatible with the matching Unity client and
fails during normal native integration validation.

This requirement keeps cancellation semantics uniform across:

- production native engines;
- native ABI fixtures;
- Unity EditMode tests;
- the Rust fake client; and
- game-specific engine implementations.

## Localhost HTTP

The synchronous localhost development transport exposes the same engine
operation without putting a UI-specific value into ordinary `Response`:

```text
POST /ui-events
Battlement-UI-Event-Disposition: 0 | 1
```

The request body is exactly one JSON-encoded `UiEventAction`. On HTTP 200, the
body is exactly the ordinary JSON-encoded `Response<C>`, and the required
`Battlement-UI-Event-Disposition` response header contains the decimal numeric
disposition. Keeping the response body unchanged lets the managed transport
preserve it as opaque response bytes.

The server invokes `Engine::submit_ui_event` and does not route this endpoint
through ordinary `Engine::submit`. HTTP 400 means an invalid request, HTTP 500
means an engine or serialization failure, and other status codes are transport
failures, matching the existing localhost rules. Failure bodies may contain
bounded diagnostic text; a failure header is ignored and never prevents the
native default.

The endpoint uses the existing 100 ms submit timeout and synchronous Unity
main-thread request. Exceeding that timeout stops the session. The production
performance gates are measured on the native transport, while the HTTP fixture
must prove identical disposition, response, failure, and ordering semantics.

## Managed Transport Contract

The managed transport exposes UI submission directly because the UI subsystem
is always available to the runner.

```csharp
public interface IBattlementTransport
{
    BattlementTransportResult Submit(ReadOnlyMemory<byte> json);
    BattlementUiEventTransportResult SubmitUiEvent(
        ReadOnlyMemory<byte> json
    );
}
```

The specialized result separates the fixed disposition from the opaque
response bytes:

```csharp
public sealed record BattlementUiEventTransportResult(
    BattlementTransportStatus Status,
    UiEventDisposition Disposition,
    ReadOnlyMemory<byte> ResponsePayload,
    string? Diagnostic = null
);
```

`BattlementNativeTransport.SubmitUiEvent` uses the same:

- live engine pointer;
- owning-thread check;
- serial call gate;
- payload limits;
- status translation;
- panic handling; and
- buffer ownership rules

as ordinary `Submit`.

`BattlementHttpTransport.SubmitUiEvent` posts the same serialized action to
`/ui-events`, validates the required disposition header, and preserves the HTTP
body as `ResponsePayload`. Both implementations return the same managed result
and failure statuses.

The transport does not decode the ordinary `Response`. It returns the fixed
disposition immediately and preserves the response payload for the runner's
normal decoding path.

`ResponsePayload` is nonempty only for a successful status. For failure
statuses, the same native buffer is consumed as `Diagnostic`, and the managed
result exposes an empty `ResponsePayload`.

## Unity Event Bridge

The Unity event bridge keeps the original UI Toolkit event object only on the
active C# stack. It never acquires, stores, or uses the pooled object later.

The bridge performs this operation for one event:

```csharp
UiEventResult result = runner.EmitUiEvent(ToUiEvent(eventValue));
if (result.Disposition == UiEventDisposition.PreventDefault)
    eventValue.PreventDefault();
```

`EmitUiEvent` is the only runner path that calls `SubmitUiEvent`. Its behavior
is:

1. Increment `uiDispatchDepth`.
2. Allocate the action ID and its pending inspection record.
3. Reserve one bounded response-queue admission token.
4. Create and serialize the `UiEventAction`.
5. Call the synchronous transport operation.
6. Validate the transport status, disposition, and response payload.
7. Commit the returned payload into the reserved queue position.
8. Return the disposition to the active event observer.
9. Decrement `uiDispatchDepth` in a `finally` block.

The reservation claims one item slot and fixes the event response's FIFO
position before Rust can change application state. No other producer may
consume that position. A reservation is either committed exactly once or
released exactly once.

The shared deferred-response queue allows 256 item slots and 64 MiB of total
serialized payload. Each payload also remains subject to the existing 16 MiB
`MaximumMessageBytes` limit. The item slot is reserved before the call; the
exact byte count is charged after the native buffer returns and before the
disposition becomes observable. These limits live beside the existing protocol
limits and apply to responses from every producer.

If item reservation fails, Unity does not call Rust. It returns `Continue`,
rejects further session input, and records a fatal pending session failure. If
the native call or exact byte charge fails, Unity releases the unused
reservation and records the same failure. After a successful call, the
disposition is not observable until the payload has been committed to the
reserved queue position.

The observer applies `PreventDefault()` only after `EmitUiEvent` returns a
validated success. It then returns normally to UI Toolkit.

`uiDispatchDepth` prohibits response application during callbacks forwarded by
the Reactant bridge. It also covers UI Toolkit callbacks nested beneath those
callbacks. Reactant does not claim to detect unrelated callbacks registered by
other systems. Leaving the outermost bridge callback does not itself drain the
response queue. The queue drains at the next normal runner response-drain
point, which may occur later in the same Unity frame or in a later frame.

### Authoritative event coverage

Every event represented as `UiEvent` uses `SubmitUiEvent`, even if the event is
not cancelable. This gives all logical events one ordering and failure model.

The following list is the complete `UiEventKind` inventory. Adding a variant
requires adding it to exactly one coverage class.

- **Root trickle observer:** `PointerDown`, `PointerMove`, `PointerUp`,
  `PointerCancel`, `Click`, `PointerOver`, `PointerOut`, `Wheel`, `KeyDown`,
  `KeyUp`, `NavigationMove`, `NavigationCancel`, `FocusIn`, and `FocusOut`.
  Each comes from its propagating `EventBase`. Navigation submit on a button
  is normalized to `Click`.
- **Owned target observer:** `PointerEnter`, `PointerLeave`, `PointerCapture`,
  `PointerCaptureOut`, `Focus`, and `Blur`. Each comes from its
  non-propagating `EventBase` on the registered host.
- **Lifecycle adapter:** `GeometryChanged`, `AttachToPanel`,
  `DetachFromPanel`, `TransitionStart`, `TransitionEnd`, `TransitionCancel`,
  `LinkEnter`, `LinkLeave`, `LinkDown`, and `LinkUp`. Each comes from a target
  callback or Unity lifecycle or link event normalized by the adapter.
- **Controlled-value adapter:** `ValueChanging`, `ValueCommitted`, `Input`,
  `SelectionChanged`, `ScrollSettled`, `ScrollChanged`,
  `TabSelectionRequested`, `TabCloseRequested`, and `TabReorderRequested`.
  Each comes from a control-specific callback or proposal emitted by the
  owning adapter.

Coverage source and logical propagation are independent. The complete logical
propagation classification is:

- **Capture, target, and bubble:** `PointerDown`, `PointerMove`, `PointerUp`,
  `PointerCancel`, `Click`, `PointerOver`, `PointerOut`, `Wheel`,
  `PointerCapture`, `PointerCaptureOut`, `KeyDown`, `KeyUp`,
  `NavigationMove`, `NavigationCancel`, `FocusIn`, `FocusOut`, `LinkEnter`,
  `LinkLeave`, `LinkDown`, and `LinkUp`.
- **Target only:** `PointerEnter`, `PointerLeave`, `Focus`, `Blur`,
  `GeometryChanged`, `AttachToPanel`, `DetachFromPanel`, `TransitionStart`,
  `TransitionEnd`, `TransitionCancel`, `ValueChanging`, `ValueCommitted`,
  `Input`, `SelectionChanged`, `ScrollSettled`, `ScrollChanged`,
  `TabSelectionRequested`, `TabCloseRequested`, and `TabReorderRequested`.

The first group permits `Trickle`, `Target`, and `Bubble` subscriptions. The
second permits only `Target`; validation rejects its ancestor subscriptions.
This list is the implementation of `UiEventKind::propagates()` in both Rust and
C# and must remain byte-for-byte equivalent at the variant level.

An **event island** is one registered UI document or panel root and all
Reactant hosts physically owned by that root. Each island installs exactly one
root observer set. Owned target observers are installed only for kinds that do
not enter through the root observer. Adapters own only their listed class. These
disjoint sources are the exactly-once rule; a source must not also forward the
raw event from which it derives a normalized event.

In particular:

- button navigation submit emits one normalized `Click`, not a second raw
  navigation event;
- button repeat emits one `Click` for each configured repeat activation;
- a control proposal emits its typed proposal event instead of separately
  forwarding the underlying raw change notification; and
- link and pointer events are distinct when both are explicitly subscribed,
  because they represent different public event kinds.

Every `EventBase`-backed source copies `EventBase.cancellable` and
`EventBase.isDefaultPrevented`. A normalized `Click` copies those values from
the `ClickEvent` or `NavigationSubmitEvent` that caused it. Sources without an
active `EventBase` use `false` for both fields. The Unity 6000.5.8f1 integration
matrix records the observed values for every inventory entry and fails when a
Unity upgrade changes them.

Subscriptions are committed Unity state. An event admitted while Unity still
has an old subscription may reach Rust after Rust removed that handler. Rust
checks its current handler tree and invokes no removed handler. The active entry
still flushes pending effects, error reports, geometry work, hooks, and element
actions, so its ordinary response is not necessarily empty. A newly added
handler cannot receive native events until its deferred subscription command is
applied. Reactant does not buffer or replay either case.

Reactant does not forward every event created internally by UI Toolkit. Existing
subscription coverage and native adapter rules still decide which events enter
the bridge.

The following traffic does not become `UiEvent` merely to use this ABI:

- global pointer, keyboard, and controller actions targeting the game world;
- geometry observation batches;
- Motion lifecycle batches, presentation samples, and value samples;
- polling responses; and
- game-specific custom actions.

## Ordered Event Lifecycle

One accepted UI event completes its Rust work before Unity resumes the native
dispatch. Its Unity mutations still occur later.

The complete lifecycle is:

1. UI Toolkit selects the physical event target.
2. A Reactant coverage callback runs during root trickle-down or the event's
   earliest supported callback point.
3. Unity maps the physical target to one Reactant host ID.
4. Unity allocates an action ID and pending inspection record.
5. Unity reserves the response's position in the normal bounded queue.
6. Unity captures the serializable payload, cancelability, and existing native
   prevention state.
7. Unity submits one `UiEventAction` synchronously.
8. Rust validates the session and resolves the current logical route.
9. Reactant snapshots the route and applicable handler slots.
10. Reactant invokes logical capture, target, and bubble handlers.
11. Reactant reconciles once when the dispatch requires a refresh.
12. Rust constructs one disposition and one ordinary response.
13. The native adapter serializes the response and returns both outputs.
14. Unity commits the payload into the reserved queue position.
15. Unity calls `PreventDefault()` when the disposition requires it.
16. The coverage callback returns and UI Toolkit continues native dispatch.
17. The runner later decodes and admits queued responses in FIFO order; batch
    execution retains its existing scheduler semantics.

Steps 4 through 15 are part of Unity's event-callback latency. Step 17 is not.

The bridge makes no second Rust call to retrieve the disposition. The
disposition and response are outputs of the same engine invocation.

## Public Rust Event API

`ReactantEvent<E>` is a typed view over one shared logical dispatch. Every clone
observes the same propagation and default-prevention state.

```rust
impl<E> ReactantEvent<E> {
    pub fn payload(&self) -> &E;
    pub fn target(&self) -> ElementTarget;
    pub fn current_target(&self) -> ElementTarget;
    pub fn phase(&self) -> EventPhase;
    pub fn cancelable(&self) -> bool;
    pub fn default_prevented(&self) -> bool;
    pub fn prevent_default(&self);
    pub fn stop_propagation(&self);
}
```

The event's shared internal state contains independent flags:

```rust
struct EventInner {
    propagation_stopped: Cell<bool>,
    default_prevented: Cell<bool>,
    prevented_by_reactant: Cell<bool>,
}
```

The methods have precise, separate meanings:

- `cancelable()` reports the actual native event's cancellation capability.
- `prevent_default()` requests cancellation only when `cancelable()` is true.
- `default_prevented()` reports whether prevention was already present on the
  native event or an earlier Reactant handler made an effective request.
- `stop_propagation()` stops later Reactant handlers for this logical dispatch.

Calling `prevent_default()` for a non-cancelable event is a no-op. It does not
set `default_prevented()` and the final disposition remains `Continue`. This
matches the useful behavior of browser events without claiming authority Unity
did not provide.

`default_prevented` initializes from `UiEvent.default_prevented`.
`prevented_by_reactant` initializes to `false` and becomes `true` only when a
Reactant handler successfully requests prevention. The final disposition is
`PreventDefault` whenever `default_prevented` is true. Calling Unity's method a
second time is harmless and preserves the fact across the ABI boundary.

The method remains available on the common event type because cancelability is
a property of the actual native event, not solely of the Rust payload enum.

### Example: dynamically preventing an arrow default

A custom slider can use current application state instead of installing a
policy during an earlier commit:

```rust
.on_key_down_event(|game, event| {
    if event.payload().physical_key == PhysicalKey::ArrowRight {
        event.prevent_default();
        game.volume = (game.volume + 1).min(100);
    }
})
```

The same callback updates Rust state and requests cancellation. Reconciliation
returns the new slider representation in the deferred response, while Unity
observes the cancellation before returning to its own default handling.

### Example: observing a post-change notification

An input notification describes text that the native text field has already
edited. It still uses the synchronous channel, but prevention is ineffective:

```rust
.on_input_event(|game, event| {
    assert!(!event.cancelable());
    game.draft = event.payload().value.clone();
})
```

The channel is uniform even though this event can only report native work.

## Default-Action Semantics

`prevent_default()` applies to the remaining default actions of the exact Unity
`EventBase` that caused the Rust dispatch. It is not a transaction rollback.

Reactant registers cancelable input coverage at the earliest supported
trickle-down point. This is necessary because UI Toolkit may run a target
default action before bubble-up callbacks.

The following rules define the guarantee:

- Reactant calls `EventBase.PreventDefault()` before the coverage callback
  returns.
- The event must report that it is cancelable.
- Unity decides which of its remaining default actions honor prevention.
- Prevention does not undo native work performed before the coverage callback.
- Prevention does not undo callbacks, manipulators, or control-adapter work
  that is not implemented as a cancelable default action.
- Prevention does not automatically cancel a later, distinct event.
- Prevention does not stop native propagation.
- Prevention does not suppress global gameplay input outside UI Toolkit.

For example, preventing `PointerDownEvent` does not create a Reactant-owned
activation latch that automatically prevents a later `ClickEvent`. If an
application must prevent the click event's own default, its click handler must
make that decision when the click is dispatched. Native controls retain any
stronger behavior UI Toolkit itself associates with pointer-down prevention.

### Cancelable raw input

Raw input events are the main users of `prevent_default()`:

- key down;
- navigation move and cancel;
- pointer down, move, up, and cancel where UI Toolkit marks them cancelable;
- click where UI Toolkit marks it cancelable; and
- wheel where UI Toolkit marks it cancelable.

`UiEventKind` has no `NavigationSubmit` variant. A button's native navigation
submit is normalized to `Click`, and that `Click` carries the source
`NavigationSubmitEvent` cancellation state. Navigation submit for other hosts
is not forwarded unless a control adapter normalizes it to a documented public
event.

The Unity integration test matrix, rather than a handwritten Rust table, is
authoritative for actual cancelability on the pinned Unity version.

### Post-default and lifecycle events

These events ordinarily describe state that already exists or a transition
that already occurred:

- text input and selection changes;
- value-changing and value-committed proposals;
- scroll changes and settling;
- focus and blur notifications;
- pointer capture and capture loss;
- attach and detach notifications;
- transition lifecycle events; and
- geometry-related notifications that are represented as UI events.

They still run synchronously for ordering consistency. Their disposition is
normally `Continue`, and application code reacts through later declarative
state rather than claiming to undo the notification.

## Logical Propagation

Reactant propagation uses the current committed logical tree, not Unity's
physical ancestry. Portals therefore preserve source-side capture and bubble
behavior.

For a propagating event, Reactant invokes handlers in this order:

1. Capture on strict ancestors from logical root to target parent.
2. Target capture with `EventPhase::Target`.
3. Target bubble with `EventPhase::Target`.
4. Bubble on strict ancestors from target parent to logical root.

For a non-propagating event, Reactant invokes only its target handler slot.
That slot is the target bubble slot. Validation rejects capture or ancestor
subscriptions for a non-propagating kind.

Before the first handler runs, Reactant snapshots:

- every logical host in the route;
- each host's event-time identity;
- the capture and bubble handler slots for the event kind; and
- the event payload shared by those handlers.

An early handler may update application or hook state so that reconciliation
removes a later host. The active dispatch still uses its snapshot and completes
unless a handler calls `stop_propagation()`.

`target()` and `current_target()` return stable logical `ElementTarget` values,
not borrowed Unity objects. `target()` is constant for the whole dispatch.
`current_target()` changes before each handler invocation and remains the last
invoked host after that handler returns. A ref command made from either target
is deferred with the ordinary response and follows the existing absent-target
behavior if that host no longer exists when Unity applies it.

There is one capture slot and one bubble slot per event kind on each host.
Reactant has no multiple-listener list inside a slot, so it does not expose
`stop_immediate_propagation()`. The stopping behavior is exact:

- From strict-ancestor capture, no later ancestor, target, or bubble handler
  runs.
- From target capture, no target bubble or ancestor bubble handler runs.
- From target bubble, no ancestor bubble handler runs.
- From strict-ancestor bubble, no later ancestor bubble handler runs.
- From a non-propagating target slot, no handler remains because dispatch is at
  its last slot.

The handler that calls `stop_propagation()` always finishes. Prevention and
propagation are independent: either, both, or neither flag may be set.

### Logical phase versus native phase

For a propagating root-observed event, all logical phases run while Unity is
physically executing the Reactant root's early coverage callback. A Reactant
handler whose `phase()` is `Bubble` is therefore not running during UI
Toolkit's physical bubble-up phase.

Owned-target and adapter events enter at their documented target or adapter
callback. Their logical route still runs entirely inside that one native
callback. They do not claim the root trickle timing guarantee unless their
coverage class uses the root observer.

This distinction has two consequences:

- `prevent_default()` from any Reactant logical phase can still reach Unity
  before its target default action.
- `stop_propagation()` cannot truthfully mean native propagation stopping.

Reactant does not expose a general native `stop_propagation` operation. If it
did, a logical bubble handler would physically stop Unity while the event was
still near the start of trickle-down, before unrelated target callbacks had
run.

Physical listeners can observe `isDefaultPrevented` after Reactant requests
prevention. They otherwise run according to UI Toolkit's own propagation
rules.

## Reconciliation and Deferred Responses

Reactant performs event callbacks and the resulting reconciliation before the
synchronous engine operation returns. Unity applies none of the produced
commands until native event dispatch reaches a safe point.

The response may contain:

- visual element creation, update, movement, and destruction;
- focus, scrolling, pointer-capture, and text-selection actions;
- Motion declarations and controls;
- non-UI Battlement commands emitted by the application; and
- an empty command list when the event produced no host work.

Every item in that list is deferred. The immediate disposition is not a command
and never appears in a batch.

### Why reconciliation remains synchronous

Reactant currently defines one event as one active entry with one resulting
commit. Keeping reconciliation inside that entry preserves:

- state batching across every handler in the logical route;
- one handler snapshot for the active event;
- one committed Rust tree before the next Rust event begins;
- existing hook and effect scheduling; and
- one causal response per UI event.

Splitting callbacks from reconciliation would create a second event-time tree
and handler model. This design keeps the existing single-entry semantics and
uses performance tests to bound their cost.

### Response queue ordering

All response producers use one admission sequencer. A reservation immediately
claims the next FIFO sequence number; committing its bytes does not change that
position. Ordinary submissions, polling, and UI events therefore have one
total order based on admission, not completion or later application time.

The admission sequencer is serialized-response ingress in front of the existing
`BattlementResponseStream` and `BattlementBatchScheduler`. At a normal drain
point, while `uiDispatchDepth == 0`, the runner performs these steps:

1. Take committed serialized payloads in admission-sequence order.
2. Decode and validate each response.
3. Enqueue it into `BattlementResponseStream` in that same order.
4. Let the response stream admit snapshots and batches in protocol order.
5. Let later normal runner steps advance admitted batches according to their
   existing `BatchStart`, dependency, and parallel-group semantics.

Response ordering is not command-execution atomicity. Draining response A may
admit its batches without executing their commands, then admit response B.
Commands from those batches execute only when `BattlementBatchScheduler`
advances and may interleave exactly as the existing batch contract permits.

No Reactant command is decoded or applied while `uiDispatchDepth` is nonzero.
Completing the outermost bridge callback only makes draining eligible; it does
not invoke `DrainResponses` itself.

If applying a deferred response later fails, the session follows existing
batch-failure behavior. Unity cannot retroactively undo a default that Reactant
already prevented. The inspector connects the immediate decision to the later
application failure so the sequence remains explainable.

## Nested Events and State Skew

Rust and Unity may briefly describe different UI trees after a synchronous
event returns. Rust has committed the new Reactant tree, while Unity has not yet
applied the corresponding response.

This **state skew** is the bounded interval between Rust committing an event
response and Unity completing the causal snapshot or batch mutations. Merely
draining the serialized response into the batch scheduler does not end it.

State skew obeys these rules:

- Rust's committed tree is authoritative for the next logical dispatch.
- Unity's live tree remains authoritative for physical target selection.
- Deferred responses enter `BattlementResponseStream` in admission order;
  their commands retain ordinary batch scheduling semantics.
- Reactant never reconstructs an old logical tree merely because Unity has not
  applied its response yet.
- An event for a host absent from the current Rust tree invokes no handler.
- A dropped stale event does not add prevention and preserves prevention
  already present on the native event. Its active entry may still return
  unrelated pending work.

### Example: an event removes its own target

Suppose event A closes a panel. Rust commits the removal, but Unity still has
the panel until A's response is applied. A native action then emits event B for
the old panel. Rust finds no current host and invokes no handler for B. The
response for B may nevertheless carry passive or already-pending work from its
active Reactant entry.

No route revision, delayed-event queue, acknowledgement protocol, or retained
old-tree tombstone is required. The synchronous lookup either finds the target
in the current Rust tree or safely ignores it.

### Native nested dispatch

The native engine remains serial and non-reentrant. Rust cannot create a nested
Unity event during its own callback because its response is not being applied.

UI Toolkit may create another event after the first Rust call returns but before
the outer native operation completely settles. That second event performs a new
synchronous engine call after the first call has released the engine gate.

Each event receives its own disposition immediately. Their deferred responses
remain ordered by the queue positions reserved at admission.

### Events raised while applying a response

Applying a response can cause focus, detach, value, or other UI Toolkit events.
Those events also use `submit_ui_event`.

For a batch command, the exact call sequence is:

1. Response A has already entered `BattlementResponseStream`, and its batch has
   been admitted to `BattlementBatchScheduler`.
2. The scheduler advances one command from A, which raises UI event B.
3. B increments `uiDispatchDepth`, reserves serialized-response ingress, and
   calls Rust. The native engine is idle because A is Unity-side work.
4. Rust commits its current tree and returns response B plus its disposition.
5. Unity commits B's serialized response and applies B's disposition.
6. B returns and the scheduler continues its ordinary current advance step.
7. At the next normal response-drain point, B is decoded and its messages are
   admitted after every earlier serialized-response reservation.
8. B's batches execute later according to ordinary batch scheduling.

If a snapshot or response-message admission itself raises B, the response
stream finishes or pauses its current ordered admission before a later normal
drain admits B. Neither case recursively drains B or changes batch semantics.

Reactant lifecycle and effect cleanup comes from reconciliation. It never
depends on delivering a native detach or focus event to a host already removed
from the committed Rust tree.

## Physical Targets, Portals, and External Listeners

An `EventBase`-backed source maps its physical target to at most one registered
Reactant host in the same event island. Mapping uses this precedence and stops
at the first applicable source:

1. For a pointer event with native capture, start at the capture owner.
2. For a keyboard, navigation, or focus event, start at Unity's focused or
   focus-related event target.
3. For all other events, start at `EventBase.target`.
4. If the starting element is an internal part of a registered native control,
   use that control's explicitly registered owner host.
5. Otherwise walk physical parents within the event island and use the nearest
   registered host.
6. Apply the event-kind eligibility test to the mapped target host.
7. Consult committed subscription coverage for applicable target, logical
   capture, or logical bubble handlers.
8. If mapping, eligibility, or subscription coverage has no result, emit no
   `UiEvent`.

A source with no active `EventBase` supplies the `ObjectId` of its owning
registered host directly. Unity verifies that the owner is live in the
adapter's event island, then applies the same eligibility and committed
subscription checks from steps 6 and 7. It never derives a target from focus,
pointer state, or an unrelated physical child.

Eligibility failure does not restart the parent walk. An eligible target may
still dispatch to subscribed logical ancestors; an ineligible child does not
retarget the event to an ancestor. Internal-control ownership outranks ancestry
because Unity may place an implementation child outside the owner's obvious
content hierarchy.

An unmapped event is an intentional native-only event. It allocates no action
ID, makes no Rust call, and produces no inspection record unless verbose
coverage diagnostics are enabled.

After mapping, Rust derives the current logical route from the committed
Reactant tree. `UiEventAction` does not carry a serialized logical path, host
generation list, or route revision because the event is not waiting in an
asynchronous delivery queue.

A portal changes physical placement but not logical ancestry. Reactant capture
and bubble therefore travel through the source tree, while external UI Toolkit
listeners continue along the physical Unity path.

`prevent_default()` sets the shared Unity event's native prevention flag.
External listeners that run later may observe that flag. Reactant's logical
`stop_propagation()` has no effect on them.

## Native Controls and Controlled Values

Synchronous Rust callbacks do not replace state machines that require native
frame timing, internal control state, or platform integration.

UI Toolkit and Reactant's native adapters continue to own:

- text editing, IME composition, caret movement, and selection;
- clipboard behavior and native text validation;
- button, toggle, radio, and dropdown pressed or open state;
- slider and range-control tracking;
- control-internal pointer capture;
- wheel and touch scrolling, chaining, and inertia;
- focus-controller state and native navigation; and
- accessibility actions exposed by native controls.

Reactant receives typed events and proposals from those adapters. Application
state may accept, clamp, replace, or reject the next declarative value according
to each control's controlled-value contract.

A proposal is observed after the native control has calculated or briefly
installed its local value, but before Rust decides the next authoritative
declarative value. Adapters that currently restore the committed value before
forwarding continue to do so. Text drafts and live drag values that are
intentionally native-local remain local until their existing commit boundary.
The deferred response is the only way Rust accepts or replaces the next
declarative value.

`prevent_default()` can stop a remaining cancelable default for the raw event.
It cannot undo the local mutation that produced a proposal or notification.
Adapter events therefore set `cancelable` to `false`; they are synchronous for
Rust ordering, not for rollback.

### Raw event bubbling from controls

Reactant does not duplicate UI Toolkit control ownership in a Rust/C# table
merely to decide logical bubbling. Raw key and pointer events may logically
bubble from native controls according to their Reactant subscriptions.

Ancestor handlers use the stable logical `event.target()` when behavior should
exclude an interactive descendant. They do not inspect the Unity control type
or physical path.

Higher-level navigation and activation adapters should emit normalized events
only when their native control contract says the generic behavior occurred.
They must not duplicate an input that a control already consumed.

This keeps native-control knowledge in the adapter that owns the control rather
than maintaining a second exhaustive `UiIntrinsicInputProfile` evaluator.

## Focus, Input Capture, and Pointer Capture

Default prevention is only one part of input ownership. Focus scopes, gameplay
input capture, and pointer capture retain their specialized designs.

### Focus and navigation

The focus system continues to install declarative scope membership, containment,
restoration, and accessibility state in Unity. This lets Unity maintain valid
focus even when no application handler runs.

A synchronous key or navigation handler may call `prevent_default()` when the
application dynamically decides that the current native navigation default
must not run. That decision does not replace:

- modal focus containment;
- focus restoration after removal;
- portal-aware focus membership;
- roving focus and explicit neighbors; or
- native accessibility traversal.

The focus design must no longer claim that Rust can never participate during an
input event. Its native ownership rules remain authoritative, while this design
supplies the synchronous cancellation boundary.

### Keyboard and controller input capture

`prevent_default()` affects the current UI Toolkit event. It does not suppress
Battlement's separate global gameplay action path.

Input rebinding therefore retains an input-capture mechanism that arbitrates
before both UI and gameplay consume a physical transition. It also retains a
release latch so removing a capture owner after key-down cannot leak the
matching key-up into gameplay.

The capture subsystem may invoke the same synchronous Reactant handler for an
accepted UI event, but its cross-input-system ownership is not encoded in
`UiEventDisposition`.

### Pointer capture

Ordinary `ElementRef::capture_pointer()` remains a deferred host action because
it is carried in the Reactant response. Calling it from a pointer-down handler
does not guarantee capture before the first subsequent native event.

Interactions requiring event-time capture must use a native control adapter or
Motion, both of which already execute in Unity. This design does not turn the
disposition into a general immediate-action list merely to make arbitrary ref
operations synchronous.

If generic event-time pointer capture becomes a required public feature, it
needs its own narrowly typed ABI result and lifecycle contract. It must not be
smuggled into the deferred response or implemented by applying arbitrary
commands during dispatch.

## Motion and High-Frequency Input

Motion remains Unity-local because continuous gesture state cannot afford a
full application render for every native sample.

Motion continues to own:

- hover, press, tap, pan, and drag recognition;
- pointer capture and loss;
- velocity, constraints, and momentum;
- sample coalescing; and
- frame-timed presentation updates.

Reliable Motion boundaries and coalesced samples retain their Motion protocol.
They do not move into `submit_ui_event` solely to share its disposition.

Raw pointer-move, wheel, scroll, and transition events use synchronous UI
submission only when Reactant subscriptions require them. The host must not
forward an unsubscribed high-frequency event.

Reactant should avoid reconciliation when dispatch invokes no handler and
creates no other dirty work. A handler that merely observes an event still
participates in the active entry; application authors are responsible for
keeping synchronous handlers bounded.

## Performance Requirements

Synchronous UI work consumes Unity's main-thread frame budget. The system must
measure the complete blocking interval rather than only the native ABI call or
only Rust handler execution.

The primary measured interval starts when the Reactant coverage callback begins
and ends immediately before that callback returns to UI Toolkit. It includes:

- subscription and target mapping;
- queue reservation and action-ID allocation;
- Unity event conversion and request serialization;
- managed-to-native transition;
- Rust request decoding and route lookup;
- logical handlers and application work;
- Reactant reconciliation;
- response serialization; and
- native-to-managed return and disposition validation;
- response-queue commit; and
- the final native `PreventDefault()` call when required.

It excludes deferred response decoding and command application because those
do not block the active event callback. They remain part of the whole Unity
frame and must be measured separately by existing response-processing tests.

Layer-specific timers may subdivide the primary interval, but the performance
gate uses the outer coverage-callback measurement.

### Reference workloads

Performance evidence uses the repository's reference Apple M5 Max machine and
Unity 6000.5.8f1 running natively on arm64. The player loop uses release
scripting defines, VSync is disabled, the frame rate is uncapped, Deep Profile
is disabled, and no profiler window is repainting during collection.

The retained benchmark suite has stable workload IDs and deterministic setup:

- `noop-depth-8` uses eight nested hosts. One target `KeyDown` handler reads the
  payload, changes no state, and returns an empty response.
- `single-update-{1,10,100,500,1000}` uses the named number of sibling hosts.
  A target `Click` increments one counter rendered as one text-property update
  in a single batch.
- `settings-transition-v1` uses a checked-in 240-host mixed-control tree with a
  maximum logical route depth of eight. One `Click` opens and then closes its
  40-host advanced-settings subtree. The fixture retains the exact serialized
  request, response, command-kind counts, and payload byte counts for both
  directions; changing that manifest creates a new workload ID.
- `burst-32-pointer-move` sends 32 subscribed `PointerMove` events to one target
  before response draining. Each event increments one displayed counter and
  produces one text-property update in one response.
- `stale-target-empty` submits one event for a target removed by the immediately
  preceding undrained response. It invokes no handler and returns an empty
  response.
- `prevent-default-empty` uses one `KeyDown` handler that only calls
  `prevent_default()` and returns an empty response.

Each workload warms for at least 2,000 iterations and measures at least 20,000
iterations. Reports retain p50, p95, p99, and maximum duration plus event count
per frame.

### Performance gates

The representative production screen must satisfy both synchronous gates on
the reference machine:

- p95 blocking duration remains below 4 ms; and
- p99 blocking duration remains below 8 ms.

The complete main-thread frame containing the event and deferred response
application must remain below 16.667 ms at p95. The report also counts frames
above 16.667 ms so a percentile does not hide frequent visible misses.
Synthetic 500-host and 1,000-host results are retained as scaling evidence even
when they are not representative production screens.

A result beyond either synchronous gate or the complete-frame gate blocks
release until the workload is reduced or the implementation is optimized. It
is not addressed by silently making only that event asynchronous, because that
would change public event semantics.

## Failure Handling

Failure behavior favors an uncanceled native event over a partial or
unverifiable application decision. It also recognizes that a handler may have
changed Rust or application state before a later serialization failure.

Failure never calls `PreventDefault()` on Reactant's behalf. It also cannot
clear prevention requested by an earlier native listener. In the descriptions
below, returning `Continue` means Reactant adds no prevention; the active
`EventBase.isDefaultPrevented` flag remains otherwise unchanged.

The runner retains Battlement's existing session phases and explicit reconnect
ownership. A fatal event failure sets `pendingSessionFailure` on the runner
thread. While that field is set, input forwarding is suppressed and any
already-entered bridge callback returns without another Rust call.

The current Unity callback unwinds without running teardown mutations. At the
next normal runner boundary, the pending failure calls the existing
`FailSession` path, which transitions to `Stopped`, clears old-session
responses and batch work, and exposes the normal failure surface. The runner
does not reconnect automatically. The host must explicitly call reconnect
after correcting the failure.

If failure occurs inside a callback raised by batch execution, the current
command callback unwinds first. No further old-session work starts after the
pending failure is observed. An explicit reconnect later supplies the
replacement snapshot.

### Engine or ABI failure

When request decoding, Rust dispatch, response serialization, response
validation, or native output validation fails:

- the effective disposition is `Continue`;
- the reserved response position is released without admitting a payload;
- the runner records a fatal pending session failure because C# cannot safely
  infer whether Rust began or committed the dispatch;
- the Unity callback unwinds normally; and
- the error records the event action ID when decoding progressed far enough to
  identify it.

A Rust panic has the same callback behavior and poisons the native engine
through the existing panic boundary. An explicit reconnect attempt against
that instance follows existing panic recovery and may require the host to
restart or explicitly replace the engine transport. The runner never recreates
the engine automatically. Other failures permit an explicit reconnect to the
surviving engine and therefore snapshot its current committed state.

For a surviving engine, event state, external-store writes, and application
side effects are not rolled back. For a panic, the poisoned in-memory engine is
discarded: only durable or external state that the engine factory reads during
recreation survives. The new snapshot may therefore omit unpersisted mutations
made before the panic.

The failed UI event is never replayed in either case. If the host explicitly
replaces a poisoned engine, its reconnect snapshot reflects only durable or
external state read by the new factory. Applications that require panic
recovery to retain a side effect must persist it independently of the poisoned
engine before exposing that side effect.

### Session mismatch

Rust rejects a UI event whose session does not equal the active session. The
operation returns an engine error rather than a successful empty response,
because accepting input into the wrong session would make ordering ambiguous.

Unity adds no prevention, records the fatal session failure, and does not
enqueue a response.

### Unknown target

An event whose session is current but whose target no longer exists in the
committed Rust tree is an expected state-skew case. Reactant invokes no event
handler, then completes the normal active-entry flush. The disposition is
`PreventDefault` only when the incoming event was already prevented; no
Reactant handler adds prevention. The ordinary response contains any pending
work produced by that flush and is empty only when no such work exists. The
inspector records the stale target. The session remains healthy.

### Deferred application failure

The response has already passed the synchronous boundary when Unity later
applies it. A batch failure cannot undo `PreventDefault()`.

The runner reports the existing batch failure and associates it with:

- the UI event action ID;
- the immediate disposition;
- the response or batch ID; and
- the command group that failed.

The runner follows existing batch-failure semantics. A recoverable command
failure reports `BatchFailed` and stops that batch; a fatal protocol or session
failure stops the session. Neither path replays the native default or resubmits
the UI event.

### Queue pressure

The runner reserves one response item before calling Rust. If no item is
available, it does not call Rust, returns `Continue`, and records a fatal
pending session failure.
Dropping a subscribed UI event while continuing the same session would
silently diverge application behavior, so queue pressure is not a recoverable
skip.

After Rust returns, Unity charges the response's exact serialized size against
the byte budget. Failure releases the item reservation, returns `Continue`, and
records the same fatal pending failure; an explicit reconnect snapshot accounts
for state Rust may already have committed. A violated token, payload-size, or
ownership invariant follows the same transition.

There is no separate UI-event queue, acknowledgement command, or cleanup
reserve. The ordinary response queue and reconnect snapshot provide the only
admission and realignment protocol; reconnect remains host-initiated.

## Diagnostics

The development event inspector must make the split timing visible. A reader
should be able to tell what Rust decided immediately and what Unity applied
later.

One inspection record contains at least:

```rust
pub struct EventInspection {
    pub action_id: ActionId,
    pub session_id: SessionId,
    pub island_id: ObjectId,
    pub target_id: ObjectId,
    pub kind: UiEventKind,
    pub cancelable: bool,
    pub prevented_before_reactant: bool,
    pub prevented_by_reactant: bool,
    pub disposition: UiEventDisposition,
    pub admission_sequence: Option<u64>,
    pub synchronous_duration_us: u64,
    pub resulting_batch_ids: Vec<BatchId>,
    pub outcome: EventInspectionOutcome,
    pub failure_reason: Option<EventFailureReason>,
}

pub enum EventInspectionOutcome {
    Pending,
    Completed,
    StaleTarget,
    RejectedBeforeDispatch,
    FailedAfterDispatch,
    DeferredApplyFailed,
}

pub enum EventFailureReason {
    QueueItemLimit,
    QueueByteLimit,
    SessionNotAccepting,
    RequestValidation,
    StaleSession,
    NativeTransport,
    Engine,
    ResponseSerialization,
    InvalidDisposition,
    ResponseCommitInvariant,
    Panic,
    DeferredApply,
}
```

The actual record also identifies:

- logical route IDs and handler phases invoked;
- whether logical propagation stopped;
- whether the native adapter applied `PreventDefault()`;
- whether the event was dropped for an unknown target;
- serialization, native-call, Rust-dispatch, and reconciliation durations when
  that layer can report them;
- response byte size, queue admission time, and later application time;
- deferred batch failure, when one occurs; and
- whether the event originated during ordinary input or response application.

`Pending` is the only nonterminal value. A completed record has one terminal
value. Precedence is:

1. `DeferredApplyFailed` if a successfully admitted response later fails.
2. `FailedAfterDispatch` if the ABI may have entered Rust but did not return a
   trustworthy complete result.
3. `RejectedBeforeDispatch` if reservation, validation, or session state
   prevented entry into Rust.
4. `StaleTarget` for the successful current-session no-handler case.
5. `Completed` for every other successfully admitted response.

The inspector creates the record when an action ID is allocated, updates the
same record through admission and application, and never emits separate records
for the immediate and deferred halves. An unmapped native-only event has no
action ID and is visible only in optional coverage diagnostics.

`failure_reason` is required for every failed or rejected outcome and absent
for `Pending`, `Completed`, and `StaleTarget`. The originating layer chooses the
most specific reason; a generic transport reason is used only when the native
status cannot distinguish engine, serialization, or panic failure. An event may
produce zero or several batches, so inspection retains every causal batch ID in
response order.

The inspector does not record typed text contents, composition contents, or
other sensitive payload values by default.

Stable diagnostics include:

- `reactant.event.stale_session` for a session mismatch;
- `reactant.event.stale_target` for a current-session unknown target;
- `reactant.event.invalid_disposition` for an unknown ABI value;
- `reactant.event.submit_failed` for transport or engine failure;
- `reactant.event.response_rejected` for failed response admission;
- `reactant.event.prevented` for an applied default-prevention decision; and
- `reactant.event.deferred_apply_failed` for a later response failure.

The terminal record selects its primary stable code as follows:

- `StaleTarget` uses `reactant.event.stale_target`.
- `StaleSession` uses `reactant.event.stale_session`.
- `InvalidDisposition` uses `reactant.event.invalid_disposition`.
- `QueueItemLimit`, `QueueByteLimit`, and `ResponseCommitInvariant` use
  `reactant.event.response_rejected`.
- `DeferredApply` uses `reactant.event.deferred_apply_failed`.
- Every other failed or rejected reason uses `reactant.event.submit_failed`.
- `Completed` has no primary failure code.

`reactant.event.prevented` is emitted only when
`prevented_by_reactant == true` and Unity successfully applies the disposition.
Prior-only native prevention remains visible in the inspection fields but does
not claim that Reactant caused it.

When several conditions occur, the terminal outcome above selects the primary
diagnostic. Cleanup failures and engine poisoning are attached as secondary
fields rather than replacing the event's causal failure.

## Migration and Compatibility

The engine, ABI, Rust protocol, managed transport, fake client, and Unity event
bridge change atomically. No compatibility shim or alternate UI path remains.

The migration performs these contract changes:

- Add `Engine::submit_ui_event` to every engine implementation.
- Add `UiEventAction`, `UiEventResponse`, and `UiEventDisposition`.
- Export `battlement_submit_ui_event` from every native engine.
- Add `POST /ui-events` to the localhost development server.
- Add `IBattlementTransport.SubmitUiEvent` to every managed transport.
- Route every Unity-produced `UiEvent` through the specialized operation.
- Remove `ActionBody::VisualElement` from the ordinary action union.
- Remove ordinary `submit` handling for UI events from engines and fake clients.
- Add shared Rust, ABI, and C# disposition fixtures.
- Update related Reactant documents that claim Rust cannot participate during a
  native input event.

Game-specific engines move their existing `ActionBody::VisualElement` match arm
into `submit_ui_event`. Their ordinary custom-action and command unions do not
gain any event disposition variant.

The change is wire-breaking and ABI-breaking. Rust and C# definitions land
together. The repository does not add protocol negotiation, dual symbols,
legacy deserialization, or version-specific behavior.

An engine with no mounted UI documents still implements
`submit_ui_event`. With no registered hosts, a current-session event is handled
as a stale target: it invokes no UI handler, adds no prevention, and returns the
ordinary response from its active entry. UI capability is not negotiated per
application.

Binary mismatch fails before session connection:

- a Unity client that cannot resolve `battlement_submit_ui_event` reports the
  missing symbol and refuses to connect;
- the exported-library fixture verifies the exact symbol and ABI shape;
- an old request containing `ActionBody::VisualElement` fails ordinary action
  decoding rather than entering an asynchronous compatibility path; and
- an unknown raw disposition returned to C# fails the active session and is
  never interpreted as prevention.

## Alternatives Considered

The rejected alternatives clarify why the dedicated synchronous operation is
part of the target architecture.

### Disposition on every ordinary response

Adding `ActionOutcome` or an optional disposition field to `Response` would
make every Battlement action understand a UI Toolkit callback-lifetime problem.
It would also require C# to decode response metadata while the event is active.

The dedicated operation keeps `Response` generic and returns the fixed
disposition without decoding deferred commands.

### Optional UI engine extension

An extension trait would imply that some Battlement engines or transports lack
the UI subsystem. Battlement snapshots, the runner, native exports, UI events,
and fake clients already treat UI as a core capability.

Putting `submit_ui_event` directly on `Engine` makes missing support a compile
error rather than a runtime feature branch.

### Asynchronous event delivery with native policies

An asynchronous callback cannot decide `PreventDefault()` before Unity runs the
default. A parallel declarative policy system could make selected decisions
from state installed by an earlier commit, but it would require duplicate Rust
and C# evaluators, selector validation, route snapshots, queue ordering,
acknowledgements, stale-event rules, and different authoring semantics.

The current production transport is already synchronous. Reactant chooses one
honest event model instead of preserving hypothetical asynchronous UI hosts.

### Disposition as a command

A command is processed after the event callback, when prevention is too late.
Special-casing one command for early application would break batch ordering and
blur the boundary that protects UI Toolkit dispatch from tree mutation.

The disposition is an ABI result, never a command.

### Applying the complete response during dispatch

Applying Reactant mutations before the callback returns could destroy,
reparent, focus, or otherwise modify elements in UI Toolkit's active propagation
path. It would also permit command-triggered events to reenter the engine before
the current Rust call had returned.

Only the fixed disposition crosses the immediate boundary. All commands remain
deferred.

### Retaining the Unity event object

Keeping a managed reference or acquiring a pooled event does not extend the
period during which UI Toolkit consults its prevention flag. Calling
`PreventDefault()` after dispatch cannot undo a default action.

The bridge keeps the event only on the active callback stack and applies the
disposition before returning.

## Acceptance Criteria

The design is complete when all of these externally reviewable conditions hold:

- **EVT-01:** Every engine and transport exposes synchronous UI event
  submission.
- **EVT-02:** `ActionBody::VisualElement` and its generic submit path are
  absent.
- **EVT-03:** Every forwarded `UiEvent` uses `submit_ui_event` exactly once.
- **EVT-04:** A Rust handler can dynamically prevent a cancelable Unity
  default.
- **EVT-05:** Initial native prevention is visible to every Rust handler and
  survives the round trip.
- **EVT-06:** A non-cancelable event reports `cancelable() == false` and
  returns `Continue`.
- **EVT-07:** `stop_propagation()` follows the defined handler-slot semantics
  without stopping physical Unity propagation.
- **EVT-08:** No response command executes while `uiDispatchDepth` is nonzero.
- **EVT-09:** Successful serialized responses enter the existing response
  stream later in total admission order without changing batch execution
  semantics.
- **EVT-10:** Nested events receive immediate independent dispositions without
  reentering an active Rust call or response drain.
- **EVT-11:** A current-session stale target invokes no handler and does not
  newly prevent the native default.
- **EVT-12:** Unmapped native events allocate no action and make no Rust call.
- **EVT-13:** A session mismatch, queue-admission failure, or engine failure
  never adds prevention, stops the session, and requires an explicit host
  reconnect. Earlier native prevention remains set.
- **EVT-14:** A surviving engine retains state committed before a later
  transport or serialization failure. A panic may discard unpersisted engine
  state. Neither path replays the failed event; an explicit host reconnect
  supplies the replacement snapshot.
- **EVT-15:** A deferred response failure never causes event replay.
- **EVT-16:** Portaled events use logical Reactant ancestry and physical Unity
  targeting.
- **EVT-17:** Native controls retain editing, tracking, scrolling, and focus
  internals.
- **EVT-18:** Motion retains Unity-local continuous gesture behavior.
- **EVT-19:** The inspector connects each disposition to its later response
  application or explicit-reconnect outcome.
- **EVT-20:** Representative production screens satisfy the synchronous and
  complete-frame performance gates.

In addition, the following compatibility condition is required:

- **EVT-21:** Old and new native binaries fail at startup or decoding; they do
  not silently choose different event timing.

## Automated Validation

Tests must prove both sides of the timing boundary. Pure Rust tests cannot prove
that Unity observes `PreventDefault()` at the correct phase.

Each retained test names the applicable `EVT-*` criteria in its test
description or fixture metadata. Every criterion must have at least one
automated test except behavior that the Manual QA section explicitly reserves
for profiler or platform inspection.

### Rust tests

Rust black-box tests cover:

- identical Rust and C# propagation classification for every `UiEventKind`;
- capture, target, and bubble ordering;
- every row of the propagation-stopping table;
- default prevention from every logical phase;
- an event that arrives already prevented;
- shared prevention state across cloned event values;
- logical propagation stopping independent of prevention;
- non-cancelable prevention as a no-op;
- one reconciliation and one response per accepted event;
- active handler snapshots when state removes a host;
- current-session unknown targets;
- session mismatch rejection;
- a session with no mounted UI documents;
- portal logical routes;
- empty successful UI responses when no other active-entry work is pending;
  and
- caused-by action identity on resulting batches.

### Native ABI tests

Exported-library tests load the real symbols and cover:

- the presence and exact signature of `battlement_submit_ui_event`;
- request decoding and complete response serialization;
- `Continue` and `PreventDefault` numeric values;
- disposition initialization before engine invocation;
- safely testable null engine, request, and output-pointer cases;
- a null request with nonzero length and poisoned initial output values;
- engine errors, serialization errors, and panics;
- serialization failure after a handler committed state;
- panic after durable and in-memory mutations, proving that recreation retains
  only state supplied again by the engine factory;
- no `PreventDefault` output on any failure;
- response-buffer ownership and freeing;
- engine disposal waiting for an active submission to leave the native call
  gate; and
- startup failure when the UI event symbol is absent.

### Localhost HTTP tests

The real development server and managed HTTP transport cover:

- `POST /ui-events` accepting exactly one serialized `UiEventAction`;
- HTTP 200 returning the unchanged ordinary response body;
- required `Continue` and `PreventDefault` header values;
- a missing, duplicate, malformed, or unknown disposition header;
- HTTP 400, HTTP 500, timeout, refusal, and diagnostic bodies adding no
  prevention and stopping the session;
- identical action IDs, response bytes, and inspection outcomes to the native
  fixture; and
- response admission order shared with ordinary HTTP submit and poll.

### Unity EditMode tests

Unity tests construct real UI Toolkit elements and controls. They cover:

- prevention from the root trickle-down callback before target defaults;
- cancelability for every forwarded event family;
- every inventory source forwarding exactly once;
- initially prevented input remaining prevented through Rust dispatch;
- key, navigation, pointer, click, and wheel default prevention;
- post-change events that remain non-cancelable;
- native callbacks outside Reactant observing the default-prevention flag;
- logical `stop_propagation()` leaving physical listeners untouched;
- no Unity mutation while `uiDispatchDepth` is nonzero;
- deferred response decoding and admission at the next safe drain point;
- nested focus, detach, and value events during response application;
- total ordering across UI, ordinary-submit, and poll responses;
- no recursive drain when a nested event completes synchronously;
- old and new subscription state during the state-skew interval;
- an intentionally unmapped event making no transport call;
- pointer-capture owner precedence over the physical target;
- focused-host targeting for key, navigation, and focus events;
- internal-control owner precedence over physical ancestry;
- nearest registered physical ancestor mapping;
- target ineligibility without ancestor fallback;
- a logical-ancestor-only subscription admitting the event;
- cross-island target rejection;
- adapter-only events using their explicit owning host;
- engine failure adding no prevention, with an unprevented native default
  proceeding;
- failure on initially prevented input preserving prior prevention;
- invalid raw disposition adding no prevention and stopping the session;
- failure after Rust commits requiring explicit reconnect with no event replay;
- the runner remaining stopped until the host explicitly reconnects;
- a poisoned native engine requiring explicit host restart or replacement;
- stale target handling during state skew; and
- response-admission failure adding no prevention and stopping the session.

### Fake-client tests

The fake client uses `Engine::submit_ui_event` directly and verifies:

- identical action and response serialization;
- identical disposition values;
- one UI event producing one ordinary response;
- state changes before the next fake UI dispatch;
- deferred client-side response application;
- absence of the removed `ActionBody::VisualElement` path;
- old visual-element action decoding failure; and
- no-UI-engine stale-target behavior.

The fake client does not claim to reproduce UI Toolkit's control defaults. Real
Unity tests remain authoritative for whether a native default is cancelable and
what behavior prevention suppresses.

### Diagnostics tests

Inspector tests follow one record from `Pending` through its terminal state and
cover:

- `Completed`, `StaleTarget`, `RejectedBeforeDispatch`,
  `FailedAfterDispatch`, and `DeferredApplyFailed`;
- terminal-outcome precedence when cleanup also fails;
- the specific failure reason and stable diagnostic code for every failure
  injection point;
- action ID, admission sequence, every resulting batch ID, and application
  timestamps;
- prior native prevention versus prevention requested by Reactant; and
- one record spanning immediate disposition and deferred application, without
  a duplicate completion record.

### Performance tests

Performance tests retain the environment, warm-up, sample count, tree size,
event kind, handler behavior, response size, and per-frame event count alongside
the recorded percentiles.

The test report separates:

- synchronous callback latency;
- deferred response decoding and command application; and
- complete frame duration.

This separation prevents a fast disposition result from hiding expensive later
application, or a fast frame average from hiding an event-time input hitch.

## Manual QA

Manual QA opens `Battlement/Reactant/Event Lab` in a development build and
keeps `Window/Battlement/Reactant Event Inspector` visible. The Event Lab has
named panels for `Keyboard Default`, `Native Text`, `Propagation`, `Portal`,
`State Skew`, `Nested Ordering`, and `Recovery`. Each flow checks both the
immediate disposition and the later deferred response.

The `Recovery` panel provides `Fill 256 Queue Slots`, `Fail Serialization After
Commit`, `Panic After Commit`, `Prevent Before Reactant`, `Reconnect`, and
`Recreate Engine` controls. The `State Skew` panel provides
`Hold Response Drain` and `Emit Stale Event`. `Nested Ordering` can admit one
ordinary response, one UI response, and one poll response in a labeled order.
These are deterministic diagnostic fixtures, not production behavior switches.

1. In `Keyboard Default`, focus the custom slider inside its scrollable parent
   and press Right Arrow. Verify that the slider changes once, the parent does
   not scroll, the inspector reports `PreventDefault`, and the resulting Unity
   mutation applies only after the key callback returns.
2. Repeat the slider test with a handler condition that allows the default.
   Verify that the inspector reports `Continue` and UI Toolkit performs its
   normal behavior.
3. In `Native Text`, edit the native text field with characters, arrows,
   selection
   modifiers, paste, and IME. Verify that native editing remains correct, input
   notifications report non-cancelable behavior, and Reactant state follows
   through deferred controlled-value updates.
4. In `Propagation`, select `Stop At Target` and activate the nested button.
   Verify that the Reactant ancestor does not run while the labeled external
   Unity listener still follows native propagation.
5. Prevent the button's cancelable event. Verify that an external later Unity
   listener observes the prevention flag and that no Reactant command executes
   during the active callback.
6. Install an earlier native listener that prevents the event before Reactant.
   Verify that the first Rust handler sees `default_prevented() == true`, the
   inspector distinguishes prior prevention from Reactant prevention, and the
   event remains prevented.
7. In `Portal`, open the overlay and activate its nested control. Verify that
   the inspector shows source-side logical ancestry while external Unity
   listeners follow the portal's physical path.
8. In `State Skew`, select `Hold Response Drain`, activate `Remove This Panel`,
   and then select `Emit Stale Event` before releasing the drain. Verify that
   the second Rust dispatch invokes no stale handler, returns `Continue`, and
   the queued removal later applies once.
9. In `Nested Ordering`, activate `Focus During Apply` and then `Detach During
   Apply`. Verify that each event's Rust response is queued behind admitted
   work and is not recursively applied inside the callback.
10. In `Nested Ordering`, select `Admit Labeled Sequence`. Verify that the
    inspector's sequence matches response-stream admission order. Then verify
    that command execution follows each labeled batch's declared start and
    dependency settings rather than response-wide atomicity.
11. In `Recovery`, select `Fill 256 Queue Slots`, then activate the unprevented
    test button. Verify that Rust is not called, Unity performs the default,
    input becomes disabled, and the runner remains stopped. Select `Reconnect`
    and verify that one replacement snapshot restores a running session.
12. In `Recovery`, select `Fail Serialization After Commit`, then activate the
    unprevented test button. Verify that Unity performs the default, the event
    is not replayed, queued old-session work is discarded, and the replacement
    snapshot contains the committed Rust counter.
13. In `Recovery`, select `Panic After Commit`, then activate the test button.
    Verify that Unity adds no prevention, no partial response is admitted,
    and the runner remains stopped. Verify that reconnecting the poisoned
    instance fails, then select `Recreate Engine` and `Reconnect`. The durable
    test marker must survive factory recreation and the volatile counter must
    return to its factory value.
14. Run `Battlement/Reactant/Run Event Benchmarks`. Verify every stable workload
    ID reports the retained p50, p95, p99, maximum, events per frame, request
    and response bytes, deferred-application time, and complete-frame result
    against all three performance gates.
