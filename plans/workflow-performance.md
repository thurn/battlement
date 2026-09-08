# Workflow performance recovery plan

Restore fast task completion before adding more workflow infrastructure.
Every optimization task must deliver a measured improvement on its own.
A slower intermediate system is not acceptable on the promise that later
optimizations will recover its cost.

This revision replaces the previous backlog. Completed work is removed from
its action items. Implementation remains paused; revising this document does
not authorize resuming it or change the current required validation policy.

## Grounding and existing capabilities

Read these owners when implementing the corresponding change:

- [CI entry point](../scripts/ci.py) and [cache](../scripts/ci_cache.py):
  required work and compiler ownership.
- [Browser selection](../scripts/web_selection.py),
  [contracts](../web/contracts.toml), and
  [browser execution](../scripts/web_compatibility.py): the new regression.
- [Web preparation](../scripts/prepare-web-demo.py) and
  [deployment](../scripts/deploy.py): build identity and publication checks.
- [Resource slots](../scripts/resource_slots.py),
  [Unity transactions](../scripts/unity_transaction.py), and
  [Ditto CI](../scripts/ditto_ci.py): capacity and execution isolation.
- [Operation events](../scripts/operation_log.py),
  [CI traces](../scripts/perf_log.py), and
  [performance reports](../scripts/perf_report.py): existing measurement.
- [Trusted gate configuration](../.tollgate/config.toml),
  [repository policy](../AGENTS.md), and
  [CI guidance](../.agents/skills/battlement-ci/SKILL.md): policy owners.

**Tollgate** certifies an exact integrated commit, promotes local `release`,
and synchronizes remote `master`. Its application is in `~/tollgate`.
Global workflow skills in `~/.llms/skills/` are separate policy owners.
Changes to either must be explicit, validated changes, not repository-local
instructions that pretend the installed policy has changed.

The starting revision is `3e578aeb24aacbe01e125c72431a67581c62788e`.
It already contains corrected timeline accounting, private operation and
process events, resource events, invocation-specific Ditto evidence, and
buildset-bound Tollgate artifact collection. Native review is already the
policy default; public demos require an explicit request. Browser contracts
and a complete-site pre-publication check also exist. Preserve useful parts;
do not schedule their implementation again. Event ingestion and producer
coverage are still incomplete, but are not prerequisites for recovery.

## What failed and what must change

The September 7 session made the performance project itself a source of load.
The final browser candidate's certified run lasted **30 minutes 41 seconds**.
Within it, Rust tests took 120.7 seconds, Unity Edit Mode took 51.5 seconds,
and the full native Ditto suite took 35.3 seconds. The successful local
browser stage alone took 526.6 seconds. These are separate observations, not
additive estimates of recoverable time.

The user observed nearly two hours to finish two pending tasks, roughly a
15-fold Web workflow regression, and slowdowns in concurrent tasks. Treat
those observations as a release failure. The exact multiplier and causal
share of each resource still need matched measurements; do not present them
as an established benchmark or use that uncertainty to dismiss the failure.

The implementation and execution exposed specific mistakes:

- **Coverage expanded before cost was controlled.** Editing the browser
  selector, transport, shared helper, or contract registry selects all six
  games. The candidate gate prepares and tests them, and local validation
  does so too. An infrastructure edit became a multi-game integration run.
- **Work was repeated at expensive boundaries.** Timeout diagnosis repeatedly
  returned to the aggregate pipeline. A focused successful replay did not
  provide a supported way to retain unaffected steps for the repaired run.
- **Warm work waited behind cold work.** The CI invocation lock encloses
  lint/test cache lookups. A cached result can wait for an unrelated writer.
  Web keys include the entire client package and `crates` tree; unrelated
  changes can invalidate all game builds.
- **Isolation was mistaken for capacity.** Independent worktrees and players
  protect correctness, but do not bound aggregate CPU, memory, or GPU use.
  Agents, gates, sample fan-out, editors, and browser contexts multiply load.
- **Timeouts replaced diagnosis.** Startup and capture limits were increased
  while the host was under pressure. One observation showed approximately
  63 GB used and 25 GB compressed. This suggests contention; it does not prove
  which task caused it. Larger deadlines do not create a faster workflow.
- **Execution ignored the project's outcome.** Two tasks remained in flight
  at the pause request. Frequent unchanged-status messages consumed attention
  while the critical path barely moved. Passing tests was treated as success
  without measuring whether the workflow had become worse.

These are end-to-end failures, not just a slow Tollgate step. Before the
final gate, local reruns, focused replays, late fixture repairs, resource
queues, and agent decisions consumed additional time. A valid job handle
prevented duplicate observation from becoming a new process, but did not
prevent the agent from scheduling another expensive aggregate later. The
wrap-up request became an unbounded repair project. Saving unfinished work
was incorrectly coupled to obtaining a green remote promotion.

The plan also put measurement infrastructure and new validation ahead of
removing known overhead. It allowed expensive intermediate changes without
an enforced regression budget. That ordering is replaced below.

## Delivery rule and measurement contract

Implement one bounded task at a time, in its own worktree. Finish its
validation, promotion, cleanup, and performance comparison before starting
another. Resume parallel implementation only after the mixed-workload
capacity checks below pass. A pause request stops new scope immediately;
report the exact committed and uncommitted inventory once.

Use retained traces and lightweight counters first. A benchmark harness must
not become another infrastructure project. Record queue and execution time,
cache identities, build count, peak child count, memory pressure, and outcomes.
Leave unavailable attribution unknown. Do not require complete process-tree
telemetry, a new report UI, or a job registry to fix an obvious extra build.

Use retained evidence to establish the pre-expansion and regressed baselines
once. Reproduce a missing baseline only for the affected boundary, in isolated
inputs without promoting historical commits or moving the release branch.
Do not run the entire historical workflow for each patch.

Compare three states: the pre-expansion behavior, the current regressed
revision above, and the proposed change. Use the same executable inputs and
required correctness claims. For pre-expansion comparisons, include any real
focused browser review that was actually required; do not compare no test
against full coverage and call the difference an optimization.

Use a small fixed workload:

- A prose-only plan edit.
- A browser transport/selector edit that does not change game bytes.
- One sample's browser input or resize defect.
- A native UI edit alongside an unrelated warm Rust validation.
- An unchanged validation requested while a different compiler target is busy.

For a performance-changing implementation, begin with one cold and three warm
paired trials of only the affected operation, outside the promotion queue.
Show each result and cache state; three trials do not establish a reliable
95th percentile. Run the mixed workload at concurrency two before enabling
any new mandatory heavy work. Test concurrency four only after two is stable,
once per capacity change, not after every documentation or helper edit.

For each actual delivery, record its one local-validation and one certified
integration path through remote synchronization. Do not create repeated
promotions as benchmark trials. Prose and policy edits use inspection and
small fixtures, not a performance matrix. Cap extra benchmark work at thirty
minutes per optimization; if confidence needs more, report uncertainty and
keep the new expensive behavior disabled rather than extending work silently.

Record these outcomes:

- Request-to-review-ready and request-to-remote latency, including queues.
- Wrap-up-request-to-committed-checkpoint and wrap-up-request-to-actual-stop.
- Time before submission: edits, focused checks, aggregate runs, review,
  resource waiting, repair decisions, and otherwise unattributed time.
- Tasks completed per machine-hour and latency of the neighboring task.
- Executed builds/tests, reuse, transferred/copied bytes, and failed attempts.
- Machine pressure and application responsiveness during the trial.

An optimization must improve its targeted metric. For paired operation and
neighbor trials, reject more than 10% regression in median latency or
throughput. For the single actual end-to-end delivery, compare elapsed time
with a retained comparable delivery and its declared absolute budget; reject
more than 10% regression, without claiming statistical confidence from one
observation. Record differences in scope and cache state. If no comparable
observation exists, use the absolute budget and collect ordinary subsequent
deliveries; do not manufacture repeated promotions to obtain a median.
A faster substep never excuses a slower request-to-stop outcome. If noise or
missing evidence prevents a conclusion, do not enable new mandatory heavy
work.
Retain failures and count all retries in the cost. Never retry a benchmark
until it looks good, loosen screenshot assertions, or relabel a test failure
as success. Each task states its rollback before implementation; revert a
regressing default instead of keeping it pending a future fix.

## 0. Make execution and wrap-up bounded

The first bounded commit changes the checkpoint and repair policy in its
existing owners. The next removes Web amplification in section 1. Integration
fixtures below accompany the tooling they exercise, not an omnibus policy
commit. Later numbered sections follow that recovery; split their independent
optimizations into separate measured changes.

Change how the agent delivers work before resuming expensive implementation.
Optimizing Tollgate cannot fix hours spent reaching submission. Apply this to
local verification, debugging, integration, and the final handoff as well.

**One active deliverable:** complete a bounded change through measurement and
remote synchronization before opening the next worktree. Do not leave a
second infrastructure task half-implemented while waiting for the first.
If a necessary fix appears, identify whether it repairs this deliverable or
creates another task; do not silently start a separate improvement. Limit
implementation to the smallest change needed for the intended benefit.

**Test the execution boundary early:** run new tooling tests inside a cheap
fixture with the actual CI environment and parent operation IDs before the
full pipeline. The telemetry tests passed alone but failed when nested under
CI; a sub-second integration fixture should expose that before Rust and Unity
validation. Test browser protocol, startup, and cleanup separately before
trying the full product interaction. Inspect one real representative result
early instead of discovering basic test-design defects in the final matrix.

**Bound repairs:** after one failure, identify the exact failed phase and
reproduce it in isolation. One unchanged replay is appropriate only with a
stated hypothesis or changed resource condition. A second failure at the same
boundary ends speculative full reruns. Spend at most fifteen further minutes
on focused diagnosis, then produce a concrete repair, rollback, or blocked
handoff. Do not increase deadlines repeatedly, wait vaguely for load to
improve, or call every new retry a final run. Record attempt count and
cumulative time, including discarded runs. A green eventual attempt does not
erase first-pass reliability or the cost paid by neighboring tasks.

**Make saving work independent of certification:** implement an explicit
checkpoint path in the workflow policy. On a wrap-up request, inventory all
owned worktrees, dirty paths, commits, submitted candidates, and live jobs.
Within five minutes, give the user one accurate inventory and stop new scope.
Target ten minutes to preserve every intended change in local Git commits,
without waiting for builds. Checkpoints are marked unvalidated and have no
promotion authority. They are consolidated into the final task commit when
ready; a checkpoint must never masquerade as a certificate.

The current global wt skill and repository commit policy couple a final
commit to successful local checks and immediate candidate submission. Update
those owners explicitly to permit the checkpoint path; do not pretend it is
supported today. Keep immediate candidate submission for a validated final
commit and preserve exact approval and certification for remote promotion.
An already-submitted immutable candidate is never edited in place. Before
changing its source, cancel that exact candidate and retain its old evidence;
a repaired final commit requires a new candidate and exact authorization under
the applicable mandate. If only observation stops, preserve the existing
candidate and job handles without resubmission.

Never checkpoint another owner's files, stage unrelated changes, or push a
worktree branch just to claim everything is remote.

For a request to finish remote synchronization, continue only the required
finite validation path for committed scope. If a new failure exceeds the
repair bound, retain the checkpoint and evidence, report the exact blocker
and remaining work, and stop the repair loop. Do not claim remote completion.
Actual stop means both scope has stopped and every owned live job has an
explicit disposition. Cancel unnecessary local builds and services, wait for
owned descendants to exit, and verify resource leases are released. Finish a
requested remote-sync job or explicitly report its retained handle and ongoing
resource use; do not silently detach and call the machine idle. Preserve any
job the user explicitly asked to keep running. Report agent stop and remaining
machine-work completion separately, counting the latter in total cost.

The user can decide whether to extend the investigation; a broad original
implementation mandate is not permission for unlimited wrap-up work.

**Measure the complete episode:** use the request timestamp and actual stop,
not only the final candidate's runtime. Explain intervals before submission
and show both parallel work and elapsed time without double counting. A
shorter gate with another hour of local preparation fails acceptance.

Acceptance:

- Two dirty owned worktrees can be checkpointed within ten minutes of a stop
  request while a build is busy. The user sees what is saved, what is remote,
  and what still needs validation; no new implementation task starts.
- A cheap nested-run fixture catches the telemetry environment failure before
  broad validation. A protocol fixture catches transport/cleanup failures
  without compiling a game.
- Repeated startup failures reach a diagnosis or committed, honestly blocked
  handoff within the repair bound, rather than another aggregate restart.
- Measure the same two-task finish workload before and after the changes.
  Target at least a 50% reduction in request-to-stop time, with the same
  requested completion state, and no regression in neighboring work. A quick
  local checkpoint cannot be compared to remote completion as a speedup.

## 1. Remove the mandatory Web amplification

The first code optimization must reduce work selected by ordinary Web tooling
changes. Do this before scheduler expansion, broader telemetry, or a new
review command. Keep the existing full-site check before publication.

Replace the all-sample fallback for harness changes with checks at the layer
that changed:

- Transport, session cleanup, and selector changes use fast protocol fixtures
  and a small real browser page through the configured Playwright service.
  They must not compile or load six Unity games to verify HTTP or selection.
- Server headers and encoding use small served assets and real browser decode
  assertions. Unity startup coverage uses one representative prepared player
  when the change affects that contract.
- A sample contract change selects that sample. Shared browser input/rendering
  changes select the smallest declared set covering distinct implementations.
  A canary means a representative real sample, with the covered risk stated.
- Registry changes validate selection rules and affected entries, rather than
  making any registry edit select every game automatically.
- Full six-game validation belongs to pre-publication or an explicit broad
  compatibility run. If an unknown platform change needs broad coverage for
  certification, expose that reason and estimated cost; never silently accept
  a narrower pass or silently make every tooling change such a change.

This deliberately reduces redundant per-candidate matrix coverage while
retaining focused browser correctness and publication coverage. It is a
policy change, not a cache trick. Change the trusted selection policy and
its tests together. A candidate must not be able to edit its selector or
registry to exempt itself; selector/policy edits require checks chosen by the
previously trusted policy. Ordinary agents cannot choose to skip current CI.

If representative coverage cannot be made bounded in this task, roll back
broad automatic matrix activation to the pre-expansion behavior and retain
explicit focused browser validation. Do not leave the expensive default in
place while building its replacement. State the temporary coverage boundary
in the policy owner, and preserve the pre-publication blocker.

Acceptance:

- A harness-only change performs zero Unity builds; its warm browser/tooling
  stage targets at most 60 seconds on the reference machine.
- A sample-only change builds/tests no unrelated sample.
- A broad compatibility run remains available and reports all six outcomes.
- A real affected browser defect and a broken pre-publication interaction
  still fail the correct check. No screenshot threshold changes are needed.
- Paired end-to-end measurements meet the regression contract above. A pass
  obtained merely by increasing startup deadlines is not acceptance.

## 2. Let cheap and unchanged work finish cheaply

Remove unnecessary work and lock waiting before adding concurrency. Deliver
cache access and the narrow prose path as separate, independently measured
commits; neither waits for a general dependency graph implementation.

For cache hits, validate immutable completed results before compiler ownership
is acquired. Pin artifacts against eviction, acquire a producer lease only on
a miss, and recheck after acquisition. Protect the actual mutable Cargo target
from concurrent writers even when their result keys differ. Two identical
misses produce one result; cancellation never publishes a passing marker.
Release writer ownership before independent test execution where supported.

Provide a trusted prose-only route through the existing CI/Tollgate entry
points. An allowlisted plan edit selects document formatting and link checks,
without Cargo, Unity, Ditto players, or browser games. Unknown files and mixed
changes retain their required checks. `AGENTS.md`, skills, configuration,
generators, and fixtures are not ordinary prose.

Tollgate currently requires a native evidence bundle from its monolithic full
step. The prose route therefore needs a trusted step/artifact contract that
explicitly records native validation as not selected. Do not manufacture an
empty successful Ditto archive or remove native evidence requirements from
code candidates. Keep exact source, authorization, certification, and push.

Acceptance:

- A valid warm hit finishes within two seconds of lookup while another target
  is busy, excluding process startup; it does not acquire that writer lock.
- A prose-only candidate targets at most 60 seconds for local checks and
  120 seconds from submission to remote on a healthy warm host. It can finish
  while a native build is queued, subject to normal promotion ordering.
- Concurrent misses, interrupted publication, and eviction preserve identity
  and correctness. Unknown dependency changes cannot reuse stale evidence.

## 3. Bound expensive work across all entry points

Use existing resource leases to protect the whole machine, including tasks
outside this plan. Worktree isolation and PID identity remain separate from
capacity admission. Do not parallelize sample preparation to hide latency
until a measured machine budget permits it.

Inventory real compiler, editor, player, and browser fan-out from manual
commands, local CI, and Tollgate. Admit expensive children at their execution
boundary. Bound Cargo jobs and native suite/browser concurrency together;
count processes rather than only top-level tasks. Preserve some capacity for
interactive use and cheap checks. Queue excess work fairly instead of letting
it thrash or start a product-readiness timer before admission.

Start with conservative limits derived from the paired workload. Use a shared
budget for overlapping heavy phases, with separate ownership locks for each
mutable target or Unity project. Pass leases through nested wrappers so a
child does not acquire its parent's capacity twice. Release capacity before
waiting for another resource; specify lock order and test cancellation.

Replace Tollgate's whole-script `unity` semaphore only after every child path
obeys the same admission boundary. Do not add a second scheduler service or
replace independent native execution with a blanket single-player lock.
Keep the private Unity index, transaction recovery, per-execution capture
exclusivity, and deterministic input contract intact.

Acceptance:

- Two mixed tasks improve or maintain aggregate throughput, and neither
  materially degrades a neighboring warm validation or interactive capture.
- Four-task testing queues excess work rather than creating pressure-driven
  startup failures. A queued job is visibly queued, not reported as failed.
- An admitted startup still has a finite deadline and can fail for a real
  product problem. Saturation does not trigger automatic rebuilds.
- Interruption frees only owned capacity and processes, including descendants;
  PID reuse and independent browser contexts remain safe.

## 4. Reuse prepared bytes and completed work

After recovery and admission are demonstrated, reduce actual build and replay
cost. Separate three identities: player build inputs, test inputs, and the
exact integrated commit certified by Tollgate. A new integration parent can
require new tests without requiring a new player build.

Replace the broad Web source key with explicit dependency closures, including
build scripts, generated assets, toolchain, profile, and runtime dependencies.
A browser harness edit must not invalidate unchanged player bytes. Relevant
builder changes must invalidate them. Keep conservative inputs for unknown
dependencies until an uncached comparison validates the narrower manifest.
Explain each miss by changed input category.

Serve immutable prepared outputs directly or through a lightweight site view;
avoid copying every sample tree into another temporary directory for each
check. Pin active outputs against pruning. Validate bytes and source identity
once at publication and verify them efficiently at consumption. Measure hash,
copy, download, startup, interaction, and capture costs separately.

For a failure, reproduce only its exact retained step first. After a repair,
reuse unaffected successful steps only when their input manifests still match.
The aggregate records reused and executed steps and returns one honest result.
Implement this in tooling and trusted policy, not by instructing an agent to
pretend a focused pass was a full gate. Local artifacts may be reusable;
local JSON claiming a pass is never a Tollgate certificate.

Move deterministic generation and cheap consistency checks ahead of expensive
work. Preparation must converge without changing semantic baselines or
inventing coverage. Freeze inputs and preserve unrelated staged changes.

Optimize browser startup only from measured costs: compression, serving,
initialization, and capture have different fixes. Correct the current mismatch
between prepared Development output and deployment's compressed-asset contract.
Use one intentional build profile; do not build twice to discover the mismatch.
Retain bounded per-phase diagnostics, not just a transport timeout.

Acceptance:

- A harness edit uses existing player bytes locally and in the trusted gate.
- Unrelated source changes preserve reusable artifacts; a relevant dependency,
  fixture, toolchain, or check definition invalidates the appropriate result.
- Repairing one browser contract does not rerun unrelated successful Rust or
  native work when trusted manifests establish unchanged inputs.
- A forged, partial, failed, or stale result cannot become certified evidence.
- A focused warm Web task is at least 50% faster than the regressed revision
  and no slower than the measured pre-expansion workflow for equivalent work.
  The recovery must also hold at concurrency two, not just on an idle host.

## 5. Remove observation and handoff overhead that remains measurable

Use current operation IDs, session handles, and Tollgate events. Extend their
readers only enough to expose the critical wait or duplicate execution. Keep
unassociated work visible. Do not restart timeline fixes or build a universal
telemetry ingestion system before a demonstrated need.

Return compact state changes and retained log links from existing status
commands. Observation timeouts preserve the original job identity. An agent
can resume after compaction without another build. Unchanged polls do not
require another full snapshot or repeated narration to the user.

Update guidance in its existing owners after the supported fast routes land.
Keep one local final validation and one exact certified validation, with
eligible reuse inside those claims. Do not require a third run for review or
because an unrelated release advanced. Retained native artifacts satisfy
native review; public demo services remain explicitly requested and owned.

Only add a richer viewer, public-demo handle, or model/effort benchmark if
remaining traces identify it as a significant cost. These are not prerequisites
or mandatory deliverables of recovery. Global skill changes require a small
cross-project check so ordinary Web applications retain appropriate review.

Acceptance: interrupt observation, resume the same run, obtain one terminal
result and cleanup, and show lower orchestration overhead without hiding
failures or claiming that unattributed turn time is model inference.

## Manual QA

Use real entry points and the fixed paired workload. Retain source, selected
checks and reasons, operation IDs, raw timings, outcomes, and artifact links.
Do not rerun this whole matrix for every small patch; each delivery exercises
its changed boundary, and final recovery runs the combined workload once.

1. Request wrap-up with two dirty owned worktrees and a busy build. Verify
   the checkpoint deadline, complete inventory, stopped scope expansion, and
   honest distinction between saved, validated, and remotely synchronized.
   Exercise a repeated failure and verify the repair bound.
2. Edit only this plan. Verify the trusted prose route performs no compiler,
   Unity, player, or browser-game work and synchronizes the certified commit.
3. Change browser transport cleanup. Verify fixtures and a small real page
   detect the fault, zero game builds occur, and another MCP context survives.
4. Break one sample's browser input. Verify its real contract fails, unrelated
   samples are not built, and a native pass cannot substitute for that result.
5. Break a shipped interaction and attempt publication. Verify the complete
   prepared-site check blocks publishing; no failed check is silently omitted.
6. Hold a compiler writer, request a warm hit, then launch two identical misses.
   Verify prompt reuse, one producer, correct pinning, and safe eviction.
7. Run the mixed workload at two tasks, then four after two passes. Observe
   neighboring latency, responsiveness, memory pressure, actual child counts,
   admission waits, and first-pass correctness. Compare every trial, not just
   the fastest result. Keep genuine product timeout failures visible.
8. Change a relevant input and an unrelated input separately. Verify accurate
   invalidation and reuse, including across a different integration parent.
   Interrupt generation/publication; no partial artifact may pass.
9. Repair a failing step. Verify supported reuse of unchanged successful work,
   exact new evidence for changed inputs, and rejection of forged certificates.
10. Interrupt the observer and cancel a separate owned job. Reattach to the
   original run, verify its identity and terminal result, and confirm unrelated
   players, processes, Git indexes, browser contexts, and worktrees survive.

Recovery is complete only when the paired workloads meet the stated budgets,
required correctness checks still catch their intended faults, and concurrent
tasks are no longer paying for this project's added infrastructure. A merged
commit, richer logs, or a green thirty-minute gate is not that outcome.
