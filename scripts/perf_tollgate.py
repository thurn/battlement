#!/usr/bin/env python3

"""Read Tollgate command and SQLite performance evidence."""

from __future__ import annotations

import json
from pathlib import Path
import sqlite3
import subprocess
from typing import Any

from perf_model import parse_timestamp, Span


def read_tollgate(repository_root: Path) -> tuple[list[Span], list[str], list[dict[str, Any]]]:
    """Query current Tollgate state and retained history without launching it."""
    try:
        status = _run_json_command(
            ["tg", "status", "--json", "--no-launch"], repository_root
        )
    except (OSError, subprocess.SubprocessError, json.JSONDecodeError) as error:
        return [], [f"Tollgate data unavailable: {error}"], []
    if not isinstance(status, dict):
        return [], ["Tollgate returned an unsupported JSON shape."], []
    items_by_id, status_warnings = _candidate_entries(status)
    history, database_spans, database_warnings = _read_tollgate_database(
        repository_root, set(items_by_id)
    )
    warnings = [*status_warnings, *database_warnings]
    if history is None:
        try:
            history = _run_json_command(
                ["tg", "history", "--json", "--no-launch"], repository_root
            )
        except (OSError, subprocess.SubprocessError, json.JSONDecodeError) as error:
            return [], [*warnings, f"Tollgate history unavailable: {error}"], []
    if not isinstance(history, list):
        return [], [*warnings, "Tollgate returned unsupported history data."], []
    return parse_tollgate_records(status, history, database_spans, warnings, items_by_id)


def parse_tollgate_records(
    status: dict[str, Any],
    history: list[Any],
    database_spans: list[Span] | tuple[Span, ...] = (),
    source_warnings: list[str] | tuple[str, ...] = (),
    items_by_id: dict[str, dict[str, Any]] | None = None,
) -> tuple[list[Span], list[str], list[dict[str, Any]]]:
    """Normalize decoded Tollgate status and history records."""
    if items_by_id is None:
        items_by_id, status_warnings = _candidate_entries(status)
        warnings = [*source_warnings, *status_warnings]
    else:
        warnings = list(source_warnings)
    spans: list[Span] = list(database_spans)
    candidates: list[dict[str, Any]] = []
    history_by_candidate: dict[str, list[dict[str, Any]]] = {}
    unsupported_history_payloads = 0
    for event in history:
        if not isinstance(event, dict):
            warnings.append("Ignored a non-object Tollgate history record.")
            continue
        payload = event.get("payload", {})
        if not isinstance(payload, dict):
            unsupported_history_payloads += 1
            continue
        candidate_id = payload.get("item_id") or payload.get("id")
        if candidate_id:
            history_by_candidate.setdefault(candidate_id, []).append(event)
    if unsupported_history_payloads:
        warnings.append(
            f"Ignored {unsupported_history_payloads} Tollgate history records with "
            "non-object payloads."
        )
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
        attempts = [attempt for attempt in raw_attempts if isinstance(attempt, dict)]
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


def _candidate_entries(
    status: dict[str, Any],
) -> tuple[dict[str, dict[str, Any]], list[str]]:
    items_by_id: dict[str, dict[str, Any]] = {}
    warnings: list[str] = []
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
    return items_by_id, warnings


def _read_tollgate_database(
    repository_root: Path,
    candidate_ids: set[str],
) -> tuple[list[dict[str, Any]] | None, list[Span], list[str]]:
    """Read complete lifecycle and operation timings from Tollgate's local database."""
    database = _tollgate_database_path(repository_root)
    if database is None or not database.is_file():
        return None, [], []
    try:
        connection = sqlite3.connect(f"file:{database}?mode=ro", uri=True)
        try:
            history = _database_candidate_history(connection, candidate_ids)
            spans = _database_promotion_spans(connection, history, candidate_ids)
        finally:
            connection.close()
    except (OSError, sqlite3.Error, json.JSONDecodeError, TypeError, ValueError) as error:
        return None, [], [f"Tollgate database timing data unavailable: {error}"]
    return history, spans, []


def _tollgate_database_path(repository_root: Path) -> Path | None:
    try:
        result = subprocess.run(
            ["git", "rev-parse", "--git-common-dir"],
            cwd=repository_root,
            check=True,
            capture_output=True,
            text=True,
            timeout=10,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    common = Path(result.stdout.strip())
    if not common.is_absolute():
        common = repository_root / common
    return common.resolve() / "tollgate/state.sqlite3"


def _database_candidate_history(
    connection: sqlite3.Connection,
    candidate_ids: set[str],
) -> list[dict[str, Any]]:
    if not candidate_ids:
        return []
    placeholders = ",".join("?" for _ in candidate_ids)
    identifiers = sorted(candidate_ids)
    query = (
        "SELECT event_json FROM events WHERE kind IN "
        "('candidate.created','candidate.promotion-authorized','queue.item-updated',"
        "'promotion.completed') AND ("
        f"json_extract(event_json, '$.payload.id') IN ({placeholders}) OR "
        f"json_extract(event_json, '$.payload.item_id') IN ({placeholders})) "
        "ORDER BY sequence"
    )
    return [
        json.loads(row[0])
        for row in connection.execute(query, [*identifiers, *identifiers])
    ]


def _database_promotion_spans(
    connection: sqlite3.Connection,
    history: list[dict[str, Any]],
    candidate_ids: set[str],
) -> list[Span]:
    boundaries: dict[str, dict[str, float]] = {}
    for event in history:
        payload = event.get("payload", {})
        if not isinstance(payload, dict):
            continue
        candidate_id = payload.get("item_id") or payload.get("id")
        timestamp = parse_timestamp(event.get("created_at"))
        if candidate_id not in candidate_ids or timestamp is None:
            continue
        candidate = boundaries.setdefault(candidate_id, {})
        if event.get("kind") == "candidate.promotion-authorized":
            candidate["authorized"] = timestamp
        elif event.get("kind") == "queue.item-updated" and payload.get("certificate_id"):
            candidate.setdefault("certified", timestamp)

    intents = []
    query = (
        "SELECT intent_id, kind, state, command_id, expected_json, created_at, updated_at "
        "FROM operation_intents WHERE kind IN "
        "('promotion','push','backup','user-master-sync','cleanup') "
        "ORDER BY CAST(created_at AS INTEGER)"
    )
    for intent_id, kind, state, command_id, expected_json, created_at, updated_at in connection.execute(query):
        expected = json.loads(expected_json)
        started = _nanosecond_timestamp(created_at)
        finished = _nanosecond_timestamp(updated_at)
        candidate_id = expected.get("item_id") or expected.get("queue_item_id")
        intents.append(
            {
                "id": intent_id,
                "kind": kind,
                "state": state,
                "command_id": command_id,
                "candidate_id": candidate_id,
                "started": started,
                "finished": finished,
                "expected": expected,
            }
        )
    observations = {
        intent_id: _nanosecond_timestamp(observed_at)
        for intent_id, observed_at in connection.execute(
            "SELECT intent_id, observed_at FROM remote_observations "
            "WHERE method='promotion-preflight-fetch'"
        )
    }
    pushes = [
        intent for intent in intents
        if intent["kind"] == "push" and intent["candidate_id"] in candidate_ids
    ]
    direct = {
        (intent["candidate_id"], intent["kind"]): intent
        for intent in intents
        if intent["candidate_id"] in candidate_ids
    }
    backups = [intent for intent in intents if intent["kind"] == "backup"]
    spans: list[Span] = []
    for push in pushes:
        candidate_id = push["candidate_id"]
        attributes = {
            "candidate_id": candidate_id,
            "command_id": push["command_id"],
            "operation_state": push["state"],
            "timing_source": "tollgate_operation_intent",
        }
        pipeline_id = f"tg-pipeline:{push['id']}"
        _append_tollgate_phase(
            spans, pipeline_id, None, "Tollgate promotion pipeline",
            push["started"], push["finished"], attributes, push["state"], container=True,
        )
        ready = max(boundaries.get(candidate_id, {}).values(), default=None)
        if ready is not None:
            _append_tollgate_phase(
                spans, f"tg-ready:{push['id']}", pipeline_id,
                "Tollgate ready dispatch", ready, push["started"], attributes, "completed",
            )
        observed = observations.get(push["id"])
        _append_tollgate_phase(
            spans, f"tg-preflight:{push['id']}", pipeline_id,
            "Tollgate remote preflight", push["started"], observed, attributes, push["state"],
        )
        promotion = direct.get((candidate_id, "promotion"))
        if promotion is not None:
            _append_tollgate_phase(
                spans, f"tg-local-promotion:{promotion['id']}", pipeline_id,
                "Tollgate local promotion", promotion["started"], promotion["finished"],
                attributes, promotion["state"],
            )
        backup = next(
            (
                intent for intent in backups
                if push["started"] <= intent["started"] <= push["finished"]
            ),
            None,
        )
        if backup is not None:
            backup_attributes = {
                **attributes,
                "reserved_allowance_bytes": backup["expected"].get("allowance"),
            }
            _append_tollgate_phase(
                spans, f"tg-backup:{backup['id']}", pipeline_id,
                "Tollgate database backup", backup["started"], backup["finished"],
                backup_attributes, backup["state"],
            )
        master_sync = direct.get((candidate_id, "user-master-sync"))
        push_started = backup["finished"] if backup is not None else (
            promotion["finished"] if promotion is not None else observed
        )
        push_finished = master_sync["started"] if master_sync is not None else push["finished"]
        _append_tollgate_phase(
            spans, f"tg-remote-push:{push['id']}", pipeline_id, "Tollgate remote push",
            push_started, push_finished,
            {**attributes, "timing_source": "inferred_between_operation_intents"},
            push["state"],
        )
        if master_sync is not None:
            _append_tollgate_phase(
                spans, f"tg-master-sync:{master_sync['id']}", pipeline_id,
                "Tollgate user-master synchronization", master_sync["started"],
                master_sync["finished"], attributes, master_sync["state"],
            )
        cleanup = direct.get((candidate_id, "cleanup"))
        if cleanup is not None:
            _append_tollgate_phase(
                spans, f"tg-cleanup:{cleanup['id']}", None,
                "Tollgate source cleanup", cleanup["started"], cleanup["finished"],
                attributes, cleanup["state"],
            )
    return spans


def _append_tollgate_phase(
    spans: list[Span],
    span_id: str,
    parent_id: str | None,
    name: str,
    started: float | None,
    finished: float | None,
    attributes: dict[str, Any],
    state: str,
    *,
    container: bool = False,
) -> None:
    if started is None or finished is None or finished < started:
        return
    spans.append(
        Span(
            span_id, parent_id, None, "tollgate", "tollgate-phase", name,
            started, finished, "passed" if state == "completed" else state,
            attributes=attributes, container=container,
        )
    )


def _nanosecond_timestamp(value: Any) -> float | None:
    try:
        return int(value) / 1_000_000_000
    except (TypeError, ValueError, OverflowError):
        return None


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
    lifecycle_events = (
        (
            "candidate.submitted",
            next((event for event in history if event.get("kind") == "candidate.created"), None),
        ),
        (
            "candidate.authorized",
            next(
                (
                    event for event in history
                    if event.get("kind") == "candidate.promotion-authorized"
                ),
                None,
            ),
        ),
        (
            "candidate.certified",
            next(
                (
                    event for event in history
                    if event.get("kind") == "queue.item-updated"
                    and isinstance(event.get("payload"), dict)
                    and event["payload"].get("certificate_id")
                ),
                None,
            ),
        ),
        (
            "candidate.promoted",
            next((event for event in history if event.get("kind") == "promotion.completed"), None),
        ),
        (
            "candidate.synchronized",
            next(
                (
                    event for event in history
                    if event.get("kind") == "queue.item-updated"
                    and isinstance(event.get("payload"), dict)
                    and event["payload"].get("remote_state") == "synchronized"
                ),
                None,
            ),
        ),
    )
    for milestone, event in lifecycle_events:
        timestamp = parse_timestamp(event.get("created_at")) if event else None
        if timestamp is None:
            continue
        spans.append(
            Span(
                f"tg-milestone:{candidate_id}:{milestone}",
                None,
                None,
                "tollgate",
                "milestone",
                milestone,
                timestamp,
                timestamp,
                "passed",
                attributes={**lifecycle_attributes, "milestone": milestone},
            )
        )
    if finished is not None and authorized is not None and authorized > finished:
        spans.append(
            Span(
                f"tg-authorization:{candidate_id}", None, None, "tollgate", "wait",
                "Tollgate authorization wait", finished, authorized, "passed",
                attributes=lifecycle_attributes,
            )
        )
    completion = next(
        (
            parse_timestamp(event.get("created_at"))
            for event in history if event.get("kind") == "promotion.completed"
        ),
        None,
    )
    if authorized is not None and completion is not None and completion > authorized:
        spans.append(
            Span(
                f"tg-promotion:{candidate_id}", None, None, "tollgate", "wait",
                "Tollgate authorization to local promotion", authorized, completion,
                "passed",
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
