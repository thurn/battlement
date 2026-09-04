# Implementation workflow

[Plan and reading guide](../chess-ui-implementation-plan.md)

## Task and Reviewer Protocol

Each numbered migration is an independently promoted task. Work begins from the
certified release containing all earlier migrations and reviewer follow-ups.

A **`wt` worktree** is the isolated checkout created and owned through Tollgate
for one task. Initial candidate submission never grants promotion authority.

The base-task workflow is:

1. Create a fresh Tollgate-owned `wt` worktree from the current release. Record
   the page's requirements and evidence paths in the compact ledger below.
2. Implement the smallest representative specimen that exercises the page's
   hardest visual requirement. Pass the early visual gate below before broad
   validation, baseline acceptance, or demo preparation.
3. Complete the page, its resettable harness, **semantic fixture** containing
   its expected roles, names, states, and relationships, and focused tests.
   Apply the [architectural challenge](review-protocol.md#mandatory-architectural-challenge)
   before accepting extra application wiring. Target roughly 500 non-test lines or fewer, but do not use this
   target to reject a necessary framework redesign.
4. Complete the validation sequence below. It includes smoke and reset checks
   for all registered pages, affected visual captures, and `./scripts/ci.py`.
5. Prepare the final web demo before freezing the candidate. Every migration
   page is web-visible: use a verified-free non-default port and a named
   Cloudflare Quick Tunnel. Record both service identities, verify the public
   walkthrough, and keep them available through review. Apply the same rule
   to a follow-up that changes rendered behavior.
6. Confirm that the staged source matches the tested source and review artifacts,
   create one Conventional Commit, and immediately submit `tg candidate HEAD`.
   Hand off the exact candidate and evidence without polling speculative CI.
7. Obtain an explicit promotion mandate for the exact candidate. Stop only the
   recorded demo and tunnel services immediately before authorization.
8. After promotion, assign a fresh port-ergonomics reviewer.

### Early visual gate

Choose the smallest state that exposes the hardest in-scope paint or layout
requirement: for example, an active and inactive clipped tab with gradients,
shadows, and text. Capture the unchanged pinned source and a native Ditto
specimen at the documented logical size and device scale. Verify image
size, stage alignment, fonts, and state before interpreting differences.

Compare the complete in-scope crop using the existing geometry and color
tolerances. Retain the source, native image, difference image, measurements,
and every permitted mask with its owner. A few sampled pixels or a successful
comparison against a newly accepted native baseline do not prove source parity.
Resolve rendering failures before multiplying states or running broad suites.
An explanation of a renderer difference does not waive its tolerance; a
substitution requires explicit user approval. Do not present a known tolerance
failure as a completed page awaiting routine promotion.

Reuse the pinned reference build and captures while their source revision,
fonts, state, viewport, scale, and relevant rendering inputs remain unchanged.
Recapture when one of those inputs changes or the evidence is incomplete.
Additional states and integration captures remain required at final validation.

### Validation sequence

1. During implementation, run focused checks for changed behavior and native
   Ditto fragments for uncertain rendering. Prefer semantic and geometry
   assertions for interaction and layout claims. Use browser automation for
   the reference, public demo, or a specifically web-owned behavior.
2. After the early gate passes and implementation is stable, run the required
   native initial, changed, and reset cases and all registered-page smoke and
   reset checks. Recapture earlier pages affected by shared changes. Satisfy
   every applicable matrix in the [validation requirements](validation.md#automated-validation).
3. Review intentional baseline differences using the
   [Ditto baseline workflow](../../ditto.md#baselines-and-review). Update only
   affected checkpoints, inspect the lock diff, and verify with an ordinary
   comparison run. Baseline acceptance cannot replace the source comparison.
4. Stage all intended deliverables, including baseline metadata, and run
   `./scripts/ci.py` once against that stable state. Follow the
   [development validation guidance](../../development.md#validation-cadence)
   for failures and reruns; these efficiency rules do not remove required CI.
5. Build the final web demo once from those sources and verify the complete
   public walkthrough. Confirm build preparation leaves intended source and
   staged content unchanged. A subsequent edit invalidates the evidence for
   the inputs it changes: rerun affected checks, rebuild affected artifacts,
   and complete required CI against the final staged state before submission.

Keep successful evidence whose inputs are unchanged. Repeat a check or capture
only for changed inputs, a failure, or a specific unresolved requirement; record
that reason. Do not duplicate the entire native matrix through interactive web
QA. Documentation-only revisions require documentation checks and repository
CI, but no new player build, screenshots, demo, or port-ergonomics review unless
they also change rendered behavior or a numbered migration's implementation.

### Compact evidence ledger

Keep one short Markdown attachment outside tracked source, with links to full
artifacts rather than copied logs. Update it at meaningful state changes:

- Exact worktree, base revision, pinned reference revision, and current source
  commit; before commit, identify the tested staged tree and any later edits.
- Requirement, proving check or capture, result, and unresolved next action.
- Command and selected scenarios, input identity, run ID, exit status, and paths
  to logs, machine results, reference images, native images, masks, and diffs.
- Live process or session handle, service identity, working directory, port,
  log path, and exact cleanup action; record terminal status after completion.
- Submitted candidate ID, source and tested OIDs, and queue revision.

Read relevant file sections once and retain their paths. Return concise results,
metrics, and failure excerpts from tools; keep full logs, JSON, and image bytes
in artifact files. Inspect rendered images when appearance changes or evidence
is uncertain, rather than repeatedly displaying the same capture.

On continuation, use the ledger to locate evidence, then verify current Git
state and any outstanding process handle. Preserve valid results; do not
restart a live run because an observation timed out. Poll boundedly with
backoff, and report meaningful changes rather than unchanged status snapshots.
