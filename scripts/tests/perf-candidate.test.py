#!/usr/bin/env python3

"""Verify exact candidate reports exclude unrelated task candidates."""

from pathlib import Path
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

import perf_candidate
from perf_model import SessionTrace, Span


with tempfile.TemporaryDirectory(prefix="battlement-perf-candidate.") as temporary:
    repository = Path(temporary)
    subprocess.run(["git", "init", "-q"], cwd=repository, check=True)
    (repository / "fixture").write_text("fixture\n")
    subprocess.run(["git", "add", "fixture"], cwd=repository, check=True)
    subprocess.run(
        [
            "git", "-c", "user.name=Fixture", "-c",
            "user.email=fixture@example.invalid", "commit", "-qm", "fixture",
        ],
        cwd=repository,
        check=True,
    )
    oid = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=repository, text=True
    ).strip()
    tree = subprocess.check_output(
        ["git", "rev-parse", "HEAD^{tree}"], cwd=repository, text=True
    ).strip()
    candidate_id = "candidate-exact"
    buildset_id = "buildset-exact"
    candidate = {
        "item": {
            "id": candidate_id,
            "source_oid": {"bytes": oid},
            "metadata": {"worktree_path": "/task/exact"},
            "state": "promoted",
        },
        "attempts": [{
            "id": buildset_id,
            "validation_generation_id": "generation-exact",
            "tested_oid": {"bytes": oid},
            "attempt": 1,
            "state": "passed",
            "started_at": 20,
            "finished_at": 30,
            "step_results": [],
        }],
    }
    local = Span(
        "ci-run:local", None, None, "ci", "ci", "ci.py", 10, 12, "passed",
        attributes={
            "run_id": "local", "head_oid": "parent", "staged_tree_oid": tree,
            "worktree_path": "/task/exact", "full": False,
        },
    )
    certified_run = Span(
        "ci-run:certified", None, None, "ci", "ci", "ci.py", 21, 29, "passed",
        attributes={
            "run_id": "certified", "head_oid": oid, "staged_tree_oid": tree,
            "worktree_path": "/tollgate/slot", "full": True,
        },
    )
    unrelated = Span(
        "ci-run:unrelated", None, None, "ci", "ci", "ci.py", 40, 50, "passed",
        attributes={
            "run_id": "unrelated", "head_oid": "other", "staged_tree_oid": "other",
            "worktree_path": "/task/exact", "full": True,
        },
    )
    cache = Span(
        "ci-event:cache", "ci-run:local", None, "ci", "event", "cache", 11, 11,
        "passed", attributes={"run_id": "local", "event": "ci.cache_lookup", "result": "hit"},
    )
    intent = Span(
        "tg-intent:exact", None, None, "tollgate", "tollgate-phase", "push",
        31, 32, "passed", attributes={"candidate_id": candidate_id},
    )
    other_intent = Span(
        "tg-intent:other", None, None, "tollgate", "tollgate-phase", "push",
        41, 42, "passed", attributes={"candidate_id": "candidate-other"},
    )
    submitted = Span(
        "tg-submitted", None, None, "tollgate", "milestone", "candidate.submitted",
        19, 19, "passed", attributes={
            "candidate_id": candidate_id, "milestone": "candidate.submitted"
        },
    )
    certified_milestone = Span(
        "tg-certified", None, None, "tollgate", "milestone", "candidate.certified",
        30, 30, "passed", attributes={
            "candidate_id": candidate_id, "milestone": "candidate.certified"
        },
    )
    report = perf_candidate.build(
        candidate_id,
        [candidate],
        [local, certified_run, unrelated, cache],
        [intent, other_intent, submitted, certified_milestone],
        repository,
    )
    assert report["source_oid"] == oid
    assert report["source_tree_oid"] == tree
    assert report["buildsets"][0]["staged_tree_oid"] == tree
    assert [run["run_id"] for run in report["ci_runs"]] == ["local", "certified"]
    assert report["ci_runs"][0]["cache"]["hits"] == 1
    assert [value["id"] for value in report["operation_intents"]] == ["tg-intent:exact"]
    assert report["timings_ms"]["submission_to_certification"] == 11_000

    session = SessionTrace("task", "Task", repository, "repository")
    session.spans = [local, certified_run, unrelated, intent, other_intent]
    perf_candidate.retain_exact_spans(session, report)
    assert unrelated not in session.spans
    assert other_intent not in session.spans
    assert local in session.spans and certified_run in session.spans and intent in session.spans

print("Candidate performance report tests passed")
