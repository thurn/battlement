---
name: battlement-ci
description: Run required Battlement validation and diagnose CI failures using focused checks, retained logs, and exact Ditto replay inputs.
---

# Validate and diagnose

Run commands in the task's own worktree. `scripts/ci.py` is the aggregate
entrypoint; read its argument parser and `scripts/ci_steps.py` for active checks.
Use `rust-toolchain.toml` for the required toolchain, not a copied version number.

While editing, choose the smallest check covering the changed behavior:
`cargo test -p <crate>`, `cargo test --manifest-path samples/<sample>/rules/Cargo.toml`,
or an existing script test. Prefer black-box behavior and native Ditto for
player-visible changes; use `battlement-ditto` for suite selection and probes.

Before final validation, stage all intended files and run `./scripts/ci.py`.
Its metadata refresh requires staged changes. The entrypoint selects its narrow
trusted plan-only check only when every changed path matches the executable
allowlist; mixed or policy-bearing changes use the aggregate suite. Inspect and
stage any resulting intended metadata, and ensure the final source has valid
required evidence. Do not substitute a focused pass for the selected run.

Run CI once as a single execution and retain its session handle, log path,
exit status, and tested source identity. If observation times out, inspect that
same job; do not launch another copy or configure automatic restarts.

On failure:

1. Read the failing step's retained log and reproduce that check first.
2. Test the failing execution boundary cheaply, including inherited operation
   IDs or worker configuration when relevant, before another aggregate run.
3. Fix the cause, verify the focused check, stage the repair, then complete CI.
4. For a timing or infrastructure hypothesis, retain the failed evidence and
   permit one unchanged replay after the resource condition changes.

A second failure at the same boundary ends aggregate retries. Spend at most
fifteen further minutes on focused diagnosis, then make a concrete repair,
rollback, or committed blocked handoff. Count attempts and elapsed cost; do not
keep enlarging deadlines or waiting indefinitely for machine load to improve.
Keep ordinary resource waiting distinct from product failure.

CI logs live under `.logs/ci/`; `scripts/perf_report.py` and `.logs/reports/`
help investigate repeated work. Read only the relevant report or failing span.
Ditto results under `artifacts/ditto-ci/` include replay inputs; use
`scripts/ditto_ci.py replay` rather than reconstructing the environment by hand.

Keep full output in logs and report only the relevant failure and result.
Reuse evidence only while its inputs remain unchanged. Finish verification
before committing and immediately submitting the candidate under the root
workflow and global wt skill.
