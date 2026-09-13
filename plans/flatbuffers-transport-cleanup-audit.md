# FlatBuffers transport cleanup audit

Audit basis: release commit `99643247be3b6588879771bfdb2374925439a6ad`
(`feat: replace native JSON transport with FlatBuffers`) and its parent. The
desired end state is the codebase that would exist if the native boundary had
always used FlatBuffers: one engine contract, one request/response path, tests
that exercise that path, and no JSON-shaped protocol metadata or compatibility
adapters.

## Verdict

The native boundary has been cut over successfully, but the codebase has not.
`contracts/wire-contract.json` declares size-prefixed FlatBuffers for connect,
submit, UI-event submission, and poll. `BattlementNativeTransport` is the only
production `IBattlementTransport`, received buffers are checked and verified,
and every sample export uses `export_deterministic_native_engine!`. There is no
production JSON fallback on that path.

The migration is nevertheless substantially additive. It introduced a second,
FlatBuffers-native implementation beside the owned JSON-era architecture, then
kept the old architecture alive for fakes, tests, Reactant, and compatibility
overloads. The migration commit changed 407 files with 170,685 insertions and
4,001 deletions. Generated schema output explains much of the insertion count,
but the hand-written duplication below explains why the repository still feels
like it supports two transports.

This is not the target described by `plans/flatbuffers-native-transport.md`.
That design requires direct writers, borrowed verified readers, no production
`Pack` layer, no receiver protocol-object-graph reconstruction, and fakes using
the same schema as production.

## Why chess audio grew

`samples/chess/rules/src/audio.rs` is a representative unfinished cutover. It
was 228 lines before the migration and is now 338 lines; the commit added 120
lines and removed 10.

The old path remains:

- `poll` returns an owned `Response<Command>`.
- `start_initial_track`, `set_volume`, `play_sound`, and
  `response_for_action` construct `CommandBody` values and owned response
  groups.
- `ChessEngine` still calls those methods from its `Engine` implementation.

A parallel native path was added:

- `poll_native` writes audio commands with `MessageWriter` and returns a
  `NativeResponse`.
- `write_initial_track`, `set_volume_target`, and `random_sound_address`
  provide writer-friendly variants.
- `ChessEngine` and `native.rs` call those methods from the `NativeEngine`
  implementation.

Only small state transitions were factored into shared helpers. The command
construction and response assembly remain duplicated. Once chess has only the
native engine implementation, the owned audio methods and their callers should
be deleted; `audio.rs` should contain playlist state plus a single direct writer
path. The growth is migration scaffolding, not an inherent FlatBuffers cost.

The same pattern exists in `samples/basic`, `samples/tictactoe`, and
`samples/ui`: each engine implements both `Engine` and `NativeEngine`.

## Remaining compatibility islands

### 1. C# still ships the JSON transport codec

`Packages/com.battlement.client/Runtime/Json/` contains 11 source files and
2,601 lines, including `BattlementJson.cs`, union reflection/conversion,
motion converters, style converters, and `Battlement.Json.asmdef`. No
production runtime outside that directory calls `BattlementJson`; its callers
are tests.

The test assembly keeps the island load-bearing:

- `JsonInteropTests.cs` is 795 lines.
- `JsonFixtureData.cs` is 452 lines.
- `BattlementDiagnosticsJsonTests.cs` is 107 lines.
- Geometry, layout, motion, UI-event, accessibility, and composition tests use
  JSON round trips as a convenient clone or assertion mechanism.
- Both editor-test assembly definitions reference `Battlement.Json`, and
  `PackageAssemblyTests` asserts that the assembly exists.
- `scripts/tests/unity-test-selection.test.py` still treats
  `BattlementJson.cs` as a test-selection input.

Delete the runtime directory, its assembly definition and metadata, the JSON
fixture suite, and the assembly/test-selection expectations. Preserve valuable
semantic cases by expressing them through FlatBuffer writers/readers or direct
domain assertions. Do not retain a production codec solely as a test helper.

The protocol assembly is also decorated for the deleted codec. Nineteen files
under `Runtime/Protocol` contain 238 `JsonProperty`, `JsonConstructor`, or
`JsonIgnore` references. `Battlement.Protocol.asmdef` references
`Newtonsoft.Json`, and `CanonicalConstructorContractResolver.cs` exists only
for JSON construction. Remove those attributes, the resolver, and the protocol
assembly's Newtonsoft reference after the JSON tests are converted.

### 2. Rust exposes two engine architectures

`crates/battlement-native/src/engine.rs` defines the owned `Engine` and
`EngineFactory` contract. `native_engine.rs` defines the exported
`NativeEngine` and `NativeEngineFactory` contract. `adapter.rs` and `handles.rs`
contain both implementations, and the old trait still documents overrides as
temporary work “during the FlatBuffers migration.”

The compatibility traits `FlatBufferResponseCommand` and
`CustomFlatBufferResponseSchema` serialize an already-built owned
`Response<Command>` through `battlement_flatbuffers::write_core_response`.
That is precisely the post-construction packing layer the design rejected.

Retain one contract. Once consumers have moved, delete the owned `Engine`,
`EngineFactory`, `IntoEngine`, `with_connect_view`, the old adapter and handle
branches, and the response-packing traits. Rename `NativeEngine` to `Engine`
only after the old name is gone; there is no compatibility requirement that
justifies permanent “native” and “legacy” concepts.

### 3. Fakes and tests bypass the production transport

`crates/battlement-fake/src/client.rs` is generic over the old `Engine`. It
passes owned `Connect`, `ClientMessage`, `UiEventAction`, and `Response` values
in process. Only a narrow batch-failure case currently exercises a FlatBuffer
submission. Many fake and Reactant tests therefore prove owned-model behavior,
not the bytes, verification, union dispatch, or lifetime rules used by Unity.

Make `FakeClient` consume the sole native engine contract. It should build all
connect, action, batch-failure, operation-failure, and UI-event inputs with the
real writers, then read and apply verified response bytes. Convert its scripted
test engines to emit `NativeResponse`. This test migration is the prerequisite
for deleting the old engine API safely.

### 4. Rust response production still builds owned graphs

`crates/battlement-flatbuffers/src/response.rs` is a 3,502-line converter from
owned protocol values into FlatBuffers. The basic, tic-tac-toe, and chess
native paths use `MessageWriter` directly for substantial parts of their
responses, but their old engine implementations still require the converter.

Reactant has not completed this part of the migration.
`crates/battlement-reactant/src/app_engine.rs` invokes the old `Engine`
methods, builds `Response<Command>`, and then calls
`FlatBufferResponseCommand::write_response`. The UI sample does the same in its
native response helper. Consequently every Reactant application, including
chess UI, crosses the ABI as FlatBuffers but first constructs the JSON-era
owned response graph.

Move Reactant rendering, reconciliation output, and UI sample responses onto
writer offsets. Retained UI snapshots may remain FlatBuffer-owned; they are not
legacy by themselves. Delete `write_core_response(&Response<Command>)` and its
production graph converter after all native callers write directly. Keep only
reader/writer code that represents actual wire semantics.

### 5. UI events can reconstruct the full owned event graph

`crates/battlement-flatbuffers/src/ui_event.rs` exposes `to_owned` and
`to_owned_event` as migration compatibility. The 915-line
`ui_event_owned.rs` reconstructs the old event model. Reactant falls back to
that conversion whenever a matched handler does not support the native view;
several public callback forms still register without a native callback.

Give every handler form a narrow borrowed-view adapter. Copy only a callback
value that must outlive the input buffer; do not materialize the complete event
union. Then remove `ui_event_owned.rs`, `to_owned`, `to_owned_event`, and tests
whose only purpose is legacy reconstruction.

### 6. Unity contains an owned response bridge beside its readers

`BattlementResponseViews.cs` contains both FlatBuffer-backed views and
`BattlementOwnedResponseView`, `BattlementOwnedSnapshotView`,
`BattlementOwnedBatchView`, and `IBattlementOwnedSnapshotView` near the end of
the 5,479-line file. Runtime branches and overloads exist mainly because tests
can inject an owned response without encoding it.

Related compatibility surfaces include:

- `BattlementTransportResult.OwnResponseView` and the UI-event equivalent.
- A `BattlementResponseStream` overload that accepts a decoder returning
  `Response<ICommand>`.
- The roughly 419-line `LaunchCore(CommandBody, ...)` switch in
  `BattlementCommandExecution`; direct FlatBuffer command execution exists
  beside it.
- Owned snapshot overloads in validation, input coordination, and snapshot
  replacement.
- Old and direct command overloads throughout the host. There are 313
  `CommandBody` references across 17 host files.
- `ValidateOwnedCommand` in
  `BattlementFlatBufferResponse.Validation.cs`, which has no caller and can be
  removed immediately.

Change test transports to return real FlatBuffer payloads. Delete the
owned-view shortcuts, then delete the owned command executor and overload
forest. Production code should never accept a pre-decoded transport response.

### 7. Unity still materializes large nested protocol objects

The six `BattlementFlatBufferMaterializer*.cs` files total 2,819 lines. They
materialize UI documents and elements, styles, paint, motion values,
transitions, selectors, and targets. These calls are not all dead: direct
command readers, snapshot replacement, and snapshot validation still use them.

This means the ABI handoff can be zero-copy while the receiver still rebuilds
large parts of the old object graph. Replace materialized nested values with
borrowed, lease-checked view interfaces in validators and immediate host
operations. Copy only values retained past command execution, and make those
copies explicit at the lifetime boundary. Remove each materializer family only
after its production callers disappear; deleting the files wholesale now
would be incorrect.

After the compatibility types are gone, split the large response-view file by
wire concern or generate its repetitive union dispatch. A single command
execution record with many nullable fields is a migration-shaped abstraction,
not the clean steady-state API.

### 8. Rust protocol crates retain JSON-only API and metadata

`crates/battlement/src/json.rs` is a generic `serde_json` wrapper exported as
`battlement::json`; its observed callers are tests. The direct `serde_json`
dependency in `battlement` exists for that module. `battlement-ui` also lists
`serde_json` as a normal dependency while observed uses are in tests.

Across `battlement`, `battlement-ui`, and `battlement-types`, 68 source files
mention serde and 458 `#[serde(...)]` attributes remain. Some serde use may be
an intentional non-transport consumer, so it should be audited by consumer
rather than mechanically removed. Delete the public JSON module and move or
remove JSON-only dependencies and encoding tests. Replace
`FiniteValueValidator`'s serializer-based validation with explicit typed
validation; validation should not depend on pretending to serialize a value.
Then remove transport-only derives and field annotations that have no remaining
non-transport caller.

### 9. Generated custom fixtures are duplicated

The generated `fixture_response_generated.cs` under
`Assets/BattlementIntegration/FlatBuffers` is byte-for-byte identical to the
copy under the package editor tests. `scripts/generate_flatbuffers.py` owns and
checks only the package copy. Consolidate the source behind a shared test
assembly, or make generation explicitly own and verify both required assembly
outputs. Do not leave a manually synchronized generated copy.

## JSON that should remain

“FlatBuffers exclusively” applies to the Rust/Unity gameplay transport, not to
every use of JSON in the repository. Keep JSON where it is the actual external
or persisted format:

- Ditto job, lifecycle, result, capture, and NDJSON contracts.
- Native tracing/log JSON Lines and Unity-side diagnostic parsing.
- Cloud diagnostics and command-line/tooling output.
- Unity manifests, settings, Addressables metadata, and test data whose
  contract is JSON rather than gameplay transport.
- Chess save persistence and Reactant animation-validation reports.

`Battlement.Runtime.asmdef` may therefore continue to need Newtonsoft while
Ditto and diagnostic code share that assembly. Splitting those facilities into
a dedicated assembly could remove Newtonsoft from the core runtime assembly,
but it is a packaging cleanup, not a prerequisite for the transport cleanup.

## Recommended execution order

1. **Make tests cross the real boundary.** Convert `FakeClient`, scripted Rust
   engines, and C# test transports to build and verify FlatBuffer bytes. Add
   coverage for malformed buffers and custom unions before deleting helpers.
2. **Delete the standalone JSON codec.** Remove `Runtime/Json`, JSON fixtures,
   JSON-only tests, protocol attributes/resolver, assembly references, and
   selection-script expectations.
3. **Finish direct Rust production.** Convert Reactant and the UI sample to
   direct writers; remove owned output helpers from chess, basic, and
   tic-tac-toe. Delete the response graph packer when its last production
   caller is gone.
4. **Collapse the Rust engine boundary.** Remove the old engine/factory,
   adapter, handle, and compatibility traits. Update examples and docs to name
   the one remaining engine contract.
5. **Delete owned Unity execution.** Remove owned response injection, owned
   response/snapshot views, `LaunchCore`, and the old command overloads.
6. **Finish borrowed input handling.** Convert Reactant event handlers and Unity
   nested command consumers to views with explicit retention copies; remove
   full event and response materializers.
7. **Prune residual serialization metadata.** Remove Rust JSON APIs,
   JSON-only serde implementations and tests, unused dependencies, duplicated
   generated fixtures, migration comments, and stale documentation.

Steps 2 and 5 can proceed in smaller deletion-only slices once their tests no
longer depend on the adapters. Steps 3 and 6 are the substantive design work;
they should not be disguised as mechanical cleanup.

## Completion criteria

The cleanup is complete when all of the following are true:

- There is no `Battlement.Json` assembly, `BattlementJson`, or gameplay JSON
  fixture suite.
- `Battlement.Protocol.asmdef` has no Newtonsoft dependency and protocol types
  have no JSON transport annotations.
- Rust exports one engine lifecycle trait and samples implement it once.
- `FakeClient` submits FlatBuffer bytes and applies verified FlatBuffer
  responses for every operation.
- No production caller packs an owned `Response<Command>` into FlatBuffers.
- `samples/chess/rules/src/audio.rs` has one command-construction path.
- There is no `BattlementOwnedResponseView`, `OwnResponseView`, owned response
  decoder, or `LaunchCore(CommandBody, ...)` path in Unity.
- Reactant does not materialize a complete UI event for callback dispatch.
- Unity validators and immediate command handlers consume verified views;
  retained copies are narrow and explicitly justified by lifetime.
- There are no comments or public APIs described as temporary migration or
  legacy transport support.
- The native ABI, FlatBuffer identifiers/verifiers, custom schemas, behavior
  tests, Unity tests, and required platform validation continue to pass after
  the deletion work.

The expected outcome is a materially smaller hand-written codebase. Generated
FlatBuffer sources will remain large; the duplicated owned/JSON adapters around
them should not.
