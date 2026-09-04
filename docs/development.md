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
