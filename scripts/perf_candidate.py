#!/usr/bin/env python3

"""Build an exact candidate-only performance section."""

from __future__ import annotations

from pathlib import Path
import subprocess
from typing import Any

from perf_model import format_timestamp, parse_timestamp, Span


def build(
    candidate_id: str,
    candidates: list[dict[str, Any]],
    ci_spans: list[Span],
    tollgate_spans: list[Span],
    repository: Path,
) -> dict[str, Any]:
    """Bind candidate identity to its exact source, attempts, CI, and intents."""
    candidate = next(
        (
            value
            for value in candidates
            if value.get("item", {}).get("id") == candidate_id
        ),
        None,
    )
    if candidate is None:
        raise ValueError(f"Tollgate candidate was not found: {candidate_id}")
    item = candidate["item"]
    source_oid = _oid(item.get("source_oid"))
    source_tree = _tree(repository, source_oid)
    worktree = item.get("metadata", {}).get("worktree_path")
    attempts = [
        attempt
        for attempt in candidate.get("attempts", [])
        if isinstance(attempt, dict)
    ]
    run_spans = [span for span in ci_spans if span.id.startswith("ci-run:")]
    selected_runs = [
        span
        for span in run_spans
        if _candidate_run(span, source_tree, worktree, attempts, repository)
    ]
    run_ids = {span.id.removeprefix("ci-run:") for span in selected_runs}
    exact_ci_spans = [
        span
        for span in ci_spans
        if span.id in {run.id for run in selected_runs}
        or span.attributes.get("run_id") in run_ids
    ]
    exact_tollgate = [
        span
        for span in tollgate_spans
        if span.attributes.get("candidate_id") == candidate_id
    ]
    lifecycle = [
        span
        for span in sorted(exact_tollgate, key=lambda span: span.started_at)
        if span.category == "milestone"
    ]
    return {
        "candidate_id": candidate_id,
        "source_oid": source_oid,
        "source_tree_oid": source_tree,
        "state": item.get("state"),
        "remote_state": item.get("remote_state"),
        "worktree_path": worktree,
        "buildsets": [_buildset(attempt, repository) for attempt in attempts],
        "ci_runs": [
            _ci_run(run, exact_ci_spans)
            for run in sorted(selected_runs, key=lambda span: span.started_at)
        ],
        "operation_intents": [
            _span(span)
            for span in sorted(exact_tollgate, key=lambda span: span.started_at)
            if span.category == "tollgate-phase"
        ],
        "lifecycle": [_span(span) for span in lifecycle],
        "timings_ms": _lifecycle_timings(lifecycle),
        "exact_span_ids": sorted(span.id for span in [*exact_ci_spans, *exact_tollgate]),
        "exact_run_ids": sorted(run_ids),
    }


def retain_exact_spans(session: Any, candidate: dict[str, Any]) -> Any:
    """Remove unrelated candidate and CI spans from a selected task aggregate."""
    exact = set(candidate["exact_span_ids"])
    session.spans = [
        span
        for span in session.spans
        if span.source not in {"ci", "tollgate"} or span.id in exact
    ]
    return session


def _candidate_run(
    run: Span,
    source_tree: str | None,
    worktree: str | None,
    attempts: list[dict[str, Any]],
    repository: Path,
) -> bool:
    staged_tree = run.attributes.get("staged_tree_oid")
    if worktree and run.attributes.get("worktree_path") == worktree:
        return staged_tree == source_tree
    for attempt in attempts:
        started = parse_timestamp(attempt.get("started_at"))
        finished = parse_timestamp(attempt.get("finished_at"))
        tested_oid = _oid(attempt.get("tested_oid"))
        if started is None or finished is None:
            continue
        if run.finished_at < started - 2 or run.started_at > finished + 2:
            continue
        if run.attributes.get("head_oid") == tested_oid:
            return staged_tree == _tree(repository, tested_oid)
    return False


def _ci_run(run: Span, spans: list[Span]) -> dict[str, Any]:
    run_id = run.id.removeprefix("ci-run:")
    children = [span for span in spans if span.attributes.get("run_id") == run_id]
    lookups = [
        span.attributes.get("result")
        for span in children
        if span.attributes.get("event") == "ci.cache_lookup"
    ]
    maintenance = [
        span.attributes
        for span in children
        if span.attributes.get("event") == "ci.cache_maintenance"
        and span.attributes.get("performed")
    ]
    return {
        "run_id": run_id,
        "started_at": format_timestamp(run.started_at),
        "finished_at": format_timestamp(run.finished_at),
        "duration_ms": run.duration_ms,
        "status": run.status,
        "full": run.attributes.get("full"),
        "head_oid": run.attributes.get("head_oid"),
        "staged_tree_oid": run.attributes.get("staged_tree_oid"),
        "cache": {
            "hits": lookups.count("hit"),
            "misses": lookups.count("miss"),
            "bypassed": lookups.count("bypassed"),
            "disabled": lookups.count("disabled"),
            "maintenance": maintenance,
            "lock_wait_ms": sum(
                span.duration_ms
                for span in children
                if span.attributes.get("event") == "ci.cache_wait"
            ),
        },
        "steps": [
            _span(span)
            for span in children
            if span.id.startswith("ci-step:") and span.parent_id == run.id
        ],
    }


def _buildset(attempt: dict[str, Any], repository: Path) -> dict[str, Any]:
    started = parse_timestamp(attempt.get("started_at"))
    finished = parse_timestamp(attempt.get("finished_at"))
    return {
        "buildset_id": attempt.get("id"),
        "validation_generation_id": attempt.get("validation_generation_id"),
        "attempt": attempt.get("attempt"),
        "state": attempt.get("state"),
        "tested_oid": _oid(attempt.get("tested_oid")),
        "staged_tree_oid": _tree(repository, _oid(attempt.get("tested_oid"))),
        "started_at": None if started is None else format_timestamp(started),
        "finished_at": None if finished is None else format_timestamp(finished),
        "duration_ms": (
            None if started is None or finished is None else round((finished - started) * 1000)
        ),
        "steps": attempt.get("step_results", []),
    }


def _lifecycle_timings(spans: list[Span]) -> dict[str, int | None]:
    boundaries = {
        span.attributes.get("milestone"): span.started_at for span in spans
    }

    def elapsed(start: str, finish: str) -> int | None:
        first = boundaries.get(start)
        last = boundaries.get(finish)
        if first is None or last is None or last < first:
            return None
        return round((last - first) * 1000)

    return {
        "submission_to_certification": elapsed(
            "candidate.submitted", "candidate.certified"
        ),
        "authorization_to_synchronization": elapsed(
            "candidate.authorized", "candidate.synchronized"
        ),
        "submission_to_synchronization": elapsed(
            "candidate.submitted", "candidate.synchronized"
        ),
    }


def _span(span: Span) -> dict[str, Any]:
    return {
        "id": span.id,
        "name": span.name,
        "started_at": format_timestamp(span.started_at),
        "finished_at": format_timestamp(span.finished_at),
        "duration_ms": span.duration_ms,
        "status": span.status,
        "attributes": span.attributes,
    }


def _tree(repository: Path, oid: str | None) -> str | None:
    if not oid:
        return None
    try:
        return subprocess.run(
            ["git", "rev-parse", f"{oid}^{{tree}}"],
            cwd=repository,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        return None


def _oid(value: Any) -> str | None:
    return value.get("bytes") if isinstance(value, dict) else None
