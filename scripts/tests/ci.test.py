#!/usr/bin/env python3
"""Verify that CI discovers every convention-based sample."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import tempfile


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("ci", REPOSITORY_ROOT / "scripts/ci.py")
assert SPEC and SPEC.loader
ci = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ci)


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="battlement-ci-test.") as temporary:
        root = Path(temporary)
        for name in ("tictactoe", "basic", "chess"):
            sample = root / "samples" / name
            sample.mkdir(parents=True)
            (sample / "sample.toml").write_text(f'executable = "{name}"\n')
        (root / "samples/incomplete").mkdir()

        ci.REPOSITORY_ROOT = root
        assert ci.sample_names() == ["basic", "chess", "tictactoe"]


if __name__ == "__main__":
    main()
