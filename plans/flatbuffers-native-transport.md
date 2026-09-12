# Safe, direct FlatBuffers communication between Rust and Unity

Battlement runs Rust game rules inside a Unity process. Its native bridge still
serializes protocol messages as JSON, copies returned bytes into managed memory,
and reconstructs C# objects. Replace that path with directly constructed
FlatBuffers and verified readers over immutable storage. Safety and measured
end-to-end performance take precedence over retaining the current authoring
APIs.

Both directions use synchronous transport processing on the Unity owning thread.
Remove background protocol encoding and decoding. Preserve ordered scheduling
and asynchronous game operations, which are separate from message processing.
This document specifies the target implementation; it does not claim that a
FlatBuffers implementation has already been benchmarked or proven safe.

## Related information

- [Native engine contract](../crates/battlement-native/src/engine.rs): current
  owned inputs, responses, and serial, non-reentrant calls.
- [Native adapter](../crates/battlement-native/src/adapter.rs): current
  allocation, panic, and buffer transfer boundary.
- [Response
  stream](../Packages/com.battlement.client/Runtime/Host/BattlementResponseStream.cs)
  and
  [scheduler](../Packages/com.battlement.client/Runtime/Host/BattlementBatchScheduler.cs):
  ordering, background decoding, and retention across frames.
- [Reactant reconciliation](../crates/battlement-reactant/src/reconcile.rs):
  retained element values, sparse updates, and current JSON-based property diff.
- [FlatBuffers Rust
  verification](https://flatbuffers.dev/languages/rust/#access-of-untrusted-buffers)
  and [C# support](https://flatbuffers.dev/languages/c_sharp/): generated
  readers, verification, and native-memory integration.
- [FlatBuffers construction](https://flatbuffers.dev/internals/#construction):
  buffer offsets and child-before-parent construction.
- [Pinned C# buffer
  implementation](https://github.com/google/flatbuffers/blob/v25.12.19/net/FlatBuffers/ByteBuffer.cs):
  concrete allocator hooks and bounds-check behavior to audit.

## Guarantees and their boundaries

**Zero-copy handoff** means that the receiver reads the exact finished bytes the
sender constructed, without a transport payload copy or an unpacked protocol
object graph. It does not mean zero writes, zero allocations, or free
validation.

- Rust constructs responses directly in FlatBuffers builders. C# constructs
  requests directly in its builders. Do not retain a normal-object-to-buffer
  `Pack` layer as the production transport API.
- Receivers verify every newly received buffer before using its generated
  readers. Verification and ownership checks remain enabled in release builds.
- Published bytes never change and never return to a builder or pool while a
  reader can legally access them.
- Rust references cannot outlive their storage. C# access after lease disposal,
  wrong-thread access, and access after host shutdown throw before reading
  bytes.
- No protocol-specific pointers or native field getters cross the ABI. A single
  bounded buffer describes each request or response.
- Ordinary string conversion, constructing new state from old state, and builder
  capacity growth can copy bytes. Measure these costs separately from handoff.

These guarantees cover correctly implemented trusted Rust/C# code and malformed
message contents. They do not sandbox arbitrary native plugins or malicious
unsafe code within the process. A verifier cannot establish that an arbitrary
native pointer addresses live memory; the bridge must establish that first.

## Schema and generated APIs

FlatBuffers schemas become the source of truth for boundary types. Generate Rust
and C# together with a pinned compiler and matching runtimes. Use upstream
v25.12.19 as the initial reproducible baseline, record artifact hashes, and
audit it for relevant known fixes before implementation lands. Upgrades require
the same generation, malformed-input, and platform checks; never fetch a
floating compiler during a build.

Use one build-composed schema containing core and game-specific command, action,
and error unions. Custom payloads are typed tables in the same buffer, not JSON,
reflection-based objects, or independently packed byte blobs. Adding a custom
type requires schema generation and an explicitly registered handler.

- Use tables for sparse properties and variable-length records. Use fixed
  structs for dense numeric values and arrays of geometry or motion samples.
- Flatten the transmitted UI hierarchy into node records with IDs and ordered
  child-ID vectors. UI depth must not become verifier recursion depth. Validate
  each document as an acyclic forest with unique IDs, one parent per non-root
  node, and no dangling or cross-document child references. Use an iterative
  traversal, visit each node/edge once, and cap logical depth at 1,024. Check
  prospective reparent operations against host hierarchy metadata before
  applying them, so incremental commands cannot introduce cycles either.
- Encode IDs as exactly 16 canonical UUID bytes; test C# Guid byte ordering
  explicitly. Never reinterpret a Rust UUID or C# Guid layout.
- Replace modifier lists with a fixed-width bit mask; reject unknown bits.
- Preserve session IDs, command IDs, action correlation, snapshots, parallel
  groups, delays, dispositions, failures, and ordering with explicit fields.
- Reject unknown union tags, invalid scalar enum values, missing required
  payloads, and duplicate object/property identities. Matching generated code is
  required; no mixed-version protocol or JSON fallback is supported.

**Prop** is Battlement's three-state property operation: leave unchanged, assign
a value, or restore its documented default. Missing fields alone cannot express
all three states. Generate a typed wrapper per value kind, as illustrated here:

```fbs
enum PropState : ubyte { Unset, Set, Reset }
table BoolProp { state:PropState; value:bool; }
table TextProp { state:PropState; value:string; }
table Button { enabled:BoolProp; text:TextProp; }
```

An absent property means Unset. Present wrappers must be Set or Reset.
Set(false) and Set(0) remain assignments even when a scalar is omitted by
FlatBuffers default elision. Set text/collection requires a present value; an
empty value is valid. Reset has no variable-length payload and uses the default
scalar slot value. The semantic validator enforces these canonical forms.

Generate narrow ergonomic wrappers from schema metadata where useful. Generated
FlatBuffers object APIs, unchecked root readers, mutable readers, and raw table
initialization are not public Battlement APIs. Schema coverage tests must fail
when a protocol variant lacks a reader, writer, validator, or host handler.

## Direct construction and retained Rust state

Rust authoring uses an explicit message builder context. Styles, buttons, and
commands under construction are typed offsets belonging to that context. Fields
and variable-length values are written directly into its storage; completion
adds tables, vectors of offsets, and the root, rather than traversing an owned
protocol graph. Normal game state can remain ordinary Rust data.

The following before/after examples use the UI sample's
[navigation helper](../samples/ui/rules/src/components.rs) and
[style helpers](../samples/ui/rules/src/design_system.rs). Before snippets show
current code, with formatting shortened. After snippets specify proposed
Battlement wrappers, not built-in FlatBuffers APIs or currently compiling code.
Imports and surrounding admission/error handling are omitted throughout these
examples; the lifetime and failure contracts in this document still apply.

### Rust: construct styles and buttons for C#

Today the navigation helper returns an independently owned node:

```rust
fn navigation_item(object_id: ObjectId, text: &str, active: bool) -> UiNode {
  UiNode::new(
    object_id,
    UiButton::new(text)
      .events([UiEventKind::Click])
      .style(design_system::navigation_item(active)),
  )
}
```

Afterward it accepts the caller's writer and returns an offset into that writer:

```rust
fn navigation_item(
  message: &mut MessageWriter, object_id: ObjectId, text: &str, active: bool,
) -> NodeOffset {
  let style = design_system::navigation_item(message, active);
  let button = message.button(text, style, UiEventKind::Click);
  message.node(object_id, button)
}
```

The node remains in the caller's buffer; constructing it does not automatically
emit a create command. The caller attaches it to its snapshot or command using
the same writer. Style helpers likewise receive that writer. For example, the
smaller brand style currently constructs an owned Style:

```rust
pub(crate) fn brand() -> Style {
  Style::new().color(CYAN).font_size(30.0).margin(6.0)
}
```

Its replacement writes the style table directly:

```rust
pub(crate) fn brand(message: &mut MessageWriter) -> StyleOffset {
  message.style().color(CYAN).font_size(30.0).margin(6.0).finish()
}
```

The style builder exclusively borrows the writer until finish. Children with
strings or nested tables are constructed before opening their parent table;
the wrapper must respect FlatBuffers construction order. Offset wrappers carry
a private builder identity and type. Every composition operation
checks the identity, including in release builds, before accepting an offset.
Reuse across builders or after reset is a developer error. Helpers accept the
context explicitly; no hidden global builder or per-object private FlatBuffer.

### Rust: emit a status-label update

The current [button-status helper](../samples/ui/rules/src/lib.rs) allocates
command and parallel-group vectors around an owned label:

```rust
fn button_status_commands(message: &str) -> Vec<ParallelCommandGroup<Command>> {
  vec![ParallelCommandGroup::new(vec![
    Command::update_visual_element(BUTTON_STATUS_ID, UiLabel::new(message)),
  ])]
}
```

The replacement returns a group offset so the caller preserves batch ordering:

```rust
fn button_status_commands(
  message: &mut MessageWriter, text: &str,
) -> ParallelGroupOffset {
  let label = message.label(text);
  let command = message.update_visual_element(BUTTON_STATUS_ID, label);
  message.parallel_group(&[command])
}
```

The caller attaches the group to its batch and finishes the response. The text
is written once into response storage; no owned UiLabel or command graph is
packed afterward. Offset vectors and table metadata still require construction.
The array above is a small local array of offsets, not copied label payloads.

### Retained component state

Retained UI state requires a separate, explicit policy. Reactant keeps immutable
FlatBuffer-owned element snapshots and scalar indices into them. It reconstructs
readers from an owner plus offset; it does not store self-referential Rust
borrows or reconstruct all fields into the old owned element types.

Snapshots store complete desired declarations, including absent properties and
explicit resets, rather than resolved Unity defaults. Removing a previously
declared property emits Reset during reconciliation. Unset in an emitted patch
still leaves the live value unchanged. Values intended to remain declared are
authored into the new snapshot; it never reads through a previous generation.

- Render a changed component's desired elements into a fresh snapshot buffer;
  compare its verified readers against previous readers using generated typed
  property comparisons. Remove JSON value conversion from reconciliation.
- Emit only the resulting sparse commands into the response builder. Copy the
  changed values needed by these commands directly from the snapshot readers.
  This is an explicit reconstruction cost, including changed strings; it is not
  a second serialization at the native handoff.
- Unchanged components retain their existing snapshot owners. Drop replaced
  component snapshots as a whole. Each render replaces every node owned directly
  by that component; child components have independent snapshots. Never retain
  selected old nodes alongside new nodes from the same component generation.
  Reconstruct surviving values into the new snapshot so old generations cannot
  accumulate. Exit animations may retain an old snapshot explicitly, charged to
  the same snapshot budget, until completion or cancellation.
- Reparenting and identity-only operations write IDs without copying element
  payloads. Snapshot replacement reconstructs the full live state when needed.
- Keep all offsets inside their own FlatBuffer. Do not introduce external
  pointers, cross-buffer offsets, or a custom segmented transport to avoid a
  state-reconstruction copy.

This deliberately favors a verifiable single-buffer transport over a claim of no
copying anywhere in reconciliation. The performance gate includes snapshot
construction, property comparison, sparse response construction, and retained
memory. Wrappers must not conceal these costs behind apparently cheap Clone or
mutation operations. In-place mutation of published tables is forbidden.

## Small native boundary

Keep native exports limited to engine lifecycle, connect/submit/UI-submit/poll,
contract inspection, response-buffer inspection, release, and diagnostics. The
ABI does not grow when a style property or command variant is added.

Engine and output allocation identities are nonzero 64-bit integer handles in
Rust-owned registries. Validate handles before lookup; never cast a caller's
integer back into an allocation pointer. Detect stale and duplicate releases.
Handles are never reused within a plugin lifetime; exhaustion is fatal.

The following conceptual signatures show the ownership contract:

```c
Status submit(uint64_t engine, const uint8_t* input, uint64_t length,
              uint64_t* output_handle);
Status buffer_info(uint64_t output_handle, const uint8_t** data,
                   uint64_t* length, uint64_t* allocation_bytes);
Status release_buffer(uint64_t output_handle);
```

Connect and synchronous UI submit follow the same pattern; poll has no input.
The UI disposition is part of the verified result. Initialize every output
before work. NoMessage has a zero handle; success has one owned result; failure
has no partially usable response. Error diagnostics use the same bounded owner
mechanism with an explicitly selected diagnostic root.

The registry keeps the original builder allocation and finished range. Do not
trim via a copying conversion to boxed slices or create a second byte vector.
`buffer_info` exposes only initialized finished bytes; unused capacity is never
readable. Release drops the registry's owner or returns exclusively owned
storage to its pool. Rust retained snapshots can keep a separate Rust owner.

All ABI sizes, pointer-width conversions, lengths, alignment requirements, and
checked arithmetic receive target-specific tests. The existing build contract
handshake must include the schema closure, generation options, runtime pins, and
ABI contract digest. Refuse a mismatched plugin before passing any payload. This
is exact build matching, not protocol version negotiation.

## Rust to C#: ownership and safe readers

Rust finishes the response and relinquishes all mutable access before exposing
its handle. C# obtains the finished range, enforces allocation limits, and
creates an internal read-only native-memory allocator for FlatBuffers. Verify
the entire root before making a response available to host code.

A **lease** is an independently disposable right to read one immutable buffer.
C# uses one shared buffer owner and lightweight reader structs containing a
lease reference and a table offset. The owner holds the native handle, pointer,
length, host generation, owning thread, and reference count of active leases.

- A reader checks its lease, host generation, and owning thread before every
  access. Default-initialized readers are invalid. Nested readers carry the same
  lease; copying a reader does not create an independent lifetime.
- Retain creates a separate lease; disposing one lease does not invalidate
  another. The final live lease releases the native handle exactly once.
- Disposed access throws ObjectDisposedException. Wrong-thread access throws
  InvalidOperationException. Both checks occur before pointer arithmetic.
- No pointer, ByteBuffer, generated table, native-backed Span, or Memory object
  escapes the internal binding assembly. Public APIs return scalars, guarded
  collection/string views, or deliberately owned strings and arrays.

For example, retaining a command for later execution is explicit:

```csharp
using var pending = response.RetainCommand(0);
response.Dispose();
Apply(pending.Command); // Its independent lease remains valid.
pending.Dispose();
Apply(pending.Command); // Throws before reading native memory.
```

Compile the C# runtime with ENABLE_SPAN_T and UNSAFE_BYTEBUFFER as required by
the pinned allocator implementation, but never BYTEBUFFER_NO_BOUNDS_CHECK. Audit
every runtime read path, including float, double, UTF-8, and vectors; these
flags alone are not a proof of safety. Make the native allocator's mutable Span,
Memory, and GrowFront paths throw. Expose its ReadOnlySpan only internally after
the lifetime/thread check; reject escaping ReadOnlyMemory paths.

Internal span use is synchronous and contains no callbacks, await, release, or
reentrant host work. The guard stays alive until the native read completes.
Decode or copy values before calling Unity APIs that may trigger user callbacks.
The public wrapper layer is generated; its unsafe storage owner is hand-audited.

### C#: apply a label received from Rust

Today [text application](../Packages/com.battlement.client/Runtime/UI/BattlementUiTypographyProperties.cs)
receives an already decoded managed string inside Prop:

```csharp
private static void ApplyText(
    TextElement target, Prop<string> value, string constructorDefault)
{
    if (value.IsSet)
        ((INotifyValueChanged<string>)target).SetValueWithoutNotify(value.Value);
    else if (value.IsReset)
        ((INotifyValueChanged<string>)target).SetValueWithoutNotify(constructorDefault);
}
```

The new helper receives a guarded property view. Only Set materializes text:

```csharp
private static void ApplyText(
    TextElement target, TextPropView value, string constructorDefault)
{
    if (value.IsSet)
    {
        string text = value.Value.ToManagedString();
        ((INotifyValueChanged<string>)target).SetValueWithoutNotify(text);
    }
    else if (value.IsReset)
        ((INotifyValueChanged<string>)target).SetValueWithoutNotify(constructorDefault);
}
```

IsSet, IsReset, Value, and ToManagedString perform the prescribed lease checks.
The native read and UTF-8 conversion finish before calling Unity. Unset performs
no assignment, and Reset uses the same constructor default as today. The
scheduler passes a LabelView to the containing typography helper instead of an
owned UiElement.Label; there is no label-wide unpack operation.

### Retain only for the operation lifetime

The scheduler owns leases through delay, parallel-group execution, and operation
completion. Custom asynchronous handlers receive a lease scoped to their tracked
operation. Off-thread work takes explicitly owned values, never message views.
Cancellation invalidates the operation's views before freeing their storage.
Buffers remain charged to the live allocation budget after leaving the queue.

Finalizers never call Rust or Unity. As a leak backstop they enqueue a lease
release for the owning thread. Normal completion and cancellation dispose
deterministically. A host-owned registry tracks every native buffer
independently of managed reader reachability, including buffers awaiting
finalizer cleanup. Shutdown runs outside all read scopes: stop new work,
invalidate the shared host generation, cancel operations, and terminally release
every registered native handle regardless of remaining managed lease references.
Then drain obsolete release records and check the registry before engine
destruction and unload. Normal final release removes its registry entry. Queued
and late releases check the generation and entry state, so they cannot free a
handle a second time. No release may call an unloaded plugin.

## C# to Rust: scoped borrowing

C# writes requests directly into a reusable managed byte-array builder. It pins
only the finished range for one synchronous call, passes its pointer and length,
and unpins in finally. It never calls ToSizedArray, Marshal.Copy, or another
payload-materializing helper on this path. Growth before pinning is allowed;
growth, reset, pool return, and mutation during the call are forbidden.

Rust creates a temporary byte slice inside the audited ABI function, verifies
the root, validates its semantics, and invokes the engine with lifetime-bound
readers. Replace DeserializeOwned input bounds and owned callback payloads with
borrowed generated inputs. No lifetime extension, transmute, or reconstruction
of a Vec from foreign storage is permitted.

### C#: send a text commit to Rust

Today the [UI event dispatcher](../Packages/com.battlement.client/Runtime/Host/BattlementUiEventDispatcher.cs)
wraps an owned UiEvent and serializes it before calling the transport. The
following is its successful submission path, condensed:

```csharp
var action = new UiEventAction(new ActionId(Guid.NewGuid()), session, value);
byte[] message = codec.SerializeUiEventAction(action);
BattlementUiEventTransportResult result = transport.SubmitUiEvent(message);
```

The replacement constructs the text event directly from callback values. The
existing action ID, target, cancelability, and prevention state are preserved:

```csharp
using var request = requests.Rent();
var before = request.StringValue(previous);
var after = request.StringValue(proposed);
var commit = request.ValueCommit(before, after);
var input = request.UiEvent(targetId, cancelable, defaultPrevented, commit);
request.FinishUiEventAction(actionId, session, input);
using var result = transport.SubmitUiEvent(request);
var disposition = result.Disposition;
reservation.Commit(result.RetainResponse());
```

StringValue encodes a managed string directly into the request builder; there
is no intermediate UTF-8 byte array or UiValue object. SubmitUiEvent pins the
finished range, calls Rust, and unpins before returning. Its successful result
already contains a verified response and disposition. Failure handling remains
outside this excerpt. Rent returns exclusive builder ownership until disposal.
The existing reservation commits an independent response lease and disposes it
on rejection; disposing result or request cannot invalidate the queued lease.

### Rust: read that text commit

The current [accepted-text handler](../samples/ui/rules/src/text_field_components.rs)
clones its UiValue, extracts an owned string, and constructs an owned update.
These are the relevant expressions from its match arm:

```rust
let proposed = text(value.proposed.clone())?;
Command::update_visual_element(ACCEPTED_ID, UiTextField::new().value(&proposed))
```

Afterward the arm receives ValueCommitView<'_> and borrows the text while it
constructs the response. Both excerpts are inside an Option-returning handler:

```rust
let proposed = value.proposed().as_str()?;
let field = message.text_field_value(proposed);
let command = message.update_visual_element(ACCEPTED_ID, field);
```

`as_str()` returns a checked `&str` tied to the request bytes. Reading it
allocates nothing. `text_field_value` writes the accepted text into independent
response storage; this deliberate copy is needed because the response outlives
the input pin. The handler appends command to its group alongside the other
status updates.

The normalized-text arm also avoids the initial clone, but normalization still
creates its new string:

```rust
let normalized = value.proposed().as_str()?.trim().to_uppercase();
let field = message.text_field_value(&normalized);
let command = message.update_visual_element(NORMALIZED_ID, field);
```

### Persist only deliberately owned input data

Motion and geometry handlers iterate borrowed vectors, copying only the values
needed by their persistent registries. Replace event-body Rc ownership with
synchronous borrowed callback dispatch. A handler that saves text in game state
uses an explicit owned conversion; normalization also creates a new value. The
input pin never survives submit, and responses cannot reference input bytes.

UTF-16 Unity strings must be encoded into UTF-8 when constructing requests.
Rust-to-Unity text becomes a managed string only where Unity needs it. Reuse
that decoded value within an application operation; avoid repeated string
accessor calls. Use strict UTF-8 validation, and preserve the existing behavior
for invalid UTF-16 input through explicit tests. No unbounded global string
intern cache.

## Synchronous processing and failure behavior

All protocol building, receiving verification, semantic validation, traversal,
and release run on the Unity thread that owns the native host. Remove background
codec marker interfaces, thresholds, Task.Run paths, continuations, and codec
task-result wrappers from both submission and response paths.

Keep admission sequence numbers, queue reservations, pause behavior, and batch
ordering. A response becomes eligible only after synchronous validation
succeeds. Do not budget-split verification or partially apply a structurally
invalid response. Batch scheduling may continue across frames without decoding
tasks. Synchronous UI event disposition still returns before the originating
callback.

Engine computations may still use workers with owned application data. Workers
enqueue owned results; the engine's owning-thread poll constructs FlatBuffers.
Workers cannot own message builders or hold borrowed inputs/host response views.
Nested native submissions are rejected; callbacks enqueue ordinary work for the
next legal admission point. No await occurs inside a native call or read scope.

Enforce the following limits before exposing readers or allocating from lengths:

- Maximum finished request or response: 16 MiB. Maximum table depth: 64.
- Maximum verified table visits: 1,000,000; maximum apparent traversal bytes: 64
  MiB. Charge repeated references to prevent amplification.
- Maximum queued responses: 256. Maximum live response allocations: 64 MiB,
  including active operations and full retained allocation capacity.
- Maximum Rust UI snapshot allocations: 64 MiB, including previous and newly
  built generations, exit retention, and allocation overlap during growth.
  Reserve capacity before every allocation or growth, counting old and new
  storage until the old allocation is freed. Reject replacement before
  allocating if both generations cannot fit; leave the previous snapshot intact.
- Limit each builder allocation to 32 MiB. Retain at most 16 MiB of idle builder
  storage per language runtime; do not pool allocations above 1 MiB.

Configure matching Rust/C# verifier limits on the first traversal of untrusted
data, including repeated references and string checks. A post-verification limit
does not qualify. If the pinned runtime lacks a required limit, add and audit a
narrow patch to its verifier traversal; do not write a separate format parser.
Check overflow before all offset/length arithmetic, including verifier code
paths. Structural checks cover every union payload and string; semantic checks
cover IDs, Prop states, enum ranges, numeric constraints, references, and
existing command invariants. Do not assume structural verification supplies
semantic validation.

Malformed input never reaches game logic. Invalid output stops admission and
releases its lease without applying it. Failures retain the existing structured
diagnostic and engine-poisoning behavior. Bound diagnostics even when a message
cannot be built. Rust panics must not unwind across the C ABI; panic containment
does not recover memory corruption or allocation aborts.

If an output cannot fit the live budget, release it and fail the session
visibly; never silently drop a command or evict a live buffer. Account for one
in-flight response builder separately: growth may transiently hold both its old
and new allocations, up to 64 MiB in total. The 64 MiB live-response limit is
not a total process-memory guarantee. Peak accounting must also include the
pinned request allocation (up to 32 MiB), request growth before pinning (up to
64 MiB), the snapshot budget, both idle pools, and verification/offset scratch
storage. Cap scratch storage at 16 MiB per active call and fail before exceeding
it. Admission remains stopped on allocation-budget failure.

## Performance and integration acceptance

Pool exclusively owned builders using bounded size classes. Record growth and
copied-byte counters; normal steady-state workloads must not grow builders after
warmup. Finish hands off storage, so a builder with outstanding readers cannot
be reset. Size selection uses observed message sizes within the fixed limits.

Application execution consumes readers directly. Replace casts to owned command
records and JSON-based custom dispatch. Fakes and sample engines use the same
schema and validations; diagnostic JSON may remain outside the native message
path. Remove obsolete HTTP transport implementations and their tests after
confirming there are no production callers. The normal transport is native,
including the shared Emscripten memory used by Unity WebGL.

Require these measurements with safety checks enabled:

- Compare JSON and FlatBuffers on identical full-message workloads: sparse label
  updates, snapshots, text commits, geometry, motion, and custom commands.
- Include construction, verification, native calls, reads, string conversion,
  reconciliation, scheduling, and release. Report each stage and total time.
- Record per-frame median and p95/p99, managed/native allocations, builder
  growth, payload copies, retained capacity, lease count, and release backlog.
- Require zero full-payload handoff copies and no receiver protocol-object-graph
  materialization. Report changed-state reconstruction separately.
- Require lower median and p95 protocol-processing time than JSON on the
  retained representative workload suite, with no p99 frame-time regression
  outside the measured baseline noise band. Report repeated runs and the actual
  noise band.

The existing borrowed-ABI fixture is evidence that removing JSON is promising,
not a FlatBuffers benchmark. Its 64 sparse label updates measured approximately
1,415 microseconds for JSON and 8.18 for borrowed views under Unity's bundled
Mono, excluding response construction and Unity execution. Do not use those
figures as a FlatBuffers acceptance threshold or promised frame-time gain.

## Automated validation

The first integration check is an actual UI sample player: click a navigation
button, commit text, and apply the resulting Rust response through the Unity
scheduler. Exercise delayed consumption and cancellation in that same player. A
standalone decoder benchmark does not satisfy this boundary check.

- Generate deterministic Rust/C# cross-language fixtures for every union and
  property state, including empty vectors, non-ASCII text, UUIDs, custom
  payloads, and default-valued Set operations. Assert observable host outcomes
  as well as field equality. Regeneration must produce no unexplained
  differences.
- Fuzz truncated buffers, offsets, lengths, tags, deep/repeated tables, invalid
  UTF-8, integer overflow, and invalid semantic references on both readers.
  Inputs must be rejected without out-of-range native access or partial effects.
- Test retained child views after parent disposal, independent leases, duplicate
  release, wrong-thread access, reentrant callbacks, canceled operations, engine
  recreation, forced GC, late finalizers, and plugin unload. Exercise pool reuse
  after every release scenario to expose stale readers.
- Instrument native allocations with supported address/undefined-behavior
  sanitizers; use Rust memory-model checks on the isolated unsafe owner where
  supported. These supplement, rather than replace, the written safety contract.
- Validate Unity Mono and IL2CPP, desktop pointer/alignment targets, iOS/Android
  native builds, and WebGL. WebGL tests must include memory growth and retained
  readers; reacquire current memory views instead of caching a JavaScript heap
  view across growth. Keep generation AOT-compatible without runtime reflection.
- Assert no background protocol tasks, handoff memcpy, unchecked roots, mutable
  published readers, or generated object Pack/UnPack calls in production paths.
  Keep counters and negative API tests for the invariants static scans cannot
  prove.

## Manual QA

Use the native UI sample for the first assembled check. Capture retained native
Ditto evidence and a Unity profiler recording with protocol stage markers and
live-buffer counters visible. Use a validation-only control to delay a batch,
cancel it, force collection, and display outstanding leases and allocated bytes.

1. Navigate between UI pages and click buttons. Labels, styles, focus, and event
   disposition must match the current behavior without stale or missing updates.
2. Commit and normalize text containing Japanese characters and emoji; test
   empty text and selection lists. Confirm accepted/rejected values and
   defaults.
3. Delay an update across frames, replace the session, and destroy its target.
   Confirm canceled work never applies and live buffers return to the expected
   baseline after cleanup. Repeat while forcing GC and reusing pooled buffers.
4. Exercise motion, geometry, parallel groups, and a custom asynchronous
   command. Confirm ordering and completion; inspect that no protocol worker
   tasks run.
5. Trigger a malformed-message fixture and a live-budget overflow. Confirm clear
   failure, no partial application of the rejected response, and safe teardown.
6. Reload the plugin or recreate the host with retained-view test handles. Old
   access must throw predictably; no finalizer may enter the unloaded plugin.
7. Repeat the representative interaction in an IL2CPP player and the declared
   mobile/WebGL targets. Compare profiles using the acceptance metrics above;
   record platform evidence rather than extrapolating from editor behavior.
