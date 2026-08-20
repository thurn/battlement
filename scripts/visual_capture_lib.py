#!/usr/bin/env python3

"""Shared, side-effect-free helpers for the visual evidence scripts."""

from __future__ import annotations

import hashlib
import os
from pathlib import Path
import re
import subprocess
import time


def is_nonnegative_number(value: str) -> bool:
    """Return whether value is an unsigned integer or decimal."""
    return re.fullmatch(r"[0-9]+(?:\.[0-9]+)?", value) is not None


def now() -> float:
    """Return the current wall-clock time with subsecond precision."""
    return time.time()


def wait_for_initial_hold(hold_started_at: float, hold_seconds: float) -> None:
    """Wait until the requested hold duration has elapsed."""
    while (remaining := hold_started_at + hold_seconds - now()) > 0:
        time.sleep(remaining)


def sha256_file(path: Path) -> str:
    """Return the SHA-256 digest of a file."""
    digest = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def project_fingerprint(
    root: Path,
    scene: str,
    scenario: str,
    transport: str,
    plugin: str = "",
) -> str:
    """Fingerprint all inputs that can affect a packaged visual capture."""
    digest = hashlib.sha256()
    for value in (scene, scenario, transport, plugin):
        digest.update(value.encode())
        digest.update(b"\n")

    paths: list[Path] = []
    excluded = {"Library", "Temp", "obj", "target"}
    for directory in ("Assets", "Packages", "ProjectSettings", "scripts", "crates"):
        base = root / directory
        if not base.exists():
            continue
        paths.extend(
            path
            for path in base.rglob("*")
            if path.is_file() and not excluded.intersection(path.relative_to(root).parts)
        )
    paths.extend(path for name in ("Cargo.toml", "Cargo.lock") if (path := root / name).is_file())

    for path in sorted(paths, key=lambda item: os.fsencode(item.relative_to(root))):
        relative = path.relative_to(root).as_posix()
        digest.update(relative.encode())
        digest.update(b"\0")
        digest.update(f"{sha256_file(path)}  {path}\n".encode())
    return digest.hexdigest()


def verify_png_dimensions(path: Path, expected_width: int, expected_height: int) -> bool:
    """Return whether a PNG has the expected pixel dimensions."""
    result = subprocess.run(
        ["sips", "-g", "pixelWidth", "-g", "pixelHeight", str(path)],
        check=True,
        capture_output=True,
        text=True,
    )
    dimensions = {
        key: int(value)
        for line in result.stdout.splitlines()
        if ": " in line
        for key, value in [line.strip().split(": ", 1)]
        if key in {"pixelWidth", "pixelHeight"}
    }
    return dimensions == {"pixelWidth": expected_width, "pixelHeight": expected_height}


def tracked_state(root: Path) -> str:
    """Return Git's complete porcelain status for a repository."""
    return subprocess.run(
        ["git", "-C", str(root), "status", "--porcelain=v1", "--untracked-files=all"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
