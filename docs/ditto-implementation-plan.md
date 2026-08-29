# Battlement Ditto implementation plan

When a task is complete, append `[DONE]` to its task heading.

Status: implementation companion to
[`ditto-technical-design.md`](ditto-technical-design.md)

This plan implements the approved Battlement Ditto contract without revising
its behavior. The technical design is normative. If this plan and the design
disagree, the technical design wins.

## Related information

- [`ditto-technical-design.md`](ditto-technical-design.md) defines every Ditto
  command, model, protocol, platform, storage, failure, and performance
  requirement implemented by this plan.
- [`implementation-plan.md`](implementation-plan.md) describes the native
  runtime, sample workflow, current CI, and Tollgate promotion mechanism on
  which Ditto builds.
- [`visual-capture.md`](visual-capture.md) describes the legacy workflow that
  remains active until the atomic cutover.
- [`battlement-ui-implementation-plan.md`](battlement-ui-implementation-plan.md)
  and
  [`reactant-implementation-plan.md`](reactant/reactant-implementation-plan.md)
  provide the feature inventories for the two lab samples.

## Decisions and starting point

Ditto does not exist yet. The repository already contains the native Rust
engine, Unity host, unified logging path, five standalone samples, sample build
commands, WebGL deployment tooling, CI cache, and the visual-capture workflow
that Ditto will replace.

The following decisions were resolved while preparing this plan:

- The transition covers every convention-based sample with `sample.toml`:
  Basic, Tic-Tac-Toe, Chess, UI, and Reactant. Reactant is included in addition
  to the four suites named by the technical design because complete migration
  of all samples is an explicit project requirement.
- A **convention-based sample** is a directory directly under `samples/` that
  contains `sample.toml`. Suite and CI discovery use this contract rather than
  a hard-coded sample list.
- **Full screenshot coverage** means every stable user-visible screen and every
  materially distinct interaction result. A state is materially distinct when
  a user action or semantic setup changes stable rendered presence, content,
  layout, style, camera, or feedback after settling. Reversible interactions
  capture the initial, changed, and restored states. A transient frame is
  required only when a controlled Battlement-owned transition communicates
  behavior that no settled state demonstrates.
- Each sample owns a compiler-visible Rust registry of its screens and stable
  visual-state families. A visual-state family is a finite semantic rendering
  branch such as empty board, selected piece, terminal result, or recoverable
  failure; its payload may still contain positions, scores, or board contents.
  Rendering and public action transitions name registry keys, and exhaustive
  matches make new variants fail compilation until they are classified.
- A separate Ditto coverage ledger maps every registry key to a scenario,
  checkpoint, canonical profile, and owning assertion. Reversible transitions
  and conditional states that the sample does not expose are recorded beside
  the registry. Adding a sample state without updating both sides fails CI.
- Screenshot checkpoints test rendered outcomes. Assertions test nonvisual
  state, input reachability, lifecycle, and error behavior. Do not multiply
  screenshots to cover facts that are clearer as black-box assertions.
- **Production input targeting** uses a Battlement UUID or normalized render
  coordinates and lets the ordinary Unity input system select the receiver. It
  never invokes a game action or private test hook directly.
- A **native semantic setup fixture** is an optional Rust engine hook that
  establishes game state before connection. It does not mutate Unity
  presentation, and visible interaction after setup still uses production
  input targeting.
- All five sample suites use one **canonical profile**, the 1280 by 720 macOS
  CI profile whose images become their required baselines. WebGL and iOS
  Simulator keep focused **adapter smokes**, which are small suites proving the
  platform boundary rather than a complete sample-platform cross-product.
- The existing capture workflow stays operational until Ditto, all five macOS
  suites, both adapter smokes, stable result output, and replacement CI have
  passed together. Do not remove or repurpose its assets earlier.
- The performance gate uses the technical design's fixed 20-scenario,
  40-screenshot benchmark on the designated CI-class Apple Silicon host. A
  cached build must complete within 20 seconds from a cold player launch and
  within 5 seconds with the player, HTTP session, and ODiff server warm.
- Every full sample suite is also measured separately. Those measurements
  report build, launch, setup, execution, capture, comparison, reset, and
  durability time, but they do not replace or dilute the normative benchmark.
- CI migration is conditional on measured evidence. The final cutover cannot
  begin while either normative time budget fails or any full sample suite is
  flaky. If measurement exposes a miss, optimize the measured critical path
  and repeat the same benchmark rather than redefining the target.
- CI runs the five macOS suites in parallel Tollgate slots. Scenarios within
  one suite remain serial and reuse one player, HTTP session, and ODiff server.
- Each task should target roughly 150–350 lines of non-test code. A larger
  change must be divided unless a wire contract or end-to-end transaction would
  become untestable when split.

## Task and testing conventions

Implementation is a mostly linear stack. Each task depends on the preceding
task unless its prerequisites say otherwise, leaves every workspace compiling,
and exposes only behavior that works end to end.

Task numbers are coordination metadata used only in this plan. Never put them
in source comments, diagnostics, filenames, sample text, or public
documentation. Durable artifacts are named after their behavior or scenario.

Every public Rust or C# API added or changed in a task receives concise
user-facing documentation in the same change. Shared Rust and C# wire fixtures
are authoritative acceptance evidence for their respective model tasks.

Black-box tests use public CLI, HTTP, player, fake-store, and Unity boundaries.
They must survive replacement of internal state machines, caches, serializers,
or task schedulers while observable behavior remains unchanged. Test-only
faults live in fixtures; production players receive no fault-injection command.

The simpler game samples add their visible-state inventories in their migration
tasks.

Before repository validation, stage every intended change and run
`./scripts/ci.py`. Tasks that add slow player or adapter checks also run their
focused validation directly. Major changes receive the repository-mandated
independent review once for the complete project, not once per task.

### Evidence contract

Before Ditto can run a useful end-to-end capture, runtime-only tasks retain the
named black-box test output and representative public job, event, or result
fixture. A private implementation snapshot is not review evidence.

Checked fixtures live with their owning tests. Executed run evidence lives in
the immutable run directory named by `result.json`. Benchmark and shadow-CI
reports are uploaded as ordinary CI artifacts. A task handoff links the exact
fixture, run result, coverage report, or benchmark report that supplies its
acceptance evidence; prose copied into the handoff is not the durable record.

After the first macOS capture slice exists, every player-visible task uses the
final staged Ditto binary and immutable player build for its own evidence. The
handoff includes the run ID, profile, result path, scenario names, screenshot
paths, and exact correlated log ranges from the technical design.

Sample migration tasks follow these rules:

- Author scenarios in the sample's checked-in `ditto.toml`; do not add
  sample-specific C# or a private Unity testing API.
- Use stable Battlement UUID aliases and production input targeting. Do not use
  hierarchy paths, object names, selectors, or direct action invocation.
- Keep scenarios deterministic through a fresh engine, explicit seed, instant
  or controlled motion, and native setup fixtures only when semantic state is
  otherwise expensive to reach.
- Capture each stable initial and changed state once per canonical profile.
  Identical restored pixels may reuse the same accepted content-addressed
  object, but the scenario still proves the complete interaction round trip.
- Keep screenshot names semantic and durable. Never encode task numbers,
  ordering, timestamps, or internal implementation details in them.
- Treat a missing coverage-ledger entry, missing baseline, unexpected skip,
  unavailable screenshot, runtime error, or log loss as a failed migration.

### Sample coverage matrix

The ledger is more detailed than this overview, but it must preserve these
minimum user-visible outcomes:

- **Basic:** connected initial scene, hover feedback, drag interaction,
  committed placement, and a fresh-engine restoration.
- **Tic-Tac-Toe:** empty board, human move, AI response, representative human
  and AI outcomes, draw, and restarted board.
- **Chess:** title or start state, initial board, selected piece and legal
  moves, committed move, AI response, capture, supported special moves, and
  supported terminal results, paused or recoverable failure, and refresh.
- **UI:** every navigation page, stable control or style variant, ledger-listed
  interaction result, reversible restoration, target texture, and world-space
  output.
- **Reactant:** every navigation screen, stable component or hook state,
  ledger-listed interaction result, reversible restoration, and world
  projection when present.

When a sample does not expose one of the listed conditional states, the ledger
records the concrete reason instead of inventing a Ditto-only behavior.

## Dependency overview

| Wave | Tasks | Result |
|---|---|---|
| 1 | 01–06 | Crates, configuration, models, and local results |
| 2 | 07–11 | Tool discovery, fingerprints, builds, and caches |
| 3 | 12–22 | Player execution and diagnostics |
| 4 | 23–32 | HTTP, macOS execution, comparison, and baselines |
| 5 | 33–39 | Local loop, review, watch, video, WebGL, and iOS |
| 6 | 40–48 | Complete screenshot coverage for all five standalone samples |
| 7 | 49–53 | Performance, shadow CI, cutover, and release |

## Wave 1: crates, configuration, and durable results

### Task 01 — Establish the tooling and Ditto crates [DONE]

**Prerequisites:** none. **Target:** 150–225 non-test lines.

Add `battlement-tooling` and `battlement-ditto` to the workspace. Give Ditto a
library entry point, standalone executable, and delegated
`cargo battlement ditto` entry point that use one parser and implementation.
Implement repository-root and suite discovery plus a complete `list` command
for one minimal macOS suite. No other command is publicly accepted yet.

**Black-box acceptance:** both entry points print byte-equivalent list output,
find the same suite from nested directories, resolve suite-relative paths after
changing the current directory, and reject paths that escape the repository.

**Evidence:** CLI transcript for direct and Cargo-delegated discovery.

### Task 02 — Parse and validate the complete suite model [DONE]

**Prerequisites:** Task 01. **Target:** 200–300 non-test lines.

Implement strict TOML models for top-level settings, player, timeouts,
defaults, comparison, aliases, baselines, profiles, scenarios, and all step
shapes. Apply defaults member by member and preserve exact decimal comparison
values. Reject unknown, ambiguous, duplicate, oversize, unsupported, and
out-of-range values with file, line, key path, and nearest-key suggestion.

**Black-box acceptance:** table-driven public parser tests cover every valid
field and every normative rejection, including path containment, timeout
relationships, paired videos, UUIDs, names, and exact decimal boundaries.

**Evidence:** stable diagnostic transcript for representative nested mistakes.

### Task 03 — Resolve profiles, filters, skips, and fragments [DONE]

**Prerequisites:** Task 02. **Target:** 200–300 non-test lines.

Implement profile selection, scenario include and exclude unions, capability
preflight, `--allow-empty`, and deterministic runnable versus skipped scenario
materialization. Add full-suite and inherited fragment resolution for files and
standard input, including alias merging, save-path bases, precedence, and
standard-input watch rejection.

**Black-box acceptance:** one matrix covers all targets, supported and skipped
capabilities, duplicate selectors, missing matches, file and standard-input
fragments, full-suite noninheritance, path containment, and deterministic
run-index assignment including host-materialized skips.

**Evidence:** `list` output showing selected checkpoints and precise skip
reasons for macOS, WebGL, and iOS Simulator profiles.

### Task 04 — Define job, profile, scenario, and step wire models [DONE]

**Prerequisites:** Task 03. **Target:** 250–350 non-test lines.

Implement the complete job, command, resolved profile, display, capability,
scenario, save, step, input, wait, assertion, screenshot, comparison, and video
wire models from the technical design. Enforce their cross-field, size, ID,
hash, timeout, decimal, capability, and tagged-union invariants outside Serde.

**Black-box acceptance:** exhaustive positive and negative Rust fixtures cover
every variant, unknown field, malformed union, bound, conditional field, and
resolution invariant in the authoring-to-job boundary.

**Evidence:** one valid resolved job plus representative rejected fixtures.

### Task 04A — Define session and lifecycle wire models [DONE]

**Prerequisites:** Task 04. **Target:** 250–350 non-test lines.

Implement startup identity, reports, log acknowledgements, infrastructure
failure, artifacts, scenario completion, step results, failure frames, native
video inputs, boundary outcomes, decisions, terminal completion and failure,
unstarted scenarios, context records, log records, HTTP errors, and their
shared enums. Enforce sequence, size, ID, hash, ownership, and conditional
invariants outside Serde.

**Black-box acceptance:** positive and negative fixtures cover every lifecycle
variant, malformed union, context body, startup identity conflict, completion
bound, and player-session or sequence mismatch.

**Evidence:** one complete lifecycle exchange and mixed exact-byte NDJSON
fixture.

### Task 04B — Define result, review, and baseline-state models [DONE]

**Prerequisites:** Task 04A. **Target:** 250–350 non-test lines.

Implement review events and acceptance, baseline store state and tombstones,
and the complete run, build, phase, player-session, job, scenario, step,
screenshot, comparison, video, error, and baseline-write result types. Enforce
every conditional result invariant and canonical ordering, indentation, decimal,
timestamp, hash, and newline rule.

**Black-box acceptance:** fixtures cover every closed result and review variant,
invalid conditional field, unresolved error reference, artifact mismatch,
canonical serialization, and baseline-state generation rule.

**Evidence:** one complete result, review exchange, and canonical baseline-state
fixture plus representative rejections.

### Task 05 — Add run-local errors, phases, and status reduction [DONE]

**Prerequisites:** Task 04B. **Target:** 150–225 non-test lines.

Implement run-local `E####` occurrence allocation, stable error codes, source
and context attachment, deadline kinds, phase recording, and final status and
exit-code precedence. Add player-local `P####` mapping with idempotent replay
so player and host occurrences cannot collide or duplicate one failure.

**Black-box acceptance:** functional failure, infrastructure failure,
interrupt, secondary durability errors, replayed completion, and caught-failure
envelopes reduce to the exact status, primary occurrence, references, and exit
code required by the design.

**Evidence:** stable machine result excerpts for each terminal status.

### Task 06 — Make local run data durable and recoverable [DONE]

**Prerequisites:** Task 05. **Target:** 200–300 non-test lines.

Allocate immutable run directories before discovery, print the stable first
stderr line, maintain atomically replaced `partial-result.json`, commit terminal
`result.json`, index repository and suite identity, and materialize exhaustive
artifact paths. Add run leases, seven-day and 1 GB retention, interrupted-run
recovery, failed-final-commit handling, and comparison-only derived-run copying
without mutating the source run.

**Black-box acceptance:** simulated crashes after each durability boundary
produce either one authoritative terminal result or one recoverable partial;
derived runs resolve every relative path inside themselves; active leases and
oversize runs survive cleanup.

**Evidence:** abandoned-run recovery and immutable derived-run transcripts.

## Wave 2: tool discovery, builds, and caches

### Task 07 — Discover tools and share Unity editor capacity [DONE]

**Prerequisites:** Task 06. **Target:** 150–225 non-test lines.

Implement supported discovery for Unity, Apple tools, ODiff, optional FFmpeg,
cache roots, and filesystem capabilities in `battlement-tooling`. Implement the
same machine-wide Unity editor lease contract used by current CI without moving
or deleting the legacy Python owner yet. Add the host checks needed by
`ditto doctor`, reported separately for required, optional, read-only, and
write operations.

**Black-box acceptance:** fake hosts cover missing, mismatched, unwritable, and
optional tools; Rust and Python leases exclude one another; doctor redacts
credentials and names installed alternatives where required.

**Evidence:** doctor output for one healthy read-only host and representative
actionable failures.

### Task 08 — Fingerprint all player source inputs [DONE]

**Prerequisites:** Task 07. **Target:** 200–300 non-test lines.

Implement the sorted, streamed SHA-256 source manifest over conservative Unity,
Rust, local-package, generated-input, mode, and byte content. Include tracked,
staged, unstaged, and untracked bytes; exclude only the normative generated and
cache paths; reject repository escapes and ambiguous symlinks. Retain the
manifest for nearest-build diagnostics.

**Black-box acceptance:** byte, mode, local dependency, generated input,
deletion, untracked file, symlink, and excluded-cache changes produce the exact
expected fingerprint behavior on case-sensitive and case-insensitive fixtures.

**Evidence:** before-and-after manifest diff naming changed inputs without
using Git commit identity or timestamps.

### Task 09 — Derive immutable build identities [DONE]

**Prerequisites:** Task 08. **Target:** 150–225 non-test lines.

Derive the build fingerprint from source, target, Unity, Rust, applicable Apple
toolchains, diagnostics, capture adapter, native inputs, and byte-affecting
options. Explicitly exclude profile name, display, device, orientation,
headless command, scenarios, aliases, baselines, seeds, saves, and motion.

**Black-box acceptance:** a table changes each included input independently,
proves every excluded runtime input reuses the same build, and keeps baseline
identity independent of both fingerprints.

**Evidence:** stable fingerprint explanation for one cached and one rejected
`--no-build` request.

### Task 10 — Implement the shared immutable build cache [DONE]

**Prerequisites:** Task 09. **Target:** 200–300 non-test lines.

Add temporary build publication, per-fingerprint leases, complete metadata and
logs, a configurable 20 GB LRU, active-entry protection, oversize-entry
reporting, global and suite cleanup, and concurrent reader/writer behavior.
Failed builds retain their logs but never publish reusable entries.

**Black-box acceptance:** concurrent callers build once; interrupted
publication remains invisible; active entries survive pressure; cleanup
selects only inactive entries; a failed current source never falls back to an
older player.

**Evidence:** cache journal covering creation, reuse, failure, and eviction.

### Task 11 — Build immutable macOS players [DONE]

**Prerequisites:** Task 10. **Target:** 200–300 non-test lines.

Implement the supported macOS Unity player build through
`battlement-tooling`, including the native Rust engine, generated inputs,
diagnostics setting, full build log, immutable metadata, and startup identity.
Use the shared editor lease and never accept arbitrary suite-provided editor
methods or shell commands.

**Black-box acceptance:** a clean fixture builds and launches from its cache
entry; the same inputs reuse it; Rust, Unity, package, diagnostics, or toolchain
changes select a new entry; compilation failure yields a terminal result and
retained build log without launching.

**Evidence:** immutable build metadata and reuse transcript.

## Wave 3: player execution and diagnostics

### Task 12 — Extend the managed log store for Ditto [DONE]

**Prerequisites:** Task 04. **Target:** 200–300 non-test lines.

Add immutable ordinary and typed context payloads to Battlement's unified
managed store while preserving viewer behavior. Add process-lifetime sequence,
UTC timestamp, source, severity, event name, exception, stack, retained
snapshot, and ordered observer registration under one lock. The observer copies
to a separate bounded delivery queue and performs no serialization, IO, HTTP,
or Unity calls.

**Black-box acceptance:** ordinary Rust, Battlement, Unity, and context records
share one exact order; observer registration loses and duplicates nothing;
viewer eviction cannot create a delivery gap; normal players without Ditto
retain their current logging behavior.

**Evidence:** public C# store transcript with interleaved sources and context.

### Task 13 — Mirror job and scenario contracts in C# [DONE]

**Prerequisites:** Tasks 04 and 12. **Target:** 200–300 non-test lines.

Implement hand-written C# models and validation for jobs, profiles, scenarios,
saves, steps, inputs, waits, assertions, screenshots, comparisons, and videos.
Add the early player bootstrap that fetches a job only when runner diagnostics
are included. A diagnostics-disabled player retains ordinary logging but omits
the viewer and executor; Task 27 rejects its build metadata before launch.

**Black-box acceptance:** shared Rust and C# job fixtures accept and reject the
same payloads, unknown fields, variants, bounds, IDs, hashes, saves, and
capability relationships; diagnostics-disabled builds contain no usable
executor.

**Evidence:** paired job-fixture report plus one diagnostics-disabled rejection.

### Task 13A — Mirror session and lifecycle contracts in C# [DONE]

**Prerequisites:** Task 13. **Target:** 250–350 non-test lines.

Implement startup, log acknowledgement, infrastructure failure, artifact,
scenario completion, step result, failure frame, native video, boundary,
decision, terminal completion and failure, context, log, and HTTP error models.
Serialize exact NDJSON event unions and enforce every player-side size,
sequence, identity, ownership, and conditional invariant.

**Black-box acceptance:** shared Rust and C# lifecycle fixtures accept and
reject the same payloads, unknown fields, malformed unions, conflicts, bounds,
and sequence relationships.

**Evidence:** paired lifecycle-fixture report with mixed log and context events.

### Task 14 — Add fresh-engine creation and destruction boundaries [DONE]

**Prerequisites:** Task 13A. **Target:** 200–300 non-test lines.

Keep the native destroy ABI panic-safe with bounded diagnostics and implement
explicit fresh engine-session creation without changing ordinary reconnect
behavior. Per the normative technical design, Ditto uses the ordinary `Connect`
message and games receive no Ditto-specific setup hook.

**Black-box acceptance:** no engine exists before the first reached scenario;
consecutive scenarios receive distinct engines; an unfinished scenario cannot
be replaced implicitly; destroy errors and caught destructor panics return the
required classifications.

**Evidence:** public native fixture journal with engine-session boundaries.

### Task 15 — Reset all Battlement-owned player state [DONE]

**Prerequisites:** Task 14. **Target:** 200–300 non-test lines.

Implement one post-scenario reset that destroys the engine, clears retained
snapshots, UI, objects, scenes, assets, input, clocks, pending requests,
commands, and Ditto-owned diagnostics without disturbing project-authored
Unity state. Starting the next scenario must never repeat the boundary.

**Black-box acceptance:** a deliberately dirty scenario leaves no engine,
identity, lease, command, request, clock, key, pointer, or runtime object for
the next scenario; destroy and reset failures identify their exact boundary
stage and mark the player non-reusable.

**Evidence:** before/reset/after public Unity state journal.

### Task 16 — Resolve conditions and production input targets [DONE]

**Prerequisites:** Task 15. **Target:** 200–300 non-test lines.

Implement `exists`, `absent`, `visible`, `hidden`, `enabled`, and `disabled`
from the latest Battlement snapshot and resulting presentation. Resolve UUID,
alias, and normalized-coordinate targets through the fixed 5 by 5 lattice,
clipped projected bounds, and production UI, EventSystem, and physics hit paths.

**Black-box acceptance:** clipping, opacity, ancestor state, absent objects,
unsupported world enabled-state queries, overlays, nested UI, and world hits
produce the normative result and blocking-object diagnostics. No target action
is invoked directly.

**Evidence:** input-target journal including one blocked UUID.

### Task 17 — Inject deterministic virtual input [DONE]

**Prerequisites:** Task 16. **Target:** 200–300 non-test lines.

Implement click, hover, segmented drag, balanced key, and iOS one-finger touch
state transitions through Unity's production Input System. Use exact move,
press, release, interpolation, frame, and top-left coordinate rules. Reset fails
if any authored key or pointer button remains held.

**Black-box acceptance:** public player fixtures observe exact frame-by-frame
input, drag segment counts, final coordinates, keyboard transitions, hover
skip on iOS, and held-input rejection. Host pointer and keyboard never move.

**Evidence:** input frame journal from one click, drag, and key sequence.

### Task 18 — Add deterministic motion and settling [DONE]

**Prerequisites:** Task 17. **Target:** 200–300 non-test lines.

Route Battlement-owned tweens, particles, and audio timing through instant,
controlled, and real-time motion. Observe command groups, pending Rust work,
finite operations, layout changes, and committed frames to implement the exact
two-quiet-frame settling contract and exact controlled-frame waits.

**Black-box acceptance:** direct action-to-assert and action-to-screenshot
fully settle; exact waits preserve intermediate state; instant commits final
values and suppresses particles and audio; controlled frames are repeatable;
custom game scripts remain uncontrolled and diagnostically visible.

**Evidence:** same fixture captured at deterministic instant, intermediate,
and real-time states.

### Task 19 — Execute steps with bounded deadlines [DONE]

**Prerequisites:** Task 18. **Target:** 200–300 non-test lines.

Implement serial setup and all non-video steps with optional names, implicit
settling, assertions, reached-step accounting, and step and scenario monotonic
deadlines. Apply the earliest configured step, remaining scenario, and
remaining run cap without letting any phase extend the run.

**Black-box acceptance:** every step shape reaches the expected production
effect; assertion and timeout stop later steps while a screenshot mismatch can
continue; each expired result names the correct deadline; setup and execution
duration exclude reset and host processing.

**Evidence:** complete player step results for pass, assertion failure, and
each deadline class owned by the player.

### Task 20 — Capture native PNGs and failure frames [DONE]

**Prerequisites:** Task 19. **Target:** 200–300 non-test lines.

Implement the macOS and iOS render-surface adapter through ScreenCapture,
render textures, and asynchronous GPU readback. Probe dimensions, alpha,
orientation, row order, and channels at startup; encode only committed frames;
capture the last responsive frame after functional failure.

**Black-box acceptance:** known-color fixtures round-trip exact pixels and
dimensions; images exclude host chrome and cursors; unsupported readback,
probe mismatch, encoding failure, process loss, and unavailable failure frame
produce bounded structured results without a silent fallback.

**Evidence:** probe PNG, ordinary screenshot, and responsive failure frame.

### Task 21 — Deliver ordered logs and artifacts

**Prerequisites:** Tasks 12, 19, and 20. **Target:** 250–350 non-test lines.

Implement bootstrap and job capture windows, native tracing drains, typed
context emission, player-side recursive redaction, exact NDJSON serialization,
bounded delivery, contiguous batches, artifact upload, acknowledgements, retry
buffers, and mandatory flushes at step, artifact, scenario, and terminal
boundaries.

**Black-box acceptance:** lost acknowledgements replay identical bytes;
conflicts, gaps, oversize records, native drops, and delivery overflow fail
without truncation; warm idle records do not enter a later job; accepted PNGs
are acknowledged before their context marker and completion.

**Evidence:** mixed exact-byte NDJSON and PNG delivery transcript.

### Task 22 — Classify failures and complete scenario boundaries

**Prerequisites:** Task 21. **Target:** 200–300 non-test lines.

Implement the functional error gate, structured Unity and Rust correlation,
freeze and failure-frame behavior, engine and scenario context closure,
boundary outcomes, remaining-step state, and cleanup ordering. Keep caught
Battlement exception envelopes as diagnostics for the original occurrence.

**Black-box acceptance:** screenshot mismatch, assertion, timeout, Unity error,
assert, exception, Rust panic, fatal, destroy failure, reset failure, and crash
produce the required scenario status, one occurrence, log span, recovery, and
remaining-step results.

**Evidence:** representative complete and incomplete correlated scenario spans.

## Wave 4: host orchestration, comparison, and baselines

### Task 23 — Serve isolated player sessions

**Prerequisites:** Tasks 11, 13, and 21. **Target:** 200–300 non-test lines.

Bind one HTTP/1.1 server to explicit IPv4 loopback, allocate an unguessable
route token and pending player-session identity, install one immutable job, and
implement `GET job`, startup, and route expiry. Enforce method, origin, media,
body-size, and token restrictions without presenting the route token as a
security boundary against another local process.

**Black-box acceptance:** unknown, expired, cross-session, wrong-origin,
oversize, wrong-media, and conflicting startup requests receive the exact
status and `HttpError`; native requests with no Origin work; accepted and
rejected startup facts remain in the result.

**Evidence:** HTTP transcript for one accepted and one rejected player.

### Task 24 — Make every mutating route idempotent

**Prerequisites:** Task 23. **Target:** 200–300 non-test lines.

Implement log, artifact, scenario-completion, job-completion, and job-failure
routes with exact-byte request identities and durable acknowledgements. Enforce
mutual exclusion of terminal operations, next expected log sequence, PNG
metadata, completion prerequisites, and the one uncertain-request retry
contract.

**Black-box acceptance:** an identical retry after every possible lost
acknowledgement returns the stored decision without duplicate bytes,
comparison, error count, or finalization; changed replays and sequence gaps
return the exact `409` diagnostics; durable host storage failure returns `500`
and terminates the run.

**Evidence:** route replay matrix with stored acknowledgement identities.

### Task 25 — Orchestrate scenario decisions, bail, and recovery jobs

**Prerequisites:** Tasks 22 and 24. **Target:** 200–300 non-test lines.

Accept only durably flushed scenario completions, materialize player and
host-derived results, count failed scenarios once, and persist the next action
before replying. Implement continue, stop, and relaunch, `--bail[=N]`, skipped
and not-run reasons, recovery jobs beginning at the next scenario, and one
unchanged run deadline across relaunches.

**Black-box acceptance:** functional failures continue without bail; bail stops
only after the active scenario's artifacts and result are durable; a failed
boundary relaunches at the next scenario; a failed relaunch marks all remaining
scenarios not-run and never retries a reached scenario.

**Evidence:** two-job result covering pass, fail, relaunch, and bail.

### Task 26 — Supervise players and reconstruct crashes

**Prerequisites:** Task 25. **Target:** 200–300 non-test lines.

Supervise owned macOS processes, configured WebGL commands, and Simulator apps.
Reconstruct a scenario from durable context when completion was lost, preserve
complete versus incomplete log spans, retain secondary diagnostics, distinguish
an idle warm-session loss, and handle a crash after durable completion without
creating a second failure.

**Black-box acceptance:** crashes before and after scenario end, after durable
completion, between scenarios, and during idle watch state produce the exact
scenario, job, player-session, occurrence, and recovery records required by the
design.

**Evidence:** crash matrix with managed and secondary log paths.

### Task 27 — Launch and complete macOS capture runs

**Prerequisites:** Tasks 20, 25, and 26. **Target:** 200–300 non-test lines.

Launch the immutable macOS player with its session URL, validate its startup
report against the selected build and runtime profile, execute capture jobs,
retain the scoped player log, and clean up only the owned process. Add bounded
launch, startup, run, reset, and durability phases plus responsive interrupt
handling and exit code `130`.

Before creating a process, read immutable build metadata and reject a build
whose diagnostics flag is false. Startup validation repeats that check against
the player report to detect corrupted or mismatched build contents.

**Black-box acceptance:** a fixture completes three scenarios through one
player with fresh engines; diagnostics-disabled metadata starts no process;
display, build, source, diagnostics, adapter, capability, and Unity startup
mismatches stop before setup; Ctrl-C returns within two seconds when responsive
and retains an interrupted result.

**Evidence:** the first complete `ditto capture` macOS run and `result.json`.

### Task 28 — Compare images through one warm ODiff server

**Prerequisites:** Task 27. **Target:** 150–225 non-test lines.

Download and verify the pinned ODiff v4.5.0 binary for each macOS architecture,
support the explicit development override, and keep one server alive per run.
Validate PNG dimensions before comparison, apply exact decimal threshold and
changed-pixel inequalities, and retain a red mask for any nonzero difference.

**Black-box acceptance:** exact match, tolerated change, boundary pixel count,
material mismatch, wrong dimensions, corrupt PNG, wrong binary, server exit,
and timeout produce the required comparison or infrastructure result. Tests
never use rounded ODiff percentages for acceptance.

**Evidence:** actual, baseline, diff, counts, and effective settings for one
pass and one mismatch.

### Task 29 — Implement filesystem baselines and atomic updates

**Prerequisites:** Task 28. **Target:** 200–300 non-test lines.

Implement canonical `ditto.lock` parsing and serialization, local hydration,
the filesystem content-addressed store, reached-only comparison, missing
baseline behavior, update eligibility, filtered and unfiltered pruning,
starting-lock digest checks, suite mutation leases, and one atomic manifest
rewrite after every eligible upload succeeds.

**Black-box acceptance:** ordinary run, capture, initial lock creation, full
update, filtered update, failed scenario, runtime skip, stale lock, partial
upload, external store root, concurrent acceptance, and offline cached run
preserve every normative all-or-nothing rule.

**Evidence:** deterministic lock diff and update result with published and
uploaded-unreferenced states.

### Task 30 — Hydrate R2 baselines without credentials

**Prerequisites:** Task 29. **Target:** 150–225 non-test lines.

Implement public HTTP reads from the R2 content-addressed namespace, verified
atomic cache insertion, same-hash request coalescing, reached-only download,
`fetch` selection, and bounded parallel `fetch --all`. Read-only commands must
not require or print Cloudflare credentials.

**Black-box acceptance:** a fresh cache hydrates and verifies; a prewarmed
cache works offline; wrong bytes never enter the cache; concurrent fetches
download once; an unreached checkpoint performs no network request; a true
miss is infrastructure rather than image mismatch.

**Evidence:** online hydration followed by an offline passing run.

### Task 31 — Publish and retain R2 baseline replacements

**Prerequisites:** Task 30. **Target:** 200–300 non-test lines.

Implement write credentials through named environment variables, immutable
PNG upload, canonical namespace state, conditional remote leases, ETag-safe
publication, tombstones, restoration, and seven-day deletion through explicit
`clean storage --apply`. Apply the same state semantics and advisory locking to
filesystem stores.

**Black-box acceptance:** feature-branch acceptance creates no tombstone;
default-branch publication advances generation exactly once; lease loss and
ETag races delete nothing; restored hashes leave tombstones; dry run is
nonmutating; cleanup deletes only eligible nonlive objects and handles missing
objects idempotently.

**Evidence:** fake R2 request journal for publish, conflict, restore, and
cleanup.

### Task 32 — Expose the core run and storage CLI surface

**Prerequisites:** Tasks 25 and 31. **Target:** 200–300 non-test lines.

Expose `run`, `capture`, `fetch`, `list`, `doctor`, `clean runs`,
`clean builds`, `clean baselines`, `clean storage`, and `storage publish`
through both entry points. Complete common filters, update, bail, no-build,
JSON, output, allow-empty, profile, and global-clean options plus exact exit
codes and stdout/stderr separation. Review and watch options remain rejected
until Tasks 34 and 36 expose their complete behavior.

**Black-box acceptance:** a command matrix proves both entry points have
identical parsing, defaults, results, errors, and side effects; `capture` never
touches baselines; JSON stdout contains no prose; cleaning prints its scoped
plan and byte count before mutation and respects every active lease.

**Evidence:** public command matrix transcript.

## Wave 5: fast loop, review, watch, and platform adapters

### Task 33 — Make macOS capture the fast verification loop

**Prerequisites:** Task 32. **Target:** 150–225 non-test lines.

Polish suite and throwaway-fragment execution for agents: print stable progress
and paths, retain assertion-only runs with no baseline access, explain nearest
cached build mismatches, and make the machine result sufficient to locate every
error, step, screenshot, timing, and log span without opening review.

**Black-box acceptance:** existing suite, file fragment, standard-input
fragment, screenshot-free assertion, image mismatch, runtime error, build
error, and configuration error are diagnosable from terminal plus result JSON
alone.

**Evidence:** machine-copyable handoffs for passed, failed, and not-applicable
development work.

### Task 34 — Build the read-only local review application

**Prerequisites:** Tasks 28 and 32. **Target:** 250–350 non-test lines.

Vendor the pinned comparison slider and pan/zoom sources and licenses. Serve a
loopback-only application that selects a retained run and reads its result.
Implement side-by-side, swipe, alpha overlay, red mask, synchronized zoom and
pan, integer-pixel rendering, coordinates, scores, thresholds, scenario and
step navigation, correlated logs, and timings.

Expose `ditto review [RUN]`, implicit newest-reviewable-run selection, and
`--review` through both CLI entry points only after the complete read-only
application works.

**Black-box acceptance:** the application works offline with no package
installation or CDN, never infers status from filenames, renders missing and
unavailable cases clearly, and can navigate an image mismatch using only one
run directory.

**Visual evidence:** full review screen for a mismatch and a passing tolerated
change.

### Task 35 — Add atomic selective acceptance to review

**Prerequisites:** Tasks 31 and 34. **Target:** 200–300 non-test lines.

Protect state-changing review requests with a random token and caller UUID.
Allocate one immutable derived attempt run, validate a nonempty duplicate-free
selection set and starting lock digest, upload selected actuals, rewrite the
manifest once, recompare, and switch review only after success. Disable
acceptance clearly when write credentials are absent.

**Black-box acceptance:** identical retries return the stored response;
different bytes under one request ID conflict; stale lock, changed actual,
invalid fragment checkpoint, failed upload, and rewrite failure leave the lock
unchanged and retain the attempt run and related error.

**Visual evidence:** multi-selection before and after acceptance plus a
read-only credentials state.

### Task 36 — Keep execution and review warm in watch mode

**Prerequisites:** Tasks 33 and 35. **Target:** 250–350 non-test lines.

Implement debounced file observation, immutable cycles, player long polling,
one pending-state coalescer, scenario reload, comparison-only lock refresh,
background replacement build, warm target replacement, stale-player handling,
explicit broken-build retry, newline-delimited JSON, atomic latest output, and
one live review tab through replayable Server-Sent Events.

Expose `-w` and `--watch` through both CLI entry points only with this complete
cycle and review behavior.

**Black-box acceptance:** scenario, lock, source, and simultaneous changes take
their exact cheap or rebuilding paths; failed replacement builds never execute
new source on the stale player; idle loss is not a run failure; dispatched loss
expires launch; acceptance produces a derived comparison-only cycle; watch
rejects update and standard-input fragments.

**Visual evidence:** one review tab updating across execution, comparison-only,
failed-build, and recovered cycles.

### Task 37 — Add experimental native video

**Prerequisites:** Tasks 20 and 32. **Target:** 200–300 non-test lines.

Implement paired video execution, motion override and restoration, checked disk
preflight, fixed 30 fps RGBA8 macOS capture, duration truncation, FFmpeg
encoding, MP4 validation, raw-input cleanup, and retained raw diagnostics on
failure. Define the shared native-video metadata and host processing boundary
that Task 39 extends to Simulator container copying. WebGL skips before setup.

**Black-box acceptance:** controlled and real-time clips include intervening
actions and screenshots; automatic and runtime-failure truncation produce the
right result; the later stop is a passing no-op; insufficient space, partial
frame, wrong byte size, and FFmpeg failure retain bounded diagnostics and stop
later scenarios. An ordinary step after successful stop, automatic truncation,
and runtime failure observes the original scenario motion rather than the
clip's override.

**Visual evidence:** one macOS MP4 and its correlated screenshot and result.

### Task 38 — Add the WebGL build, launcher, and capture adapter

**Prerequisites:** Tasks 24, 26, and 32. **Target:** 250–350 non-test lines.

Build immutable WebGL players, serve the build, launcher, and HTTP API from one
loopback origin, pass the route without permissive CORS, and upload Unity canvas
PNG blobs through a small `.jslib` bridge. Support operating-system launch and
one configured headless command; bridge responsive console, exception, and
promise failures into the managed store.

**Black-box acceptance:** startup conformance validates canvas identity,
dimensions, alpha, orientation, and colors; click, hover, drag, key, settling,
PNG, comparison, log upload, browser failure, supervised exit, and launch
deadline use the common result contract. No browser screenshot or automation
package is part of execution.

**Visual evidence:** focused WebGL adapter run with a passing comparison and
responsive browser failure diagnostics.

### Task 39 — Add the iOS Simulator builder and launcher

**Prerequisites:** Tasks 20, 24, 26, 32, and 37.
**Target:** 250–350 non-test lines.

Build the app with the required local-network settings, select installed
runtime and device types, create one ephemeral Ditto-owned Simulator, boot,
install, orient, launch through explicit IPv4 loopback, query dimensions, scale,
and safe area, and retain app-scoped logs. Reuse the device only for its warm
player session and delete it during normal or stale cleanup.

Extend Task 37's native video path by resolving and copying a completed raw
recording from the Ditto-owned app container before common FFmpeg processing.

**Black-box acceptance:** portrait and landscape profiles use observed device
facts; touch input, PNG capture, comparison, runtime failure, log delivery, and
native video use the common protocol; hover skips before setup; unavailable
tools, runtime, device, endpoint, boot, install, launch, and cleanup have
bounded diagnostics and no leaked Ditto-owned device.

**Visual evidence:** focused Simulator adapter run in portrait and landscape.

## Wave 6: complete sample screenshot coverage

### Task 40 — Establish the checked sample coverage ledger

**Prerequisites:** Tasks 33, 38, and 39. **Target:** 150–225 non-test lines.

Define a machine-readable ledger for every convention-based sample. Discover
screens, stable states, interaction round trips, scenarios, checkpoints,
profiles, baselines, and test owners without hard-coding the current sample
count. The checker consumes compiler-visible Rust visual-state registries and
separate Ditto mappings. It validates canonical names, one owner per state, no
orphan checkpoints, expected platform skips, exact reachability, and missing
baselines without requiring every current gap to be closed in this task.

For the UI and Reactant labs, adapt their exhaustive page, screen, and feature
registries. Tasks 41–43 add equivalent finite visual-state enums to the
simpler games and route presentation and public visual action transitions
through exhaustive matches. The Ditto ledger may reference registry keys but
may not define or generate them.

**Black-box acceptance:** synthetic complete and incomplete sample fixtures
prove that removing or duplicating any sample, registry state, transition,
scenario, checkpoint, test owner, or baseline produces a precise gap. Adding a
new `sample.toml` requires a registry and ledger. The current repository report
is allowed to list each sample as pending migration under Tasks 41–48; it does
not claim state-level completeness until that sample supplies its registry.

**Evidence:** synthetic gap-detection matrix and a discovery report assigning
each pending sample migration to its owning later task.

### Task 41 — Cover the Basic sample completely

**Prerequisites:** Task 40. **Target:** 150–225 non-test lines.

Author the Basic suite for the connected initial scene, hover feedback, drag
pickup, a controlled in-flight position when it differs visibly from both
endpoints, committed placement, and fresh-engine restoration. Add the
sample-owned visual-state enum from its authored hover and drag input branches
and persistent placement outcomes. Make presentation and visual action
transitions match that enum exhaustively. Reuse the sample's ordinary scene and
Rust rules; delete no legacy fixture yet.

**Black-box acceptance:** every interaction uses UUID or normalized production
input, the final placement is asserted from visible Battlement state, every
scenario starts from a fresh engine, and the coverage ledger has no Basic gap.

**Screenshots:** every stable Basic state named by the ledger at 1280 by 720.

### Task 42 — Cover the Tic-Tac-Toe sample completely

**Prerequisites:** Task 41. **Target:** 150–225 non-test lines.

Author deterministic scenarios for the empty board, human move, delayed AI
response, every distinct terminal outcome represented by the sample's game
state, and restart. Add a finite visual-state enum over the board and terminal
state families and make presentation and visual action transitions match it
exhaustively. Use native semantic setup fixtures only where reaching an endgame
through public input would obscure the state under test; all visible
transitions still use normal player input.

**Black-box acceptance:** marks, turn ownership, input gating, AI response,
terminal state, and reset are asserted through public state; seeds are stable;
the coverage ledger has no Tic-Tac-Toe gap or redundant visual-only assertion.

**Screenshots:** each stable board and terminal result plus the restored board.

### Task 43 — Cover the Chess sample completely

**Prerequisites:** Task 42. **Target:** 200–300 non-test lines.

Author deterministic scenarios for the title or start state, initial board,
selection and legal targets, committed move, AI response, capture, every
special move and terminal result represented by the sample's public game and
presentation states, paused or recoverable failure, refresh, and restart. Add a
finite visual-state enum over those presentation families and make presentation
and public visual action transitions match it exhaustively. Use named semantic
fixtures for deep positions while preserving production selection and move
input.

**Black-box acceptance:** piece counts, selection, legal reachability, movement,
AI state, capture, audio-independent visible feedback, persistence boundaries,
and terminal results use public behavior; the ledger records concrete reasons
for any conditional state the sample does not expose.

**Screenshots:** every stable Chess state in the completed ledger.

### Task 44 — Cover the UI lab's foundations and style pages

**Prerequisites:** Task 43. **Target:** 200–300 non-test lines.

Author scenarios and checkpoints for Components, Interactions, Hierarchy,
Assets, Layout, Appearance, Backgrounds, Transforms, and Typography. Exercise
each state-changing control listed by the UI feature ledger, capture restored
states where applicable, and reuse that ledger to prove the visual catalog and
Ditto ledger agree.

**Black-box acceptance:** navigation and interactions go through public UI
input; word budgets, text floors, contrast roles, asset loads, state
restoration, and every mapped element or style assertion pass; no page or
stable variant is missing from either ledger.

**Screenshots:** initial and materially changed states for all nine pages.

### Task 45 — Cover the UI lab's control pages

**Prerequisites:** Task 44. **Target:** 200–300 non-test lines.

Add complete scenarios for Buttons, Containers, Scroll, Tabs, Text Fields,
Boolean Controls, Choice Groups, Dropdowns, Sliders, Ranges, Parts, and Complex
Parts. Capture stable control states without replacing value, event, ordering,
or settlement assertions with pixels.

**Black-box acceptance:** each control uses real click, drag, key, focus,
scroll, and selection paths; controlled writes do not echo; every reversible
sample action returns to its exact initial visible and behavioral state; both
ledgers remain complete.

**Screenshots:** initial and materially changed states for all twelve pages.

### Task 46 — Cover the UI lab's event and render-mode pages

**Prerequisites:** Task 45. **Target:** 200–300 non-test lines.

Complete Pointer Routing, Keyboard and Navigation, Remaining Events, Actions,
Render Modes, World Space, and Coverage scenarios. Exercise production event
routing and action paths, target-texture output, screen-space composition,
world-space input, and every stable coverage summary state.

**Black-box acceptance:** routed phase, focus, keyboard, action, render-target,
world-space, and restoration facts are asserted independently of screenshots;
expected unsupported platform capabilities skip before setup; the UI feature
ledger and Ditto ledger have no uncovered capability or page.

**Screenshots:** every stable state across the final seven UI pages, including
target-texture and world-space output.

### Task 47 — Cover Reactant composition, events, and state completely

**Prerequisites:** Task 46. **Target:** 200–300 non-test lines.

Author scenarios for every stable state on Composition, Events and Portals,
and State and Identity. Include keyed reorder, logical event routing, portal
placement, ordinary state changes, and exact restoration through public UI
input. Discover screen names through the ledger so later sample expansion
cannot silently bypass coverage.

**Black-box acceptance:** visible tree identity, physical portal placement,
logical routing order, keyed state retention, remount reset, and restoration
are asserted through the public engine and `battlement-fake` before image
comparison.

**Screenshots:** every initial, changed, reordered, and restored state on these
screens.

### Task 48 — Finish Reactant coverage and accept all sample baselines

**Prerequisites:** Task 47. **Target:** 200–300 non-test lines.

Cover Context and Memo, Effects and Stores, and every additional Reactant
screen in Reactant's checked screen inventory, including resources, boundaries,
refs, geometry, or world projection when present. Run one unfiltered update per
canonical macOS profile, inspect every proposal for intended state, complete
content, legible rendering, correct dimensions, and absence of unintended
overlays, then publish the complete content-addressed baseline manifest for all
five samples.

**Black-box acceptance:** context, memo bailout, effects, store swaps, async
states, geometry, error recovery, and restoration use deterministic public
behavior where present. The convention-based sample discovery and both feature
ledgers report every sample, screen, state, scenario, checkpoint, test owner,
and baseline exactly once.

**Screenshots:** the complete five-sample accepted baseline set, generated
coverage report, accepted lock digest, and review handoff used for approval.

## Wave 7: performance, CI migration, and release

### Task 49 — Build reproducible performance measurement

**Prerequisites:** Task 48. **Target:** 150–225 non-test lines.

Create a fixed ordinary-run benchmark containing exactly 20 representative
scenarios and 40 screenshots at 1280 by 720. Drive it through public Ditto
interfaces and a previously compiled local build. Record source hashing, cold
launch, warm watch execution, setup, settle, capture, baseline read, ODiff,
reset, durability, CPU, and peak memory without subtracting unreported time.

Pin the benchmark lane in Tollgate configuration to its designated Apple
silicon host class. Record hardware model, CPU count, memory, macOS, Unity,
Rust, ODiff, filesystem, power mode, and competing-load policy with every run.
The lane holds the Unity capacity lease and admits no concurrent Ditto or Unity
job during a measurement.

Also measure every complete sample suite separately with its scenario and
checkpoint counts. Keep build, baseline download, and Simulator boot outside
the normative execution budgets but visible in the report.

**Black-box acceptance:** repeated inputs produce the same scenario order,
checkpoint count, hashes, phase accounting, and result status; the harness
rejects missing phases, hidden exclusions, stale builds, baseline downloads,
or a benchmark whose exact 20/40 shape changes unintentionally.

**Evidence:** machine-readable benchmark and per-sample timing reports from the
designated CI-class host.

### Task 50 — Meet the fingerprint and execution budgets

**Prerequisites:** Task 49. **Target:** measured optimization slices.

Run the source fingerprint benchmark and the fixed 20/40 benchmark from cold
player launch and from a warm player, HTTP session, and ODiff server. Profile
the actual critical path and optimize only measured costs while preserving
fresh engines, two quiet frames, exact PNGs, durable logs, and exact comparison.
Do not weaken coverage, tolerance, diagnostics, or phase reporting.

For cold repetitions, reuse the compiled build and hydrated baselines but start
a new player, HTTP session, and ODiff server. For warm repetitions, reuse those
three processes through watch cycles. Neither form runs a build, downloads a
baseline, boots Simulator, clears an operating-system cache, or includes
unrelated load.

**Black-box acceptance:** each of 20 source-hash repetitions is at most 250
milliseconds; each of 10 cold-launch repetitions is at most 20 seconds; each
of 20 warm repetitions is at most 5 seconds. Retain minimum, median, 95th
percentile, and maximum phase timings so one lucky run cannot conceal a
regression. Every full sample suite also completes without a runtime,
infrastructure, missing-baseline, or ledger failure.

**Evidence:** before-and-after profiles plus the passing fixed-budget report.

### Task 51 — Run Ditto beside legacy CI without cutting over

**Prerequisites:** Task 50. **Target:** 150–225 non-test lines.

Add the five macOS sample suites in parallel Tollgate slots, focused WebGL and
iOS adapter smokes, failed-run-directory artifact upload, cached immutable
build reuse, benchmark result retention, and default-branch baseline
publication. Keep the legacy player smoke and visual-capture checks active
during this shadow interval.

Run the complete shadow matrix on both supported macOS host architectures.
Adapter and performance checks remain on their applicable hosts, but neither
architecture may reach cutover with an untested native capture binary.

**Black-box acceptance:** clean and dirty cache runs use the intended builds;
all five suites fail CI on visual, assertion, runtime, logging, baseline, or
infrastructure problems; adapter smokes exercise startup, input, settling, PNG,
comparison, runtime failure, and logs; failed jobs upload the complete local
run directory without publishing private diagnostics to R2.

**Evidence:** ten consecutive complete shadow CI results, including at least two
cache-cold and two cache-warm executions and retained reports from both host
architectures, with the normative performance budgets still passing and no
suite or adapter retry or unexplained outcome.

### Task 52 — Perform the conditional atomic CI cutover

**Prerequisites:** Task 51 and passing performance evidence from Task 50.
**Target:** 200–300 non-test lines.

Begin only when the retained benchmark proves the 250 millisecond fingerprint,
20 second cold, and 5 second warm targets and Task 51 has ten clean shadow runs
plus native capture evidence from both host architectures. Move the generic
Unity editor lease to neutral tooling, switch CI and active guidance to Ditto,
require Ditto evidence for player-affecting Tollgate worktree tasks, and remove
the old visual-capture document, scripts, tests, build methods, assets,
assembly references, fixtures, and smoke in the same change.

If the gate does not pass, do not partially migrate CI or delete the old
workflow. Leave the cutover task incomplete and return to measured optimization
under Task 50.

**Black-box acceptance:** repository-wide searches find no active legacy
capture reference; the five macOS suites and both adapter smokes replace the
old checks; documentation-only tasks use the explicit not-applicable handoff;
player-visible tasks use the exact Ditto handoff; default-branch publication
and failed-run artifact retention work from a clean checkout.

The cutover test scans every tracked text file for the removed script names,
`VisualCapture` namespaces and types, editor build methods, legacy asset paths,
and documented commands. Its explicit allowlist may contain only negative test
fixtures whose purpose is to prove those strings are rejected; it may not
allow active code, documentation, configuration, or assembly references.

**Evidence:** atomic cutover diff, clean-checkout CI result, and passing
performance report linked to the migrated revision.

### Task 53 — Complete release documentation and validation

**Prerequisites:** Task 52. **Target:** 150–225 non-test lines.

Finish user documentation for installation, suites, fragments, inputs,
assertions, baselines, review, watch, storage, diagnostics, results,
performance, CI, and agent handoffs. Validate every example through the public
binaries and remove temporary fixtures. Run the complete automated and manual
matrices on both supported macOS host architectures before release.

**Black-box acceptance:** both CLI entry points, shared wire fixtures, five
macOS suites, WebGL and iOS smokes, store tests, review tests, watch tests,
performance budgets, documentation examples, and repository CI pass from the
final staged tree. No sample-specific C#, private testing API, legacy capture
artifact, or uncovered ledger state remains.

**Evidence:** final result paths, coverage report, platform reports, benchmark
report, and documentation example transcript.

## Completion criteria

Ditto is complete when all tasks are marked done and all of the following are
true:

- The normative design and public documentation match the implemented CLI,
  job, HTTP, storage, result, review, platform, and failure behavior.
- Basic, Tic-Tac-Toe, Chess, UI, and Reactant each have complete checked
  screenshot coverage and accepted canonical macOS baselines.
- Focused WebGL and iOS Simulator suites prove their adapters without requiring
  a full sample-platform cross-product.
- The fixed benchmark completes 20 scenarios and 40 screenshots within 20
  seconds from a cold player launch and 5 seconds warm, and source hashing stays
  within 250 milliseconds on the designated host.
- Shadow CI is reliable, the atomic cutover is complete, and no active command,
  documentation, assembly, fixture, script, test, or asset refers to the old
  capture workflow.
- A player-affecting Tollgate worktree task can use Ditto for its retained
  screenshot, logs, timings, and machine result without scraping terminal prose
  or opening the review application.

## Manual QA

The technical design contains additional failure-level checks. This final
project checklist is self-contained for release signoff. Run each item from a
clean checkout, save its named result or CI artifact, and attach those paths to
the release review.

1. On Apple silicon and Intel macOS, run `ditto doctor`, `ditto list`, and the
   same commands through `cargo battlement ditto`. Repeat one passing and one
   invalid invocation with `--json` and `--output`. Confirm stdout contains
   only the stable result, stderr starts with `DITTO_RUN_DIR`, both entry points
   agree, secrets are absent, and exit codes are `0`, `1`, `2`, or `130` as
   specified.
2. Run a checked suite, file fragment, and standard-input fragment with
   `capture`; run matching and mismatching checkpoints with `run`; run an
   assertion-only scenario; and interrupt a responsive run. Confirm capture
   performs no baseline IO, every reached step and log range is present, image
   failures remain distinct from infrastructure failures, and interruption
   finalizes within two seconds.
3. Run every canonical Basic, Tic-Tac-Toe, Chess, UI, and Reactant macOS suite.
   Confirm the generated report has no uncovered inventory key, unexpected
   skip, missing baseline, unavailable image, or orphan checkpoint. Inspect
   every accepted image for intended state, complete and legible content,
   correct 1280 by 720 dimensions, and absence of unintended overlays. Record
   the reviewed lock digest in the release handoff.
4. Open one passing run and one mismatch with `ditto review`. Exercise
   side-by-side, swipe, alpha, red mask, synchronized zoom and pan, integer
   pixels, coordinates, logs, timings, missing-image state, and offline use.
   Accept two checkpoints in one request, retry the request, then remove
   write credentials and confirm review remains usable while acceptance is
   disabled.
5. Run the WebGL adapter smoke through operating-system launch and its
   configured headless command. Exercise click, hover, drag, key, exact canvas
   PNG, passing comparison, responsive browser failure, supervised exit, and
   launch timeout. Confirm images contain only the Unity surface and no browser
   automation package participates in execution.
6. Run iOS Simulator adapter smokes in portrait and landscape. Exercise touch,
   key, settling, exact framebuffer PNG, passing comparison, responsive failure,
   application loss, and app-scoped logs. Confirm hover skips before setup,
   device facts come from Simulator, and no Ditto-owned device remains after
   normal or stale cleanup.
7. Record a bounded macOS and Simulator video with an intervening action and
   screenshot. Exercise automatic truncation and one FFmpeg failure. Confirm
   successful MP4s are 30 fps Unity-surface recordings, screenshot status is
   independent, successful raw inputs are deleted, failed raw inputs remain in
   diagnostics, and WebGL video skips before setup. Follow successful stop,
   automatic truncation, and failure with an ordinary step and confirm each
   resumes the scenario's original motion mode.
8. With filesystem and R2 stores, delete the hydration cache, run online, run
   `fetch --all`, disconnect, and run offline. Perform full and filtered
   updates, stale-lock acceptance, feature-branch replacement,
   default-branch publication, tombstone restoration, cleanup dry run, and
   applied cleanup. Confirm locks change atomically and no live object is
   deleted.
9. In watch mode, change a scenario, `ditto.lock`, and one build input, then
   force a replacement build failure and recovery. Confirm one review tab
   follows immutable execution and comparison-only cycles, the stale player
   never runs new source, NDJSON emits one result per cycle, and `--output`
   atomically names the latest completed cycle.
10. On the pinned performance lane, run 20 source-hash repetitions, 10 cold
    20-scenario/40-screenshot repetitions, and 20 warm repetitions with the
    Task 50 preparation rules. Confirm every observed maximum is within 250
    milliseconds, 20 seconds, and 5 seconds respectively, and inspect the
    per-sample and phase reports for hidden downloads, builds, or load.
11. Inspect ten consecutive shadow CI runs, including the cache-cold,
    cache-warm, Apple silicon, and Intel evidence. From a final clean checkout,
    run repository CI and the cutover reference scan. Confirm all five macOS
    suites and both adapter smokes replace the legacy workflow and the scan's
    only matches are approved negative fixtures.
