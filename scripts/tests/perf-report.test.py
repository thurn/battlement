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
import perf_hotspots  # noqa: E402
import perf_log  # noqa: E402
from perf_model import exclusive_durations, interval_difference_ms, interval_union_ms, parse_timestamp, SessionTrace, Span, Thresholds  # noqa: E402
import perf_report  # noqa: E402
import perf_sources  # noqa: E402
import perf_timeline_cases  # noqa: E402
import workflow_event  # noqa: E402


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
        _verify_operation_parsing(root)
        _verify_workflow_milestones(root)
        _verify_interval_analysis(root_session)
        _verify_ci_step_hotspots()
        _verify_correlation(root, root_session)
        _verify_tollgate_retries(root)
        _verify_private_report(root)
        perf_timeline_cases.verify(root)
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
            {"timestamp": "2026-01-01T00:00:03Z", "event": "ditto.invocation", "run_id": "run", "invocation_id": "missing-invocation", "artifact_root": str(root / "missing-invocation"), "evidence_path": str(root / "missing-invocation/evidence.json")},
            {"timestamp": "2026-01-01T00:00:04Z", "event": "ci.run_finished", "run_id": "run", "outcome": "passed", "exit_code": 0},
        ],
    )
    spans, warnings = perf_sources.read_ci_traces(root / "ci-logs")
    assert any("non-object" in warning for warning in warnings)
    assert len(spans) == 3
    run = next(span for span in spans if span.id == "ci-run:run")
    assert run.attributes["ditto_evidence"][0]["integrity"] == "missing"
    assert any("missing Ditto evidence" in warning for warning in warnings)
    assert next(span for span in spans if span.id == "ci-step:step").duration_ms == 2000
    cache_wait = next(span for span in spans if span.name == "cache wait")
    assert cache_wait.parent_id == "ci-run:run"


def _verify_operation_parsing(root: Path) -> None:
    log_root = root / "operation-logs"
    operation = log_root / "operations/2026-01-01/web.jsonl"
    context = {"task_id": "root", "root_operation_id": "web", "head_oid": "head"}
    process = {"pid": 42, "birth": "fixture", "host": "host"}
    _write_jsonl(operation, [
        {"timestamp": "2026-01-01T00:00:00Z", "event": "operation.started",
         "operation_id": "web", "name": "Web preparation", "context": context,
         "metadata": {"build_profile": "release"}},
        {"timestamp": "2026-01-01T00:00:01Z", "event": "resource.queued",
         "operation_id": "web", "resource": "unity-editor"},
        {"timestamp": "2026-01-01T00:00:02Z", "event": "resource.acquired",
         "operation_id": "web", "resource": "unity-editor"},
        {"timestamp": "2026-01-01T00:00:02Z", "event": "process.started",
         "operation_id": "web", "process": process, "executable": "cargo"},
        {"timestamp": "2026-01-01T00:00:03Z", "event": "process.finished",
         "operation_id": "web", "process": process, "exit_code": 0},
        {"timestamp": "2026-01-01T00:00:04Z", "event": "resource.released",
         "operation_id": "web", "resource": "unity-editor"},
        {"timestamp": "2026-01-01T00:00:04.500Z", "event": "review.ready",
         "operation_id": "web", "handle_id": "review"},
        {"timestamp": "2026-01-01T00:00:05Z", "event": "operation.finished",
         "operation_id": "web", "outcome": "passed"},
    ])
    _write_jsonl(log_root / "operations/2026-01-01/incomplete.jsonl", [
        {"timestamp": "2026-01-01T00:00:00Z", "event": "operation.started",
         "operation_id": "incomplete", "name": "External", "context": {},
         "metadata": {}},
    ])
    _write_jsonl(log_root / "operations/2026-01-01/duplicate.jsonl", [
        {"timestamp": "2026-01-01T00:00:00Z", "event": "operation.started",
         "operation_id": "ci-step", "name": "CI duplicate", "context": {},
         "metadata": {}},
    ])
    spans, warnings = perf_sources.read_operation_traces(
        log_root, {"ci-step": "ci-step:ci-step"},
    )
    assert len(spans) == 6
    web = next(span for span in spans if span.id == "operation:web")
    assert web.attributes["task_id"] == "root"
    assert web.attributes["build_profile"] == "release"
    assert next(span for span in spans if span.category == "wait").duration_ms == 1000
    assert next(span for span in spans if span.category == "resource").duration_ms == 2000
    assert next(span for span in spans if span.category == "process").name == "cargo"
    assert next(span for span in spans if span.category == "milestone").name == "review.ready"
    assert any("no terminal event" in warning for warning in warnings)


def _verify_workflow_milestones(root: Path) -> None:
    repository = _repository(root / "workflow-repository")
    log_root = root / "workflow-logs"
    workflow_id = "11111111-2222-4333-8444-555555555555"
    with patch.dict(os.environ, {"CODEX_THREAD_ID": workflow_id}):
        first = workflow_event.record(
            "focused.passed",
            repository=repository,
            log_root=log_root,
            workflow_id=workflow_id,
            outcome="passed",
        )
        second = workflow_event.record(
            "review.ready",
            repository=repository,
            log_root=log_root,
            workflow_id=workflow_id,
            candidate_id="candidate-1",
        )
    assert first == second
    assert stat.S_IMODE(first.stat().st_mode) == 0o600
    spans, warnings = perf_sources.read_workflow_milestones(log_root)
    assert not warnings
    assert [span.name for span in spans] == ["focused.passed", "review.ready"]
    assert all(span.attributes["task_id"] == workflow_id for span in spans)
    assert not perf_log._completed_trace(first)
    with patch.dict(os.environ, {"CODEX_THREAD_ID": workflow_id}):
        workflow_event.record(
            "jobs.finished",
            repository=repository,
            log_root=log_root,
            workflow_id=workflow_id,
        )
    assert perf_log._completed_trace(first)
    malformed = log_root / "workflows/2026-01-01/malformed.jsonl"
    _write_jsonl(malformed, [{"event": "workflow.milestone", "milestone": "review.ready"}])
    _, warnings = perf_sources.read_workflow_milestones(log_root)
    assert any("malformed workflow milestone" in warning for warning in warnings)


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


def _verify_ci_step_hotspots() -> None:
    spans = [
        _normalized_ci_span("one", "run-one", "Rust tests", 10_000),
        _normalized_ci_span("two", "run-two", "Rust tests", 20_000),
        _normalized_ci_span("three", "run-three", "Rust tests", 100_000, "failed"),
        _normalized_ci_span("format", "run-one", "Format", 5_000),
        _normalized_ci_span(
            "nested", "parent-step", "root workspace", 99_000,
            parent_prefix="ci-step",
        ),
        {
            **_normalized_ci_span("tollgate", "run-four", "Tollgate", 200_000),
            "source": "tollgate",
        },
    ]
    hotspots = perf_hotspots.ci_step_hotspots(spans, 10)
    assert [hotspot["name"] for hotspot in hotspots] == ["Rust tests", "Format"]
    rust = hotspots[0]
    assert rust["occurrence_count"] == 3
    assert rust["run_count"] == 3
    assert rust["failed_count"] == 1
    assert rust["total_duration_ms"] == 130_000
    assert rust["average_duration_ms"] == 43_333
    assert rust["p50_duration_ms"] == 20_000
    assert rust["p95_duration_ms"] == 100_000
    assert rust["max_duration_ms"] == 100_000

    aggregate = perf_analysis.aggregate_reports(
        [
            {
                "metadata": {"thread_id": "root", "title": "Fixture"},
                "timing": {
                    "wall_time_ms": 0,
                    "recorded_active_coverage_ms": 0,
                    "known_wait_union_ms": 0,
                    "unattributed_agent_turn_ms": 0,
                    "category_exclusive_ms": {},
                },
                "longest_operations": [],
                "largest_contributors": [],
                "longest_waits": [],
                "findings": [],
                "spans": spans,
            }
        ],
        1,
    )
    assert aggregate["ci_step_hotspots"] == [rust]


def _normalized_ci_span(
    span_id: str,
    run_id: str,
    name: str,
    duration_ms: int,
    status: str = "passed",
    parent_prefix: str = "ci-run",
) -> dict[str, object]:
    return {
        "id": f"ci-step:{span_id}",
        "parent_id": f"{parent_prefix}:{run_id}",
        "source": "ci",
        "category": "ci",
        "name": name,
        "duration_ms": duration_ms,
        "status": status,
        "attributes": {"run_id": run_id},
    }


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
    operation = Span(
        "operation:web", None, None, "operation", "operation", "Web preparation",
        2, 2.5, "passed",
        attributes={"operation_id": "web", "task_id": "root", "root_operation_id": "web"},
    )
    milestone = Span(
        "workflow:root:focused", None, None, "workflow", "milestone", "focused.passed",
        2.75, 2.75, "passed",
        attributes={"workflow_id": "root", "task_id": "root", "milestone": "focused.passed"},
    )
    perf_analysis.correlate_activity(
        [session], [run], [operation, milestone], [tollgate], [candidate], repository, warnings,
    )
    assert run.session_id == "root"
    assert run.parent_id == wrapper.id
    assert tollgate.session_id == "root"
    assert tollgate.association == "exact_tree"
    assert operation.session_id == "root"
    assert milestone.session_id == "root"
    report = perf_analysis.analyze_session(session, Thresholds(1, 1, 1, 1), 20)
    assert wrapper.id not in {span["id"] for span in report["longest_operations"]}
    assert next(
        span for span in report["spans"] if span["id"] == wrapper.id
    )["exclusive_duration_ms"] == 1000
    assert report["lifecycle"]["events"][0]["name"] == "focused.passed"


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
    history = [
        {"kind": "candidate.created", "created_at": "2026-01-01T00:00:08Z", "payload": {"id": candidate_id}},
        {"kind": "candidate.promotion-authorized", "created_at": "2026-01-01T00:00:09Z", "payload": {"item_id": candidate_id}},
        {"kind": "queue.item-updated", "created_at": "2026-01-01T00:00:10Z", "payload": {"id": candidate_id, "certificate_id": "certificate"}},
        {"kind": "promotion.completed", "created_at": "2026-01-01T00:00:11Z", "payload": {"id": candidate_id}},
        {"kind": "queue.item-updated", "created_at": "2026-01-01T00:00:12Z", "payload": {"id": candidate_id, "remote_state": "synchronized"}},
        "unknown",
    ]
    with patch.object(perf_sources, "_run_json_command", side_effect=[status, history]):
        spans, warnings, candidates = perf_sources.read_tollgate(root)
    assert len([span for span in spans if span.id.startswith("tg-buildset:")]) == 2
    assert candidates[0]["attempts"][1]["attempt"] == 2
    assert any("non-object" in warning for warning in warnings)
    findings = perf_analysis.workflow_findings(spans, Thresholds(1, 1, 1, 1))
    assert any(finding["code"] == "tollgate-retry" for finding in findings)
    assert len([span for span in spans if span.category == "milestone"]) == 5
    lifecycle = perf_analysis.workflow_lifecycle(
        spans, parse_timestamp("2025-12-31T23:59:59Z")
    )
    assert lifecycle["deliveries"] == [
        {
            "candidate_id": candidate_id,
            "submitted_at": "2026-01-01T00:00:08.000Z",
            "authorized_at": "2026-01-01T00:00:09.000Z",
            "certified_at": "2026-01-01T00:00:10.000Z",
            "promoted_at": "2026-01-01T00:00:11.000Z",
            "synchronized_at": "2026-01-01T00:00:12.000Z",
            "submission_to_remote_ms": 4000,
            "approval_to_remote_ms": 3000,
        }
    ]
    assert lifecycle["durations_ms"]["request_to_final_remote"] == 13_000


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
