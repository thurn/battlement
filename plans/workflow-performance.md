# Workflow performance improvement plan

Make a working change reviewable quickly, and make the remaining validation
cost visible. Prefer focused native evidence for shared game behavior, reuse
verified build inputs, and coordinate scarce build resources across tasks.

The baseline is promoted commit `97fade4c8718a4aae906a2e64e9c9fecece21dd0`,
"fix: make Ditto activation deterministic." Its execution isolation is already
implemented. Re-establish measurements on that revision: historical native-lock
failures explain the investigation but do not predict remaining bottlenecks.

This document specifies future changes to tools and guidance. It does not
change today's required validation, promotion authority, or test contracts.
Proposed command options and event fields below are design examples, not
currently supported interfaces.

## Related information

- [Repository policy](../AGENTS.md): validation and promotion requirements.
- [CI guidance](../.agents/skills/battlement-ci/SKILL.md): focused checks,
  staged validation, and retained failure evidence.
- [Ditto guidance](../.agents/skills/battlement-ditto/SKILL.md): native
  scenarios,
  deterministic checkpoints, screenshots, and exact replay.
- [Web guidance](../.agents/skills/battlement-web/SKILL.md): the current
  local/public demo requirement and service lifecycle.
- [Build guidance](../.agents/skills/battlement-build/SKILL.md): generation,
  native players, and artifact identity.
- [Performance report](../scripts/perf_report.py),
  [source readers](../scripts/perf_sources.py), and
  [interval analysis](../scripts/perf_analysis.py): measurement implementation.
- [CI orchestration](../scripts/ci.py), [cache](../scripts/ci_cache.py), and
  [resource slots](../scripts/resource_slots.py): build scheduling boundaries.
- [Web preparation](../scripts/prepare-web-demo.py) and
  [Web serving](../scripts/serve_web.py): reusable browser review machinery.
- [Ditto CI](../scripts/ditto_ci.py): native preparation and suite execution.
- [Native execution ownership][native-execution]
  and [live contract validation][live-contract]: concurrency invariants.
- [Unity source transactions](../scripts/unity_transaction.py): private indexes
  and source restoration.

[live-contract]: ../crates/battlement-ditto/src/wire/lifecycle_validation.rs
[native-execution]: ../crates/battlement-ditto/src/native_execution.rs

The global skills at `~/.llms/skills/wt/SKILL.md` and
`~/.llms/skills/independent-review/SKILL.md` are additional policy owners.
They are outside this repository. Changes to them must be delivered separately
and checked for effects on other projects; a Battlement commit cannot update
those installed skills by itself.

**Tollgate** is the existing service that validates immutable candidates,
promotes certified commits to local `release`, and synchronizes remote
`master`. Its trusted configuration and application own certification and
promotion; this plan does not introduce a competing release controller.

## Landed concurrency contract

The promoted implementation supports independent native players without a
machine-wide player lock. Preserve these boundaries in later optimizations:

- Each `NativeExecution` has a unique identity and permits one active capture.
  Overlapping launches on that same identity remain an ownership error.
- Live players require the `ditto-v2` determinism contract. Semantic activation
  needs a delivery receipt and a later committed presentation. During Ditto
  control, the executor owns frame advancement and physical input is suppressed.
  Focus changes do not route input to another player; application suspension
  still interrupts activation. Native semantic coverage does not replace tests
  of physical pointer routing or browser-specific input when those are changed.
- `scripts/ditto_ci.py gate` already runs sample suites concurrently.
  Preparation still visits samples serially; build/cache/resource costs remain
  separate optimization opportunities, subject to measurement.
- CI artifacts now live under
  `artifacts/ditto-ci/executions/<invocation-id>/`. Top-level report files are
  atomically replaced latest-report aliases, not stable evidence identities.
- Unity transactions already snapshot a private Git index, pass it to child
  commands, and restore project sources against it. This isolates Git index
  operations; it does not make simultaneous writers to one project safe.

The existing concurrent gate tests use a substitute Ditto executable, and the
native capture overlap test uses a fixture launcher. Keep those fast regression
checks, but establish performance and end-to-end isolation with real native
players before tuning machine capacity. Do not infer those results from mocks.

The priority order below remains appropriate: this fix removes an execution
correctness bottleneck, but does not remove duplicate Web review, compiler
serialization, inaccurate performance accounting, or evidence-discovery gaps.

## Evidence and expected benefit

The September 7, 2026 investigation examined 30 recent Codex sessions and 90
CI invocations started that local day, through approximately 14:33 PDT.
The CI sample contained:

- 42 passes, 39 failures, six interruptions, and three unfinished runs.
- 176 accumulated minutes in failed CI runs.
- 72 accumulated minutes waiting for the shared compiler-cache lock;
  the longest individual wait was 9.4 minutes.
- 134 accumulated minutes testing Rust workspaces across 81 runs.
- 18 additional invocations with the same staged tree and CI mode,
  accumulating 43 minutes after the first invocation in each group.

These totals overlap across concurrent tasks. They are neither machine CPU
usage nor additive recoverable wall time. Failed runs sometimes caught real
bugs, and identical-tree checks can have different certification contexts.

The particle task demonstrated native rendering at 13:39, but public Web
review was still being checked at 14:36. The music task passed its native menu
check at 13:33; its public menu became visible at 14:01. Other CI and review
work overlapped these intervals, so they do not establish a pure Web cost.
They do establish that review infrastructure remained unfinished after useful
native evidence existed.

The determinism conversation also contained two substantial implementations:
one was promoted around 10:55, followed by a request for semantic input and
restored parallel execution. Measure separate work episodes instead of treating
all elapsed conversation time as one stalled implementation.

Private supporting files from the investigation are under the shared local
`.logs/reports/` directory:

- `20260907-analysis.json`: uncorrected report output for 30 sessions.
- `20260907-daily-metrics.json`: separately derived daily measurements.
- `20260907-audit-evidence.json`: selected transcript evidence and source paths.

These ignored files are optional supporting evidence, may be removed by
retention, and must not become implementation or test dependencies. The facts
needed to understand this plan are recorded above; do not commit raw sessions.

## Priority and implementation order

Deliver changes as bounded, independently valid tasks. Priority expresses
expected leverage and confidence, not a promised speedup. Avoid turning this
plan into one large infrastructure rewrite.

1. **P0: Fix performance logs.** First correct date selection, turn accounting,
   correlation, and classifications, and repair discovery of the new Ditto
   artifact paths. Add process/resource spans next. This enables trustworthy
   measurement of every following optimization.
2. **P0: Make native evidence the default review path.** Replace conflicting
   guidance immediately; do not wait for complete telemetry instrumentation.
   Use existing screenshots and recordings before building a new review UI.
3. **P1: Remove unnecessary compiler serialization.** Use corrected telemetry
   to establish a post-Ditto baseline. Deliver cache-hit fast paths, then
   resource scheduling, then narrower invalidation as separate changes.
4. **P1: Prepare and freeze inputs before expensive validation.** Deliver
   generation/preflight first, artifact invalidation explanations second, and
   conservative affected-check selection third.
5. **P2: Automate explicitly requested Web review.** Add prepared compressed
   output and reliable service/readiness handling, using the resource controls
   from item 3. Keep it out of ordinary native task completion.
6. **P2: Provide compact job status and completion events.** Extend existing
   job handles and telemetry; avoid a new general orchestration service.
7. **P2: Reuse validation evidence within certification boundaries.** Start
   with immutable local per-step cache results. Tollgate evidence integration
   is separate work and must retain its trust and exact-source guarantees.
8. **P3: Reduce integration and agent overhead where measured.** Make small
   replacements to workflow guidance and benchmark model/effort choices only
   after infrastructure delays can be separated from model time.

Items 1 and 2 can proceed independently. Items 3 and 4 can overlap when their
cache identity and resource interfaces are agreed. Item 7 depends on correct
input manifests from item 4. Do not hold the earlier gains for item 7.

After the Ditto change, independently verify that full gates, individual
suites, manual captures, replays, and CLI tests share its isolation contract.
Fix any missed entry point as a regression in that work. Preserve its existing
per-execution guard; do not recreate the former global native-player lock.

## 1. Fix performance logs

Extend the current reporting scripts and `perf_log.py`. The first useful
release must explain an ongoing task correctly, even before every producer
has detailed telemetry. Missing evidence must remain visibly unknown.

### Correct lifecycle and time accounting

The current report uses `completed_at or latest_event_at` as the task endpoint.
A task with a finished morning turn and unfinished afternoon work therefore
ends at the morning turn. The lifecycle parser also creates agent intervals
only on `task_complete`, losing unfinished and aborted turn coverage.

Required behavior:

- Support an explicit local date/time zone and an immutable observation cutoff.
  Include tasks and processes that started earlier but overlap the window.
- Close completed and aborted turns at their recorded terminal event. Represent
  open turns through the observation cutoff when their running state is known;
  otherwise end at the last observation and expose the unobserved tail.
- An earlier completed turn must never hide a later started turn. Handle
  interrupted/restarted turns and incomplete log tails without inventing time.
- Print actual completed, running, interrupted, and unknown counts.
- Separate conversation age, active turn intervals, between-turn gaps, explicit
  approval waits, and process execution. A between-turn gap alone does not
  prove that the user was being asked for approval.
- Use interval unions for elapsed coverage. Parent spans and child operations
  must not count the same time twice. Show summed parallel work separately.
- Keep actor activity and machine-job activity as separate views when they
  overlap. Do not invent a causal critical path from temporal overlap alone.
- Record lifecycle milestones with evidence: first focused pass, review-ready,
  candidate submission, authorization, certification, promotion, and sync.
  Unknown milestones stay unknown; an assistant saying "done" is not a pass.

Example proposed report invocation:

```sh
python3 scripts/perf_report.py --date 2026-09-07 \
  --timezone America/Los_Angeles --include-incomplete
```

An ongoing 80-minute turn with a background build must show the whole observed
turn, the build's actual interval, and overlap. It must not show a nine-minute
morning answer as the entire task.

### Correlate processes, resources, and outcomes

Instrument CI, native preparation, Web preparation, resource acquisition, and
review-service startup with one shared event writer. Carry identifiers through
subprocess environments rather than recovering ownership from branch names.

Each operation needs:

- Operation and parent IDs, repository identity, task/thread and turn IDs when
  present, plus candidate/buildset/attempt IDs supplied by Tollgate.
- Source tree or manifest digest, actual tested revision, toolchain identity,
  relevant configuration, and build profile.
- Start and terminal timestamps, monotonic duration, exit status, retained log
  path, and artifact identities. Identify the clock domain for monotonic time.
- Process identity robust to PID reuse and a containment/cleanup handle.
- Resource requested, capacity, owner, queued/acquired/released events, and
  separate queue and execution durations.
- Cache hit/miss/bypass, reason, changed input categories, and producer run.
- Distinct outcomes for product failure, infrastructure failure, cancellation,
  invalidated inputs, and successful reuse. Ordinary capacity waiting is not
  a failed test.

Reuse the IDs already emitted by Ditto: `DITTO_CI_INVOCATION_ID` identifies a
CI invocation, while `native_execution_id` identifies one native execution.
Link suite/run/job IDs as children rather than substituting one ID for all of
these scopes. A retry that executes work gets a new attempt identity; resuming
observation keeps the existing identity. Do not reuse a CI invocation ID to
write a second artifact of the same name; the current runner rejects that.

Resolve retained evidence through `DITTO_CI_ARTIFACT_ROOT` and the invocation
report's `invocation_id` and `artifact_root`. Update performance readers, replay
and review consumers, cleanup/retention, and Tollgate artifact discovery to
preserve this ownership. The repository configuration and observed trusted
configuration still use `artifacts/ditto-ci/*/result.json` and
`artifacts/ditto-ci/*/run.tar.gz`, which describe the former directory layout.
Replace those assumptions through the appropriate configuration owners; bind
collected files to the expected invocation instead of broadly ingesting every
old execution. A top-level `gate.json` alias alone cannot establish that
binding.

This evidence-discovery repair belongs in P0, before measuring gains or relying
on retained results. Test two simultaneous invocations in one checkout: both
reports and failure archives must remain independently discoverable after the
latest-report alias changes. Missing evidence must be visible, not a false pass.

Example event, with abbreviated identities for readability:

```json
{
  "event": "resource.acquired",
  "operation_id": "build-17",
  "task_id": "task-4",
  "resource": "unity-editor",
  "queue_duration_ms": 8700
}
```

Recognize wrapped tool calls and both `input` and `arguments` payloads. A shell
exit code of one must not become "passed" merely because a tool returned an
output object. Decode structured outcomes where available; otherwise report
unknown rather than interpreting arbitrary log text as a definitive result.

A poll observes an existing process; it is not a second build. Associate
`exec_command`, `write_stdin`, yielded cells, and terminal completion with one
job. Prefer producer events for actual job lifetime. For legacy logs, label
inferred associations and leave ambiguous jobs unassigned.

Keep unmatched CI runs in a machine-wide report with explicit association
coverage. They must not disappear because Codex metadata was absent in a
Tollgate worker. Deduplicate shared runs by stable operation ID when showing
multiple tasks.

### Make model time and report quality explicit

Retain the category "unattributed active-turn time" until actual request
telemetry exists. It includes generation, reasoning, transport, tool scheduling,
and observation gaps around asynchronous work; it is not pure inference.

Where Codex exposes request timing, distinguish request latency, retries,
compaction, generated tokens, and tool latency. Do not infer model performance
from cumulative input-token counts, most of which can be cached. Integration
must tolerate unavailable request telemetry without blocking local reporting.

Report repeated work by cause: source edit, generation, environment change,
certification context change, infrastructure interruption, or unexplained.
Repeated polling with unchanged status belongs in an orchestration metric,
not automatically in a list of expensive duplicate operations.

Use synthetic rollout/trace fixtures covering open turns, aborted turns,
overlapping agents, background jobs, missing terminal events, failed shell
commands, duplicate imports, and local-midnight boundaries. Assert exact known
intervals and attribution. Keep fixture-based regression tests independent of
private sessions and live Codex databases.

Protect source data: private permissions, redaction, bounded output, and
retention that preserves referenced evidence. Writing a report must not chmod
an arbitrary parent directory; the current custom-output path can attempt to
chmod `/tmp`. Apply private permissions to created report directories and the
file, not an existing caller-owned output parent.

## 2. Native validation and review by default

Shared Rust/Unity behavior and appearance should use native Ditto evidence.
Web validation is selected because of a browser-specific risk, not simply
because the same sample can also be built for a browser.

The policy must distinguish three outputs:

- Product validation: semantic behavior, deterministic screenshots, and
  controlled animation checkpoints.
- Human review: screenshots, reference comparisons, recordings, a native
  player, or a lightweight artifact viewer.
- Web compatibility: actual browser startup, browser input/audio restrictions,
  hosting headers, resize behavior, and platform-specific rendering.

A native pass does not prove Web compatibility. Conversely, a public tunnel
must not be required to demonstrate a native layout or animation repair.

### Selection rules

For an ordinary native/shared UI change:

1. Reproduce with an existing Ditto scenario or a temporary fragment.
2. Inspect one representative rendered state early, before a broad matrix.
3. Capture only distinct changed risks, including animation checkpoints where
   needed. Compare supplied references directly, not only existing baselines.
4. Provide retained native evidence and complete required aggregate validation.
5. Submit the candidate and hand off immediately under the existing workflow.

Run focused Web validation when the task changes browser integration or
addresses a browser-reported defect. Changes to shared renderer/shader code
with credible platform differences can also require Web coverage; document
that specific risk in the validation selection, not a vague "web-visible"
label. Preserve declared Web contracts in affected-check selection.

A public interactive demo is required only when explicitly requested. Local
browser validation does not imply a public tunnel. Validate hosting or public
URL behavior publicly when that is itself the requested behavior.

Make the integrated Web compatibility check a required pre-publication stage
of `scripts/deploy.py`, before the publishing command. The deployment workflow
owns it and tests the complete prepared site from the exact committed deployment
revision on a local server using the configured Playwright service. Exercise
startup and one declared representative interaction for each shipped sample;
keep the existing live smoke check after publication for hosting verification.

A failing pre-publication check blocks site publication. It does not block an
otherwise valid native candidate's certification or promotion. Per-change Web
checks selected for browser-specific behavior or declared platform risks remain
required before those candidates can pass their gate. Public interactive review
continues to be separately requested. Land this boundary and its executable
selection rules with the removal of the broad Web-review requirement.

### Review artifacts without another game build

Use Ditto's existing retained results and review command first. If review
friction remains, extend that viewer with source/run identity, expected/actual
images, differences, and recordings. A browser displaying those files does
not require a Web player or Playwright game walkthrough.

Make artifact references survive worktree cleanup: retain them in the existing
run/artifact store, and show a retention expiration when one applies. Store
static files or a small manifest; do not introduce another application stack
solely for review. Public sharing remains an explicit action.

Acceptance: a native Settings repair produces useful review evidence without
calling Web preparation, starting a browser game, or creating a tunnel. A
browser-resize defect still receives a real browser test at affected sizes.

## 3. Compiler caches and machine resource scheduling

Parallel isolated players still compete for CPU, memory, and GPU time. Build
writers also share mutable compiler targets and Unity projects. Coordinate
those resources independently of correctness isolation.

The observed CI takes one invocation lock around Rust lint and tests. Cache
hits wait behind misses, and root/sample fingerprints include broad source
sets. Optimize in this order:

- Check immutable valid cache results before taking a compiler writer lock.
  After acquiring a miss lock, recheck for a result another writer produced.
- Lock the actual shared mutable compiler target, not the entire CI invocation.
  Two different result keys using one Cargo target still require writer safety.
- Release compiler capacity after compilation where execution can safely be
  separated; do not hold it while unrelated native tests or Web downloads run.
- Make cache publication atomic. Coordinate eviction with active consumers so
  the cache-hit path cannot race pruning or read partially published artifacts.
- Narrow input manifests using dependency closure plus explicit fixture,
  build-script, configuration, and generated-asset inputs. Cargo dependencies
  alone do not describe tests that inspect repository files at runtime.
- Keep broader invalidation for unknown dependencies until a representative
  uncached comparison establishes safe selection.

Extend existing resource slots to cover real shared writers and bounded build
capacity across manual CLI invocations, CI, Web preparation, and Tollgate.
Acquire capacity at the execution boundary, not separately at every wrapper.
Nested calls must pass an existing lease or acquire in a consistent order to
avoid double acquisition and deadlock. Cancellation releases owned capacity;
stale-owner recovery checks process identity before reclaiming it.

Preserve exclusive access to a particular Unity project while the editor
mutates it. Preserve transactional source restoration and the private
`GIT_INDEX_FILE` passed to Unity transaction children. Do not clear or overwrite
that environment variable in new wrappers. Allow independent prepared players
to run concurrently under the landed Ditto contract. Same-execution capture
conflicts and invalid activation receipts remain errors, not capacity waits.
Do not replace the compiler bottleneck with a blanket native-player semaphore.

Measure concurrency levels of one, two, and four independent tasks before
choosing capacity. Count actual child players and compiler/editor processes:
one gate already fans out across samples, so four agent tasks are not merely
four players. Measure the still-serial preparation path separately before
parallelizing it. Increase throughput without exhausting memory or making
interactive review unusable. CPU/GPU utilization and memory pressure need
measurement; the old logs do not prove that more hardware is the first fix.

Inspect Tollgate's trusted resource configuration as part of this work. During
the investigation its single voting CI step held a named `unity` semaphore
for the entire script. Determine the semaphore's actual capacity and scope;
its name alone does not prove serialization. Narrow redundant outer leases
only after all child entry points enforce their own resource ownership.

Acceptance: a warm independent cache hit returns while another target is
compiling; concurrent misses for one identity publish one result; different
writers never corrupt a shared target; a native suite can run while an
unrelated compiler job proceeds within the configured resource budget.

## 4. Preparation, stable inputs, and affected checks

Extend existing generation/build/CI entry points with a preparation operation.
It should turn source declarations into a stable validation input manifest
before launching expensive work.

The preparation operation must:

- Resolve the selected sample and required generation from existing manifests.
- Generate assets/constants in dependency order and check that a second
  preparation pass makes no further changes. Fail with the non-converging
  producer identified instead of rebuilding until fingerprints stabilize.
- Check formatting, authoring constraints, scenario/coverage references, and
  metadata consistency early. Generate mechanical catalog entries from one
  authoritative declaration where possible.
- Preserve semantic decisions: never invent scenario coverage, mark a state
  covered because a file exists, or accept a screenshot baseline automatically.
- Return the exact intended generated diff for staging and a manifest of
  relevant source, tools, environment, and build inputs. Do not stage unrelated
  working-tree changes on the agent's behalf.
- Check that intended inputs are staged before validation and identify
  conflicting unstaged paths before consuming build capacity.

Build from the frozen inputs. Reuse Tollgate isolation or existing cached
build-project mechanisms where available; avoid cold-copying Unity projects
and rebuilding `Library` for ordinary checks. If a producer must read a live
worktree, detect source changes before accepting its output. Never attach a
result from changed inputs to the original identity.

Reuse the landed Unity transaction's private index and journal rather than
building another index-isolation layer. Freeze the intended source identity
before starting transactions; preserve their child environment and recovery
protocol. A private index snapshot does not freeze working-tree file contents.
Keep project-scoped writer ownership and verify source restoration on both
success and interruption without changing unrelated staged work.

Expose invalidation explanations at the artifact boundary:

```text
Native player unavailable for selected inputs.
Changed input: client runtime assembly
Reusable: generated paint, unchanged asset bundles
Required: player rebuild
```

Do not recommend restarting every build merely because a branch advanced.
Tollgate owns integration reconstruction. Rebase a task for an actual
dependency,
conflict, or explicit user instruction; a newer unrelated release alone is not
reason to repeat a completed local evidence collection.

### Conservative affected-check selection

A separate tooling change should derive required checks from the diff and
explicit dependency manifests. Keep one selector shared by local CI and the
trusted gate, with the chosen checks and reasons retained as evidence.

- Prose-only changes may use document/link/format checks if they cannot affect
  executable configuration, generated inputs, fixtures, or agent policy.
- Skills and `AGENTS.md` are behavior-bearing guidance, not automatically
  ordinary prose. Validate their references and workflow examples explicitly.
- Rust/UI/runtime changes select the affected checks and declared cross-layer
  contracts. Unknown inputs fall back to the broader gate.
- Selection must not be editable by an untrusted candidate to remove its own
  checks. Tollgate's trusted policy validates the required set.

The current successful `./scripts/ci.py` requirement remains until this
selector and its trusted integration land. Do not tell agents to skip required
CI based on their own assessment that a patch is small.

Acceptance: a scenario inventory omission fails before Unity compilation;
preparation reaches stable generated inputs; editing a frozen build's inputs
cannot produce a falsely valid result; a prose-only plan can eventually take
an explicitly supported narrow path without bypassing certification.

## 5. Efficient Web review when required

Improve `prepare-web-demo.py` and `serve_web.py` rather than teaching agents to
assemble a different service setup for every task. The existing Web cache
already uses staged fingerprints and Unity editor slots; build on it.

- Publish compressed runtime/assets with verified content types and encoding.
  Test that browser requests decode them correctly. Select debug/release
  intentionally; do not build debug, discover a large download, and rebuild
  release automatically without a measured reason.
- Include relevant generator/build/serving tool changes in cache identities.
  Explain hits, misses, and invalidation; retain exact source and build profile.
- Use the shared resource budget. Prevent native and Web editors from racing
  on the same project, even when launched by different wrappers.
- Return one structured review handle with build identity, port, service
  labels, logs, readiness, optional public URL, and exact cleanup action.
- Distinguish server-ready, assets-loaded, player-initialized, and the requested
  interaction passing. A responding HTTP server or canvas is insufficient.
- Use bounded readiness checks with actionable network/console diagnostics.
  Retain a failed startup instead of silently rebuilding or restarting forever.
- Start a tunnel only for requested public review. Cleanup must target the
  owned services and preserve other tasks' browsers, servers, and players.
- Continue using the configured Playwright MCP service; do not launch separate
  browser automation processes or request shared browser contexts.

Acceptance: a required Web review uses one prepared build, one local
walkthrough,
and only a requested public walkthrough. Slow downloads are visible separately
from build time and browser interaction. Service cleanup works after
cancellation
and promotion, and a public-demo failure does not masquerade as a native
failure.

## 6. Compact status and completion events

Agents currently spend many tool calls reading quiet logs or large Tollgate
snapshots. Give them a stable operation handle and bounded, structured status.
This also prevents observation timeouts from becoming duplicate invocations.

Extend existing process/CI handles with states such as queued, preparing,
running, passed, failed, canceled, and inputs-invalidated. Include the current
step, resource wait, last progress, source identity, and relevant failure path.

```json
{
  "job_id": "ci-42",
  "state": "queued",
  "waiting_for": "cargo-target:root",
  "source_tree": "abbreviated-digest",
  "revision": 12
}
```

Provide status changes after a cursor and a bounded wait for terminal state or
required action. Reuse native Tollgate status/events where supported; adapt
existing tools before adding a new API. Do not claim proposed interfaces are
available until command help and an executable test establish them.

Observation timeout returns the same job handle. It does not restart the job.
An active-job registry must check real process identity and source identity
before attaching to an existing operation. State survives agent compaction;
job cancellation must not terminate an unrelated process after PID reuse.

Default status output should summarize current work, not dump all historical
candidates and events. Full evidence remains available by explicit selector.
Do not expose full transcripts, credentials, or enormous nested tool output
in every poll. Completion events must support concise progress communication
without requiring repeated model decisions while a job is unchanged.

Acceptance: disconnect or compact the observing agent during a long build,
then resume observation of the original operation. Verify one invocation,
one terminal result, and no leaked child processes.

## 7. Reuse evidence without weakening Tollgate

A local pass and a certified integration pass are different claims. Reuse
work only when the complete inputs and trust requirements match.

First improve existing local per-step caches. Reused results need producer
identity, immutable artifacts, input manifest, toolchain/configuration identity,
and successful completion. Never reuse failed, interrupted, or partially
published outputs as passing evidence.

For Tollgate, make a separate integration change through its application and
trusted configuration. A task-written JSON file saying "passed" is not a
certificate. Tollgate must either rerun the check or independently validate an
eligible trusted result and bind it to the exact tested candidate/generation.

Distinguish compile artifacts from test evidence. Compilation can be reused
while tests rerun. A changed integration parent may leave compile artifacts
usable but require new integration checks. Changes to environment, selected
checks, or toolchain invalidate the relevant evidence even with identical
source text.

Do not use `tg check` as a replacement for candidate submission: the current
workflow explicitly distinguishes independent checks from promotable evidence.
Do not bypass authorization, manually move `release`, or push task branches.

Acceptance: unchanged valid inputs reuse eligible work; a changed dependency,
check definition, or environment forces the relevant validation; forged or
stale evidence is rejected; remote synchronization uses only certified state.

## 8. Guidance revisions and agent behavior

Replace conflicting rules in their existing homes. Put algorithms, capacities,
selection rules, and event fields in tools/configuration, with short links from
skills. Do not append this plan as another agent checklist.

### `AGENTS.md`

Replace the current unconditional "web-visible features" demo requirement with
an explicit native-default selection rule. Suggested replacement:

> Use native Ditto evidence for shared game behavior and appearance. Run Web
> validation for browser-specific changes or declared platform risks. Create a
> public demo only when explicitly requested; use battlement-web for its
> lifecycle.

Keep the successful aggregate CI requirement until item 4's trusted selector
exists. Then point to the selector instead of asking agents to manually choose
whether the gate applies. Keep worktree ownership, explicit promotion authority,
local-only task branches, and the existing single-review policy.

Do not add reminders to batch searches or avoid overthinking. Those are ordinary
agent practices, not project-specific optimization infrastructure.

### `battlement-ditto` and `battlement-build`

Keep native-first validation and the deterministic execution invariant. Update
any stale exclusive-player assumptions after the parallelism change. Point to
machine resource controls for capacity, not to manual pauses of other tasks.

Document the prepared-input command once available and link to its help. Teach
one representative early capture and exact replay; preserve the distinction
between controlled animation checkpoints and arbitrary settling delays.

Use retained review artifacts that survive cleanup. Do not require another
native or Web build solely to copy evidence into the handoff. Link to the live
contract and invocation-specific artifacts; retain historical result readability
without accepting an old live player or historical report as current evidence.

### `battlement-web`

Narrow the trigger to selected Web validation and requested public demos.
Separate local validation from public sharing; remove the requirement that
both URLs be exercised for every visual change. Preserve browser correctness
checks, configured Playwright use, and exact service cleanup.

Once item 5 lands, replace manual service recipes with the review handle's
prepare/status/stop workflow. Keep details in command help and implementation.

### `battlement-ci`

Point to corrected performance reports and structured jobs. Reattach to the
same active operation for the same task, inputs, and validation purpose instead
of starting a duplicate. This is not a global ban on independent invocations
with identical source; certification contexts may differ. Use focused diagnosis
before a repair rerun. Make resource waiting visibly different from product
failure; do not prescribe
"retry until green" or tell users to pause unrelated tasks.

Point to preparation and affected-check selection when implemented. Remove
redundant validation instructions from other skills rather than copying the
new rules into every skill.

### Global `wt` skill

Replace "leave a running demo server" as a universal handoff requirement with
"provide the review artifact appropriate to the target." Keep browser service
instructions conditional on an actual requested/required browser demo. Native
screenshots, recordings, or a reviewed native player satisfy a native handoff.

Keep local evidence complete before freezing the candidate. Keep immediate
submission without promotion authority, followed by the review handoff without
waiting for speculative CI. Preserve the durable promotion mandate for in-scope
repairs and exact candidate approval. Do not add another approval checkpoint.

Keep Tollgate responsible for integration freshness. Remove any instruction
that would encourage rebasing simply because unrelated work advanced release.

### Global `independent-review` and design skills

Preserve the existing review threshold and at-most-one review policy; review
waits were not the dominant measured bottleneck. Update validation references
to the supported aggregate/affected-check entry point when available, rather
than requiring redundant full runs around a read-only review.

For large implementation plans, require an early assembled-product check at a
real integration boundary. Specimen/gallery success alone did not expose the
late music-ownership and render-stack problems in the chess work. Put concrete
integration scenarios in the relevant plan, not a new universal matrix in
`AGENTS.md`.

Benchmark model and effort choices on comparable bounded tasks only after
item 1 separates model time from background work. Record correctness, repair
iterations, review-ready time, and output/compaction volume. Do not promise
savings from lower reasoning effort without evidence.

Global skill edits need a cross-project check: ordinary browser applications
must still receive browser review, while native and prose-only tasks must not
inherit an irrelevant server requirement. This plan schedules those edits;
it does not silently modify globally installed skills with a repository patch.

## Measurement and completion criteria

Use the landed parallel-Ditto revision as the comparison baseline. Run the same
representative changes at concurrency one, two, and four, including warm and
cold build cases. Compare like-for-like hardware, source, toolchain, and cache
state. Repeat a small fixed set of performance trials; never retry correctness
failures away or use looser screenshot thresholds to improve results.

The representative set should include a native layout repair, an animation
change, a Rust-only change, a cross-layer rendering change requiring Web
coverage, and a prose-only plan. Report results by class and sample count;
small-sample percentiles must be labeled as such.

Primary outcomes:

- Time to review-ready, excluding explicit user deliberation but including
  required evidence preparation.
- Time from approval to certified remote synchronization.
- Completed tasks per machine-hour at each concurrency level.
- Required validation first-pass rate, separated by failure cause.
- Queue time by resource, build time, scenario time, and Web review time.
- Duplicate executions, artifact invalidations, cache-hit latency, and report
  association coverage.

Provisional performance targets, to confirm against the corrected baseline:

- Zero automatic Web builds or public tunnels for native-only review tasks.
- Zero product-test failures caused solely by ordinary resource saturation.
- A valid warm cache hit does not wait behind an unrelated compiler writer.
- At least a 50% reduction in compiler queue time for the fixed four-task warm
  workload, without cache corruption or increased correctness failures.
- At least a 30% reduction in median native-task review-ready time for the
  representative set, with no required coverage removed.
- Every newly instrumented local operation has a source and task/job identity;
  external unassociated operations are displayed, never silently omitted.
- Correct synthetic timeline fixtures and visible accounting gaps are release
  requirements, regardless of speedup.

The percentages are engineering targets, not conclusions drawn from the old
logs. Publish actual results and retain a change only if its intended benefit
or correctness improvement is demonstrated. If safe concurrency regresses,
reduce capacity or revert the scheduling change, not the isolation contract.
Do not create an indefinite monitoring automation as part of this plan.

## Manual QA

Perform these scenarios against the completed tools and guidance. Capture the
selected validation plan, operation IDs, resulting report, and artifact links.
Use intentional faults only in isolated test inputs.

1. **Native layout repair:** complete a small Settings change. Verify native
   evidence and required CI are sufficient; no Web build, Playwright game
   session, or tunnel is launched. Review artifacts survive worktree cleanup.
2. **Browser-specific defect:** reproduce a browser resize/startup issue.
   Verify a real focused browser check is selected. A native screenshot alone
   must not satisfy it; a public tunnel is unnecessary unless requested.
3. **Requested public demo:** prepare once, exercise the requested interaction,
   then stop the owned services. Verify compressed responses, readiness stages,
   and no remaining task-rooted processes or occupied ports.
4. **Independent parallel runs:** start two separate CI processes plus a manual
   Ditto capture and an exact replay. Verify isolated inputs/capture/cleanup,
   bounded capacity, and no false test failure on saturation. Use real players;
   include simultaneous gates in the same checkout. Verify distinct invocation
   roots and native execution IDs, and retrieve each run's reports and archives
   after the latest-report alias changes. Tollgate must retain the corresponding
   evidence. Confirm a second capture on one execution ID still fails.
5. **Cache reuse under load:** hold one compiler target busy, request a valid
   independent cache hit, and launch two misses for the same identity. Verify
   prompt hit completion, one producer, and atomic result publication.
6. **Input invalidation:** change a relevant generated/runtime input during a
   build. Verify refusal or an isolated original snapshot, an exact reason,
   and no passing result attached to the wrong source.
7. **Early bookkeeping failure:** omit a required scenario catalog entry.
   Verify actionable preflight failure before Unity starts; correcting it
   cannot silently accept a screenshot baseline or fabricate coverage.
8. **Agent interruption:** stop observation during compilation and resume it
   after compaction. Verify attachment to one original job and reliable final
   status. Cancel a different job and verify unrelated processes survive.
9. **Timeline accuracy:** load fixtures with a morning completed turn, an open
   afternoon turn, aborted work, parallel children, and an overnight process.
   Verify correct date clipping, status counts, overlap, and unknown time.
10. **Missing or failed evidence:** remove a trace tail and include a shell
    failure with a nonempty output. Verify partial/failed classifications and
    visible association gaps. Save to a custom directory without changing its
    existing permissions.
11. **Certification:** exercise trusted reuse, changed integration inputs, and
    forged local evidence. Verify only eligible results are reused and exact
    authorization, promotion, and synchronization remain mandatory.
12. **Guidance selection:** have a fresh agent follow the revised instructions
    for native, browser, public-demo, and documentation tasks. Verify each
    selects the intended tool path without inventing a mandatory demo or
    bypassing required validation. Check global skill behavior in another
    browser-oriented project before installing the revised global guidance.
13. **Deterministic native ownership:** run independent players while changing
    window focus and generating unrelated physical input. Verify semantic
    delivery receipts, later committed presentations, controlled simulation
    time, and isolated captures. Application suspension must remain an explicit
    failure. An old live determinism contract must be rejected; reading an old
    retained report must not qualify it as current passing evidence.
14. **Unity transaction isolation:** run transactions for different projects,
    interrupt one, and verify each uses its private index and restores its own
    source state. Unrelated staged changes remain intact. Concurrent writers
    to the same project must still acquire project-scoped exclusive ownership.
