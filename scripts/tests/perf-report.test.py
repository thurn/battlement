#!/usr/bin/env python3

"""Verify daemonless CI and agent performance reporting."""

from __future__ import annotations

import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
from threading import Thread
from unittest.mock import patch


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY_ROOT / "scripts"))

import perf_analysis  # noqa: E402
import perf_log  # noqa: E402
from perf_model import exclusive_durations, interval_difference_ms, interval_union_ms, SessionTrace, Span, Thresholds  # noqa: E402
import perf_report  # noqa: E402
import perf_sources  # noqa: E402


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="battlement-perf-report-test.") as temporary:
        root = Path(temporary)
        _verify_repository_normalization()
        _verify_content_sanitization()
        _verify_candidate_evidence(root)
        _verify_ci_trace(root)
        _verify_ci_entrypoint(root)
        _verify_unloggable_trace(root)
        _verify_retention(root)
        root_session, child_session = _verify_codex_parsing(root)
        _verify_child_folding(root_session, child_session)
        _verify_ci_parsing(root)
        _verify_interval_analysis(root_session)
        _verify_correlation(root, root_session)
        _verify_tollgate_retries(root)
        _verify_private_report(root)
        _verify_tollgate_failure(root)
    print("Performance report tests passed.")


def _verify_repository_normalization() -> None:
    assert perf_log.normalize_repository_url("git@github.com:Thurn/Battlement.git") == (
        "github.com/thurn/battlement"
    )
    assert perf_log.normalize_repository_url("https://github.com/thurn/battlement/") == (
        "github.com/thurn/battlement"
    )


def _verify_content_sanitization() -> None:
    sanitized = perf_sources.sanitize(
        {
            "text": "preserved",
            "encrypted_content": "omitted",
            "image": {"data": "binary", "mime_type": "image/png"},
        }
    )
    assert sanitized["text"] == "preserved"
    assert "encrypted_content" not in sanitized
    assert sanitized["image"]["data_metadata"]["size"] == 6
    assert "data" not in sanitized["image"]
    data_url = perf_sources.sanitize(
        {"image_url": "data:image/png;base64,aGVsbG8="}
    )
    metadata = data_url["image_url"]["data_url_metadata"]
    assert metadata["mime_type"] == "image/png"
    assert metadata["encoding"] == "base64"
    assert metadata["size"] == 5
    assert "aGVsbG8" not in json.dumps(data_url)


def _verify_candidate_evidence(root: Path) -> None:
    session = SessionTrace("candidate", "Candidate", root, "repository")
    candidate_id = "11111111-2222-3333-4444-555555555555"
    perf_sources._collect_candidate_ids(
        session,
        {"input": {"cmd": "tg status --json; tg candidate --help"}},
        {"output": json.dumps({"item": {"id": candidate_id, "source_oid": "oid"}})},
    )
    assert not session.candidate_ids
    perf_sources._collect_candidate_ids(
        session,
        {"input": {"cmd": "tg --no-launch --json candidate HEAD"}},
        {
            "output": "completed\n" + json.dumps(
                {"item_id": candidate_id, "source_oid": {"bytes": "oid"}}
            )
        },
    )
    assert session.candidate_ids == {candidate_id}


def _verify_ci_trace(root: Path) -> None:
    repository = _repository(root / "trace-repository")
    ticks = iter((0, 1_000_000, 3_000_000, 4_000_000, 8_000_000))
    trace = perf_log.CiTrace(
        repository,
        {"full": False, "ditto": False, "ci_cache_enabled": True},
        root / "trace-logs",
        monotonic_ns=lambda: next(ticks),
    )
    with trace.span("serial"):
        trace.event("ci.cache_lookup", result="hit", cache_key="fixture")
    threads = [Thread(target=trace.event, args=("fixture",), kwargs={"index": index}) for index in range(8)]
    for thread in threads:
        thread.start()
    for thread in threads:
        thread.join()
    trace.finish("passed", 0)
    assert trace.path is not None
    records = [json.loads(line) for line in trace.path.read_text().splitlines()]
    assert records[0]["event"] == "ci.run_started"
    assert records[-1]["event"] == "ci.run_finished"
    assert len([record for record in records if record["event"] == "fixture"]) == 8
    assert stat.S_IMODE(trace.path.stat().st_mode) == 0o600


def _verify_ci_entrypoint(root: Path) -> None:
    log_root = root / "entrypoint-logs"
    environment = {**os.environ, "BATTLEMENT_LOG_ROOT": str(log_root)}
    cases = [
        ([], "passed", 0),
        (["--full"], "passed", 0),
        ([], "failed", 1),
        (["--full"], "interrupted", 130),
    ]
    for arguments, outcome, expected_code in cases:
        before = set((log_root / "ci").glob("**/*.jsonl"))
        result = subprocess.run(
            [
                sys.executable,
                "scripts/ci.py",
                *arguments,
                "--test-trace-outcome",
                outcome,
            ],
            cwd=REPOSITORY_ROOT,
            env=environment,
            capture_output=True,
            text=True,
        )
        assert result.returncode == expected_code
        created = set((log_root / "ci").glob("**/*.jsonl")) - before
        assert len(created) == 1
        records = [json.loads(line) for line in created.pop().read_text().splitlines()]
        assert records[0]["full"] == ("--full" in arguments)
        assert records[-1]["event"] == "ci.run_finished"
        assert records[-1]["outcome"] == outcome
        assert records[-1]["exit_code"] == expected_code
        parallel = next(
            record for record in records
            if record.get("event") == "ci.step_started"
            and record.get("name") == "Trace smoke parallel"
        )
        children = [
            record for record in records
            if record.get("event") == "ci.step_started"
            and record.get("name", "").startswith("trace child")
        ]
        assert len(children) == 2
        assert all(record["parent_span_id"] == parallel["span_id"] for record in children)
    spans, warnings = perf_sources.read_ci_traces(log_root)
    assert not warnings
    assert len([span for span in spans if span.id.startswith("ci-run:")]) == 4


def _verify_unloggable_trace(root: Path) -> None:
    repository = _repository(root / "unloggable-repository")
    blocked = root / "blocked"
    blocked.write_text("file")
    trace = perf_log.CiTrace(repository, {}, blocked)
    trace.event("ignored")
    trace.finish("failed", 1)
    assert trace.path is None


def _verify_retention(root: Path) -> None:
    log_root = root / "retention"
    reports = log_root / "reports"
    ci = log_root / "ci/2026-01-01"
    reports.mkdir(parents=True)
    ci.mkdir(parents=True)
    report = reports / "old.json"
    first_ci = ci / "old.jsonl"
    protected_ci = ci / "protected.jsonl"
    active_ci = ci / "active.jsonl"
    report.write_bytes(b"x" * 10)
    completed = b'{"event":"ci.run_finished"}\n'
    first_ci.write_bytes(completed)
    protected_ci.write_bytes(completed)
    active_ci.write_bytes(b'{"event":"ci.run_started"}\n')
    for index, path in enumerate((report, first_ci, protected_ci, active_ci), 1):
        os.utime(path, ns=(index, index))
    removed = perf_log.enforce_retention(
        log_root,
        protected_ci.stat().st_size + active_ci.stat().st_size,
        {protected_ci},
    )
    assert removed == [report, first_ci]
    assert protected_ci.exists()
    assert active_ci.exists()


def _verify_codex_parsing(root: Path) -> tuple[SessionTrace, SessionTrace]:
    rollout = root / "root.jsonl"
    _write_jsonl(
        rollout,
        [
            _entry("2026-01-01T00:00:00Z", "session_meta", {
                "id": "root", "git": {"repository_url": "git@github.com:thurn/battlement.git"}
            }),
            _response("2026-01-01T00:00:01Z", "message", {"role": "user", "content": [{"text": "build it"}]}),
            _response("2026-01-01T00:00:01Z", "message", {"role": "developer", "content": [{"text": "base instructions"}]}),
            _event("2026-01-01T00:00:02Z", "task_started", {"turn_id": "turn", "started_at": "2026-01-01T00:00:02Z"}),
            _response("2026-01-01T00:00:03Z", "custom_tool_call", {
                "call_id": "call", "name": "exec", "input": {"cmd": "echo hello"},
                "internal_chat_message_metadata_passthrough": {"turn_id": "turn"},
            }),
            _response("2026-01-01T00:00:05Z", "custom_tool_call_output", {
                "call_id": "call", "output": [{"type": "text", "text": "hello"}],
            }),
            _response("2026-01-01T00:00:05Z", "reasoning", {
                "summary": ["short"], "encrypted_content": "secret",
            }),
            _event("2026-01-01T00:00:06Z", "task_complete", {
                "turn_id": "turn", "started_at": "2026-01-01T00:00:02Z",
                "completed_at": "2026-01-01T00:00:06Z", "time_to_first_token_ms": 25,
            }),
            ["unknown", "record"],
            {"timestamp": "2026-01-01T00:00:06Z", "type": "unknown", "payload": {}},
        ],
    )
    record = perf_sources.ThreadRecord(
        "root", rollout, "Fixture task", "github.com/thurn/battlement", 1
    )
    session = perf_sources.parse_codex_rollout(record)
    assert session.completed
    assert session.first_user_at is not None
    tool = next(span for span in session.spans if span.category == "tool")
    assert tool.duration_ms == 2000
    assert session.time_to_first_token_ms == [25]
    assert "encrypted_content" not in json.dumps(session.transcript)
    assert "base instructions" not in json.dumps(session.transcript)
    assert any("non-object" in warning for warning in session.warnings)

    child_rollout = root / "child.jsonl"
    _write_jsonl(
        child_rollout,
        [
            _entry("2026-01-01T00:00:03Z", "session_meta", {
                "id": "child", "parent_thread_id": "root", "agent_path": "/root/review"
            }),
            _event("2026-01-01T00:00:03Z", "task_started", {"turn_id": "child-turn"}),
            _event("2026-01-01T00:00:04Z", "task_complete", {"turn_id": "child-turn"}),
        ],
    )
    child = perf_sources.parse_codex_rollout(
        perf_sources.ThreadRecord(
            "child", child_rollout, "Review", "github.com/thurn/battlement", 2, "root"
        )
    )
    return session, child


def _verify_child_folding(root: SessionTrace, child: SessionTrace) -> None:
    records = {
        "root": perf_sources.ThreadRecord("root", root.rollout_path, root.title, root.repository_url, 1),
        "child": perf_sources.ThreadRecord("child", child.rollout_path, child.title, child.repository_url, 2, "root"),
    }
    loaded = perf_sources.load_session_tree(records["root"], records, {"root": ["child"]})
    assert any(span.category == "subagent" for span in loaded.spans)
    assert any(item.get("session_id") == "child" for item in loaded.transcript) is False


def _verify_ci_parsing(root: Path) -> None:
    path = root / "ci-logs/ci/2026-01-01/run.jsonl"
    _write_jsonl(
        path,
        [
            {"timestamp": "2026-01-01T00:00:00Z", "event": "ci.run_started", "run_id": "run", "full": True, "staged_tree_oid": "tree"},
            {"timestamp": "2026-01-01T00:00:01Z", "event": "ci.step_started", "run_id": "run", "span_id": "step", "parent_span_id": "run", "name": "tests"},
            {"timestamp": "2026-01-01T00:00:03Z", "event": "ci.step_finished", "run_id": "run", "span_id": "step", "parent_span_id": "run", "name": "tests", "outcome": "passed"},
            {"timestamp": "2026-01-01T00:00:03Z", "event": "ci.cache_wait", "run_id": "run", "parent_span_id": "run", "duration_ms": 100},
            "unknown",
            {"timestamp": "2026-01-01T00:00:04Z", "event": "ci.run_finished", "run_id": "run", "outcome": "passed", "exit_code": 0},
        ],
    )
    spans, warnings = perf_sources.read_ci_traces(root / "ci-logs")
    assert any("non-object" in warning for warning in warnings)
    assert len(spans) == 3
    assert next(span for span in spans if span.id == "ci-step:step").duration_ms == 2000
    cache_wait = next(span for span in spans if span.name == "cache wait")
    assert cache_wait.parent_id == "ci-run:run"


def _verify_interval_analysis(session: SessionTrace) -> None:
    session.spans.extend(
        [
            Span("outer", None, "root", "ci", "ci", "ci.py", 10, 20, "passed", container=True, attributes={"staged_tree_oid": "tree", "full": False}),
            Span("inner", "outer", "root", "ci", "ci", "tests", 12, 18, "passed"),
            Span("slow", None, "root", "codex", "tool", "slow tool", 20, 55, "passed", content={"input": "same"}),
            Span("slow-2", None, "root", "codex", "tool", "slow tool", 56, 91, "passed", content={"input": "same"}),
        ]
    )
    assert interval_union_ms([(0, 10), (5, 15)]) == 15_000
    assert interval_difference_ms([(0, 100)], [(10, 90)]) == 20_000
    exclusive = exclusive_durations(session.spans)
    assert exclusive["outer"] == 4000
    report = perf_analysis.analyze_session(
        session, Thresholds(30_000, 300_000, 1_000, 120_000), 10
    )
    codes = {finding["code"] for finding in report["findings"]}
    assert "slow-tool" in codes
    assert "repeated-tool" in codes
    assert "slow-ci-step" in codes

    wait_session = SessionTrace("wait", "Wait", session.rollout_path, session.repository_url)
    wait_session.first_user_at = 0
    wait_session.completed_at = 100
    wait_session.completed = True
    wait_session.spans = [
        Span("wait-turn", None, "wait", "codex", "agent", "Agent turn", 0, 100, "passed", container=True),
        Span("wait-tool", "wait-turn", "wait", "codex", "wait", "wait_agent", 10, 90, "passed"),
    ]
    wait_report = perf_analysis.analyze_session(wait_session, Thresholds(1, 1, 1, 1), 10)
    assert wait_report["timing"]["recorded_active_coverage_ms"] == 20_000
    assert wait_report["timing"]["known_wait_union_ms"] == 80_000
    assert wait_report["timing"]["unattributed_agent_turn_ms"] == 20_000


def _verify_correlation(root: Path, session: SessionTrace) -> None:
    repository = _repository(root / "correlation-repository")
    head = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=repository, check=True,
        capture_output=True, text=True,
    ).stdout.strip()
    tree = subprocess.run(
        ["git", "rev-parse", "HEAD^{tree}"], cwd=repository, check=True,
        capture_output=True, text=True,
    ).stdout.strip()
    wrapper = Span(
        "ci-wrapper", None, "root", "codex", "tool", "exec", 0.5, 2.5,
        "passed", content={"input": {"cmd": "python3 scripts/ci.py"}},
    )
    session.spans.append(wrapper)
    run = Span(
        "ci-run:correlated", None, None, "ci", "ci", "ci.py", 1, 2, "passed",
        attributes={"codex_thread_id": "root", "run_id": "correlated", "staged_tree_oid": tree},
    )
    tollgate = Span(
        "tg-buildset:candidate", None, None, "tollgate", "ci", "Tollgate", 3, 4, "passed",
        attributes={"candidate_id": "candidate", "source_oid": head},
    )
    candidate = {"item": {"id": "candidate", "source_oid": {"bytes": head}}, "buildset": {}}
    warnings: list[str] = []
    perf_analysis.correlate_activity([session], [run], [tollgate], [candidate], repository, warnings)
    assert run.session_id == "root"
    assert run.parent_id == wrapper.id
    assert tollgate.session_id == "root"
    assert tollgate.association == "exact_tree"
    report = perf_analysis.analyze_session(session, Thresholds(1, 1, 1, 1), 20)
    assert wrapper.id not in {span["id"] for span in report["longest_operations"]}
    assert next(
        span for span in report["spans"] if span["id"] == wrapper.id
    )["exclusive_duration_ms"] == 1000


def _verify_tollgate_retries(root: Path) -> None:
    candidate_id = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
    status = {
        "queue": [],
        "checks": [
            {
                "item": {"id": candidate_id, "state": "passed"},
                "buildset": {},
                "attempts": [
                    {
                        "id": "failed", "attempt": 1, "state": "failed",
                        "created_at": "2026-01-01T00:00:00Z",
                        "started_at": "2026-01-01T00:00:01Z",
                        "finished_at": "2026-01-01T00:00:03Z",
                    },
                    {
                        "id": "passed", "attempt": 2, "state": "passed",
                        "created_at": "2026-01-01T00:00:04Z",
                        "started_at": "2026-01-01T00:00:05Z",
                        "finished_at": "2026-01-01T00:00:07Z",
                    },
                ],
            }
        ],
        "history_items": [],
    }
    with patch.object(perf_sources, "_run_json_command", side_effect=[status, ["unknown"]]):
        spans, warnings, candidates = perf_sources.read_tollgate(root)
    assert len([span for span in spans if span.id.startswith("tg-buildset:")]) == 2
    assert candidates[0]["attempts"][1]["attempt"] == 2
    assert any("non-object" in warning for warning in warnings)
    findings = perf_analysis.workflow_findings(spans, Thresholds(1, 1, 1, 1))
    assert any(finding["code"] == "tollgate-retry" for finding in findings)


def _verify_private_report(root: Path) -> None:
    output = root / "private/report.json"
    perf_report._write_private_json(output, {"transcript": "full"})
    assert json.loads(output.read_text()) == {"transcript": "full"}
    assert stat.S_IMODE(output.stat().st_mode) == 0o600


def _verify_tollgate_failure(root: Path) -> None:
    with patch.object(perf_sources, "_run_json_command", side_effect=FileNotFoundError("tg")):
        spans, warnings, candidates = perf_sources.read_tollgate(root)
    assert not spans and not candidates
    assert warnings and "unavailable" in warnings[0]


def _repository(path: Path) -> Path:
    path.mkdir()
    subprocess.run(["git", "init", "--quiet"], cwd=path, check=True)
    (path / "fixture").write_text("fixture")
    subprocess.run(["git", "add", "fixture"], cwd=path, check=True)
    subprocess.run(
        ["git", "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid", "commit", "--quiet", "-m", "fixture"],
        cwd=path, check=True,
    )
    return path


def _write_jsonl(path: Path, records: list[object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("".join(json.dumps(record) + "\n" for record in records))


def _entry(timestamp: str, entry_type: str, payload: dict[str, object]) -> dict[str, object]:
    return {"timestamp": timestamp, "type": entry_type, "payload": payload}


def _response(timestamp: str, payload_type: str, payload: dict[str, object]) -> dict[str, object]:
    return _entry(timestamp, "response_item", {"type": payload_type, **payload})


def _event(timestamp: str, payload_type: str, payload: dict[str, object]) -> dict[str, object]:
    return _entry(timestamp, "event_msg", {"type": payload_type, **payload})


if __name__ == "__main__":
    main()
