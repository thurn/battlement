# Source architecture refactoring and test audit

Battlement runs Rust game rules against a Unity rendering and input host.
**Reactant** is the Rust declarative UI layer above the UI protocol; the
**fake client** executes protocol commands without Unity. This plan separates
responsibilities at those boundaries and removes tests that obstruct internal
changes without providing proportionate regression protection.

The unit of delivery is a working task, not a file-size reduction. Each task
below includes its caller migration, relevant tests, and cleanup. No task ends
with a disconnected replacement, two competing owners, or a broken build.
This document requests future implementation; it does not implement it.

## Evidence and related information

The audit inspected release `b9bed0c96ac6591f4551e0e3cb5a0d8e92fc8ad9`.
Findings are from source inspection, not executed mutation experiments. The
scope covers the production files linked below, the four suites named in
Task 2, their relevant collaborators, and selected host, serialization, and
Python tests. It is not a claim that every
source file or test in the repository has been audited.

- [Runner](../Packages/com.battlement.client/Runtime/Host/BattlementRunner.cs):
  session composition, frame execution, failure handling, and UI dispatch.
- [UI documents](../Packages/com.battlement.client/Runtime/UI/BattlementUiDocuments.cs):
  identity indexes, hierarchy mutation, control updates, and synthetic input.
- [JSON converters](../Packages/com.battlement.client/Runtime/Json/JsonConverters.cs):
  tagged unions, scalar validation, payload construction, and extension
  commands.
- [Rust styles](../crates/battlement-ui/src/elements/style.rs),
  [validation](../crates/battlement-ui/src/validation.rs), and
  [panel settings](../crates/battlement-ui/src/documents.rs): typed authoring,
  wire values, and cross-field invariants.
- [Performance sources](../scripts/perf_sources.py): local database and log
  discovery, parsing, candidate correlation, and content sanitization.
- [Native validation guidance](../.agents/skills/battlement-ditto/SKILL.md) and
  [CI guidance](../.agents/skills/battlement-ci/SKILL.md): executable checks and
  retained evidence. **Ditto** runs deterministic native Unity scenarios.
- [Web contracts](../web/contracts.toml): checks for browser-specific changes
  and declared platform risks; shared UI changes use native evidence.

## Architectural findings

These are responsibility problems, with different remedies. A long declarative
catalog deserves less disruption than a smaller object with conflicting state
owners.

### C# host and documents: prioritize ownership

`BattlementRunner` already delegates to `BattlementSessionState`,
`BattlementResponseStream`, `BattlementBatchAdmission`, snapshot replacement,
and individual runtime services. Reuse these; adding another general-purpose
host framework would obscure the existing design.

- `Configure` composes many nullable services, while lifecycle methods and
  disposal must keep their availability synchronized. Separate configured
  resource ownership from session phase; do not introduce a second phase enum
  that competes with `BattlementSessionState`.
- `EmitUiEvent` combines reservation ordering, synchronous transport, input
  disposition, inspection records, activation acknowledgements, and deferred
  failure. This is an identifiable transaction boundary, not arbitrary helpers.
- `BattlementUiDocuments` maintains seven parallel identity/hierarchy
  collections. Those collections describe one owned graph and must mutate
  together. Logical child order differs from Unity's physical hierarchy for
  controls and overlays; one cannot be reconstructed blindly from the other.
- Document creation, sparse property validation/application, topology changes,
  and synthetic pointer state have independent reasons to change. Existing
  focus, overlay, sticky, motion, and control coordinators remain their owners.
  A new coordinator must not absorb their policies.

### Rust: preserve closed variation and isolate validation contexts

`Style` is predominantly typed data, documented builders, and explicit native
properties. Its size does not justify replacing it with string-keyed maps,
trait objects, or a generator that hides the public API.

- Move coherent value families out of `style.rs`; retain one authoritative
  sparse property schema and the distinction between unset, set, and reset.
- `validate_node` passes `parent_kind`, `unplaced_root`, and scroll ancestry
  separately. An enum should distinguish an unattached subtree root from an
  attached node with known placement; a named context carries traversal facts.
- `validate_element(..., require_complete: bool)` conflates complete state and
  a sparse patch at the call site. Name the modes and retain the distinction:
  omission in a patch is not an absent required property in complete state.
- `PanelSettings` exposes a mode plus fields for every scale mode. Its validator
  must reject combinations such as physical scaling with a pixel multiplier.
  A payload-bearing scale enum can make those combinations unconstructible in
  authored Rust values, while raw incoming data is still validated.

### Python: separate inputs from interpretation

`perf_sources.py` contains several independently changing input formats.
Database discovery, subprocess execution, log decoding, span construction,
child-session folding, and sanitization should not share one module.

Use ordinary modules, functions, context managers, and small frozen dataclasses.
Keep `SessionTrace` and `Span` in `perf_model.py`. Do not introduce an adapter
inheritance hierarchy, plugin registry, or new reporting model for this work.
External dictionaries may remain at parsing boundaries; validated internal
records should stop repeating the same shape checks downstream.

## Useless-tests audit

A test's value is the plausible regression it detects, not whether it contains
constants, uses a fake, or asserts an exact value. The following dispositions
apply to specific assertions, not wholesale deletion of these large suites.
No finding below is a measured surviving mutation yet.

### Replace or remove weak checks

1. **Source tokens presented as CI execution proof.**
   [ditto-cutover.test.py](../scripts/tests/ditto-cutover.test.py),
   `check_ci_opt_in`, checks that `ci.py` contains a flag literal and a function
   call spelling. A comment or unreachable call satisfies those assertions;
   actually skipping native validation need not fail them. Replace these two
   assertions with a black-box selection/execution check in the existing CI
   test harness. Keep independently justified configuration-policy checks.
   Do not replace them with a more elaborate source parser.

2. **A dashboard label presented as coverage proof.**
   [ui_commands.rs](../samples/ui/rules/tests/ui_commands.rs),
   `release_coverage_maps_every_capability_to_live_and_automated_proof`, checks
   fixed totals, parses Rust declarations with `include_str!`, and looks for
   `LIVE`/`TEST` text. This can detect a stale catalog, but cannot prove an
   associated test exists or exercises the capability. Keep one navigation and
   catalog-consistency check if that dashboard remains a product requirement.
   Remove duplicated textual totals and claims of automated proof; any retained
   coverage assertion must resolve an actual scenario/test identifier. Do not
   build a repository-wide coverage system just to preserve this test.

3. **Incidental sample layout locked to exact structure and prose.**
   [composition.rs](../samples/reactant/rules/tests/composition.rs),
   `sample_opens_on_an_accessible_composition_screen`, fixes wrapper child
   order/count, exact word budget, and a 24-point title. In
   `resources_screen_uses_phone_safe_navigation_and_cards`, fixed 14-point text
   and full-width styling do not establish that a phone can use the screen.
   Remove incidental structure, exact word-count equality, and cosmetic values
   unless an independently documented requirement explains them. Preserve
   navigation, readable text, and interaction checks. Use native geometry and
   rendered evidence for clipping and usable targets. A word-budget upper bound
   is defensible only if the budget itself is an intended requirement.

4. **Serializer smoke coverage with a weak oracle.**
   [JsonInteropTests.cs](../Packages/com.battlement.client/Tests/Editor/JsonInteropTests.cs),
   `BuiltInCorpusRoundTripsAsStructuralJson`, counts decoded command types and
   checks that its generated JSON parses. It does not compare the payloads
   despite its name. Keep it as broad smoke coverage, but replace the parse-only
   assertion with meaningful payload checks or independent Rust fixture
   comparisons. A swapped/lost payload field must fail even if command count
   and concrete types are unchanged.

### Retain, with accurate claims

These tests have plausible bugs to catch. Do not delete them to make a
refactoring pass:

- [ui_api_tests.rs](../crates/battlement-ui/tests/ui_api_tests.rs): exact tags,
  default omission, sparse reset, shorthand expansion, and style merge protect
  the Rust–Unity contract. A serializer emitting explicit null for an omitted
  property or a merge dropping a reset is a real bug. Prefer structural JSON
  over irrelevant property ordering; retain independent expected values.
- [BattlementUiDocumentTests.cs](../Packages/com.battlement.client/Tests/Editor/BattlementUiDocumentTests.cs):
  rejected-update atomicity, authored-document isolation, conditional title
  children, synthetic event routing, and cross-domain identity checks protect
  real host behavior. Split these by responsibility without weakening them.
- `StyleProtocolPropertiesTargetWritableIStyleMembers` is a useful SDK
  compatibility check, not proof that style application works. Pair it with the
  existing public inline-state tests; writable property names alone miss an
  omitted or incorrectly mapped writer.
- `layout_performance_builds_the_exact_mixed_workload` and
  `motion_performance_builds_the_exact_transform_workload` in `composition.rs`
  protect benchmark workload integrity. Exact counts prevent apparent speedups
  caused by doing less work. They do not measure rendering performance.
- [perf-report.test.py](../scripts/tests/perf-report.test.py): sanitization,
  interval arithmetic, child folding, retries, private file permissions, and
  failed-source handling have concrete failure modes. Private helper calls can
  be moved to input/output boundaries during extraction; privacy does not make
  their assertions worthless.
- Repository policy scans and exact Tollgate configuration checks are policy
  enforcement. Keep them separately identified; do not count them as runtime
  correctness coverage or remove the underlying policy during cleanup.

### Evidence required before deleting coverage

For each proposed deletion, name the protected requirement, a plausible bug,
and the remaining test or a reason the requirement does not exist. If replacing
coverage, introduce and verify the replacement before removing the old check.

Use a few deliberate, temporary faults to check ambiguous oracles: omit a style
write, lose a reset, swap a union payload, skip native CI selection, or fail to
release an identity. Run the relevant test, restore the production code, then
run it cleanly. Record the exact test and outcome in review evidence. These are
future implementation checks, not experiments already performed by this audit.
Do not inject faults into a live user session or leave mutation code committed.

## Task breakdown

Execute the numbered order by default. Explicit prerequisites below identify
which tasks are independently mergeable. Each task must pass its focused checks
and the repository-selected CI before it is complete. Stage intended changes
before starting the durable CI job; follow the linked CI guidance rather than
assuming every Markdown file qualifies for the narrow prose-only lane.

For native behavior, retain deterministic Ditto evidence for the affected flow.
Do not update screenshot baselines merely to approve an intended no-behavior
refactor. Every task removes its obsolete implementation after switching callers
and preserves standalone Rust sample builds and Unity compilation. API changes
migrate all in-repository consumers in that task; do not add compatibility
versions or indefinite forwarding layers.

### Task 1 — Replace false confidence before moving code

**Prerequisites:** none. Keep this task focused on the audit findings.

- Replace the CI source-token assertions with observable native-check selection.
  Use existing tests around `scripts/ci.py`, selection, and Ditto invocation.
- Strengthen the JSON corpus payload oracle with independently expected data.
  Remove incidental composition assertions and redundant coverage-label totals
  according to the dispositions above; retain actual sample interactions.
- Verify representative replacements with the temporary faults described above.
  The runtime implementation is restored before the task ends.

**Working boundary:** all retained tests pass against unchanged production
behavior. Renaming or moving a production module no longer breaks the removed
source-format checks. A skipped selected check and a corrupted JSON payload
fail the replacement tests.

### Task 2 — Organize behavioral suites without changing their assertions

**Prerequisites:** Task 1.

Split these four suites: `crates/battlement-ui/tests/ui_api_tests.rs`,
`samples/ui/rules/tests/ui_commands.rs`,
`samples/reactant/rules/tests/composition.rs`, and
`Packages/com.battlement.client/Tests/Editor/BattlementUiDocumentTests.cs`.

Group them by contract: protocol encoding versus
validation; sample event/control/hierarchy flows; Reactant state/effect/resource
versus layout/motion flows; Unity document lifetime versus hierarchy, input,
and style application.

- Share only setup, identity generation, and navigation helpers. Keep assertions
  near the behavior they explain. Do not create a base fixture with mutable
  state shared across otherwise independent tests.
- Ensure moved Rust integration tests remain discoverable by Cargo. Ensure
  Unity test classes remain selected and include required Unity metadata.
- Compare discovered test identities before and after; every retained case must
  still execute. Update selection rules only where an actual file move requires
  it, within this same task.

**Working boundary:** all four original suites' retained contracts execute at
new homes. Existing sample clicks, controlled values, and public host outcomes
remain asserted. File movement alone is not a reason to add new tests.

### Task 3 — Separate JSON wire mechanics from the closed case catalog

**Prerequisites:** Task 1; coordinate paths with Task 2 if it is complete.

Move the existing scalar/color/byte and sparse-value converters into coherent
files. Inside `BattlementUnionConverter`, separate the fixed tag/type catalog
from tagged-payload reading/writing and reflection-based record construction.

- Keep one authoritative built-in case catalog; group entries by protocol
  family without separate divergent read and write mappings.
- Keep `CustomCommandJsonConverter` at the explicit extension registration
  boundary. Closed built-in variants do not need a plugin interface.
- Preserve wrapper, scalar, unit, direct payload, and flattened property-command
  shapes. Preserve the scoped `disabledTypes` recursion guard and its `finally`
  cleanup, including a failed conversion followed by another conversion.
- Avoid a source generator or serializer replacement in this task. Move actual
  responsibilities, not pieces of one class into partial declarations.

**Working boundary:** the public codec registration still works, built-in Rust
fixtures and custom commands decode correctly, and malformed tags/payloads
fail. Run JSON interop, layout/motion/geometry protocol checks, and applicable
Rust fixture checks. A payload field corruption must be observable.

### Task 4 — Give the UI hierarchy one owner

**Prerequisites:** Task 2.

Extract an internal hierarchy owner from `BattlementUiDocuments`. It owns
node identity, native element lookup, reverse lookup, logical parent/children,
document affiliation, and roots. Callers receive queries, not mutable maps.

- Represent node-associated data together where it removes synchronized-map
  obligations; derived reverse indexes remain private implementation details.
- Provide operations for adding, moving, reordering, and removing a subtree.
  Preserve root protection, identity reservation/release, depth checks, and
  logical ordering independently of Unity's physical content containers.
- Migrate all document, event route, focus, overlay, and geometry lookups in the
  same task. Keep topology-dependent validation next to the graph it needs.
- Do not promise global rollback for arbitrary Unity exceptions. Preserve
  current preflight rejection guarantees and the host's failure/cleanup path
  after a native mutation has begun.

**Working boundary:** duplicate IDs, cycles, invalid placement, and excess depth
are rejected without visible partial changes where currently guaranteed.
Conditional title controls retain their logical children; reparent/destroy
updates event routes and releases identities. Run document hierarchy and
snapshot validation tests plus the affected native hierarchy flow.

### Task 5 — Separate UI property admission from application

**Prerequisites:** Task 4.

Extract the properties branch of `BattlementUiDocuments.Update` into one
operation that owns validation, resource preparation, and ordered application.
Keep `CreateElement`/`Populate` construction responsibilities separate from
sparse update semantics, using existing control/property handlers.

- Validate type, complete controlled state, layout placement, and parts before
  writes. A prepared update owns acquired parts/motion resources until commit
  or disposal; an abandoned prepared update releases them exactly once.
- Preserve ordering around style preparation, paint, motion, focus, layout,
  parts, and controls. Refresh coordinators at the same observable boundary.
- The document facade coordinates operations through narrow collaborators;
  helpers must not accept the whole manager to reach into its internals.
- Keep unset/set/reset distinct and constructor-state restoration intact.

**Working boundary:** invalid style, part, or control updates leave prior
observable state intact under existing admission guarantees. Valid combined
updates apply once and failed preparation leaks no asset lease. Run public
style/control atomicity cases and native update/reset plus overlay/focus flows.

### Task 6 — Move synthetic UI input into a state-owning adapter

**Prerequisites:** Tasks 4–5.

Extract synthetic pointer target/position, hover transitions, click completion,
and semantic activation from document management. The adapter queries the
hierarchy and existing focus/event/control services; it does not own nodes.

- Clear pointer/activation state on target destruction, document replacement,
  disabled input, and disposal using the existing lifecycle boundaries.
- Preserve one event route for semantic activation and repeat behavior. Keep
  native event forwarding in its existing services.
- Keep Ditto-specific entrypoints explicit; do not turn normal rendering into
  a generic automation framework.

**Working boundary:** hover leaves the previous target, enter-only handlers
work, destroyed targets do not receive later clicks, and a semantic activation
fires once. Run synthetic-input editor cases and a native Ditto activation
through Rust state change to the presented frame.

### Task 7 — Extract the runner's synchronous UI event transaction

**Prerequisites:** Task 1; Task 6 first in the default sequence.

Create a concrete dispatcher owning UI dispatch depth, response reservations,
inspection records, and pending native-prevention acknowledgement. Pass the
existing transport, codec, response stream, clock, and narrowly scoped failure
and activation collaborators at construction.

- Preserve reservation-before-transport ordering and deferred application of
  reentrant responses. Release a reservation on every rejected/failed path.
- Keep `Continue`/`PreventDefault` synchronous; do not defer native prevention
  to a later frame. Retain disposition and nonempty-response validation.
- Preserve failure classification before versus after dispatch, bounded
  inspection retention, and deferred session failure timing. The session owner
  owns pending session failure; the dispatcher reports it through one explicit
  callback and retains no duplicate pending-failure state.
- Move activation bookkeeping with its lifecycle owner or pass a small explicit
  operation; do not duplicate transaction state between runner and dispatcher.

**Working boundary:** nested submission still applies after the outer response,
failed dispatch does not block the response stream, and cancellation prevents
native default behavior. Run response processing, UI-event/activation tests,
and native cancelable input. Public runner behavior remains usable throughout.

### Task 8 — Consolidate configured runtime ownership and shutdown

**Prerequisites:** Task 7.

Reduce `BattlementRunner` to Unity lifecycle entrypoints, serialized options,
and composition. A configured runtime object owns the services currently
created by `Configure` and their teardown; `BattlementSessionState` remains the
single owner of stopped/awaiting/applying/running session transitions.

- Replace independent nullable service availability with one optional owned
  configured runtime. Do not require a giant interface for that internal owner.
- Keep application focus/pause facts separate from session phase. Make terminal
  runtime failure explicit rather than allowing contradictory recovery flags;
  preserve the existing native-panic versus restart-required behavior.
- Move failure record construction/diagnostic formatting to the existing error
  boundary or a cohesive collaborator; the session owner decides transitions.
- Preserve main-thread guards, frame order, snapshot completion/input gating,
  reverse dependency cleanup, reconnect behavior, and Unity serialized fields.

**Working boundary:** two runners remain independent; stop/reconnect/dispose
release resources and subscriptions once; malformed initial snapshots never
open input; poisoned runtime recovery is unchanged. Run runner host, snapshot,
response, failure/recovery checks and native reconnect/shutdown evidence. Do
not refactor command execution or all individual runtime services in this task.

### Task 9 — Split Rust style value families, retain the sparse schema

**Prerequisites:** Tasks 1–2.

Move length/numeric values, text values, background/cursor values, and
transform/transition values into cohesive modules. Keep intentional public
exports at the crate authoring boundary; those exports define the API rather
than acting as temporary compatibility shims.

- Keep `Style` and its sparse merge semantics authoritative in one place.
  Group builders by family only where it improves navigation without hiding
  field ownership. A still-large cohesive schema is acceptable.
- Retain `StyleValue`, `Prop`, `LengthOrAuto`, and closed enums. Do not replace
  them with unconstrained strings or dynamic style-property trait objects.
- Retain genuinely polymorphic ergonomic conversions such as shorthand tuple
  inputs; no new trait is needed merely to split a file.

**Working boundary:** Rust consumers compile, serialized style shapes are
unchanged, reset survives merge, and shorthand expansion is unchanged. Run
`battlement-ui` tests and UI/Reactant sample tests. Any remaining source-based
catalog consumer must migrate within this task, not break until a later task.

### Task 10 — Make validation contexts explicit and split invariant families

**Prerequisites:** Task 9.

Separate document/hierarchy validation, panel validation, common/style value
validation, and element/control validation. Keep public validation entrypoints
small and retain one authority for each Rust invariant.

- Replace `require_complete` with a closed enum naming complete state and sparse
  update. Pass a traversal context that names depth and scroll ancestry.
- Replace the independent parent/unplaced combination with attached placement
  carrying parent information versus detached root placement. Keep document
  root rules explicit, rather than treating every missing parent identically.
- Use exhaustive matching over closed element variants. Traits are unnecessary
  for the finite set of protocol controls.
- Preserve traversal/error precedence, identity limits, and existing categories.
  Value validation stays read-only; fake execution still merges a patch and
  validates the resulting complete state before replacing stored state.

**Working boundary:** the same valid documents pass; detached placement defers
only checks requiring a live parent; attached tabs/sticky/grid items retain
context rules. Test sparse versus merged state, duplicate IDs, invalid ranges,
and no fake-state mutation on rejection. C# host validation remains necessary
at the wire/native boundary and is not deleted as duplication.

### Task 11 — Encode mutually exclusive panel scaling in Rust types

**Prerequisites:** Task 10.

Replace authored scale-mode-plus-unrelated-fields with a scale enum whose
variants own only their relevant fields: pixel multiplier, physical density,
or reference resolution and screen matching. Screen matching itself carries a
factor only for the mode that uses one. Keep checked numeric fields private.

- Convert through a private wire representation so the existing flattened JSON
  shape and native defaults remain unchanged. Decode raw values with validation
  before constructing the typed state; serialization cannot produce a cross-
  mode combination from safe authored values. Preserve omitted-field defaults,
  omission of defaults on encoding, and the current acceptance of unknown
  object fields. Known irrelevant fields remain acceptable only at their
  existing defaults; nondefault cross-mode values remain invalid.
- Migrate builders, direct field access, sample authors, fixtures, and fake
  consumers in this task. Do not retain the invalid public field combination
  alongside the new API. Keep unrelated panel target/render-mode work out.
- Authored impossible values are developer errors; follow Rust conventions for
  checked constructors. Malformed incoming serialized data still returns a
  decoding/validation error at the external boundary.

**Working boundary:** authored values cannot combine incompatible scale fields;
all three modes still render through the current C# codec. Run invalid raw JSON
cases, independent wire fixtures, and native panel scaling cases. No protocol
version negotiation or compatibility layer is introduced.

### Task 12 — Extract performance input readers by source

**Prerequisites:** none; default after Task 11 for a serial implementation.

Split `perf_sources.py` into Codex discovery/rollout parsing, CI/operation/
workflow log readers, Tollgate reading, and content sanitization. Keep
report orchestration in `perf_report.py`; keep normalized models in
`perf_model.py`.

- Functions that read files, execute commands, or open SQLite own and close
  those resources. Parsing functions take decoded records and return normalized
  results plus warnings. Preserve read-only database mode and fallback order.
- Keep child-session folding next to session parsing; do not mix it into
  reporting or repeat correlation in each input reader.
- Preserve input cutoff handling, candidate-ID attribution, unknown/malformed
  record warnings, sanitization, and command timeouts.
- Migrate report/test imports together and remove the obsolete catch-all facade.
  Do not make a new module import its former parent for shared helpers.

**Working boundary:** fixed synthetic input logs/databases yield the same
report data, warnings, candidate associations, and sanitized output. Run
`python3 scripts/tests/perf-report.test.py` and related candidate tests. Keep
real user transcripts out of fixtures and published review artifacts.

### Task 13 — Normalize performance parser state at ingress

**Prerequisites:** Task 12.

Within each reader, replace repeated validated dictionary access and opaque
internal tuples with named records where the same shape travels between
functions. Keep unknown external records outside the normalized model.

- Give pending turns/tools and candidate attempts explicit ownership in the
  parser invocation. Do not add global caches or mutable module state.
- Keep absent evidence distinct from zero time, and incomplete spans distinct
  from completed spans. Preserve retry attempts without double-counting time.
- Use closed internal enums only for statuses the reader controls; unknown
  external statuses remain a supported parsing outcome with warnings.
- Test realistic malformed, partial, retry, and interleaved records at the
  reader boundary rather than asserting the new dataclass layout.

**Working boundary:** missing sources remain reported as unavailable, child and
retry durations retain their semantics, and report JSON retains its external
schema and field meanings. Compare representative before/after fixture outputs
for equal normalized values, including missing values and warnings. Run
performance parser/report tests including overlapping intervals, interrupted
tools, missing database fallback, and sanitized content.

## Completion criteria for the implementation sequence

The architecture is improved when responsibility and mutation are clear from
callers, not when every file falls below an arbitrary threshold.

- One owner controls each runtime lifetime, UI graph, dispatch transaction, and
  parser's pending state. Collaborators cannot mutate its collections directly.
- Closed protocol variation remains represented by enums/case catalogs;
  extension abstractions exist only at actual extension or external boundaries.
- Rejected operations preserve the documented pre-mutation guarantees, and
  post-mutation native failure still follows explicit host cleanup behavior.
- Removed tests have a reviewed rationale; replacements fail representative
  faults. Wire contracts, benchmark sizes, and meaningful negative cases remain.
- Each completed task compiles and passes its selected checks on its own.
  Record retained run handles and task-specific evidence in the implementation
  review, not as a growing progress log in maintained guidance.

## Manual QA

Use the native UI and Reactant sample suites and their existing navigation;
consult their current Ditto scenario definitions for identifiers. Preserve
controlled time and normal readiness rather than adding arbitrary frame waits.

- **Early assembled check, Task 4:** open the native UI sample, create/update a
  child, reparent it, activate it, and restore the page. Confirm the Rust
  handler
  runs once and the next presented frame shows the expected state and order.
- **Documents and input, Tasks 5–7:** exercise a controlled value rejection and
  acceptance, set/reset style, hover between controls, and destroy the hovered
  target. Confirm no stuck hover, duplicate events, stale focus, or leaked UI.
  Open/close an overlay and check modal focus and logical order after reparent.
- **Runner lifetime, Task 8:** connect, exercise input, lose/regain focus,
  reconnect, and stop. Input stays disabled until the replacement snapshot is
  ready; prior transient state disappears. Use existing failure fixtures to
  confirm recoverable versus restart-required presentation.
- **Panel authoring, Task 11:** run pixel, physical, and screen-size scaling
  examples. Check their expected scale and text/layout after viewport changes;
  invalid incoming cross-mode data is rejected before native panel mutation.
- **Reports, Tasks 12–13:** generate a report from synthetic complete, partial,
  retrying, and missing-source inputs. Inspect warnings and durations. Confirm
  unavailable evidence is not shown as zero and binary/encrypted content is
  excluded while ordinary transcript text remains available as intended.

Retain native screenshots for the visual flows and the relevant run results.
Any changed rendering in this behavior-preserving sequence needs investigation;
baseline approval is not a substitute for explaining it.
