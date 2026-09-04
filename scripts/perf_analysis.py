#!/usr/bin/env python3

"""Correlate performance sources and derive deterministic workflow findings."""

from __future__ import annotations

from collections import defaultdict
import json
from pathlib import Path
import subprocess
from typing import Any

from perf_model import (
    exclusive_durations,
    interval_difference_ms,
    interval_union_ms,
    SessionTrace,
    Span,
    Thresholds,
)


def correlate_activity(
    sessions: list[SessionTrace],
    ci_spans: list[Span],
    tollgate_spans: list[Span],
    candidates: list[dict[str, Any]],
    repository_root: Path,
    warnings: list[str],
) -> None:
    """Attach CI and Tollgate spans only when an exact association is available."""
    owner_by_thread = {
        thread_id: session
        for session in sessions
        for thread_id in _session_thread_ids(session)
    }
    ci_runs = [span for span in ci_spans if span.id.startswith("ci-run:")]
    owner_by_run: dict[str, tuple[SessionTrace, str]] = {}
    for run in ci_runs:
        thread_id = run.attributes.get("codex_thread_id")
        owner = owner_by_thread.get(thread_id)
        if owner is not None:
            owner_by_run[run.id.removeprefix("ci-run:")] = (owner, "exact_thread")
    candidate_owner: dict[str, tuple[SessionTrace, str]] = {}
    for candidate in candidates:
        item = candidate.get("item", {})
        candidate_id = item.get("id")
        explicit = [session for session in sessions if candidate_id in session.candidate_ids]
        if len(explicit) == 1:
            candidate_owner[candidate_id] = (explicit[0], "exact_candidate")
    tree_owners: dict[str, set[str]] = defaultdict(set)
    sessions_by_id = {session.thread_id: session for session in sessions}
    for run_id, (owner, _method) in owner_by_run.items():
        run = next(span for span in ci_runs if span.id == f"ci-run:{run_id}")
        tree = run.attributes.get("staged_tree_oid")
        if tree:
            tree_owners[tree].add(owner.thread_id)
    for candidate in candidates:
        item = candidate.get("item", {})
        candidate_id = item.get("id")
        if not candidate_id or candidate_id in candidate_owner:
            continue
        source_oid = _oid(item.get("source_oid"))
        tree = _commit_tree(repository_root, source_oid)
        owners = tree_owners.get(tree, set())
        if len(owners) == 1:
            candidate_owner[candidate_id] = (
                sessions_by_id[next(iter(owners))],
                "exact_tree",
            )
        elif len(owners) > 1:
            warnings.append(
                f"Candidate {candidate_id} matched multiple sessions by tree {tree}."
            )
    candidate_by_oid = {}
    for candidate in candidates:
        item = candidate.get("item", {})
        buildset = candidate.get("buildset", {})
        attempts = candidate.get("attempts", [])
        tested_oids = (
            _oid(attempt.get("tested_oid"))
            for attempt in attempts
            if isinstance(attempt, dict)
        )
        for oid in (
            _oid(item.get("source_oid")),
            _oid(buildset.get("tested_oid")),
            *tested_oids,
        ):
            if oid:
                candidate_by_oid[oid] = item.get("id")
    for run in ci_runs:
        run_id = run.id.removeprefix("ci-run:")
        if run_id in owner_by_run:
            continue
        candidate_id = candidate_by_oid.get(run.attributes.get("head_oid"))
        if candidate_id in candidate_owner:
            owner_by_run[run_id] = (candidate_owner[candidate_id][0], "exact_commit")
    for span in ci_spans:
        run_id = span.attributes.get("run_id")
        if run_id is None and span.id.startswith("ci-run:"):
            run_id = span.id.removeprefix("ci-run:")
        owner = owner_by_run.get(run_id)
        if owner is None:
            continue
        _attach(owner[0], span, owner[1])
    for span in tollgate_spans:
        candidate_id = span.attributes.get("candidate_id")
        owner = candidate_owner.get(candidate_id)
        if owner is None:
            continue
        _attach(owner[0], span, owner[1])
    unmatched_ci = sum(1 for run in ci_runs if run.id.removeprefix("ci-run:") not in owner_by_run)
    unmatched_candidates = sum(
        1
        for candidate in candidates
        if candidate.get("item", {}).get("id") not in candidate_owner
    )
    if unmatched_ci:
        warnings.append(f"{unmatched_ci} CI runs could not be associated exactly.")
    if unmatched_candidates:
        warnings.append(
            f"{unmatched_candidates} Tollgate candidates could not be associated exactly."
        )


def analyze_session(
    session: SessionTrace,
    thresholds: Thresholds,
    top: int,
) -> dict[str, Any]:
    """Calculate interval-aware summaries, rankings, and workflow findings."""
    spans = sorted(session.spans, key=lambda span: (span.started_at, span.finished_at))
    spans.extend(_inter_turn_waits(session, spans))
    exclusive = exclusive_durations(spans)
    first = session.first_user_at
    finished = session.completed_at or session.latest_event_at
    wall_ms = 0 if first is None or finished is None else max(0, round((finished - first) * 1000))
    waits = [span for span in spans if span.category == "wait"]
    wait_intervals = _bounded_intervals(waits, first, finished)
    active_ms = interval_difference_ms(
        _bounded_intervals(
            [span for span in spans if span.category != "wait"], first, finished
        ),
        wait_intervals,
    )
    wait_ms = interval_union_ms(wait_intervals)
    category_totals: dict[str, int] = defaultdict(int)
    for span in spans:
        category_totals[span.category] += exclusive[span.id]
    ci_wrapper_ids = {
        span.parent_id
        for span in spans
        if span.id.startswith("ci-run:") and span.parent_id is not None
    }
    operations = [
        span
        for span in spans
        if span.category != "agent"
        and span.id not in ci_wrapper_ids
        and (not span.container or span.category in {"subagent", "ci"})
    ]
    findings = workflow_findings(spans, thresholds)
    longest = sorted(operations, key=lambda span: span.duration_ms, reverse=True)[:top]
    longest_waits = sorted(waits, key=lambda span: span.duration_ms, reverse=True)[:top]
    contributors = sorted(spans, key=lambda span: exclusive[span.id], reverse=True)[:top]
    return {
        "metadata": session.as_metadata(),
        "timing": {
            "wall_time_ms": wall_ms,
            "recorded_active_coverage_ms": active_ms,
            "known_wait_union_ms": wait_ms,
            "inter_turn_user_wait_ms": _span_union_ms(
                spans, source="codex", name="Between agent turns", bounds=(first, finished)
            ),
            "tollgate_queue_ms": _span_union_ms(
                spans, source="tollgate", name="Tollgate queue", bounds=(first, finished)
            ),
            "tollgate_execution_ms": _span_union_ms(
                spans, source="tollgate", category="ci", bounds=(first, finished)
            ),
            "tollgate_authorization_ms": _span_union_ms(
                spans, source="tollgate", name="Tollgate authorization wait", bounds=(first, finished)
            ),
            "tollgate_promotion_ms": _span_union_ms(
                spans, source="tollgate", name="Tollgate promotion", bounds=(first, finished)
            ),
            "unattributed_agent_turn_ms": sum(
                exclusive[span.id] for span in spans if span.category == "agent"
            ),
            "unattributed_wall_time_ms": max(0, wall_ms - interval_union_ms(
                [(span.started_at, span.finished_at) for span in spans]
            )),
            "parallel_work_ms": sum(exclusive.values()),
            "category_exclusive_ms": dict(sorted(category_totals.items())),
            "time_to_first_token_ms": session.time_to_first_token_ms,
            "token_usage": session.token_usage,
        },
        "longest_operations": [_ranked_span(span, exclusive) for span in longest],
        "longest_waits": [_ranked_span(span, exclusive) for span in longest_waits],
        "largest_contributors": [_ranked_span(span, exclusive) for span in contributors],
        "findings": findings,
        "spans": [span.as_dict(exclusive[span.id]) for span in spans],
        "transcript": sorted(session.transcript, key=lambda item: item.get("timestamp", "")),
        "warnings": session.warnings,
    }


def aggregate_reports(reports: list[dict[str, Any]], top: int) -> dict[str, Any]:
    """Combine session reports without treating parallel work as wall time."""
    operations = [
        span for report in reports for span in report["longest_operations"]
    ]
    contributors_source = [
        span for report in reports for span in report["largest_contributors"]
    ]
    findings = [finding for report in reports for finding in report["findings"]]
    waits = [span for report in reports for span in report["longest_waits"]]
    categories: dict[str, int] = defaultdict(int)
    for report in reports:
        for category, duration in report["timing"]["category_exclusive_ms"].items():
            categories[category] += duration
    longest = sorted(
        operations, key=lambda span: span["duration_ms"], reverse=True
    )[:top]
    contributors = sorted(
        contributors_source,
        key=lambda span: span["exclusive_duration_ms"],
        reverse=True,
    )[:top]
    tasks = sorted(
        (
            {
                "thread_id": report["metadata"]["thread_id"],
                "title": report["metadata"]["title"],
                "wall_time_ms": report["timing"]["wall_time_ms"],
            }
            for report in reports
        ),
        key=lambda task: task["wall_time_ms"],
        reverse=True,
    )
    return {
        "session_count": len(reports),
        "total_wall_time_ms": sum(report["timing"]["wall_time_ms"] for report in reports),
        "total_active_coverage_ms": sum(
            report["timing"]["recorded_active_coverage_ms"] for report in reports
        ),
        "total_known_wait_ms": sum(
            report["timing"]["known_wait_union_ms"] for report in reports
        ),
        "total_unattributed_agent_turn_ms": sum(
            report["timing"]["unattributed_agent_turn_ms"] for report in reports
        ),
        "category_exclusive_ms": dict(sorted(categories.items())),
        "longest_operations": longest,
        "longest_waits": sorted(
            waits, key=lambda span: span["duration_ms"], reverse=True
        )[:top],
        "largest_contributors": contributors,
        "longest_tasks": tasks[:top],
        "findings": sorted(findings, key=lambda finding: finding["duration_ms"], reverse=True),
    }


def workflow_findings(spans: list[Span], thresholds: Thresholds) -> list[dict[str, Any]]:
    """Return explainable findings from fixed structural and duration rules."""
    findings: list[dict[str, Any]] = []
    ci_runs = [span for span in spans if span.source == "ci" and span.id.startswith("ci-run:")]
    by_tree_mode: dict[tuple[str, bool], list[Span]] = defaultdict(list)
    by_tree: dict[str, list[Span]] = defaultdict(list)
    for span in ci_runs:
        tree = span.attributes.get("staged_tree_oid")
        if tree:
            by_tree_mode[(tree, bool(span.attributes.get("full")))].append(span)
            by_tree[tree].append(span)
    for (_tree, _full), repeated in by_tree_mode.items():
        if len(repeated) > 1:
            findings.append(_finding(
                "repeated-ci", "Repeated identical CI run",
                sum(span.duration_ms for span in repeated[1:]),
                "The same CI mode ran more than once for an identical staged tree.", repeated,
            ))
    for tree_spans in by_tree.values():
        fast_passed = [span for span in tree_spans if not span.attributes.get("full") and span.status == "passed"]
        full_failed = [span for span in tree_spans if span.attributes.get("full") and span.status != "passed"]
        if fast_passed and full_failed:
            findings.append(_finding(
                "fast-full-mismatch", "Fast CI passed before full CI failed",
                sum(span.duration_ms for span in full_failed),
                "Fast and full CI used the same staged tree, but only full CI exposed the failure.",
                [fast_passed[-1], *full_failed],
            ))
    failed_full = [span for span in ci_runs if span.attributes.get("full") and span.status != "passed"]
    passed_full = [span for span in ci_runs if span.attributes.get("full") and span.status == "passed"]
    for failed in failed_full:
        repaired = next((span for span in passed_full if span.started_at > failed.finished_at), None)
        if repaired is not None:
            findings.append(_finding(
                "full-ci-repair", "Full CI required a repair iteration", failed.duration_ms,
                "A failed full CI run was followed by another full run that passed.",
                [failed, repaired],
            ))
    findings.extend(_slow_findings(spans, thresholds))
    findings.extend(_repeated_tool_findings(spans))
    findings.extend(_cache_findings(spans, thresholds))
    findings.extend(_tollgate_findings(spans))
    findings.extend(_delayed_subagent_findings(spans, thresholds.long_wait_ms))
    return sorted(findings, key=lambda finding: finding["duration_ms"], reverse=True)


def _slow_findings(spans: list[Span], thresholds: Thresholds) -> list[dict[str, Any]]:
    findings = []
    for span in spans:
        threshold = None
        code = ""
        if span.category == "tool":
            threshold, code = thresholds.slow_tool_ms, "slow-tool"
        elif span.category == "subagent":
            threshold, code = thresholds.slow_subagent_ms, "slow-subagent"
        elif span.category == "ci" and not span.container:
            threshold, code = thresholds.slow_ci_step_ms, "slow-ci-step"
        elif span.category == "wait":
            threshold, code = thresholds.long_wait_ms, "long-wait"
        if threshold is None or span.duration_ms < threshold:
            continue
        findings.append(_finding(
            code,
            f"{span.name} took {_human_duration(span.duration_ms)}",
            span.duration_ms,
            f"The operation exceeded the configured {threshold / 1000:g}-second threshold.",
            [span],
        ))
    return findings


def _repeated_tool_findings(spans: list[Span]) -> list[dict[str, Any]]:
    groups: dict[str, list[Span]] = defaultdict(list)
    for span in spans:
        if span.category != "tool":
            continue
        identity = json.dumps(
            [span.name, span.content.get("input")], sort_keys=True, default=str
        )
        groups[identity].append(span)
    return [
        _finding(
            "repeated-tool", f"Repeated tool operation: {group[0].name}",
            sum(span.duration_ms for span in group[1:]),
            "The same normalized tool name and input occurred multiple times.", group,
        )
        for group in groups.values()
        if len(group) > 1 and sum(span.duration_ms for span in group) >= 60_000
    ]


def _cache_findings(spans: list[Span], thresholds: Thresholds) -> list[dict[str, Any]]:
    misses: dict[str, list[Span]] = defaultdict(list)
    findings = []
    for span in spans:
        if span.attributes.get("event") == "ci.cache_lookup" and span.attributes.get("result") == "miss":
            misses[str(span.attributes.get("cache_key"))].append(span)
        if span.attributes.get("event") == "ci.cache_wait" and span.duration_ms >= thresholds.long_wait_ms:
            findings.append(_finding(
                "cache-lock-wait", "Shared compiler cache lock was slow", span.duration_ms,
                "A CI invocation waited longer than the configured wait threshold for the cache lock.",
                [span],
            ))
    for group in misses.values():
        if len(group) > 1:
            findings.append(_finding(
                "repeated-cache-miss", "Repeated cache miss for one identity", 0,
                "The same content-addressed cache identity missed more than once.", group,
            ))
    return findings


def _tollgate_findings(spans: list[Span]) -> list[dict[str, Any]]:
    by_candidate: dict[str, list[Span]] = defaultdict(list)
    for span in spans:
        candidate_id = span.attributes.get("candidate_id")
        if candidate_id:
            by_candidate[candidate_id].append(span)
    findings = []
    for candidate_id, group in by_candidate.items():
        attempts = {span.attributes.get("attempt") for span in group if span.attributes.get("attempt")}
        failed = any(
            span.attributes.get("candidate_state") in {"failed", "check-failed"}
            or span.attributes.get("attempt_state") in {"failed", "check-failed"}
            for span in group
        )
        if len(attempts) > 1 or failed:
            findings.append(_finding(
                "tollgate-retry", f"Tollgate candidate {candidate_id} required attention",
                sum(span.duration_ms for span in group if span.category == "ci"),
                "The candidate failed or recorded multiple validation attempts.", group,
            ))
    return findings


def _delayed_subagent_findings(spans: list[Span], threshold_ms: int) -> list[dict[str, Any]]:
    findings = []
    ordered = sorted(spans, key=lambda span: span.finished_at)
    for span in spans:
        if span.category != "subagent":
            continue
        label = f"{span.name} {span.attributes.get('agent_path', '')}".casefold()
        classification = next((kind for kind in ("review", "research") if kind in label), None)
        if classification is None:
            continue
        prior = [item for item in ordered if item.finished_at <= span.started_at and item.category == "tool"]
        if not prior:
            continue
        delay_ms = round((span.started_at - prior[-1].finished_at) * 1000)
        if delay_ms >= threshold_ms:
            findings.append(_finding(
                "delayed-subagent", f"Delayed {classification} subagent start", delay_ms,
                f"The {classification} subagent began after a gap exceeding the wait threshold.",
                [prior[-1], span],
            ))
    return findings


def _inter_turn_waits(session: SessionTrace, spans: list[Span]) -> list[Span]:
    turns = sorted(
        (span for span in spans if span.category == "agent" and span.session_id == session.thread_id),
        key=lambda span: span.started_at,
    )
    return [
        Span(
            f"user-wait:{session.thread_id}:{index}", None, session.thread_id,
            "codex", "wait", "Between agent turns", previous.finished_at,
            current.started_at, "passed", association="session_timeline",
        )
        for index, (previous, current) in enumerate(zip(turns, turns[1:]), 1)
        if current.started_at > previous.finished_at
    ]


def _session_thread_ids(session: SessionTrace) -> set[str]:
    return {session.thread_id, *(span.session_id for span in session.spans if span.session_id)}


def _span_union_ms(
    spans: list[Span],
    *,
    source: str,
    category: str | None = None,
    name: str | None = None,
    bounds: tuple[float | None, float | None] = (None, None),
) -> int:
    selected = [
        span
        for span in spans
        if span.source == source
        and (category is None or span.category == category)
        and (name is None or span.name == name)
    ]
    return interval_union_ms(_bounded_intervals(selected, *bounds))


def _bounded_intervals(
    spans: list[Span],
    lower: float | None,
    upper: float | None,
) -> list[tuple[float, float]]:
    return [
        (
            max(span.started_at, lower) if lower is not None else span.started_at,
            min(span.finished_at, upper) if upper is not None else span.finished_at,
        )
        for span in spans
    ]


def _attach(session: SessionTrace, span: Span, association: str) -> None:
    span.session_id = session.thread_id
    span.association = association
    if span.id.startswith("ci-run:"):
        wrappers = [
            candidate
            for candidate in session.spans
            if candidate.source == "codex"
            and candidate.category in {"tool", "wait"}
            and "scripts/ci.py" in json.dumps(candidate.content.get("input"), default=str)
            and candidate.started_at <= span.started_at
            and candidate.finished_at >= span.finished_at
        ]
        if wrappers:
            span.parent_id = min(wrappers, key=lambda candidate: candidate.duration_ms).id
    session.spans.append(span)


def _commit_tree(repository_root: Path, oid: str | None) -> str:
    if not oid:
        return ""
    try:
        return subprocess.run(
            ["git", "rev-parse", f"{oid}^{{tree}}"], cwd=repository_root,
            check=True, capture_output=True, text=True,
        ).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        return ""


def _oid(value: Any) -> str | None:
    return value.get("bytes") if isinstance(value, dict) else None


def _ranked_span(span: Span, exclusive: dict[str, int]) -> dict[str, Any]:
    return {
        "id": span.id,
        "name": span.name,
        "category": span.category,
        "source": span.source,
        "duration_ms": span.duration_ms,
        "exclusive_duration_ms": exclusive[span.id],
        "inclusive": span.container,
        "status": span.status,
    }


def _finding(
    code: str,
    title: str,
    duration_ms: int,
    rule: str,
    spans: list[Span],
) -> dict[str, Any]:
    return {
        "code": code,
        "title": title,
        "duration_ms": duration_ms,
        "rule": rule,
        "span_ids": [span.id for span in spans],
    }


def _human_duration(duration_ms: int) -> str:
    seconds = duration_ms / 1000
    if seconds < 60:
        return f"{seconds:.1f}s"
    return f"{int(seconds // 60)}m {seconds % 60:.0f}s"
