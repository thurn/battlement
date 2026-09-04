#!/usr/bin/env python3

"""Structured, best-effort performance logging shared by Battlement tooling."""

from __future__ import annotations

from collections.abc import Callable, Iterator
from contextlib import contextmanager
from dataclasses import dataclass
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import subprocess
import sys
from threading import Lock, local
import time
from typing import Any, TextIO
import uuid

from platform_support import lock_file, unlock_file


DEFAULT_LOG_ROOT = Path.home() / "battlement/.logs"
DEFAULT_MAX_LOG_BYTES = 2 * 1024**3


@dataclass
class TraceSpan:
    """One timed operation recorded in a CI trace."""

    span_id: str
    started_ns: int
    duration_ms: int = 0


def configured_log_root() -> Path:
    """Return the shared performance-log directory."""
    return Path(os.environ.get("BATTLEMENT_LOG_ROOT", DEFAULT_LOG_ROOT))


def configured_max_log_bytes() -> int:
    """Return the maximum retained bytes for generated performance data."""
    configured = os.environ.get("BATTLEMENT_LOG_MAX_BYTES")
    if configured is None:
        return DEFAULT_MAX_LOG_BYTES
    try:
        value = int(configured)
    except ValueError:
        return DEFAULT_MAX_LOG_BYTES
    return value if value > 0 else DEFAULT_MAX_LOG_BYTES


def utc_now() -> str:
    """Return the current UTC time in a sortable ISO-8601 form."""
    return datetime.now(timezone.utc).isoformat(timespec="milliseconds").replace(
        "+00:00", "Z"
    )


def normalize_repository_url(url: str | None) -> str:
    """Normalize common Git remote spellings for repository matching."""
    if not url:
        return ""
    normalized = url.strip().removesuffix("/").removesuffix(".git")
    if normalized.startswith("git@") and ":" in normalized:
        host, path = normalized[4:].split(":", 1)
        normalized = f"{host}/{path}"
    else:
        normalized = normalized.removeprefix("ssh://git@")
        normalized = normalized.removeprefix("https://")
        normalized = normalized.removeprefix("http://")
    return normalized.casefold()


def git_metadata(repository_root: Path) -> dict[str, Any]:
    """Return non-fatal Git identity used to correlate local and gated runs."""
    def query(*arguments: str) -> str:
        try:
            return subprocess.run(
                ["git", *arguments],
                cwd=repository_root,
                check=True,
                capture_output=True,
                text=True,
            ).stdout.strip()
        except (OSError, subprocess.CalledProcessError):
            return ""

    status = query("status", "--porcelain=v1", "--untracked-files=all")
    repository_url = query("remote", "get-url", "origin")
    return {
        "repository_url": normalize_repository_url(repository_url),
        "worktree_path": str(repository_root.resolve()),
        "branch": query("branch", "--show-current"),
        "head_oid": query("rev-parse", "HEAD"),
        "staged_tree_oid": query("write-tree"),
        "dirty": bool(status),
    }


class CiTrace:
    """Write one concurrency-safe JSONL trace for a CI invocation."""

    def __init__(
        self,
        repository_root: Path,
        metadata: dict[str, Any],
        log_root: Path | None = None,
        monotonic_ns: Callable[[], int] = time.monotonic_ns,
    ) -> None:
        self.run_id = str(uuid.uuid4())
        self.log_root = configured_log_root() if log_root is None else log_root
        self.path: Path | None = None
        self._file: TextIO | None = None
        self._lock = Lock()
        self._thread_state = local()
        self._monotonic_ns = monotonic_ns
        self._warned = False
        self._started_ns = monotonic_ns()
        try:
            directory = self.log_root / "ci" / datetime.now(timezone.utc).date().isoformat()
            directory.mkdir(mode=0o700, parents=True, exist_ok=True)
            directory.chmod(0o700)
            self.path = directory / f"{self.run_id}.jsonl"
            descriptor = os.open(
                self.path,
                os.O_APPEND | os.O_CREAT | os.O_WRONLY,
                0o600,
            )
            os.fchmod(descriptor, 0o600)
            self._file = os.fdopen(descriptor, "a", encoding="utf-8")
        except OSError as error:
            self._warn(error)
        self.event(
            "ci.run_started",
            run_span_id=self.run_id,
            codex_session_id=os.environ.get("CODEX_SESSION_ID"),
            codex_thread_id=os.environ.get("CODEX_THREAD_ID"),
            python_executable=sys.executable,
            **git_metadata(repository_root),
            **metadata,
        )

    def current_span_id(self) -> str:
        """Return the innermost span in the current worker thread."""
        stack = getattr(self._thread_state, "stack", ())
        return stack[-1] if stack else self.run_id

    def event(self, event: str, **attributes: Any) -> None:
        """Append one event without allowing telemetry failure to affect CI."""
        record = {
            "timestamp": utc_now(),
            "event": event,
            "run_id": self.run_id,
            **attributes,
        }
        with self._lock:
            if self._file is None:
                return
            try:
                self._file.write(json.dumps(record, sort_keys=True) + "\n")
                self._file.flush()
            except (OSError, TypeError, ValueError) as error:
                self._warn(error)
                self._close_unlocked()

    @contextmanager
    def span(
        self,
        name: str,
        *,
        parent_span_id: str | None = None,
        kind: str = "step",
        attributes: dict[str, Any] | None = None,
    ) -> Iterator[TraceSpan]:
        """Record a start and terminal event around one operation."""
        span = TraceSpan(str(uuid.uuid4()), self._monotonic_ns())
        parent = parent_span_id or self.current_span_id()
        stack = list(getattr(self._thread_state, "stack", ()))
        stack.append(span.span_id)
        self._thread_state.stack = stack
        self.event(
            "ci.step_started",
            span_id=span.span_id,
            parent_span_id=parent,
            name=name,
            kind=kind,
            **(attributes or {}),
        )
        outcome = "passed"
        error_type = None
        try:
            yield span
        except BaseException as error:
            outcome = "interrupted" if isinstance(error, KeyboardInterrupt) else "failed"
            error_type = type(error).__name__
            raise
        finally:
            span.duration_ms = round((self._monotonic_ns() - span.started_ns) / 1_000_000)
            self.event(
                "ci.step_finished",
                span_id=span.span_id,
                parent_span_id=parent,
                name=name,
                kind=kind,
                duration_ms=span.duration_ms,
                outcome=outcome,
                error_type=error_type,
                **(attributes or {}),
            )
            self._thread_state.stack = stack[:-1]

    def finish(self, outcome: str, exit_code: int) -> None:
        """Record the run result and close its file."""
        duration_ms = round((self._monotonic_ns() - self._started_ns) / 1_000_000)
        self.event(
            "ci.run_finished",
            run_span_id=self.run_id,
            duration_ms=duration_ms,
            outcome=outcome,
            exit_code=exit_code,
        )
        with self._lock:
            self._close_unlocked()

    def _warn(self, error: BaseException) -> None:
        if self._warned:
            return
        self._warned = True
        print(f"CI performance logging disabled: {error}", file=sys.stderr)

    def _close_unlocked(self) -> None:
        if self._file is None:
            return
        try:
            self._file.close()
        except OSError:
            pass
        self._file = None


@contextmanager
def retention_guard(log_root: Path) -> Iterator[None]:
    """Serialize report reads with retention and other report readers."""
    log_root.mkdir(mode=0o700, parents=True, exist_ok=True)
    lock_path = log_root / ".retention.lock"
    with lock_path.open("a+") as lease:
        lock_file(lease)
        try:
            yield
        finally:
            unlock_file(lease)


def enforce_retention(
    log_root: Path,
    maximum_bytes: int,
    protected: set[Path] | None = None,
) -> list[Path]:
    """Remove oldest generated reports, then completed CI traces, under a cap."""
    protected = {path.resolve() for path in (protected or set())}
    candidates: list[tuple[int, int, Path]] = []
    total = 0
    for priority, pattern in ((0, "reports/*.json"), (1, "ci/**/*.jsonl")):
        for path in log_root.glob(pattern):
            try:
                metadata = path.stat()
            except FileNotFoundError:
                continue
            total += metadata.st_size
            completed = priority == 0 or _completed_ci_trace(path)
            if path.resolve() not in protected and completed:
                candidates.append((priority, metadata.st_mtime_ns, path))
    removed = []
    for _priority, _modified, path in sorted(candidates):
        if total <= maximum_bytes:
            break
        try:
            size = path.stat().st_size
            path.unlink()
        except FileNotFoundError:
            continue
        total -= size
        removed.append(path)
    if total > maximum_bytes:
        print(
            f"Performance logs exceed the configured cap by {total - maximum_bytes} bytes; "
            "protected files were retained.",
            file=sys.stderr,
        )
    return removed


def _completed_ci_trace(path: Path) -> bool:
    try:
        with path.open("rb") as source:
            source.seek(max(0, path.stat().st_size - 4096))
            tail = source.read().splitlines()
    except OSError:
        return False
    for line in reversed(tail):
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue
        return isinstance(record, dict) and record.get("event") == "ci.run_finished"
    return False
