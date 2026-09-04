#!/usr/bin/env python3

"""Analyze recent Battlement CI, Codex, and Tollgate performance."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
from typing import Any

import perf_analysis
import perf_log
from perf_model import Thresholds
import perf_sources


REPOSITORY_ROOT = Path(__file__).resolve().parent.parent


def main(arguments: argparse.Namespace) -> Path:
    """Build, print, and save one deterministic performance report."""
    repository_url = _repository_url()
    records, children, warnings = perf_sources.discover_codex_threads(
        perf_sources.codex_root(), repository_url
    )
    sessions = _load_sessions(arguments, records, children)
    ci_spans, ci_warnings = perf_sources.read_ci_traces(perf_log.configured_log_root())
    warnings.extend(ci_warnings)
    tollgate_spans = []
    candidates = []
    if not arguments.no_tollgate:
        tollgate_spans, tollgate_warnings, candidates = perf_sources.read_tollgate(
            REPOSITORY_ROOT
        )
        warnings.extend(tollgate_warnings)
    perf_analysis.correlate_activity(
        sessions,
        ci_spans,
        tollgate_spans,
        candidates,
        REPOSITORY_ROOT,
        warnings,
    )
    sessions = _filter_explicit_selection(arguments, sessions)
    thresholds = Thresholds(
        round(arguments.slow_tool_seconds * 1000),
        round(arguments.slow_subagent_seconds * 1000),
        round(arguments.slow_ci_step_seconds * 1000),
        round(arguments.long_wait_seconds * 1000),
    )
    session_reports = [
        perf_analysis.analyze_session(session, thresholds, arguments.top)
        for session in sessions
    ]
    report = {
        "generated_at": perf_log.utc_now(),
        "selection": {
            "sessions": arguments.sessions,
            "thread": arguments.thread,
            "commit": arguments.commit,
            "candidate": arguments.candidate,
            "include_incomplete": arguments.include_incomplete,
            "top": arguments.top,
        },
        "thresholds_ms": {
            "slow_tool": thresholds.slow_tool_ms,
            "slow_subagent": thresholds.slow_subagent_ms,
            "slow_ci_step": thresholds.slow_ci_step_ms,
            "long_wait": thresholds.long_wait_ms,
        },
        "repository_url": repository_url,
        "warnings": warnings,
        "aggregate": perf_analysis.aggregate_reports(session_reports, arguments.top),
        "sessions": session_reports,
    }
    output = _output_path(arguments)
    _write_private_json(output, report)
    _print_report(report, output)
    return output


def parse_arguments() -> argparse.Namespace:
    """Parse the performance-report command line."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sessions", type=_positive_int, default=10)
    selector = parser.add_mutually_exclusive_group()
    selector.add_argument("--thread")
    selector.add_argument("--commit")
    selector.add_argument("--candidate")
    parser.add_argument("--include-incomplete", action="store_true")
    parser.add_argument("--top", type=_positive_int, default=10)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--no-tollgate", action="store_true")
    parser.add_argument("--slow-tool-seconds", type=_positive_float, default=30.0)
    parser.add_argument("--slow-subagent-seconds", type=_positive_float, default=300.0)
    parser.add_argument("--slow-ci-step-seconds", type=_positive_float, default=60.0)
    parser.add_argument("--long-wait-seconds", type=_positive_float, default=120.0)
    return parser.parse_args()


def _load_sessions(
    arguments: argparse.Namespace,
    records: dict[str, perf_sources.ThreadRecord],
    children: dict[str, list[str]],
) -> list[Any]:
    roots = sorted(
        (record for record in records.values() if record.parent_thread_id is None),
        key=lambda record: record.updated_at_ms,
        reverse=True,
    )
    if arguments.thread:
        selected = records.get(arguments.thread)
        roots = [selected] if selected is not None and selected.parent_thread_id is None else []
    scan_limit = 200 if arguments.commit or arguments.candidate else len(roots)
    sessions = []
    for record in roots[:scan_limit]:
        session = perf_sources.load_session_tree(record, records, children)
        if session.completed or arguments.include_incomplete:
            sessions.append(session)
        if not arguments.commit and not arguments.candidate and len(sessions) >= arguments.sessions:
            break
    return sessions


def _filter_explicit_selection(
    arguments: argparse.Namespace,
    sessions: list[Any],
) -> list[Any]:
    if arguments.candidate:
        return [
            session
            for session in sessions
            if arguments.candidate in session.candidate_ids
            or any(
                span.attributes.get("candidate_id") == arguments.candidate
                for span in session.spans
            )
        ]
    if arguments.commit:
        return [
            session
            for session in sessions
            if any(
                arguments.commit
                in {
                    span.attributes.get("head_oid"),
                    span.attributes.get("source_oid"),
                    span.attributes.get("tested_oid"),
                }
                for span in session.spans
            )
        ]
    return sessions


def _repository_url() -> str:
    result = subprocess.run(
        ["git", "remote", "get-url", "origin"],
        cwd=REPOSITORY_ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    return perf_log.normalize_repository_url(result.stdout)


def _output_path(arguments: argparse.Namespace) -> Path:
    if arguments.output is not None:
        return arguments.output.resolve()
    timestamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    if arguments.thread:
        selection = f"thread-{arguments.thread}"
    elif arguments.commit:
        selection = f"commit-{arguments.commit[:12]}"
    elif arguments.candidate:
        selection = f"candidate-{arguments.candidate}"
    else:
        selection = f"last-{arguments.sessions}"
    return perf_log.configured_log_root() / "reports" / f"{timestamp}-{selection}.json"


def _write_private_json(path: Path, report: dict[str, Any]) -> None:
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    path.parent.chmod(0o700)
    descriptor = os.open(path, os.O_CREAT | os.O_TRUNC | os.O_WRONLY, 0o600)
    os.fchmod(descriptor, 0o600)
    with os.fdopen(descriptor, "w", encoding="utf-8") as output:
        json.dump(report, output, indent=2, sort_keys=True)
        output.write("\n")


def _print_report(report: dict[str, Any], output: Path) -> None:
    aggregate = report["aggregate"]
    print(f"Performance report: {aggregate['session_count']} completed sessions")
    print(
        f"Wall time {_duration(aggregate['total_wall_time_ms'])} · "
        f"active {_duration(aggregate['total_active_coverage_ms'])} · "
        f"known waits {_duration(aggregate['total_known_wait_ms'])} · "
        f"unattributed agent-turn "
        f"{_duration(aggregate['total_unattributed_agent_turn_ms'])}"
    )
    _print_ranking("Longest operations", aggregate["longest_operations"], "duration_ms")
    _print_ranking(
        "Largest exclusive contributors",
        aggregate["largest_contributors"],
        "exclusive_duration_ms",
    )
    _print_ranking("Longest waits", aggregate["longest_waits"], "duration_ms")
    print("\nAggregate categories")
    for category, duration in sorted(
        aggregate["category_exclusive_ms"].items(),
        key=lambda item: item[1],
        reverse=True,
    ):
        print(f"  {_duration(duration):>9}  {category}")
    print("\nLongest tasks")
    for task in aggregate["longest_tasks"]:
        print(f"  {_duration(task['wall_time_ms']):>9}  {_single_line(task['title'])}")
    print("\nWorkflow findings")
    if not aggregate["findings"]:
        print("  None")
    displayed_findings = aggregate["findings"][: report["selection"]["top"]]
    for finding in displayed_findings:
        print(f"  {_duration(finding['duration_ms']):>9}  {finding['title']}")
    remaining = len(aggregate["findings"]) - len(displayed_findings)
    if remaining:
        print(f"  … {remaining} more findings in the JSON report")
    print("\nSessions")
    for session in report["sessions"]:
        timing = session["timing"]
        print(
            f"  {_duration(timing['wall_time_ms']):>9}  "
            f"{_single_line(session['metadata']['title'])} · "
            f"active {_duration(timing['recorded_active_coverage_ms'])} · "
            f"wait {_duration(timing['known_wait_union_ms'])} · "
            f"unattributed {_duration(timing['unattributed_agent_turn_ms'])} · "
            f"{len(session['findings'])} findings"
        )
    if report["warnings"]:
        print("\nData warnings")
        for warning in report["warnings"]:
            print(f"  - {warning}")
    print(f"\nSaved private JSON report: {output}")


def _print_ranking(title: str, entries: list[dict[str, Any]], field: str) -> None:
    print(f"\n{title}")
    if not entries:
        print("  None")
    for entry in entries:
        suffix = " (inclusive)" if entry.get("inclusive") else ""
        print(f"  {_duration(entry[field]):>9}  {entry['name']}{suffix}")


def _duration(milliseconds: int) -> str:
    seconds = milliseconds / 1000
    if seconds < 60:
        return f"{seconds:.1f}s"
    hours, remainder = divmod(seconds, 3600)
    minutes, remaining_seconds = divmod(remainder, 60)
    if hours >= 1:
        return f"{int(hours)}h {int(minutes):02d}m"
    return f"{int(minutes)}m {remaining_seconds:02.0f}s"


def _single_line(value: str, maximum: int = 100) -> str:
    text = " ".join(value.split())
    return text if len(text) <= maximum else f"{text[:maximum - 1]}…"


def _positive_int(value: str) -> int:
    parsed = int(value)
    if parsed < 1:
        raise argparse.ArgumentTypeError("must be positive")
    return parsed


def _positive_float(value: str) -> float:
    parsed = float(value)
    if parsed <= 0:
        raise argparse.ArgumentTypeError("must be positive")
    return parsed


if __name__ == "__main__":
    arguments = parse_arguments()
    log_root = perf_log.configured_log_root()
    try:
        with perf_log.retention_guard(log_root):
            output_path = main(arguments)
            perf_log.enforce_retention(
                log_root,
                perf_log.configured_max_log_bytes(),
                {output_path},
            )
    except (OSError, sqlite3.Error, subprocess.SubprocessError) as error:
        print(f"Performance report failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
