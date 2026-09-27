#!/usr/bin/env python3
"""Run a focused Cargo check in CI's leased warm target, or print a private target."""

from __future__ import annotations

import argparse
from pathlib import Path

import ci


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", nargs="?", default="Cargo.toml")
    parser.add_argument("--run", nargs=argparse.REMAINDER, help="Cargo command to run under a target lease")
    arguments = parser.parse_args()
    try:
        manifest = (ci.REPOSITORY_ROOT / arguments.manifest).resolve().relative_to(
            ci.REPOSITORY_ROOT.resolve()
        )
    except ValueError:
        parser.error("manifest must belong to this checkout")
    workspace = None if manifest == Path("Cargo.toml") else manifest
    if workspace is not None and workspace not in ci.sample_rust_workspaces():
        parser.error("select the root or a standalone sample Cargo.toml")
    if arguments.run is not None:
        if not arguments.run or arguments.run[0] != "cargo":
            parser.error("--run requires a cargo command")
        ci.run_cargo(workspace, arguments.run)
    else:
        print(ci.cargo_environment(workspace)["CARGO_TARGET_DIR"])


if __name__ == "__main__":
    main()
