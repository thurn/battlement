#!/usr/bin/env python3

"""Build or reuse a content-addressed Web sample for demos and screenshots."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile
import time

from platform_support import lock_file, user_cache_path
from resource_slots import unity_editor_lease


REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_CACHE_ROOT = Path(
    os.environ.get(
        "BATTLEMENT_WEB_DEMO_CACHE",
        user_cache_path("Battlement", "web-demos"),
    )
)
WEB_SHARED_INPUTS = (
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "Packages/com.battlement.client",
    "crates",
    "web/init.js",
)


def staged_fingerprint(sample: str, release: bool) -> str:
    """Fingerprint staged Web build inputs and the local build toolchain."""
    editor = unity_editor(sample)
    editor_metadata = editor.stat()
    pathspecs = (*WEB_SHARED_INPUTS, f"samples/{sample}")
    staged = subprocess.run(
        ["git", "ls-files", "--stage", "--", *pathspecs],
        cwd=REPOSITORY_ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    status = subprocess.run(
        [
            "git", "status", "--porcelain=v1", "-z", "--untracked-files=all",
            "--", *pathspecs,
        ],
        cwd=REPOSITORY_ROOT,
        check=True,
        capture_output=True,
    ).stdout
    if any(
        record.startswith(b"??") or len(record) < 2 or record[1:2] != b" "
        for record in status.split(b"\0")
        if record
    ):
        raise RuntimeError("Stage all Web demo inputs before preparing a reusable build.")
    identity = {
        "schema": 1,
        "sample": sample,
        "release": release,
        "staged": staged,
        "host": [platform.system(), platform.machine()],
        "editor": [str(editor.resolve()), editor_metadata.st_mtime_ns, editor_metadata.st_size],
        "cargo": command_version(["cargo", "--version"]),
        "rustc": command_version(["rustc", "-Vv"]),
    }
    return hashlib.sha256(
        json.dumps(identity, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()


def prepare(sample: str, release: bool, cache_root: Path) -> Path:
    """Materialize one exact cached Web build and return its local path."""
    validate_sample(sample)
    key = staged_fingerprint(sample, release)
    profile = "release" if release else "debug"
    output_name = "WebThreads"
    output = REPOSITORY_ROOT / "samples" / sample / "Build" / profile / output_name
    cached = cache_root / "entries" / sample / key / output_name
    lock = cache_root / "locks" / sample / f"{key}.lock"
    lock.parent.mkdir(parents=True, exist_ok=True)
    started = time.monotonic()
    with lock.open("a+") as lease:
        lock_file(lease)
        if not valid_web_build(cached):
            print(f"Web demo cache miss {key[:12]}; building {sample}", flush=True)
            with unity_editor_lease():
                subprocess.run(build_command(sample, release), cwd=REPOSITORY_ROOT, check=True)
            if not valid_web_build(output):
                raise RuntimeError(f"Web build is incomplete: {output}")
            publish_directory(output, cached, cache_root / "entries")
        else:
            print(f"Web demo cache hit {key[:12]}", flush=True)
        materialize_directory(cached, output)
    print(f"Prepared {output} in {time.monotonic() - started:.1f}s", flush=True)
    return output


def build_command(sample: str, release: bool) -> list[str]:
    command = [
        "cargo", "run", "--quiet", "-p", "battlement-cli", "--",
        "sample", "build", sample, "--web",
    ]
    if release:
        command.append("--release")
    return command


def publish_directory(source: Path, destination: Path, entries_root: Path) -> None:
    entries_root.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix="web-demo.", dir=entries_root))
    try:
        clone_directory(source, staging / destination.name)
        destination.parent.mkdir(parents=True, exist_ok=True)
        if not destination.exists():
            (staging / destination.name).rename(destination)
    finally:
        shutil.rmtree(staging, ignore_errors=True)


def materialize_directory(source: Path, destination: Path) -> None:
    if destination.exists():
        shutil.rmtree(destination)
    destination.parent.mkdir(parents=True, exist_ok=True)
    clone_directory(source, destination)


def clone_directory(source: Path, destination: Path) -> None:
    if platform.system() == "Darwin":
        subprocess.run(["cp", "-cR", str(source), str(destination)], check=True)
    else:
        shutil.copytree(source, destination)


def valid_web_build(path: Path) -> bool:
    wasm = tuple((path / "Build").glob("*.wasm")) + tuple(
        (path / "Build").glob("*.wasm.unityweb")
    )
    return (path / "index.html").is_file() and bool(wasm)


def unity_editor(sample: str) -> Path:
    version_file = REPOSITORY_ROOT / "samples" / sample / "ProjectSettings/ProjectVersion.txt"
    version = next(
        line.removeprefix("m_EditorVersion: ")
        for line in version_file.read_text(encoding="utf-8").splitlines()
        if line.startswith("m_EditorVersion: ")
    )
    if configured := os.environ.get("UNITY_EDITOR"):
        return Path(configured)
    if platform.system() == "Windows":
        program_files = Path(os.environ.get("PROGRAMFILES", "C:/Program Files"))
        return program_files / f"Unity/Hub/Editor/{version}/Editor/Unity.exe"
    return Path(f"/Applications/Unity/Hub/Editor/{version}/Unity.app/Contents/MacOS/Unity")


def command_version(command: list[str]) -> str:
    return subprocess.run(
        command,
        cwd=REPOSITORY_ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def validate_sample(sample: str) -> None:
    if not sample or any(character not in "abcdefghijklmnopqrstuvwxyz0123456789-" for character in sample):
        raise RuntimeError("Sample names may contain lowercase letters, numbers, and hyphens.")
    if not (REPOSITORY_ROOT / "samples" / sample / "sample.toml").is_file():
        raise RuntimeError(f"Unknown sample: {sample}")


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("sample")
    parser.add_argument("--release", action="store_true")
    parser.add_argument("--cache-root", type=Path, default=DEFAULT_CACHE_ROOT)
    return parser.parse_args()


if __name__ == "__main__":
    arguments = parse_arguments()
    try:
        prepare(
            arguments.sample,
            arguments.release,
            arguments.cache_root,
        )
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(error)
        raise SystemExit(1) from error
