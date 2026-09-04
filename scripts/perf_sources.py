#!/usr/bin/env python3

"""Read Codex, CI, and Tollgate performance sources without mutating them."""

from __future__ import annotations

import base64
import binascii
from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path
import re
import sqlite3
import subprocess
from typing import Any

from perf_log import normalize_repository_url
from perf_model import parse_timestamp, SessionTrace, Span


UUID_PATTERN = re.compile(
    r"\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b",
    re.IGNORECASE,
)


@dataclass(frozen=True)
class ThreadRecord:
    """Indexed metadata needed to find and order one Codex rollout."""

    id: str
    rollout_path: Path
    title: str
    repository_url: str
    updated_at_ms: int
    parent_thread_id: str | None = None


def codex_root() -> Path:
    """Return the configured Codex state root."""
    return Path(os.environ.get("CODEX_HOME", Path.home() / ".codex"))


def discover_codex_threads(
    root: Path,
    repository_url: str,
) -> tuple[dict[str, ThreadRecord], dict[str, list[str]], list[str]]:
    """Discover repository threads from SQLite, falling back to rollouts."""
    warnings: list[str] = []
    try:
        records, children = _discover_from_database(root, repository_url)
        if records:
            return records, children, warnings
    except (OSError, sqlite3.Error) as error:
        warnings.append(f"Codex thread database unavailable: {error}")
    records, children = _discover_from_rollouts(root, repository_url, warnings)
    return records, children, warnings


def _discover_from_database(
    root: Path,
    repository_url: str,
) -> tuple[dict[str, ThreadRecord], dict[str, list[str]]]:
    sqlite_root = Path(os.environ.get("CODEX_SQLITE_HOME", root))
    database = sqlite_root / "state_5.sqlite"
    connection = sqlite3.connect(f"file:{database}?mode=ro", uri=True)
    try:
        rows = connection.execute(
            "SELECT id, rollout_path, title, git_origin_url, updated_at_ms "
            "FROM threads ORDER BY updated_at_ms DESC"
        ).fetchall()
        edges = connection.execute(
            "SELECT parent_thread_id, child_thread_id FROM thread_spawn_edges"
        ).fetchall()
    finally:
        connection.close()
    parents = {child: parent for parent, child in edges}
    records = {}
    for thread_id, rollout, title, remote, updated_at_ms in rows:
        normalized = normalize_repository_url(remote)
        if normalized != repository_url:
            continue
        records[thread_id] = ThreadRecord(
            thread_id,
            Path(rollout),
            title or thread_id,
            normalized,
            int(updated_at_ms or 0),
            parents.get(thread_id),
        )
    children: dict[str, list[str]] = {}
    for child, parent in parents.items():
        if child in records and parent in records:
            children.setdefault(parent, []).append(child)
    return records, children


def _discover_from_rollouts(
    root: Path,
    repository_url: str,
    warnings: list[str],
) -> tuple[dict[str, ThreadRecord], dict[str, list[str]]]:
    records = {}
    children: dict[str, list[str]] = {}
    paths = [*(root / "sessions").glob("**/*.jsonl")]
    paths.extend((root / "archived_sessions").glob("*.jsonl"))
    for path in paths:
        try:
            with path.open(encoding="utf-8") as source:
                first = json.loads(source.readline())
        except (OSError, json.JSONDecodeError):
            continue
        if not isinstance(first, dict):
            continue
        if first.get("type") != "session_meta":
            continue
        payload = first.get("payload", {})
        if not isinstance(payload, dict):
            continue
        normalized = normalize_repository_url(payload.get("git", {}).get("repository_url"))
        if normalized != repository_url:
            continue
        thread_id = payload.get("id") or payload.get("session_id")
        if not thread_id:
            continue
        parent = payload.get("parent_thread_id")
        records[thread_id] = ThreadRecord(
            thread_id,
            path,
            thread_id,
            normalized,
            round(path.stat().st_mtime * 1000),
            parent,
        )
        if parent:
            children.setdefault(parent, []).append(thread_id)
    warnings.append("Codex sessions were discovered by scanning rollout files.")
    return records, children


def load_session_tree(
    root: ThreadRecord,
    records: dict[str, ThreadRecord],
    children: dict[str, list[str]],
) -> SessionTrace:
    """Load a top-level Codex session and fold all descendant agents into it."""
    session = parse_codex_rollout(root)
    for child_id in children.get(root.id, []):
        child_record = records.get(child_id)
        if child_record is None:
            continue
        child = load_session_tree(child_record, records, children)
        _fold_child(session, child)
    return session


def parse_codex_rollout(record: ThreadRecord) -> SessionTrace:
    """Parse one rollout into normalized turns, tools, and transcript content."""
    session = SessionTrace(
        record.id,
        record.title,
        record.rollout_path,
        record.repository_url,
        record.parent_thread_id,
    )
    pending_turns: dict[str, float] = {}
    pending_tools: dict[str, tuple[float, dict[str, Any]]] = {}
    lifecycle: list[tuple[str, str, bool]] = []
    try:
        source = record.rollout_path.open(encoding="utf-8")
    except OSError as error:
        session.warnings.append(f"Could not read {record.rollout_path}: {error}")
        return session
    with source:
        for line_number, line in enumerate(source, 1):
            try:
                entry = json.loads(line)
            except json.JSONDecodeError:
                session.warnings.append(
                    f"Ignored invalid JSON at {record.rollout_path}:{line_number}"
                )
                continue
            if not isinstance(entry, dict):
                session.warnings.append(
                    f"Ignored non-object record at {record.rollout_path}:{line_number}"
                )
                continue
            timestamp = parse_timestamp(entry.get("timestamp"))
            if timestamp is not None:
                session.latest_event_at = max(session.latest_event_at or timestamp, timestamp)
            _parse_codex_entry(
                session,
                entry,
                timestamp,
                pending_turns,
                pending_tools,
                lifecycle,
            )
    for call_id, (started, payload) in pending_tools.items():
        session.warnings.append(f"Tool call {call_id} has no recorded output.")
        if session.latest_event_at is not None:
            session.spans.append(_tool_span(session, payload, started, session.latest_event_at, {}))
    if lifecycle:
        kind, _turn_id, successful = lifecycle[-1]
        session.completed = kind == "complete" and successful
    return session


def _parse_codex_entry(
    session: SessionTrace,
    entry: dict[str, Any],
    timestamp: float | None,
    pending_turns: dict[str, float],
    pending_tools: dict[str, tuple[float, dict[str, Any]]],
    lifecycle: list[tuple[str, str, bool]],
) -> None:
    payload = entry.get("payload", {})
    if not isinstance(payload, dict):
        session.warnings.append("Ignored a Codex record with a non-object payload.")
        return
    entry_type = entry.get("type")
    if entry_type == "session_meta":
        session.agent_name = payload.get("agent_nickname")
        session.agent_path = payload.get("agent_path")
        return
    if entry_type == "token_usage_record":
        session.token_usage = sanitize(payload.get("thread_token_usage") or payload.get("usage") or {})
        return
    if entry_type == "event_msg":
        _parse_lifecycle(session, payload, timestamp, pending_turns, lifecycle)
        return
    if entry_type != "response_item" or timestamp is None:
        return
    payload_type = payload.get("type")
    if payload_type in {"custom_tool_call", "function_call"}:
        call_id = payload.get("call_id") or payload.get("id")
        if call_id:
            pending_tools[call_id] = (timestamp, payload)
        return
    if payload_type in {"custom_tool_call_output", "function_call_output"}:
        call_id = payload.get("call_id")
        pending = pending_tools.pop(call_id, None)
        if pending is not None:
            started, call = pending
            session.spans.append(_tool_span(session, call, started, timestamp, payload))
            _collect_candidate_ids(session, call, payload)
        return
    if payload_type == "message":
        role = payload.get("role", "unknown")
        if role in {"developer", "system"}:
            return
        session.transcript.append(
            {
                "timestamp": entry.get("timestamp"),
                "kind": "message",
                "role": role,
                "content": sanitize(payload.get("content")),
            }
        )
        if role == "user" and session.first_user_at is None:
            session.first_user_at = timestamp
        return
    if payload_type == "reasoning":
        session.transcript.append(
            {
                "timestamp": entry.get("timestamp"),
                "kind": "reasoning_summary",
                "content": sanitize(payload.get("summary")),
            }
        )


def _parse_lifecycle(
    session: SessionTrace,
    payload: dict[str, Any],
    timestamp: float | None,
    pending_turns: dict[str, float],
    lifecycle: list[tuple[str, str, bool]],
) -> None:
    event = payload.get("type")
    turn_id = payload.get("turn_id")
    if event == "task_started" and turn_id:
        started = parse_timestamp(payload.get("started_at")) or timestamp
        if started is not None:
            pending_turns[turn_id] = started
            lifecycle.append(("started", turn_id, False))
        return
    if event != "task_complete" or not turn_id:
        return
    finished = parse_timestamp(payload.get("completed_at")) or timestamp
    started = parse_timestamp(payload.get("started_at")) or pending_turns.pop(turn_id, None)
    if started is None or finished is None:
        return
    error = payload.get("error")
    successful = error is None or error == ""
    session.spans.append(
        Span(
            f"turn:{turn_id}",
            None,
            session.thread_id,
            "codex",
            "agent",
            "Agent turn",
            started,
            finished,
            "passed" if successful else "failed",
            container=True,
        )
    )
    lifecycle.append(("complete", turn_id, successful))
    if successful:
        session.completed_at = max(session.completed_at or finished, finished)
    first_token = payload.get("time_to_first_token_ms")
    if isinstance(first_token, int):
        session.time_to_first_token_ms.append(first_token)


def _tool_span(
    session: SessionTrace,
    call: dict[str, Any],
    started: float,
    finished: float,
    output: dict[str, Any],
) -> Span:
    name = call.get("name") or "tool"
    metadata = call.get("internal_chat_message_metadata_passthrough", {})
    turn_id = metadata.get("turn_id")
    category = "wait" if _is_wait_tool(name, call.get("input")) else "tool"
    return Span(
        f"tool:{call.get('call_id') or call.get('id')}",
        f"turn:{turn_id}" if turn_id else None,
        session.thread_id,
        "codex",
        category,
        name,
        started,
        finished,
        "passed" if output else "incomplete",
        attributes={"tool_name": name},
        content={
            "input": sanitize(call.get("input")),
            "output": sanitize(output.get("output")),
        },
    )


def _is_wait_tool(name: str, raw_input: Any) -> bool:
    lowered = name.casefold()
    if "wait" in lowered or lowered == "request_user_input":
        return True
    text = flatten_text(raw_input).casefold()
    return lowered == "exec" and ("tg wait" in text or "wait_agent" in text)


def _collect_candidate_ids(
    session: SessionTrace,
    call: dict[str, Any],
    output: dict[str, Any],
) -> None:
    input_text = flatten_text(call.get("input")).casefold()
    if re.search(r"\btg\b[^\n]*\bcandidate\b", input_text) is None:
        return
    session.candidate_ids.update(_candidate_result_ids(output.get("output")))


def _candidate_result_ids(value: Any) -> set[str]:
    ids: set[str] = set()
    for candidate in _structured_json_values(value):
        if not isinstance(candidate, dict) or "source_oid" not in candidate:
            continue
        item_id = candidate.get("item_id")
        if isinstance(item_id, str) and UUID_PATTERN.fullmatch(item_id):
            ids.add(item_id)
    return ids


def _structured_json_values(value: Any) -> list[Any]:
    if isinstance(value, dict):
        return [value, *(item for child in value.values() for item in _structured_json_values(child))]
    if isinstance(value, list):
        return [item for child in value for item in _structured_json_values(child)]
    if not isinstance(value, str):
        return []
    decoder = json.JSONDecoder()
    decoded = []
    for match in re.finditer(r"[\[{]", value):
        try:
            item, _end = decoder.raw_decode(value, match.start())
        except json.JSONDecodeError:
            continue
        decoded.append(item)
    return decoded


def _fold_child(parent: SessionTrace, child: SessionTrace) -> None:
    starts = [span.started_at for span in child.spans]
    finishes = [span.finished_at for span in child.spans]
    if starts and finishes:
        summary_id = f"subagent:{child.thread_id}"
        parent.spans.append(
            Span(
                summary_id,
                None,
                parent.thread_id,
                "codex",
                "subagent",
                child.agent_path or child.agent_name or child.title,
                min(starts),
                max(finishes),
                "passed" if child.completed else "incomplete",
                attributes={
                    "child_thread_id": child.thread_id,
                    "agent_name": child.agent_name,
                    "agent_path": child.agent_path,
                },
                container=True,
            )
        )
        for span in child.spans:
            if span.parent_id is None:
                span.parent_id = summary_id
            parent.spans.append(span)
    parent.transcript.extend(
        {**item, "session_id": child.thread_id} for item in child.transcript
    )
    parent.warnings.extend(child.warnings)
    parent.candidate_ids.update(child.candidate_ids)


def read_ci_traces(log_root: Path) -> tuple[list[Span], list[str]]:
    """Read every retained CI JSONL file into run and child spans."""
    spans: list[Span] = []
    warnings: list[str] = []
    for path in sorted((log_root / "ci").glob("**/*.jsonl")):
        records = _read_jsonl(path, warnings)
        if not records:
            continue
        started = records[0]
        run_id = started.get("run_id")
        start_time = parse_timestamp(started.get("timestamp"))
        last_time = parse_timestamp(records[-1].get("timestamp"))
        if not run_id or start_time is None or last_time is None:
            continue
        finished_record = next(
            (record for record in reversed(records) if record.get("event") == "ci.run_finished"),
            None,
        )
        finish_time = (
            parse_timestamp(finished_record.get("timestamp"))
            if finished_record is not None
            else last_time
        )
        metadata = {
            key: value
            for key, value in started.items()
            if key not in {"event", "timestamp", "run_id"}
        }
        metadata["source_path"] = str(path)
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
                finished_record.get("outcome", "incomplete") if finished_record else "incomplete",
                attributes=metadata,
                container=True,
            )
        )
        _append_ci_children(spans, records, run_id, metadata)
    return spans, warnings


def _append_ci_children(
    spans: list[Span],
    records: list[dict[str, Any]],
    run_id: str,
    metadata: dict[str, Any],
) -> None:
    starts = {
        record.get("span_id"): record
        for record in records
        if record.get("event") == "ci.step_started"
    }
    for record in records:
        event = record.get("event")
        if event == "ci.step_finished" and record.get("span_id") in starts:
            start = starts[record["span_id"]]
            started_at = parse_timestamp(start.get("timestamp"))
            finished_at = parse_timestamp(record.get("timestamp"))
            if started_at is None or finished_at is None:
                continue
            parent = record.get("parent_span_id")
            spans.append(
                Span(
                    f"ci-step:{record['span_id']}",
                    f"ci-run:{run_id}" if parent == run_id else f"ci-step:{parent}",
                    None,
                    "ci",
                    "ci",
                    record.get("name", "CI step"),
                    started_at,
                    finished_at,
                    record.get("outcome", "unknown"),
                    attributes={"run_id": run_id, **metadata},
                )
            )
        elif event in {"ci.cache_lookup", "ci.cache_wait", "ci.cache_maintenance"}:
            finished_at = parse_timestamp(record.get("timestamp"))
            if finished_at is None:
                continue
            duration_ms = int(record.get("duration_ms") or 0)
            parent = record.get("parent_span_id")
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
                    attributes={**record, **metadata},
                )
            )


def read_tollgate(repository_root: Path) -> tuple[list[Span], list[str], list[dict[str, Any]]]:
    """Query current Tollgate state and retained history without launching it."""
    warnings: list[str] = []
    try:
        status = _run_json_command(
            ["tg", "status", "--json", "--no-launch"], repository_root
        )
        history = _run_json_command(
            ["tg", "history", "--json", "--no-launch"], repository_root
        )
    except (OSError, subprocess.SubprocessError, json.JSONDecodeError) as error:
        return [], [f"Tollgate data unavailable: {error}"], []
    if not isinstance(status, dict) or not isinstance(history, list):
        return [], ["Tollgate returned an unsupported JSON shape."], []
    items_by_id = {}
    for section in ("queue", "checks", "history_items"):
        entries = status.get(section, [])
        if not isinstance(entries, list):
            warnings.append(f"Ignored unsupported Tollgate {section} data.")
            continue
        for entry in entries:
            if not isinstance(entry, dict):
                warnings.append(f"Ignored a non-object Tollgate {section} record.")
                continue
            item = entry.get("item", {})
            if not isinstance(item, dict):
                warnings.append(f"Ignored a Tollgate {section} record without an item object.")
                continue
            if item.get("id"):
                items_by_id[item["id"]] = entry
    spans: list[Span] = []
    candidates: list[dict[str, Any]] = []
    history_by_candidate: dict[str, list[dict[str, Any]]] = {}
    for event in history:
        if not isinstance(event, dict):
            warnings.append("Ignored a non-object Tollgate history record.")
            continue
        payload = event.get("payload", {})
        if not isinstance(payload, dict):
            warnings.append("Ignored a Tollgate history record with a non-object payload.")
            continue
        candidate_id = payload.get("item_id") or payload.get("id")
        if candidate_id:
            history_by_candidate.setdefault(candidate_id, []).append(event)
    for candidate_id, entry in items_by_id.items():
        item = entry.get("item", {})
        buildset = entry.get("buildset") or {}
        if not isinstance(buildset, dict):
            warnings.append(f"Ignored a non-object buildset for candidate {candidate_id}.")
            buildset = {}
        raw_attempts = entry.get("attempts") or []
        if not isinstance(raw_attempts, list):
            warnings.append(f"Ignored non-list attempts for candidate {candidate_id}.")
            raw_attempts = []
        attempts = [
            attempt for attempt in raw_attempts if isinstance(attempt, dict)
        ]
        if buildset and not any(attempt.get("id") == buildset.get("id") for attempt in attempts):
            attempts.append(buildset)
        history_events = history_by_candidate.get(candidate_id, [])
        candidates.append(
            {
                "item": item,
                "buildset": buildset,
                "attempts": attempts,
                "history": history_events,
            }
        )
        _append_tollgate_spans(spans, item, attempts, history_events)
    return spans, warnings, candidates


def _append_tollgate_spans(
    spans: list[Span],
    item: dict[str, Any],
    attempts: list[dict[str, Any]],
    history: list[dict[str, Any]],
) -> None:
    candidate_id = item.get("id")
    finishes = []
    for buildset in attempts:
        buildset_id = buildset.get("id")
        created = parse_timestamp(buildset.get("created_at"))
        started = parse_timestamp(buildset.get("started_at"))
        finished = parse_timestamp(buildset.get("finished_at"))
        attributes = {
            "candidate_id": candidate_id,
            "buildset_id": buildset_id,
            "source_oid": _oid(item.get("source_oid")),
            "tested_oid": _oid(buildset.get("tested_oid")),
            "attempt": buildset.get("attempt"),
            "candidate_state": item.get("state"),
            "attempt_state": buildset.get("state"),
        }
        if created is not None and started is not None:
            spans.append(
                Span(
                    f"tg-queue:{buildset_id}", None, None, "tollgate", "wait",
                    "Tollgate queue", created, started, "passed", attributes=attributes,
                )
            )
        if started is not None and finished is not None:
            finishes.append(finished)
            spans.append(
                Span(
                    f"tg-buildset:{buildset_id}", None, None, "tollgate", "ci",
                    f"Tollgate buildset attempt {buildset.get('attempt', 1)}",
                    started, finished, buildset.get("state", "unknown"),
                    attributes=attributes, container=True,
                )
            )
            for result in buildset.get("step_results", []):
                if not isinstance(result, dict):
                    continue
                duration_ms = int(result.get("elapsed_ms") or 0)
                spans.append(
                    Span(
                        f"tg-step:{buildset_id}:{result.get('name')}",
                        f"tg-buildset:{buildset_id}", None, "tollgate", "ci",
                        f"Tollgate: {result.get('name', 'step')}",
                        finished - duration_ms / 1000, finished,
                        result.get("result_class", "unknown"), attributes=attributes,
                    )
                )
    authorized = parse_timestamp(item.get("promotion_authorized_at"))
    finished = max(finishes, default=None)
    lifecycle_attributes = {"candidate_id": candidate_id, "candidate_state": item.get("state")}
    if finished is not None and authorized is not None and authorized > finished:
        spans.append(
            Span(
                f"tg-authorization:{candidate_id}", None, None, "tollgate", "wait",
                "Tollgate authorization wait", finished, authorized, "passed",
                attributes=lifecycle_attributes,
            )
        )
    completion = next(
        (parse_timestamp(event.get("created_at")) for event in history if event.get("kind") == "promotion.completed"),
        None,
    )
    if authorized is not None and completion is not None and completion > authorized:
        spans.append(
            Span(
                f"tg-promotion:{candidate_id}", None, None, "tollgate", "wait",
                "Tollgate promotion", authorized, completion, "passed",
                attributes=lifecycle_attributes,
            )
        )


def _run_json_command(command: list[str], cwd: Path) -> Any:
    result = subprocess.run(
        command,
        cwd=cwd,
        check=True,
        capture_output=True,
        text=True,
        timeout=10,
    )
    return json.loads(result.stdout)


def _oid(value: Any) -> str | None:
    return value.get("bytes") if isinstance(value, dict) else None


def _read_jsonl(path: Path, warnings: list[str]) -> list[dict[str, Any]]:
    records = []
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
    return records


def sanitize(value: Any) -> Any:
    """Preserve text and structured data while replacing unsafe large blobs."""
    if isinstance(value, str) and value.startswith("data:"):
        return {"data_url_metadata": _data_url_metadata(value)}
    if isinstance(value, list):
        return [sanitize(item) for item in value]
    if not isinstance(value, dict):
        return value
    result = {}
    for key, child in value.items():
        if key == "encrypted_content":
            continue
        if key in {"data", "blob"} and isinstance(child, str):
            encoded = child.encode(errors="replace")
            result[f"{key}_metadata"] = {
                "size": len(encoded),
                "sha256": hashlib.sha256(encoded).hexdigest(),
            }
            continue
        result[key] = sanitize(child)
    return result


def _data_url_metadata(value: str) -> dict[str, Any]:
    header, separator, payload = value.partition(",")
    mime_type = header[5:].partition(";")[0] or "text/plain"
    encoded = payload.encode(errors="replace") if separator else value.encode(errors="replace")
    decoded = encoded
    encoding = "text"
    if separator and ";base64" in header.casefold():
        try:
            decoded = base64.b64decode(encoded, validate=True)
            encoding = "base64"
        except (binascii.Error, ValueError):
            encoding = "invalid-base64"
    return {
        "mime_type": mime_type,
        "encoding": encoding,
        "size": len(decoded),
        "sha256": hashlib.sha256(decoded).hexdigest(),
    }


def flatten_text(value: Any) -> str:
    """Return searchable text from nested tool payloads."""
    if isinstance(value, str):
        return value
    if isinstance(value, list):
        return "\n".join(flatten_text(item) for item in value)
    if isinstance(value, dict):
        return "\n".join(flatten_text(item) for item in value.values())
    return ""
