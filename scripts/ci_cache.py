#!/usr/bin/env python3

"""Content-addressed reuse for expensive local CI steps."""

from __future__ import annotations

from collections.abc import Callable, Iterator, Sequence
from contextlib import contextmanager
from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import uuid

from platform_support import lock_file, resolve_executable, unlock_file


CACHE_SCHEMA = 1
MAINTENANCE_SCHEMA = 1
DEFAULT_TARGET_BYTES = 20 * 1024**3
DEFAULT_HIGH_WATER_BYTES = 25 * 1024**3
CHROME_CLONE_MINIMUM_AGE_SECONDS = 60 * 60
CHROME_LSOF_TIMEOUT_SECONDS = 5
MAINTENANCE_INTERVAL_SECONDS = 60 * 60


@dataclass(frozen=True)
class CachePruneResult:
    """Allocated storage reclaimed by one cache maintenance pass."""

    before_bytes: int
    after_bytes: int
    removed: tuple[Path, ...]


def chrome_clone_root() -> Path:
    """Return Chrome's per-user macOS code-signing clone directory."""
    return Path(tempfile.gettempdir()).parent / "X/com.google.Chrome.code_sign_clone"


def prune_chrome_code_sign_clones(
    root: Path | None = None,
    open_clones: set[str] | None = None,
    now_ns: int | None = None,
    minimum_age_seconds: int = CHROME_CLONE_MINIMUM_AGE_SECONDS,
) -> CachePruneResult:
    """Remove abandoned Chrome signing clones after proving they are not in use."""
    root = chrome_clone_root() if root is None else root
    if not root.is_dir() or root.is_symlink():
        return CachePruneResult(0, 0, ())
    lock = root / ".battlement.lock"
    with lock.open("a+") as lease:
        lock_file(lease)
        try:
            open_clones = _open_chrome_clones(root) if open_clones is None else open_clones
            before = charged_size(root)
            if open_clones is None:
                return CachePruneResult(before, before, ())
            cutoff = (time.time_ns() if now_ns is None else now_ns) - (
                minimum_age_seconds * 1_000_000_000
            )
            removed = []
            for path in root.iterdir():
                suffix = path.name.removeprefix("code_sign_clone.")
                owned = len(suffix) == 6 and suffix.isalnum()
                if not owned or path.name in open_clones or path.is_symlink():
                    continue
                if not path.is_dir() or path.stat().st_mtime_ns > cutoff:
                    continue
                quarantine = path.with_name(f".battlement-pruning-{uuid.uuid4()}")
                path.rename(quarantine)
                shutil.rmtree(quarantine)
                removed.append(path)
            after = charged_size(root)
            if removed:
                print(
                    f"    Chrome Cache: reclaimed {before - after} allocated bytes; "
                    f"{after} bytes remain",
                    flush=True,
                )
            return CachePruneResult(before, after, tuple(removed))
        finally:
            unlock_file(lease)


def _open_chrome_clones(root: Path) -> set[str] | None:
    try:
        result = subprocess.run(
            [resolve_executable("lsof"), "-Fn", "+D", str(root)],
            capture_output=True,
            text=True,
            timeout=CHROME_LSOF_TIMEOUT_SECONDS,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    if result.returncode not in {0, 1}:
        return None
    prefix = f"{root}/"
    open_clones = set()
    for line in result.stdout.splitlines():
        if not line.startswith(f"n{prefix}"):
            continue
        relative = line[1 + len(prefix) :]
        open_clones.add(relative.split("/", 1)[0])
    return open_clones


def charged_size(path: Path) -> int:
    """Return allocated bytes without following symbolic links."""
    try:
        metadata = path.lstat()
    except FileNotFoundError:
        return 0
    if path.is_symlink():
        raise RuntimeError(f"CI Cache path is a symbolic link: {path}")
    return _charged_size_entry(path, metadata)


def _charged_size_entry(path: Path, metadata: os.stat_result) -> int:
    """Count one owned tree entry while treating descendant links as leaves."""
    size = getattr(metadata, "st_blocks", 0) * 512 or metadata.st_size
    if path.is_symlink() or not path.is_dir():
        return size
    total = size
    for child in path.iterdir():
        try:
            child_metadata = child.lstat()
        except FileNotFoundError:
            continue
        total += _charged_size_entry(child, child_metadata)
    return total


class CiCache:
    """Reuse successful CI steps with identical staged inputs and environments."""

    def __init__(
        self,
        repository_root: Path,
        cache_root: Path,
        environment: dict[str, str | int],
        enabled: bool = True,
    ) -> None:
        self.repository_root = repository_root
        self.cache_root = cache_root
        self.environment = environment
        self.enabled = enabled
        self._invocation_lease = None

    @contextmanager
    def invocation(self) -> Iterator[None]:
        """Serialize shared compiler writers and perform periodic maintenance."""
        lock = self.cache_root / "locks" / "invocation.lock"
        lock.parent.mkdir(parents=True, exist_ok=True)
        with lock.open("a+") as lease:
            started = time.monotonic()
            lock_file(lease)
            waited = time.monotonic() - started
            if waited >= 1:
                print(
                    f"    Shared compiler cache: waited {waited:.1f}s for another writer",
                    flush=True,
                )
            try:
                self.maintain()
                yield
            finally:
                unlock_file(lease)

    def maintain(
        self,
        now_ns: int | None = None,
        interval_seconds: int = MAINTENANCE_INTERVAL_SECONDS,
    ) -> bool:
        """Run cache maintenance when the last completed pass is stale."""
        now_ns = time.time_ns() if now_ns is None else now_ns
        marker = self.cache_root / "maintenance.json"
        try:
            state = json.loads(marker.read_text(encoding="utf-8"))
        except (FileNotFoundError, json.JSONDecodeError):
            state = {}
        completed_at = state.get("completedAt")
        interval_ns = interval_seconds * 1_000_000_000
        current_schema = state.get("schema") == MAINTENANCE_SCHEMA
        recent = isinstance(completed_at, int) and 0 <= now_ns - completed_at < interval_ns
        if current_schema and recent:
            return False
        started = time.monotonic()
        cache = self.prune()
        chrome = prune_chrome_code_sign_clones()
        marker.parent.mkdir(parents=True, exist_ok=True)
        staging = marker.with_name(f"{marker.name}.{os.getpid()}.staging")
        staging.write_text(
            json.dumps(
                {
                    "schema": MAINTENANCE_SCHEMA,
                    "completedAt": now_ns,
                },
                indent=2,
                sort_keys=True,
            )
            + "\n"
        )
        staging.replace(marker)
        print(
            "    CI Cache maintenance: "
            f"{cache.after_bytes} allocated bytes remain; "
            f"removed {len(cache.removed)} compiler targets and "
            f"{len(chrome.removed)} Chrome clones ({time.monotonic() - started:.1f}s)",
            flush=True,
        )
        return True

    def prune(
        self,
        target_bytes: int = DEFAULT_TARGET_BYTES,
        high_water_bytes: int = DEFAULT_HIGH_WATER_BYTES,
    ) -> CachePruneResult:
        """Remove legacy checkout trees and least-recent shared targets under pressure."""
        before = charged_size(self.cache_root)
        targets = self.cache_root / "cargo-targets"
        if not targets.is_dir():
            return CachePruneResult(before, before, ())
        candidates: list[Path] = []
        for child in targets.iterdir():
            if child.name == "shared" and child.is_dir() and not child.is_symlink():
                candidates.extend(
                    path
                    for path in child.iterdir()
                    if path.is_dir() and not path.is_symlink()
                )
            elif child.is_dir() and not child.is_symlink():
                candidates.append(child)
        legacy = [path for path in candidates if path.parent == targets]
        shared = [path for path in candidates if path.parent.name == "shared"]
        selected = list(legacy)
        projected = before - sum(charged_size(path) for path in legacy)
        if projected > high_water_bytes:
            shared.sort(key=lambda path: (path.stat().st_mtime_ns, path.name))
            for path in shared:
                if projected <= target_bytes:
                    break
                selected.append(path)
                projected -= charged_size(path)
        removed = []
        for path in selected:
            quarantine = path.with_name(f".pruning-{uuid.uuid4()}")
            path.rename(quarantine)
            shutil.rmtree(quarantine)
            removed.append(path)
        after = charged_size(self.cache_root)
        if removed:
            print(
                f"    CI Cache: reclaimed {before - after} allocated bytes; "
                f"{after} bytes remain",
                flush=True,
            )
        return CachePruneResult(before, after, tuple(removed))

    def run(
        self,
        step: str,
        pathspecs: Sequence[str],
        function: Callable[[], None],
    ) -> bool:
        """Run a step on a cache miss and return whether execution was needed."""
        if not self.enabled:
            print(f"    {step}: CI Cache disabled", flush=True)
            function()
            return True
        if self._has_unstaged_inputs(pathspecs):
            print(f"    {step}: CI Cache bypassed for unstaged inputs", flush=True)
            function()
            return True
        key = self._key(step, pathspecs)
        marker = self.cache_root / "entries" / step / f"{key}.json"
        lock = self.cache_root / "locks" / step / f"{key}.lock"
        lock.parent.mkdir(parents=True, exist_ok=True)
        with lock.open("a+") as lease:
            lock_file(lease)
            if self._valid_marker(marker, step, key):
                print(f"    {step}: CI Cache hit {key[:12]}", flush=True)
                return False
            print(f"    {step}: CI Cache miss {key[:12]}", flush=True)
            function()
            self._publish(marker, step, key)
            return True

    def _key(self, step: str, pathspecs: Sequence[str]) -> str:
        staged = subprocess.run(
            [resolve_executable("git"), "ls-files", "--stage", "--", *pathspecs],
            cwd=self.repository_root,
            check=True,
            capture_output=True,
            text=True,
        ).stdout
        identity = {
            "schema": CACHE_SCHEMA,
            "step": step,
            "pathspecs": list(pathspecs),
            "staged": staged,
            "environment": self.environment,
        }
        encoded = json.dumps(identity, sort_keys=True, separators=(",", ":")).encode()
        return hashlib.sha256(encoded).hexdigest()

    def _has_unstaged_inputs(self, pathspecs: Sequence[str]) -> bool:
        status = subprocess.run(
            [
                resolve_executable("git"),
                "status",
                "--porcelain=v1",
                "-z",
                "--untracked-files=all",
                "--",
                *pathspecs,
            ],
            cwd=self.repository_root,
            check=True,
            capture_output=True,
        ).stdout
        return any(
            record.startswith(b"??") or len(record) < 2 or record[1:2] != b" "
            for record in status.split(b"\0")
            if record
        )

    def _valid_marker(self, marker: Path, step: str, key: str) -> bool:
        try:
            entry = json.loads(marker.read_text(encoding="utf-8"))
        except (FileNotFoundError, json.JSONDecodeError):
            return False
        return entry == {
            "schema": CACHE_SCHEMA,
            "step": step,
            "key": key,
            "completedAt": entry.get("completedAt"),
        } and isinstance(entry["completedAt"], int)

    def _publish(self, marker: Path, step: str, key: str) -> None:
        marker.parent.mkdir(parents=True, exist_ok=True)
        staging = marker.with_name(f"{marker.name}.{os.getpid()}.staging")
        staging.write_text(
            json.dumps(
                {
                    "schema": CACHE_SCHEMA,
                    "step": step,
                    "key": key,
                    "completedAt": time.time_ns(),
                },
                indent=2,
                sort_keys=True,
            )
            + "\n"
        )
        staging.replace(marker)
