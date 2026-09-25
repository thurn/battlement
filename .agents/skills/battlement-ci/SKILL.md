---
name: battlement-ci
description: Run required Battlement validation and diagnose CI failures using focused checks, retained logs, and exact Ditto replay inputs.
---

# Validate and diagnose

Run commands in the task's own worktree. `scripts/ci.py` is the aggregate
entrypoint; read its argument parser and `scripts/ci_steps.py` for active checks.
Use `rust-toolchain.toml` for the required toolchain, not a copied version number.

While editing, choose the smallest check covering the changed behavior. For Cargo
checks, set `CARGO_TARGET_DIR` from `python3 scripts/cargo_target.py` (pass the
sample manifest for standalone workspaces). This warms CI's same isolated target
instead of compiling dependencies twice. Run `cargo test -p <crate>`, a sample
manifest's tests, or an existing script test. Prefer black-box behavior and native Ditto for
player-visible changes; use `battlement-ditto` for suite selection and probes.
After the first relevant focused pass, record `focused.passed` with
`scripts/workflow_event.py`; record `review.ready` only when the actual review
artifact or inspectable change is ready. Tollgate supplies later candidate,
certificate, promotion, and synchronization milestones to the performance
report. Record only observed boundaries; missing evidence remains unknown.

Before final validation, stage all intended files and run
`scripts/ci_job.py start`. Keep its returned job ID; use `status` or bounded
`wait --after <revision>` calls to resume the same operation after a timeout.
The handle records source identity and refuses to attach after inputs change.
Use `scripts/ci.py` directly only inside another supervisor such as Tollgate.
Its metadata refresh requires staged changes. The entrypoint selects its narrow
trusted plan-only check only when every changed path matches the executable
allowlist; mixed or policy-bearing changes use the aggregate suite. Inspect and
stage any resulting intended metadata, and ensure the final source has valid
required evidence. Do not substitute a focused pass for the selected run.

Retain each CI job handle and exact source; resume it after observation timeouts
instead of launching duplicates. Cancel through the handle to protect unrelated PIDs.

For CI repair, this continuation policy replaces fixed retry and diagnosis-time
limits in general worktree guidance. Retain timings and replay evidence, isolate
the expensive or failing boundary, and continue in-scope diagnosis and repair.
Use focused checks to test a concrete hypothesis before another aggregate run;
repeat unchanged validation only when evidence supports a changed condition.
Repeated failures call for a different repair approach, not a retry-count or
elapsed-time cutoff for the task. A slow passing run or ordinary capacity wait
is work to investigate, not by itself an external blocker.

Optimize shared setup, caches, concurrency and test selection from measured
costs. Consolidate or remove redundant, obsolete or low-value tests when their
cost exceeds their protection; record the risk rationale and retained or
replacement coverage. Existing tests are not an immutable checklist. Preserve
required behavior contracts, fix real failures, and validate the revised suite;
do not hide failures or raise deadlines to manufacture a pass.

Continue authorized repair until validation meets the task's requirements. Stop
only for a concrete dependency that cannot be resolved within available access
or authorized scope, or an explicit user pause; record what external change is
needed. Missing a timing target or exhausting one hypothesis does not qualify.

Use `.logs/ci/` and `scripts/perf_report.py` for timings; retain full logs and
report relevant results. Replay `artifacts/ditto-ci/` evidence with
`scripts/ditto_ci.py replay`. Reuse evidence only while its inputs remain unchanged.
Finish verification before committing and submitting under the root delivery policy.
