#!/usr/bin/env python3

"""Validate and deterministically regenerate every checked-in Trox bundle."""

from __future__ import annotations

import os
from pathlib import Path
import shutil
import subprocess
import tempfile

from platform_support import executable_name, user_cache_path


REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
TROX_VERSION = "0.2.0"
CONFIGURATIONS = (
    (
        Path("samples/reactant"),
        (Path("rules/src"), Path("localization"), Path("trox.ron")),
        (
            Path("localization/en-US.csv"),
            Path("localization/en-US.trox.json"),
        ),
    ),
    (
        Path("samples/chess-ui"),
        (Path("rules/src"), Path("localization"), Path("trox.ron")),
        (
            Path("localization/en-US.csv"),
            Path("localization/en-US.trox.json"),
        ),
    ),
    (
        Path("crates/battlement-reactant/tests"),
        (Path("localization.rs"), Path("localization"), Path("trox.ron")),
        (
            Path("localization/en-US.csv"),
            Path("localization/fr.csv"),
            Path("localization/bundles/en-US.trox.json"),
            Path("localization/bundles/fr.trox.json"),
        ),
    ),
)


def trox_executable() -> Path:
    """Return a verified pinned Trox CLI, installing it into the tool cache if needed."""
    configured = os.environ.get("BATTLEMENT_TROX")
    if configured:
        executable = Path(configured)
    else:
        root = user_cache_path("Battlement", f"trox-cli-{TROX_VERSION}")
        executable = root / "bin" / executable_name("trox")
        if not executable.is_file():
            root.mkdir(parents=True, exist_ok=True)
            subprocess.run(
                [
                    "cargo",
                    "install",
                    "trox-cli",
                    "--version",
                    TROX_VERSION,
                    "--locked",
                    "--root",
                    str(root),
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
    if version != f"trox {TROX_VERSION}":
        raise RuntimeError(f"Expected trox {TROX_VERSION}, found {version!r}.")
    return executable


def copy_inputs(source: Path, destination: Path, inputs: tuple[Path, ...]) -> None:
    for relative in inputs:
        source_path = source / relative
        destination_path = destination / relative
        if source_path.is_dir():
            shutil.copytree(source_path, destination_path)
        else:
            destination_path.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source_path, destination_path)


def validate() -> None:
    """Regenerate in temporary trees and reject stale reports or bundles."""
    executable = trox_executable()
    stale: list[Path] = []
    with tempfile.TemporaryDirectory(prefix="battlement-trox-") as temporary:
        temporary_root = Path(temporary)
        for index, (relative_root, inputs, artifacts) in enumerate(CONFIGURATIONS):
            source = REPOSITORY_ROOT / relative_root
            destination = temporary_root / str(index) / relative_root
            copy_inputs(source, destination, inputs)
            config = destination / "trox.ron"
            for command in ("extract", "bundle", "check"):
                subprocess.run(
                    [str(executable), command, "--config", str(config), "--deny", "warnings"],
                    cwd=destination,
                    check=True,
                )
            for artifact in artifacts:
                if (source / artifact).read_bytes() != (destination / artifact).read_bytes():
                    stale.append(relative_root / artifact)
    if stale:
        formatted = "\n".join(str(path) for path in stale)
        raise RuntimeError(f"Trox generated artifacts are stale:\n{formatted}")


if __name__ == "__main__":
    validate()
