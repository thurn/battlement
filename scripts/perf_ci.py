#!/usr/bin/env python3

"""Read retained CI, operation, and workflow performance records."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from perf_model import parse_timestamp, Span


_KNOWN_OUTCOMES = frozenset({
    "passed", "failed", "canceled", "cancelled", "unknown", "incomplete",
    "running", "success", "skipped", "interrupted",
})


@dataclass(frozen=True)
class _CIRecord:
    raw: dict[str, Any]
    event: str
    timestamp: float | None
    span_id: Any
    parent_span_id: Any
    duration_ms: int | None


@dataclass(frozen=True)
class _OperationEvent:
    raw: dict[str, Any]
    source_index: int
    event: str
    timestamp: float
    resource: str
    process_key: str
    executable: Any
    exit_code: Any


@dataclass(frozen=True)
class _WorkflowEvent:
    workflow_id: str
    milestone: str
    timestamp: float
    outcome: Any
    attributes: dict[str, Any]


def read_ci_traces(log_root: Path) -> tuple[list[Span], list[str]]:
    """Read every retained CI JSONL file into run and child spans."""
    spans: list[Span] = []
    warnings: list[str] = []
    for path in sorted((log_root / "ci").glob("**/*.jsonl")):
        records, source_warnings = _read_jsonl(path)
        parsed, parse_warnings = parse_ci_records(records, path)
        spans.extend(parsed)
        warnings.extend(source_warnings)
        warnings.extend(parse_warnings)
    return spans, warnings


def parse_ci_records(
    records: list[dict[str, Any]], source_path: Path,
) -> tuple[list[Span], list[str]]:
    """Normalize decoded CI records and return spans with parsing warnings."""
    spans: list[Span] = []
    warnings: list[str] = []
    if not records:
        return spans, warnings
    decoded = [_normalize_ci_record(record, warnings) for record in records]
    started = decoded[0]
    run_id = started.raw.get("run_id")
    start_time = started.timestamp
    last_time = decoded[-1].timestamp
    if not run_id or start_time is None or last_time is None:
        return spans, warnings
    finished_record = next(
        (record for record in reversed(decoded) if record.event == "ci.run_finished"),
        None,
    )
    finish_time = (
        finished_record.timestamp
        if finished_record is not None
        else last_time
    )
    metadata = {
        key: value
        for key, value in started.raw.items()
        if key not in {"event", "timestamp", "run_id"}
    }
    metadata["source_path"] = str(source_path)
    evidence = []
    for record in decoded:
        if record.event != "ditto.invocation":
            continue
        reference = {
            key: record.raw.get(key)
            for key in ("invocation_id", "artifact_root", "evidence_path")
        }
        reference["integrity"] = "not_checked"
        evidence_path = Path(reference["evidence_path"] or "")
        if not evidence_path.is_file():
            reference["integrity"] = "missing"
            warnings.append(f"CI run {run_id} has missing Ditto evidence: {evidence_path}")
        evidence.append(reference)
    metadata["ditto_evidence"] = evidence
    spans.append(
        Span(
            f"ci-run:{run_id}",
            None,
            None,
            "ci",
            "ci",
            "ci.py --full" if metadata.get("full") else "ci.py",
            start_time,
            finish_time or last_time,
            finished_record.raw.get("outcome", "incomplete")
            if finished_record else "incomplete",
            attributes=metadata,
            container=True,
        )
    )
    _append_ci_children(spans, decoded, run_id, metadata)
    return spans, warnings


def _normalize_ci_record(record: dict[str, Any], warnings: list[str]) -> _CIRecord:
    event = record.get("event", "")
    duration = record.get("duration_ms")
    if duration is None:
        duration_ms = None
        if event in {"ci.cache_lookup", "ci.cache_wait", "ci.cache_maintenance"}:
            warnings.append("Ignored a CI cache event without duration evidence.")
    else:
        try:
            if isinstance(duration, bool):
                raise ValueError
            duration_ms = int(duration)
        except (TypeError, ValueError, OverflowError):
            duration_ms = None
            warnings.append(f"Ignored invalid CI duration {duration!r}.")
    outcome = record.get("outcome")
    if (
        event in {"ci.step_finished", "ci.run_finished"}
        and isinstance(outcome, str)
        and outcome not in _KNOWN_OUTCOMES
    ):
        warnings.append(f"Accepted unknown CI outcome {outcome!r}.")
    return _CIRecord(
        record,
        event,
        parse_timestamp(record.get("timestamp")),
        record.get("span_id"),
        record.get("parent_span_id"),
        duration_ms,
    )


def read_operation_traces(
    log_root: Path, known_operations: dict[str, str] | None = None,
) -> tuple[list[Span], list[str]]:
    """Read retained non-CI operation events without duplicating CI spans."""
    known_operations = known_operations or {}
    spans: list[Span] = []
    warnings: list[str] = []
    for path in sorted((log_root / "operations").glob("**/*.jsonl")):
        records, source_warnings = _read_jsonl(path)
        parsed, parse_warnings = parse_operation_records(records, path, known_operations)
        spans.extend(parsed)
        warnings.extend(source_warnings)
        warnings.extend(parse_warnings)
    return spans, warnings


def parse_operation_records(
    records: list[dict[str, Any]], source_path: Path,
    known_operations: dict[str, str] | None = None,
) -> tuple[list[Span], list[str]]:
    """Normalize decoded operation records and return spans with warnings."""
    known_operations = known_operations or {}
    spans: list[Span] = []
    warnings: list[str] = []
    if not records:
        return spans, warnings
    started = next(
        (record for record in records if record.get("event") == "operation.started"),
        None,
    )
    if started is None or not started.get("operation_id"):
        warnings.append(f"Operation trace has no start event: {source_path}")
        return spans, warnings
    operation_id = started["operation_id"]
    if operation_id in known_operations:
        return spans, warnings
    start_time = parse_timestamp(started.get("timestamp"))
    last_time = parse_timestamp(records[-1].get("timestamp"))
    if start_time is None or last_time is None:
        warnings.append(f"Operation trace has invalid timestamps: {source_path}")
        return spans, warnings
    terminal = next(
        (record for record in reversed(records) if record.get("event") == "operation.finished"),
        None,
    )
    finish_time = parse_timestamp(terminal.get("timestamp")) if terminal else last_time
    if finish_time is None:
        finish_time = last_time
    if terminal is None:
        warnings.append(f"Operation {operation_id} has no terminal event: {source_path}")
    elif (
        isinstance(terminal.get("outcome"), str)
        and terminal["outcome"] not in _KNOWN_OUTCOMES
    ):
        warnings.append(f"Accepted unknown operation outcome {terminal['outcome']!r}.")
    context = started.get("context") if isinstance(started.get("context"), dict) else {}
    metadata = started.get("metadata") if isinstance(started.get("metadata"), dict) else {}
    attributes = {
        **context,
        **metadata,
        "root_operation_id": context.get("root_operation_id"),
        "operation_id": operation_id,
        "source_path": str(source_path),
        "process": started.get("process"),
        "clock_domain": started.get("clock_domain"),
        "events": [
            {
                key: value for key, value in record.items()
                if key not in {"schema", "operation_id", "monotonic_ns"}
            }
            for record in records
            if record.get("event") in {
                "preparation.inputs", "cache.lookup", "artifact.published",
                "review.build_identified", "review.ready", "review.inputs_invalidated",
            }
        ],
    }
    parent = started.get("parent_operation_id")
    parent_id = known_operations.get(parent, f"operation:{parent}" if parent else None)
    spans.append(Span(
        f"operation:{operation_id}", parent_id, None, "operation", "operation",
        started.get("name", "Operation"), start_time, finish_time,
        terminal.get("outcome", "incomplete") if terminal else "incomplete",
        attributes=attributes, container=True,
    ))
    events = [
        event for index, record in enumerate(records)
        if (event := _normalize_operation_event(record, index)) is not None
    ]
    _append_operation_children(spans, events, operation_id, attributes, warnings)
    return spans, warnings


def _normalize_operation_event(record: dict[str, Any], source_index: int) -> _OperationEvent | None:
    timestamp = parse_timestamp(record.get("timestamp"))
    if timestamp is None:
        return None
    process = record.get("process")
    return _OperationEvent(
        record,
        source_index,
        record.get("event", ""),
        timestamp,
        str(record.get("resource", "unknown")),
        json.dumps(process, sort_keys=True),
        record.get("executable"),
        record.get("exit_code", 0),
    )


def read_workflow_milestones(log_root: Path) -> tuple[list[Span], list[str]]:
    """Read explicit task milestones without inferring intent from transcript prose."""
    spans: list[Span] = []
    warnings: list[str] = []
    for path in sorted((log_root / "workflows").glob("**/*.jsonl")):
        records, source_warnings = _read_jsonl(path)
        parsed, parse_warnings = parse_workflow_records(records, path)
        spans.extend(parsed)
        warnings.extend(source_warnings)
        warnings.extend(parse_warnings)
    return spans, warnings


def parse_workflow_records(
    records: list[dict[str, Any]], source_path: Path,
) -> tuple[list[Span], list[str]]:
    """Normalize decoded workflow records and return milestones with warnings."""
    spans: list[Span] = []
    warnings: list[str] = []
    for index, record in enumerate(records):
        event = _normalize_workflow_event(record, source_path, warnings)
        if event is None:
            continue
        spans.append(
            Span(
                f"workflow:{event.workflow_id}:{index}:{event.milestone}",
                None,
                None,
                "workflow",
                "milestone",
                event.milestone,
                event.timestamp,
                event.timestamp,
                event.outcome or "passed",
                attributes=event.attributes,
            )
        )
    return spans, warnings


def _normalize_workflow_event(
    record: dict[str, Any], source_path: Path, warnings: list[str],
) -> _WorkflowEvent | None:
    if record.get("event") != "workflow.milestone":
        warnings.append(f"Ignored unsupported workflow event in {source_path}.")
        return None
    workflow_id = record.get("workflow_id")
    milestone = record.get("milestone")
    timestamp = parse_timestamp(record.get("timestamp"))
    context = record.get("context")
    if (
        not isinstance(workflow_id, str)
        or not isinstance(milestone, str)
        or timestamp is None
        or not isinstance(context, dict)
    ):
        warnings.append(f"Ignored malformed workflow milestone in {source_path}.")
        return None
    outcome = record.get("outcome")
    if isinstance(outcome, str) and outcome not in _KNOWN_OUTCOMES:
        warnings.append(f"Accepted unknown workflow outcome {outcome!r}.")
    return _WorkflowEvent(
        workflow_id,
        milestone,
        timestamp,
        outcome,
        {
            **context,
            "workflow_id": workflow_id,
            "milestone": milestone,
            "candidate_id": record.get("candidate_id"),
            "job_id": record.get("job_id"),
            "clock_domain": record.get("clock_domain"),
            "source_path": str(source_path),
        },
    )


def _append_operation_children(
    spans: list[Span], records: list[_OperationEvent], operation_id: str,
    attributes: dict[str, Any], warnings: list[str],
) -> None:
    parent = f"operation:{operation_id}"
    queued: dict[str, list[_OperationEvent]] = {}
    held: dict[str, list[_OperationEvent]] = {}
    processes: dict[str, list[_OperationEvent]] = {}
    for index, record in enumerate(records):
        event = record.event
        resource = record.resource
        if event == "resource.queued":
            queued.setdefault(resource, []).append(record)
        elif event == "resource.acquired":
            start = queued.get(resource, []).pop(0) if queued.get(resource) else record
            _event_span(spans, parent, f"resource-queue:{operation_id}:{record.source_index}",
                        "wait", f"Wait for {resource}", start, record, attributes)
            held.setdefault(resource, []).append(record)
        elif event == "resource.released":
            start = held.get(resource, []).pop(0) if held.get(resource) else record
            _event_span(spans, parent, f"resource-held:{operation_id}:{record.source_index}",
                        "resource", f"Use {resource}", start, record, attributes)
        elif event == "process.started":
            key = record.process_key
            processes.setdefault(key, []).append(record)
        elif event == "process.finished":
            key = record.process_key
            start = processes.get(key, []).pop(0) if processes.get(key) else record
            _event_span(spans, parent, f"process:{operation_id}:{record.source_index}", "process",
                        record.executable
                        if record.executable is not None
                        else start.executable or "Child process",
                        start, record, attributes)
        elif event == "review.ready":
            milestone = _OperationEvent(
                {**record.raw, "milestone": "review.ready"},
                record.source_index,
                record.event,
                record.timestamp,
                record.resource,
                record.process_key,
                record.executable,
                record.exit_code,
            )
            _event_span(
                spans,
                parent,
                f"milestone:{operation_id}:{record.source_index}",
                "milestone",
                "review.ready",
                milestone,
                milestone,
                attributes,
            )
    for resource, starts in queued.items():
        for _start in starts:
            warnings.append(f"Operation {operation_id} has an unfinished {resource} queue wait.")
    for resource, starts in held.items():
        for _start in starts:
            warnings.append(f"Operation {operation_id} did not release {resource}.")
    for starts in processes.values():
        for _start in starts:
            warnings.append(f"Operation {operation_id} has an unfinished child process.")


def _event_span(
    spans: list[Span], parent: str, span_id: str, category: str, name: str,
    started: _OperationEvent, finished: _OperationEvent, attributes: dict[str, Any],
) -> None:
    spans.append(Span(
        span_id, parent, None, "operation", category, name,
        started.timestamp, finished.timestamp,
        "passed" if finished.exit_code == 0 else "failed",
        attributes={**attributes, **finished.raw},
    ))


def _append_ci_children(
    spans: list[Span],
    records: list[_CIRecord],
    run_id: str,
    metadata: dict[str, Any],
) -> None:
    starts = {
        record.span_id: record
        for record in records
        if record.event == "ci.step_started"
    }
    for record in records:
        event = record.event
        if event == "ci.step_finished" and record.span_id in starts:
            start = starts[record.span_id]
            started_at = start.timestamp
            finished_at = record.timestamp
            if started_at is None or finished_at is None:
                continue
            parent = record.parent_span_id
            spans.append(
                Span(
                    f"ci-step:{record.span_id}",
                    f"ci-run:{run_id}" if parent == run_id else f"ci-step:{parent}",
                    None,
                    "ci",
                    "ci",
                    record.raw.get("name", "CI step"),
                    started_at,
                    finished_at,
                    record.raw.get("outcome", "unknown"),
                    attributes={"run_id": run_id, **metadata},
                )
            )
        elif event in {"ci.cache_lookup", "ci.cache_wait", "ci.cache_maintenance"}:
            finished_at = record.timestamp
            if finished_at is None:
                continue
            duration_ms = record.duration_ms
            if duration_ms is None:
                continue
            parent = record.parent_span_id
            spans.append(
                Span(
                    f"ci-event:{run_id}:{len(spans)}",
                    f"ci-run:{run_id}" if parent == run_id else f"ci-step:{parent}",
                    None,
                    "ci",
                    "wait" if event == "ci.cache_wait" else "ci",
                    event.removeprefix("ci.").replace("_", " "),
                    finished_at - duration_ms / 1000,
                    finished_at,
                    "passed",
                    attributes={**record.raw, **metadata},
                )
            )


def _read_jsonl(path: Path) -> tuple[list[dict[str, Any]], list[str]]:
    """Read and decode a JSONL source while owning its file handle."""
    records: list[dict[str, Any]] = []
    warnings: list[str] = []
    try:
        with path.open(encoding="utf-8") as source:
            for line_number, line in enumerate(source, 1):
                try:
                    record = json.loads(line)
                except json.JSONDecodeError:
                    warnings.append(f"Ignored invalid JSON at {path}:{line_number}")
                    continue
                if not isinstance(record, dict):
                    warnings.append(f"Ignored non-object record at {path}:{line_number}")
                    continue
                records.append(record)
    except OSError as error:
        warnings.append(f"Could not read {path}: {error}")
    return records, warnings
