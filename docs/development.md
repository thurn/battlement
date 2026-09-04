# Development setup

Battlement requires exactly Rust 1.98.1, including the Clippy and rustfmt
components shipped with that release. Install the complete pinned toolchain
with rustup:

```sh
rustup toolchain install 1.98.1 --profile minimal --component clippy --component rustfmt
```

Commands run from the repository use `rust-toolchain.toml`, so `rustc`, Cargo,
Clippy, and rustfmt resolve from Rust 1.98.1. Run the complete local validation
suite after staging all intended changes:

```sh
git add <changed-files>
./scripts/ci.py
```

The suite checks the active tools before any build work and fails when the
repository pin, declared MSRV, Tollgate command, or installed tools disagree.
Tollgate invokes the same Rust 1.98.1 toolchain for certified builds.

## Validation cadence

Use focused checks while editing. Once the change is stable, stage all intended
files and run `./scripts/ci.py`. Record the tested source identity, command,
exit status, and log path so a later continuation can reuse valid evidence.
Successful checks remain evidence only while their relevant inputs are unchanged.

On failure, inspect the failing check's retained output and reproduce that check
first. After a repair, verify the focused check, stage the repair, and complete
required CI against the final staged source. For a suspected timing flake,
retain the failed result and rerun the exact check without unrelated source
changes. An isolated pass does not replace the required successful CI run.
Avoid simultaneous expensive builds or browser sessions when investigating a
latency-sensitive failure.

Run CI as a single-run job and retain its session handle and exit status.
Reserve automatically restarting services for long-lived servers and tunnels;
a completed CI command must not silently start again. If CI needs a service
manager, configure a single execution with restart disabled. Recheck the same
live handle after an observation timeout instead of launching a duplicate.

Keep full output in log files and return only the failing check, relevant
excerpt, and result. Poll running jobs with bounded waits and backoff. Rebuild
review artifacts only when their inputs change, and prepare them before
committing and submitting the candidate for review.
