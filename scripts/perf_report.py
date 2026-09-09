#!/usr/bin/env python3

"""Analyze recent Battlement CI, Codex, and Tollgate performance."""

from __future__ import annotations

import argparse
from dataclasses import replace
from datetime import datetime, timedelta, timezone
from zoneinfo import ZoneInfo
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
from typing import Any

import perf_analysis
import perf_candidate
import perf_log
from perf_model import Thresholds
import perf_sources


REPOSITORY_ROOT = Path(__file__).resolve().parent.parent


def main(arguments: argparse.Namespace) -> Path:
    """Build, print, and save one deterministic performance report."""
    _observation_window(arguments)
    repository_url = _repository_url()
    records, children, warnings = perf_sources.discover_codex_threads(
        perf_sources.codex_root(), repository_url
    )
    if arguments.live_state:
        snapshot = json.loads(arguments.live_state.read_text())
        observed = datetime.fromisoformat(snapshot["observed_at"].replace("Z", "+00:00"))
        if observed.tzinfo is None or observed.timestamp() != arguments.cutoff:
            raise ValueError("Live-state snapshot must match --observed-at exactly")
        for thread_id, turn_id in snapshot["running_turns"].items():
            if thread_id in records:
                records[thread_id] = replace(records[thread_id], live_turn_id=turn_id,
                                             live_observed_at=arguments.cutoff)
    sessions = _load_sessions(arguments, records, children)
    ci_spans, ci_warnings = perf_sources.read_ci_traces(perf_log.configured_log_root())
    warnings.extend(ci_warnings)
    known_operations = {}
    for span in ci_spans:
        if span.id.startswith("ci-run:"):
            known_operations[span.id.removeprefix("ci-run:")] = span.id
        elif span.id.startswith("ci-step:"):
            known_operations[span.id.removeprefix("ci-step:")] = span.id
    operation_spans, operation_warnings = perf_sources.read_operation_traces(
        perf_log.configured_log_root(), known_operations,
    )
    warnings.extend(operation_warnings)
    workflow_spans, workflow_warnings = perf_sources.read_workflow_milestones(
        perf_log.configured_log_root()
    )
    warnings.extend(workflow_warnings)
    tollgate_spans = []
    candidates = []
    if not arguments.no_tollgate:
        tollgate_spans, tollgate_warnings, candidates = perf_sources.read_tollgate(
            REPOSITORY_ROOT
        )
        warnings.extend(tollgate_warnings)
    machine_operations = perf_analysis.correlate_activity(
        sessions,
        ci_spans,
        [*operation_spans, *workflow_spans],
        tollgate_spans,
        candidates,
        REPOSITORY_ROOT,
        warnings,
    )
    sessions = _filter_explicit_selection(arguments, sessions)
    candidate_report = None
    if arguments.candidate:
        candidate_report = perf_candidate.build(
            arguments.candidate,
            candidates,
            ci_spans,
            tollgate_spans,
            REPOSITORY_ROOT,
        )
        sessions = [
            perf_candidate.retain_exact_spans(session, candidate_report)
            for session in sessions
        ]
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
            "date": arguments.date,
            "timezone": arguments.timezone,
            "observation_cutoff": arguments.observed_at,
            "live_state_source": str(arguments.live_state) if arguments.live_state else None,
            "window_start": arguments.window_start,
            "window_end": arguments.window_end,
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
        "machine_operations": [span.as_dict() for span in machine_operations],
        "candidate": candidate_report,
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
    parser.add_argument("--date", help="Local calendar date (YYYY-MM-DD)")
    parser.add_argument("--timezone", default="UTC", help="IANA time zone for --date")
    parser.add_argument("--live-state", type=Path,
                        help='Live observer JSON: {"observed_at": ISO timestamp, "running_turns": {thread_id: turn_id}}; requires matching --observed-at')
    parser.add_argument("--observed-at", help="Immutable ISO-8601 observation cutoff with offset")
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
    arguments = parser.parse_args()
    try:
        _observation_window(arguments)
    except (ValueError, KeyError) as error:
        parser.error(str(error))
    return arguments


def _observation_window(arguments: argparse.Namespace) -> None:
    if not arguments.observed_at:
        arguments.observed_at = datetime.now(timezone.utc).isoformat()
    cutoff = datetime.fromisoformat(arguments.observed_at.replace("Z", "+00:00"))
    if cutoff.tzinfo is None:
        raise ValueError("--observed-at requires a time zone offset")
    zone = ZoneInfo(arguments.timezone)
    arguments.cutoff = cutoff.timestamp()
    arguments.window_start = None
    arguments.window_end = arguments.cutoff
    if arguments.date:
        start = datetime.strptime(arguments.date, "%Y-%m-%d").replace(tzinfo=zone)
        arguments.window_start = start.timestamp()
        arguments.window_end = min((start + timedelta(days=1)).timestamp(), arguments.cutoff)
        if arguments.window_end < arguments.window_start:
            raise ValueError("Observation cutoff precedes the selected date")


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
        session = perf_sources.load_session_tree(record, records, children, arguments.cutoff)
        session.window_start = arguments.window_start
        session.window_end = arguments.window_end
        if arguments.date:
            if session.latest_event_at is None or session.latest_event_at < arguments.window_start:
                continue
            if session.first_user_at is not None and session.first_user_at > arguments.window_end:
                continue
        if session.completed or arguments.include_incomplete:
            sessions.append(session)
        if not arguments.commit and not arguments.candidate and not arguments.date and len(sessions) >= arguments.sessions:
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
    missing = []
    parent = path.parent
    while not parent.exists():
        missing.append(parent)
        parent = parent.parent
    for directory in reversed(missing):
        directory.mkdir(mode=0o700, exist_ok=True)
    descriptor = os.open(path, os.O_CREAT | os.O_TRUNC | os.O_WRONLY, 0o600)
    os.fchmod(descriptor, 0o600)
    with os.fdopen(descriptor, "w", encoding="utf-8") as output:
        json.dump(report, output, indent=2, sort_keys=True)
        output.write("\n")


def _print_report(report: dict[str, Any], output: Path) -> None:
    aggregate = report["aggregate"]
    if report.get("candidate"):
        candidate = report["candidate"]
        print(
            f"Exact candidate {candidate['candidate_id']} · "
            f"source {candidate['source_oid']} · tree {candidate['source_tree_oid']}"
        )
        for run in candidate["ci_runs"]:
            mode = "full" if run["full"] else "quick"
            cache = run["cache"]
            print(
                f"  {mode} CI {run['run_id']} {_duration(run['duration_ms'])} · "
                f"{cache['hits']} hits · {cache['misses']} misses · "
                f"lock wait {_duration(cache['lock_wait_ms'])}"
            )
    print(f"Performance report: {aggregate['session_count']} sessions")
    print(" · ".join(f"{count} {status}" for status, count in aggregate["status_counts"].items()))
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
    _print_ci_hotspots(aggregate["ci_step_hotspots"])
    _print_tollgate_hotspots(aggregate["tollgate_phase_hotspots"])
    lifecycle = aggregate["lifecycle"]
    print(
        "\nLifecycle evidence\n"
        f"  {lifecycle['synchronized_delivery_count']}/{lifecycle['delivery_count']} "
        "candidate deliveries reached synchronized remote evidence"
    )
    for milestone, count in lifecycle["milestone_counts"].items():
        print(f"  {count:>9}  {milestone}")
    deliveries = lifecycle["deliveries"]
    selected_candidate = report["selection"].get("candidate")
    if selected_candidate:
        deliveries = [
            delivery for delivery in deliveries
            if delivery["candidate_id"] == selected_candidate
        ]
    for delivery in deliveries:
        print(f"  candidate {delivery['candidate_id']}")
        _print_delivery_timing(
            "submission → certification",
            delivery["submission_to_certification_ms"],
        )
        if delivery["authorization_to_certification_ms"] is not None:
            _print_delivery_timing(
                "authorization → certification",
                delivery["authorization_to_certification_ms"],
            )
        else:
            _print_delivery_timing(
                "certification → authorization",
                delivery["certification_to_authorization_ms"],
            )
        _print_delivery_timing("ready → remote", delivery["ready_to_remote_ms"])
        _print_delivery_timing(
            "local promotion → remote",
            delivery["local_promotion_to_remote_ms"],
        )
        _print_delivery_timing(
            "submission → remote", delivery["submission_to_remote_ms"]
        )
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


def _print_ci_hotspots(entries: list[dict[str, Any]]) -> None:
    print("\nCI step hotspots")
    if not entries:
        print("  None")
    for entry in entries:
        failures = entry["failed_count"]
        failure_text = f" · {failures} failed" if failures else ""
        run_label = "run" if entry["run_count"] == 1 else "runs"
        print(
            f"  {_duration(entry['total_duration_ms']):>9} total · "
            f"{_duration(entry['average_duration_ms'])} avg · "
            f"p95 {_duration(entry['p95_duration_ms'])} · "
            f"max {_duration(entry['max_duration_ms'])} · "
            f"{entry['run_count']} {run_label}{failure_text}  {entry['name']}"
        )


def _print_tollgate_hotspots(entries: list[dict[str, Any]]) -> None:
    print("\nTollgate promotion phase hotspots")
    if not entries:
        print("  None")
    for entry in entries:
        failures = entry["failed_count"]
        failure_text = f" · {failures} failed" if failures else ""
        print(
            f"  {_duration(entry['total_duration_ms']):>9} total · "
            f"{_duration(entry['average_duration_ms'])} avg · "
            f"p95 {_duration(entry['p95_duration_ms'])} · "
            f"max {_duration(entry['max_duration_ms'])} · "
            f"{entry['occurrence_count']} occurrence(s){failure_text}  {entry['name']}"
        )


def _print_delivery_timing(label: str, milliseconds: int | None) -> None:
    value = "unknown" if milliseconds is None else _duration(milliseconds)
    print(f"    {value:>9}  {label}")


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
    except (OSError, ValueError, KeyError, sqlite3.Error, subprocess.SubprocessError) as error:
        print(f"Performance report failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
