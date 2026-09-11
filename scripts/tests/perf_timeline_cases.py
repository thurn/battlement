"""Synthetic lifecycle and local-date regression cases without private logs."""

from argparse import Namespace
from dataclasses import replace
import json
from pathlib import Path
import stat

import perf_analysis
import perf_codex
from perf_model import parse_timestamp, Thresholds
import perf_report


def verify(root: Path) -> None:
    def event(hour: str, kind: str, **payload: object) -> dict:
        return {"timestamp": f"2026-01-01T{hour}:00Z", "type": "event_msg",
                "payload": {"type": kind, **payload}}

    path = root / "timeline.jsonl"
    entries = [
        {"timestamp": "2026-01-01T09:00:00Z", "type": "response_item",
         "payload": {"type": "message", "role": "user", "content": "Fixture"}},
        event("09:00", "task_started", turn_id="morning"),
        event("09:09", "task_complete", turn_id="morning", started_at="2026-01-01T09:00:00Z"),
        event("13:00", "task_started", turn_id="afternoon"),
        event("14:20", "token_count"),
        event("15:00", "task_complete", turn_id="afternoon"),
    ]
    path.write_text("".join(json.dumps(entry) + "\n" for entry in entries))
    record = perf_codex.ThreadRecord("fixture", path, "Fixture", "example.invalid", 0)
    cutoff = parse_timestamp("2026-01-01T14:30:00Z")
    entries, warnings = perf_codex.read_codex_rollout(record.rollout_path)
    session = perf_codex.parse_codex_rollout(record, entries, cutoff, warnings)
    assert session.status == "unknown" and not session.completed
    assert len(session.spans) == 2  # completed_at/started_at must pop the pending turn
    report = perf_analysis.analyze_session(session, Thresholds(1, 1, 1, 1), 10)
    timing = report["timing"]
    assert timing["wall_time_ms"] == 320 * 60_000
    assert timing["actor_activity_ms"] == 89 * 60_000
    assert timing["between_turn_gap_ms"] == 231 * 60_000
    assert timing["known_wait_union_ms"] == 0
    assert timing["unobserved_tail_ms"] == 10 * 60_000
    assert report["metadata"]["status"] == "unknown"

    live_record = replace(record, live_turn_id="afternoon", live_observed_at=cutoff)
    entries, warnings = perf_codex.read_codex_rollout(live_record.rollout_path)
    live_session = perf_codex.parse_codex_rollout(live_record, entries, cutoff, warnings)
    live_report = perf_analysis.analyze_session(live_session, Thresholds(1, 1, 1, 1), 10)
    assert live_session.status == "running"
    assert live_report["timing"]["actor_activity_ms"] == 99 * 60_000
    assert live_report["timing"]["unobserved_tail_ms"] == 0
    stale_record = replace(live_record, live_observed_at=cutoff - 1)
    entries, warnings = perf_codex.read_codex_rollout(stale_record.rollout_path)
    assert perf_codex.parse_codex_rollout(stale_record, entries, cutoff, warnings).status == "unknown"

    parent_path = root / "parent.jsonl"
    parent_path.write_text(json.dumps({"timestamp": "2025-12-31T12:00:00Z", "type": "response_item",
                                      "payload": {"type": "message", "role": "user", "content": "Fixture"}}) + "\n")
    parent = perf_codex.ThreadRecord("parent", parent_path, "Parent", "example.invalid", 0)
    args = Namespace(date="2026-01-01", timezone="UTC", observed_at="2026-01-01T14:30:00Z",
                     thread=None, commit=None, candidate=None, include_incomplete=True, sessions=10)
    perf_report._observation_window(args)
    folded = perf_report._load_sessions(args, {"parent": parent, "fixture": replace(live_record, parent_thread_id="parent")},
                                       {"parent": ["fixture"]})
    assert len(folded) == 1 and folded[0].status == "running"
    assert perf_analysis.analyze_session(folded[0], Thresholds(1, 1, 1, 1), 10)["timing"]["actor_activity_ms"] == 99 * 60_000

    entries = entries[:4] + [event("13:05", "turn_aborted")]
    path.write_text("".join(json.dumps(entry) + "\n" for entry in entries))
    entries, warnings = perf_codex.read_codex_rollout(record.rollout_path)
    interrupted = perf_codex.parse_codex_rollout(record, entries, cutoff, warnings)
    assert interrupted.status == "interrupted"
    assert interrupted.spans[-1].duration_ms == 300_000
    assert interrupted.spans[-1].status == "interrupted"
    counts = perf_analysis.aggregate_reports([
        perf_analysis.analyze_session(interrupted, Thresholds(1, 1, 1, 1), 10), report,
    ], 10)["status_counts"]
    assert counts == {"completed": 0, "running": 0, "interrupted": 1, "unknown": 1}

    selection = Namespace(date="2026-01-01", timezone="America/Los_Angeles",
                          observed_at="2026-01-03T00:00:00+00:00")
    perf_report._observation_window(selection)
    assert selection.window_start == parse_timestamp("2026-01-01T08:00:00Z")
    assert selection.window_end == parse_timestamp("2026-01-02T08:00:00Z")
    # The local DST transition is a 23-hour day, not a fixed UTC duration.
    selection.date = "2026-03-08"
    selection.observed_at = "2026-03-10T00:00:00+00:00"
    perf_report._observation_window(selection)
    assert selection.window_end - selection.window_start == 23 * 3600
    session.window_start = parse_timestamp("2026-01-01T12:30:00Z")
    session.window_end = cutoff
    gap_report = perf_analysis.analyze_session(session, Thresholds(1, 1, 1, 1), 10)
    assert gap_report["timing"]["between_turn_gap_ms"] == 30 * 60_000
    session.window_start = parse_timestamp("2026-01-01T13:30:00Z")
    session.window_end = cutoff
    clipped = perf_analysis.analyze_session(session, Thresholds(1, 1, 1, 1), 10)
    assert clipped["timing"]["actor_activity_ms"] == 50 * 60_000
    assert clipped["timing"]["between_turn_gap_ms"] == 0

    call = {"name": "functions.exec", "call_id": "wrapped", "arguments": "await tools.exec_command(...)"}
    output = {"output": json.dumps({"content": [{"text": json.dumps({"exit_code": 1, "output": "nonempty"})}]})}
    span = perf_codex._tool_span(session, call, 0, 1, output)
    assert span.status == "failed" and span.content["input"] == call["arguments"]
    assert perf_codex._tool_span(session, call, 0, 1, {"output": "done"}).status == "unknown"

    parent = root / "caller-owned"
    parent.mkdir(mode=0o755)
    before = stat.S_IMODE(parent.stat().st_mode)
    perf_report._write_private_json(parent / "report.json", {"safe": True})
    assert stat.S_IMODE(parent.stat().st_mode) == before
    perf_report._write_private_json(parent / "nested/deeper/report.json", {})
    assert stat.S_IMODE((parent / "nested").stat().st_mode) == 0o700
    assert stat.S_IMODE((parent / "nested/deeper").stat().st_mode) == 0o700
