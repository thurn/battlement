#!/usr/bin/env python3
"""Exercise the production IDBFS bridge with controlled transaction outcomes."""

from pathlib import Path
import subprocess


if __name__ == "__main__":
    subprocess.run(
        ["node", "--test", "web/tests/persistence.cjs"],
        cwd=Path(__file__).resolve().parents[2],
        check=True,
    )
