#!/usr/bin/env python3

"""Validate the chess rules crate with the pinned Stylon formatter."""

from __future__ import annotations

import os
from pathlib import Path
import subprocess

from platform_support import executable_name, user_cache_path


REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
STYLON_VERSION = "0.1.0"
STYLON_REVISION = "5179c3a4a3cb8273127d18f8c223691094e10f79"
STYLON_REPOSITORY = "https://github.com/thurn/stylon.git"
TARGET = Path("samples/chess/rules")


def stylon_executable() -> Path:
    """Return the verified Stylon pin, installing it into the tool cache if needed."""
    configured = os.environ.get("BATTLEMENT_STYLON")
    if configured:
        executable = Path(configured)
    else:
        root = user_cache_path(
            "Battlement", f"stylon-{STYLON_VERSION}-{STYLON_REVISION[:12]}"
        )
        executable = root / "bin" / executable_name("stylon")
        if not executable.is_file():
            root.mkdir(parents=True, exist_ok=True)
            subprocess.run(
                [
                    "cargo",
                    "install",
                    "--git",
                    STYLON_REPOSITORY,
                    "--rev",
                    STYLON_REVISION,
                    "--locked",
                    "--root",
                    str(root),
                    "stylon",
                ],
                cwd=REPOSITORY_ROOT,
                check=True,
            )
    version = subprocess.run(
        [str(executable), "--version"],
        cwd=REPOSITORY_ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    if version != f"stylon {STYLON_VERSION}":
        raise RuntimeError(
            f"Expected stylon {STYLON_VERSION}, found {version!r}."
        )
    return executable


def validate() -> None:
    """Reject chess rules source that differs from Stylon's default style."""
    subprocess.run(
        [str(stylon_executable()), "--format", "human", str(TARGET)],
        cwd=REPOSITORY_ROOT,
        check=True,
    )


if __name__ == "__main__":
    validate()
