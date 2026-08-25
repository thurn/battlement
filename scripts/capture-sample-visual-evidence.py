#!/usr/bin/env python3

"""Overlay the capture harness onto a C#-free sample and run packaged-player evidence."""

from __future__ import annotations

import argparse
from pathlib import Path
import subprocess
import sys

from sample_validation import validate_runtime_ui_package, validate_sample_input_backend


REPOSITORY_ROOT = Path(__file__).resolve().parent.parent


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--sample-project", type=Path, required=True)
    parser.add_argument("--cargo-manifest", type=Path, required=True)
    arguments, capture_arguments = parser.parse_known_args()
    sample_project = (REPOSITORY_ROOT / arguments.sample_project).resolve()
    cargo_manifest = (REPOSITORY_ROOT / arguments.cargo_manifest).resolve()
    validate_sample_input_backend(sample_project)
    validate_runtime_ui_package(
        REPOSITORY_ROOT / "Packages/com.battlement.client",
        REPOSITORY_ROOT,
    )
    command = [
        sys.executable,
        str(REPOSITORY_ROOT / "scripts/capture-visual-evidence.py"),
        "--project-root",
        str(sample_project),
        "--sample-harness-root",
        str(REPOSITORY_ROOT),
        "--cargo-manifest",
        str(cargo_manifest),
        "--transport",
        "native",
        "--build-method",
        "Battlement.Editor.SampleVisualCaptureBuild.Build",
        *capture_arguments,
    ]
    subprocess.run(command, cwd=REPOSITORY_ROOT, check=True)


if __name__ == "__main__":
    main()
