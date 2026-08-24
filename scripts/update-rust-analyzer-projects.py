#!/usr/bin/env python3

"""Keep VS Code's rust-analyzer projects aligned with sample workspaces."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from ci import REPOSITORY_ROOT, sample_rust_workspaces


SETTINGS_PATH = REPOSITORY_ROOT / ".vscode/settings.json"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail instead of updating settings when the project list is stale",
    )
    arguments = parser.parse_args()
    settings = json.loads(SETTINGS_PATH.read_text())
    projects = [
        "Cargo.toml",
        *(path.as_posix() for path in sample_rust_workspaces()),
    ]
    if settings.get("rust-analyzer.linkedProjects") == projects:
        return
    if arguments.check:
        raise SystemExit(
            "rust-analyzer projects are stale; run "
            "scripts/update-rust-analyzer-projects.py"
        )
    settings["rust-analyzer.linkedProjects"] = projects
    SETTINGS_PATH.write_text(f"{json.dumps(settings, indent=2)}\n")


if __name__ == "__main__":
    main()
