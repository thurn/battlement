#!/usr/bin/env python3

"""Durable, exclusively leased build workspaces for visual capture."""

from __future__ import annotations

from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import time

from platform_support import try_lock_file, unlock_file


SLOT_SCHEMA = 2
SLOT_COUNT = 2
PRESERVED_DIRECTORIES = ("Library", "target")
TRANSIENT_DIRECTORIES = ("Build", "Logs", "Temp", "artifacts", "build", "obj")
SYNC_EXCLUDES = (
    ".git",
    ".worktrees",
    "Build",
    "Library",
    "Logs",
    "Temp",
    "artifacts",
    "build",
    "obj",
    "target",
)


def source_project_identity(project_root: Path) -> str:
    """Return a worktree-independent identity for one source project."""
    try:
        repository_root = Path(
            subprocess.run(
                ["git", "-C", str(project_root), "rev-parse", "--show-toplevel"],
                check=True,
                capture_output=True,
                text=True,
            ).stdout.strip()
        )
        common_directory = Path(
            subprocess.run(
                ["git", "-C", str(project_root), "rev-parse", "--git-common-dir"],
                check=True,
                capture_output=True,
                text=True,
            ).stdout.strip()
        )
        if not common_directory.is_absolute():
            common_directory = repository_root / common_directory
        identity = f"git:{common_directory.resolve()}:{project_root.resolve().relative_to(repository_root.resolve())}"
    except (subprocess.CalledProcessError, ValueError):
        identity = f"path:{project_root.resolve()}"
    return hashlib.sha256(identity.encode()).hexdigest()


def compatibility_manifest(
    project_root: Path,
    unity_version: str,
    layout: str,
    harness_root: Path | None = None,
) -> dict[str, str | int]:
    """Describe inputs that make imported Unity and Cargo state compatible."""
    manifest: dict[str, str | int] = {
        "schema": SLOT_SCHEMA,
        "sourceProject": source_project_identity(project_root),
        "unity": unity_version,
        "hostSystem": platform.system(),
        "hostArchitecture": platform.machine(),
        "buildTarget": (
            "StandaloneWindows64" if platform.system() == "Windows" else "StandaloneOSX"
        ),
        "layout": layout,
    }
    if harness_root is not None:
        manifest["harnessSource"] = source_project_identity(harness_root)
    return manifest


def compatibility_key(manifest: dict[str, str | int]) -> str:
    """Return the stable directory key for a compatibility manifest."""
    encoded = json.dumps(manifest, sort_keys=True, separators=(",", ":")).encode()
    digest = hashlib.sha256(encoded).hexdigest()
    return digest[:16] if platform.system() == "Windows" else digest


def remove_owned_path(path: Path, parent: Path) -> None:
    """Remove one validated child path without accepting a cache root."""
    resolved_path = path.resolve()
    resolved_parent = parent.resolve()
    if resolved_path == resolved_parent or resolved_parent not in resolved_path.parents:
        raise ValueError(f"Refusing to remove path outside owned parent: {path}")
    if path.is_dir() and not path.is_symlink():
        shutil.rmtree(path)
    elif path.exists() or path.is_symlink():
        path.unlink()


def _rsync(source: Path, destination: Path, extra: tuple[str, ...] = ()) -> None:
    if shutil.which("rsync") is None:
        _copy_tree(
            source,
            destination,
            tuple(
                value.removeprefix("/")
                for option, value in zip(extra[::2], extra[1::2])
                if option == "--exclude"
            ),
        )
        return
    destination.mkdir(parents=True, exist_ok=True)
    command = ["rsync", "-a", "--checksum", "--delete"]
    for excluded in SYNC_EXCLUDES:
        command.extend(("--exclude", excluded))
    command.extend(extra)
    command.extend((f"{source}/", f"{destination}/"))
    subprocess.run(command, check=True)


def sync_standard_project(source: Path, destination: Path) -> None:
    """Mirror a complete source project while retaining accelerator directories."""
    _clean_transients(destination)
    _rsync(source, destination)


def sync_sample_project(
    source: Path,
    harness: Path,
    destination: Path,
    materialized_repository: Path,
) -> None:
    """Mirror a sample plus the capture harness into one durable project."""
    _clean_transients(destination)
    _rsync(
        source,
        destination,
        (
            "--exclude", "/Assets/VisualCapture",
            "--exclude", "/Assets/Editor",
            "--exclude", "/Packages/com.battlement.client",
            "--exclude", "/Packages/manifest.json",
        ),
    )
    _rsync(harness / "Assets/VisualCapture", destination / "Assets/VisualCapture")
    _rsync(
        harness / "Packages/com.battlement.client",
        destination / "Packages/com.battlement.client",
    )
    _rsync(harness / "crates", materialized_repository / "crates")
    for name in ("Cargo.toml", "Cargo.lock"):
        source_file = harness / name
        destination_file = materialized_repository / name
        if not destination_file.is_file() or source_file.read_bytes() != destination_file.read_bytes():
            shutil.copy2(source_file, destination_file)
    editor = destination / "Assets/Editor"
    editor.mkdir(parents=True, exist_ok=True)
    expected_editor_files = {
        f"SampleVisualCaptureBuild.cs{suffix}" for suffix in ("", ".meta")
    }
    for child in editor.iterdir():
        if child.name not in expected_editor_files:
            remove_owned_path(child, editor)
    for suffix in ("", ".meta"):
        source_file = harness / f"Assets/Editor/SampleVisualCaptureBuild.cs{suffix}"
        destination_file = editor / source_file.name
        if not destination_file.is_file() or source_file.read_bytes() != destination_file.read_bytes():
            shutil.copy2(source_file, destination_file)
    manifest = json.loads((source / "Packages/manifest.json").read_text(encoding="utf-8"))
    manifest["dependencies"]["com.battlement.client"] = "file:com.battlement.client"
    manifest["dependencies"]["com.unity.modules.screencapture"] = "1.0.0"
    manifest_text = json.dumps(manifest, indent=2) + "\n"
    manifest_path = destination / "Packages/manifest.json"
    if not manifest_path.is_file() or manifest_path.read_text(encoding="utf-8") != manifest_text:
        manifest_path.write_text(manifest_text)


class FileLease:
    """Hold one exclusive non-blocking file lock."""

    def __init__(self, path: Path) -> None:
        self.path = path
        self.file = None

    def try_acquire(self) -> bool:
        """Acquire the lease if it is available."""
        self.path.parent.mkdir(parents=True, exist_ok=True)
        candidate = self.path.open("a+")
        if not try_lock_file(candidate):
            candidate.close()
            return False
        self.file = candidate
        return True

    def close(self) -> None:
        """Release the lease."""
        if self.file is None:
            return
        unlock_file(self.file)
        self.file.close()
        self.file = None


@dataclass
class BuildSlot:
    """One exclusively leased durable build workspace."""

    path: Path
    project: Path
    lease: FileLease
    disposition: str
    compatibility: dict[str, str | int]

    def close(self) -> None:
        """Release the slot without deleting its incremental state."""
        self.lease.close()

    def __enter__(self) -> "BuildSlot":
        return self

    def __exit__(self, _type, _value, _traceback) -> None:
        self.close()


class BuildSlotPool:
    """Allocate compatible build slots without sharing writable state."""

    def __init__(
        self,
        cache_root: Path,
        compatibility: dict[str, str | int],
        count: int = SLOT_COUNT,
    ) -> None:
        self.cache_root = cache_root
        self.compatibility = compatibility
        self.key = compatibility_key(compatibility)
        self.count = count
        self.slots_root = cache_root / "slots" / self.key
        self.seed_root = cache_root / "seeds" / self.key
        self.locks_root = cache_root / "locks" / "build-slots" / self.key

    def acquire(self) -> BuildSlot:
        """Wait for an idle compatible slot, creating or cloning one when possible."""
        self.slots_root.mkdir(parents=True, exist_ok=True)
        while True:
            for index in range(self.count):
                lease = FileLease(self.locks_root / f"slot-{index}.lock")
                if not lease.try_acquire():
                    continue
                path = self.slots_root / f"slot-{index}"
                try:
                    disposition = self._prepare(path, index)
                except BaseException:
                    lease.close()
                    raise
                return BuildSlot(path, path / "project", lease, disposition, self.compatibility)
            time.sleep(0.1)

    def _prepare(self, path: Path, index: int) -> str:
        manifest_path = path / "compatibility.json"
        if path.is_dir() and _read_json(manifest_path) == self.compatibility:
            return "reused"
        if path.exists():
            remove_owned_path(path, self.slots_root)
        disposition = "empty"
        seed_lease = FileLease(self.locks_root / "seed.lock")
        if seed_lease.try_acquire():
            try:
                seed = self.seed_root / "seed"
                if _read_json(seed / "compatibility.json") == self.compatibility:
                    disposition = self._clone_or_copy(seed, path)
            finally:
                seed_lease.close()
        if disposition != "empty":
            path.mkdir(parents=True, exist_ok=True)
            manifest_path.write_text(
                json.dumps(self.compatibility, indent=2, sort_keys=True) + "\n"
            )
            return disposition
        for source_index in range(self.count):
            if source_index == index:
                continue
            source = self.slots_root / f"slot-{source_index}"
            if _read_json(source / "compatibility.json") != self.compatibility:
                continue
            source_lease = FileLease(self.locks_root / f"slot-{source_index}.lock")
            if not source_lease.try_acquire():
                continue
            try:
                disposition = self._clone_or_copy(source, path)
            finally:
                source_lease.close()
            break
        path.mkdir(parents=True, exist_ok=True)
        manifest_path.write_text(json.dumps(self.compatibility, indent=2, sort_keys=True) + "\n")
        return disposition

    def publish_seed(self, slot: BuildSlot) -> str:
        """Publish a disposable cloneable snapshot of a successfully built slot."""
        if slot.compatibility != self.compatibility or slot.path.parent != self.slots_root:
            raise ValueError("Cannot seed this pool from a foreign build slot.")
        seed_lease = FileLease(self.locks_root / "seed.lock")
        while not seed_lease.try_acquire():
            time.sleep(0.1)
        try:
            self.seed_root.mkdir(parents=True, exist_ok=True)
            staging = self.seed_root / f"seed-{os.getpid()}.staging"
            if staging.exists():
                remove_owned_path(staging, self.seed_root)
            disposition = self._clone_or_copy(slot.path, staging)
            seed = self.seed_root / "seed"
            if seed.exists():
                remove_owned_path(seed, self.seed_root)
            staging.rename(seed)
            return disposition
        finally:
            seed_lease.close()

    def _clone_or_copy(self, source: Path, destination: Path) -> str:
        destination.parent.mkdir(parents=True, exist_ok=True)
        if platform.system() != "Darwin":
            shutil.copytree(source, destination, dirs_exist_ok=True)
            return "seeded with copied fallback"
        clone = subprocess.run(
            ["cp", "-cR", str(source), str(destination)],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        if clone.returncode == 0:
            return "seeded with APFS clone"
        destination.mkdir(parents=True, exist_ok=True)
        subprocess.run(["rsync", "-a", f"{source}/", f"{destination}/"], check=True)
        return "seeded with copied fallback"


def accelerator_state(project: Path) -> str:
    """Describe whether a slot begins with imported and incremental state."""
    states = []
    for name in PRESERVED_DIRECTORIES:
        directory = project / name
        entries = sum(1 for _ in directory.iterdir()) if directory.is_dir() else 0
        states.append(f"{name}={entries} entries")
    return ", ".join(states)


def _clean_transients(project: Path) -> None:
    for name in TRANSIENT_DIRECTORIES:
        path = project / name
        if path.exists() or path.is_symlink():
            remove_owned_path(path, project)


def _copy_tree(source: Path, destination: Path, excluded_paths: tuple[str, ...]) -> None:
    destination.mkdir(parents=True, exist_ok=True)

    def excluded(relative: Path) -> bool:
        return bool(set(relative.parts).intersection(SYNC_EXCLUDES)) or any(
            relative == Path(path) or Path(path) in relative.parents
            for path in excluded_paths
        )

    for child in sorted(destination.rglob("*"), key=lambda path: len(path.parts), reverse=True):
        relative = child.relative_to(destination)
        if not excluded(relative) and not (source / relative).exists():
            remove_owned_path(child, destination)

    def ignore(directory: str, names: list[str]) -> set[str]:
        relative = Path(directory).relative_to(source)
        return {
            name for name in names if excluded(relative / name)
        }

    shutil.copytree(
        source,
        destination,
        dirs_exist_ok=True,
        ignore=ignore,
    )


def _read_json(path: Path) -> dict:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (FileNotFoundError, json.JSONDecodeError):
        return {}
