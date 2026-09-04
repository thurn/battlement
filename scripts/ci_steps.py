#!/usr/bin/env python3

"""Timed serial and parallel execution boundaries for Battlement CI."""

from __future__ import annotations

from collections.abc import Callable
from concurrent.futures import as_completed, ThreadPoolExecutor
from contextlib import AbstractContextManager, nullcontext
from pathlib import Path
import subprocess
import time

import perf_log


_repository_root = Path(__file__).resolve().parent.parent
_trace: perf_log.CiTrace | None = None


def configure(repository_root: Path, trace: perf_log.CiTrace | None = None) -> None:
    """Set the checkout and optional trace used by execution boundaries."""
    global _repository_root, _trace
    _repository_root = repository_root
    _trace = trace


def interrupted(_signal_number: int, _frame: object) -> None:
    """Translate termination signals into the CI interruption path."""
    raise KeyboardInterrupt


def trace_smoke_test(outcome: str) -> None:
    """Exercise real serial and parallel trace boundaries for black-box tests."""
    run_step("Trace smoke serial", function=lambda: None)
    run_step(
        "Trace smoke parallel",
        function=lambda: run_parallel_steps(
            [("trace child one", lambda: None), ("trace child two", lambda: None)]
        ),
    )
    if outcome == "failed":
        raise RuntimeError("requested trace smoke failure")
    if outcome == "interrupted":
        raise KeyboardInterrupt


def run_step(
    name: str,
    command: list[str] | None = None,
    function: Callable[[], None] | None = None,
    environment: dict[str, str] | None = None,
) -> float:
    """Run and print one serial CI step while recording its span."""
    print(f"\n==> {name}", flush=True)
    started = time.monotonic()
    attributes = {"command": command} if command is not None else {}
    try:
        with span(name, attributes=attributes):
            if function is not None:
                function()
            else:
                subprocess.run(
                    command,
                    cwd=_repository_root,
                    env=environment,
                    check=True,
                )
    finally:
        elapsed = time.monotonic() - started
        print(f"<== {name} ({elapsed:.1f}s)", flush=True)
    return elapsed


def run_parallel_steps(
    steps: list[tuple[str, Callable[[], None]]],
    workers: int = 2,
) -> None:
    """Run named child spans concurrently and raise the first failure."""
    parent_span_id = _trace.current_span_id() if _trace is not None else None

    def execute(name: str, function: Callable[[], None]) -> tuple[float, Exception | None]:
        started = time.monotonic()
        try:
            with span(name, parent_span_id=parent_span_id):
                function()
        except Exception as error:
            return time.monotonic() - started, error
        return time.monotonic() - started, None

    failures: list[Exception] = []
    with ThreadPoolExecutor(max_workers=min(workers, len(steps))) as executor:
        futures = {
            executor.submit(execute, name, function): name
            for name, function in steps
        }
        for future in as_completed(futures):
            name = futures[future]
            elapsed, error = future.result()
            if error is not None:
                failures.append(error)
            print(
                f"    {name}: {'failed' if error else 'passed'} ({elapsed:.1f}s)",
                flush=True,
            )
    if failures:
        raise failures[0]


def record_cache_event(event: str, attributes: dict[str, object]) -> None:
    """Attach a cache event to the currently executing CI span."""
    if _trace is None:
        return
    _trace.event(
        event,
        parent_span_id=_trace.current_span_id(),
        **attributes,
    )


def span(
    name: str,
    *,
    parent_span_id: str | None = None,
    attributes: dict[str, object] | None = None,
) -> AbstractContextManager[object]:
    """Return a traced or inert context for hand-timed CI work."""
    if _trace is None:
        return nullcontext()
    return _trace.span(
        name,
        parent_span_id=parent_span_id,
        attributes=attributes,
    )
