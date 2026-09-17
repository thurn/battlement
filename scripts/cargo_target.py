#!/usr/bin/env python3
"""Print CI's isolated Cargo target directory for a focused workspace check."""

from __future__ import annotations

import argparse
from pathlib import Path

import ci


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", nargs="?", default="Cargo.toml")
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
    print(ci.cargo_environment(workspace)["CARGO_TARGET_DIR"])


if __name__ == "__main__":
    main()
