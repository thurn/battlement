#!/usr/bin/env python3

"""Overlay the capture harness onto a C#-free sample and run packaged-player evidence."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

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
    with tempfile.TemporaryDirectory(prefix="battlement-sample-capture.") as temporary:
        temporary_root = Path(temporary)
        project = temporary_root / "project"
        subprocess.run(
            [
                "rsync",
                "-a",
                "--exclude",
                "Library",
                "--exclude",
                "Temp",
                "--exclude",
                "Logs",
                "--exclude",
                "Build",
                f"{sample_project}/",
                f"{project}/",
            ],
            check=True,
        )
        shutil.copytree(
            REPOSITORY_ROOT / "Assets/VisualCapture",
            project / "Assets/VisualCapture",
        )
        (project / "Assets/Editor").mkdir(exist_ok=True)
        for suffix in ("", ".meta"):
            shutil.copy2(
                REPOSITORY_ROOT / f"Assets/Editor/SampleVisualCaptureBuild.cs{suffix}",
                project / f"Assets/Editor/SampleVisualCaptureBuild.cs{suffix}",
            )
        shutil.copytree(
            REPOSITORY_ROOT / "Packages/com.battlement.client",
            project / "Packages/com.battlement.client",
        )
        manifest_path = project / "Packages/manifest.json"
        manifest = json.loads(manifest_path.read_text())
        manifest["dependencies"]["com.battlement.client"] = "file:com.battlement.client"
        manifest["dependencies"]["com.unity.modules.screencapture"] = "1.0.0"
        manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")
        rust_target = temporary_root / "rust-target"
        subprocess.run(
            [
                "cargo",
                "build",
                "--quiet",
                "--release",
                "--manifest-path",
                str(cargo_manifest),
                "--target-dir",
                str(rust_target),
            ],
            check=True,
        )
        command = [
            sys.executable,
            str(REPOSITORY_ROOT / "scripts/capture-visual-evidence.py"),
            "--project-root",
            str(project),
            "--plugin",
            str(rust_target / "release/libbattlement_rules.dylib"),
            "--transport",
            "native",
            "--build-method",
            "Battlement.Editor.SampleVisualCaptureBuild.Build",
            *capture_arguments,
        ]
        subprocess.run(command, cwd=REPOSITORY_ROOT, check=True)


if __name__ == "__main__":
    main()
