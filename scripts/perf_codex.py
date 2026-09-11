#!/usr/bin/env python3

"""Discover and parse Codex sessions into normalized performance spans."""

from __future__ import annotations

from dataclasses import dataclass
import json
import os
from pathlib import Path
import re
import sqlite3
from typing import Any

from perf_content import flatten_text, sanitize
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
    live_turn_id: str | None = None
    live_observed_at: float | None = None


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


def read_codex_rollout(path: Path) -> tuple[list[dict[str, Any]], list[str]]:
    """Decode one rollout file, returning records and source warnings."""
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


def load_session_tree(
    root: ThreadRecord,
    records: dict[str, ThreadRecord],
    children: dict[str, list[str]],
    observation_cutoff: float | None = None,
) -> SessionTrace:
    """Load a top-level Codex session and fold all descendant agents into it."""
    entries, warnings = read_codex_rollout(root.rollout_path)
    session = parse_codex_rollout(root, entries, observation_cutoff, warnings)
    for child_id in children.get(root.id, []):
        child_record = records.get(child_id)
        if child_record is None:
            continue
        child = load_session_tree(child_record, records, children, observation_cutoff)
        _fold_child(session, child)
    return session


def parse_codex_rollout(
    record: ThreadRecord,
    entries: list[dict[str, Any]],
    observation_cutoff: float | None = None,
    source_warnings: list[str] | tuple[str, ...] = (),
) -> SessionTrace:
    """Normalize decoded rollout records into turns, tools, and transcript content."""
    session = SessionTrace(
        record.id,
        record.title,
        record.rollout_path,
        record.repository_url,
        record.parent_thread_id,
    )
    session.observation_cutoff = observation_cutoff
    session.warnings.extend(source_warnings)
    pending_turns: dict[str, float] = {}
    pending_tools: dict[str, tuple[float, dict[str, Any]]] = {}
    lifecycle: list[tuple[str, str, bool]] = []
    for entry in entries:
        timestamp = parse_timestamp(entry.get("timestamp"))
        if timestamp is not None and observation_cutoff is not None:
            if timestamp > observation_cutoff:
                continue
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
    for turn_id, started in pending_turns.items():
        finished = session.latest_event_at if session.latest_event_at is not None else started
        known_live = record.live_turn_id == turn_id and record.live_observed_at == observation_cutoff
        if known_live and observation_cutoff is not None:
            finished = observation_cutoff
            session.latest_event_at = max(session.latest_event_at or finished, finished)
            session.status = "running"
        session.spans.append(Span(
            f"turn:{turn_id}", None, session.thread_id, "codex", "agent",
            "Agent turn", started, finished, "running" if known_live else "unknown", container=True,
            attributes={"missing_terminal": True, "live_state_evidence": known_live},
        ))
    if lifecycle:
        kind, _turn_id, successful = lifecycle[-1]
        session.completed = kind == "complete" and successful and not pending_turns
        if session.completed:
            session.status = "completed"
        elif kind == "interrupted":
            session.status = "interrupted"
    if pending_turns and session.status != "running":
        session.status = "unknown"
        session.warnings.append("Open turns have no live-state evidence; their unobserved tails are unknown.")
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
    if event not in {"task_complete", "turn_aborted", "task_aborted"}:
        return
    if not turn_id and len(pending_turns) == 1:
        turn_id = next(iter(pending_turns))
    if not turn_id:
        return
    finished = parse_timestamp(payload.get("completed_at")) or timestamp
    pending = pending_turns.pop(turn_id, None)
    started = parse_timestamp(payload.get("started_at"))
    if started is None:
        started = pending
    if started is None or finished is None:
        return
    successful = event == "task_complete" and not payload.get("error")
    session.spans.append(Span(
        f"turn:{turn_id}", None, session.thread_id, "codex", "agent",
        "Agent turn", started, finished, "passed" if successful else "interrupted",
        container=True,
    ))
    lifecycle.append(("complete" if successful else "interrupted", turn_id, successful))
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
    category = "wait" if _is_wait_tool(name, call.get("input", call.get("arguments"))) else "tool"
    return Span(
        f"tool:{call.get('call_id') or call.get('id')}",
        f"turn:{turn_id}" if turn_id else None,
        session.thread_id,
        "codex",
        category,
        name,
        started,
        finished,
        _tool_outcome(output),
        attributes={"tool_name": name},
        content={
            "input": sanitize(call.get("input", call.get("arguments"))),
            "output": sanitize(output.get("output")),
        },
    )


def _tool_outcome(output: dict[str, Any]) -> str:
    if not output:
        return "incomplete"
    values = _structured_json_values(output)
    codes = [value["exit_code"] for value in values
             if isinstance(value, dict) and isinstance(value.get("exit_code"), int)]
    if any(code != 0 for code in codes):
        return "failed"
    if any(value.get("isError") is True for value in values if isinstance(value, dict)):
        return "failed"
    return "passed" if codes else "unknown"


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
    input_text = flatten_text(call.get("input", call.get("arguments"))).casefold()
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
        decoded.extend(_structured_json_values(item))
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
    if child.latest_event_at is not None:
        parent.latest_event_at = max(parent.latest_event_at or child.latest_event_at, child.latest_event_at)
    if child.status == "running":
        parent.status = "running"
        parent.completed = False
    parent.transcript.extend(
        {**item, "session_id": child.thread_id} for item in child.transcript
    )
    parent.warnings.extend(child.warnings)
    parent.candidate_ids.update(child.candidate_ids)
