#!/usr/bin/env python3

"""Content-addressed reuse for expensive local CI steps."""

from __future__ import annotations

from collections.abc import Callable, Sequence
import fcntl
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time


CACHE_SCHEMA = 1


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
            fcntl.flock(lease, fcntl.LOCK_EX)
            if self._valid_marker(marker, step, key):
                print(f"    {step}: CI Cache hit {key[:12]}", flush=True)
                return False
            print(f"    {step}: CI Cache miss {key[:12]}", flush=True)
            function()
            self._publish(marker, step, key)
            return True

    def _key(self, step: str, pathspecs: Sequence[str]) -> str:
        staged = subprocess.run(
            ["git", "ls-files", "--stage", "--", *pathspecs],
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
                "git",
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
            entry = json.loads(marker.read_text())
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
