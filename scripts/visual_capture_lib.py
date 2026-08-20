#!/usr/bin/env python3

"""Shared, side-effect-free helpers for the visual evidence scripts."""

from __future__ import annotations

import fcntl
import hashlib
import json
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


class SlotLease:
    """Hold one cross-process slot until the lease is closed."""

    def __init__(self, directory: Path, name: str, count: int) -> None:
        self.directory = directory
        self.name = name
        self.count = count
        self.file = None

    def acquire(self) -> "SlotLease":
        """Wait for and exclusively lock one named slot."""
        self.directory.mkdir(parents=True, exist_ok=True)
        while self.file is None:
            for index in range(self.count):
                candidate = (self.directory / f"{self.name}-{index}.lock").open("a+")
                try:
                    fcntl.flock(candidate, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    self.file = candidate
                    break
                except BlockingIOError:
                    candidate.close()
            if self.file is None:
                time.sleep(0.1)
        return self

    def close(self) -> None:
        """Release the held slot."""
        if self.file is not None:
            fcntl.flock(self.file, fcntl.LOCK_UN)
            self.file.close()
            self.file = None

    def __enter__(self) -> "SlotLease":
        return self.acquire()

    def __exit__(self, _type, _value, _traceback) -> None:
        self.close()


def write_capture_command(control: Path, command_id: int, command: dict) -> Path:
    """Atomically publish one sequenced command for an in-player capture."""
    command_directory = control / "commands"
    command_directory.mkdir(parents=True, exist_ok=True)
    path = command_directory / f"{command_id:06d}.json"
    temporary = path.with_suffix(".json.new")
    temporary.write_text(json.dumps({"commandId": command_id, **command}, indent=2) + "\n")
    temporary.replace(path)
    return path


def wait_for_capture_ack(
    control: Path, command_id: int, timeout: float, process_id: int | None = None
) -> dict:
    """Wait for one successful player command acknowledgement."""
    path = control / "acks" / f"{command_id:06d}.json"
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if process_id is not None:
            try:
                os.kill(process_id, 0)
            except ProcessLookupError as error:
                raise RuntimeError("Player exited before acknowledging a command.") from error
        try:
            acknowledgement = json.loads(path.read_text())
        except (FileNotFoundError, json.JSONDecodeError):
            time.sleep(0.05)
            continue
        if acknowledgement.get("commandId") != command_id:
            raise RuntimeError("Player acknowledged the wrong capture command.")
        if not acknowledgement.get("success"):
            raise RuntimeError(acknowledgement.get("error") or "Player capture command failed.")
        return acknowledgement
    raise TimeoutError(f"Capture command {command_id} timed out.")


def inspect_video(ffprobe: Path, path: Path) -> dict:
    """Return the first video stream and container duration from FFprobe."""
    output = subprocess.run(
        [
            str(ffprobe), "-v", "error", "-count_frames", "-select_streams", "v:0",
            "-show_entries", "stream=codec_name,width,height,r_frame_rate,nb_read_frames",
            "-show_entries", "format=duration", "-of", "json", str(path),
        ],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return json.loads(output)
