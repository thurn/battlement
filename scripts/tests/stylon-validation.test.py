#!/usr/bin/env python3
"""Verify the pinned Stylon CI boundary."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import sys
from types import SimpleNamespace
from unittest.mock import patch


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY_ROOT / "scripts"))
SPEC = importlib.util.spec_from_file_location(
    "stylon_validation", REPOSITORY_ROOT / "scripts/stylon_validation.py"
)
assert SPEC and SPEC.loader
stylon_validation = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(stylon_validation)


def main() -> None:
    configured = REPOSITORY_ROOT / "tools/stylon"
    calls = []

    def run(arguments, **options):
        calls.append((arguments, options))
        return SimpleNamespace(stdout="stylon 0.1.0\n")

    with (
        patch.dict("os.environ", {"BATTLEMENT_STYLON": str(configured)}, clear=False),
        patch.object(stylon_validation.subprocess, "run", side_effect=run),
    ):
        assert stylon_validation.stylon_executable() == configured
        stylon_validation.validate()

    assert calls[0][0] == [str(configured), "--version"]
    assert calls[0][1]["capture_output"] is True
    assert calls[1][0] == [str(configured), "--version"]
    assert calls[2][0] == [
        str(configured),
        "--format",
        "human",
        "samples/chess/rules",
    ]
    assert all(call[1]["cwd"] == REPOSITORY_ROOT for call in calls)
    assert all(call[1]["check"] is True for call in calls)

    with (
        patch.dict("os.environ", {"BATTLEMENT_STYLON": str(configured)}, clear=False),
        patch.object(
            stylon_validation.subprocess,
            "run",
            return_value=SimpleNamespace(stdout="stylon 9.9.9\n"),
        ),
    ):
        try:
            stylon_validation.stylon_executable()
        except RuntimeError as error:
            assert "Expected stylon 0.1.0" in str(error)
        else:
            raise AssertionError("an unpinned Stylon executable was accepted")


if __name__ == "__main__":
    main()
