#!/usr/bin/env python3

"""Record one task-scoped workflow milestone for performance correlation."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import fcntl
import json
import os
from pathlib import Path
import re
import time
import uuid

import perf_log
import process_identity


REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
MILESTONES = (
    "focused.passed",
    "review.ready",
    "wrapup.requested",
    "checkpoint.committed",
    "agent.stopped",
    "jobs.finished",
)


def record(
    milestone: str,
    *,
    repository: Path = REPOSITORY_ROOT,
    log_root: Path | None = None,
    workflow_id: str | None = None,
    candidate_id: str | None = None,
    job_id: str | None = None,
    outcome: str | None = None,
) -> Path:
    """Append one bounded milestone using the task ID as its durable episode key."""
    if milestone not in MILESTONES:
        raise ValueError(f"unsupported milestone {milestone!r}")
    workflow_id = workflow_id or os.environ.get("CODEX_THREAD_ID")
    if not workflow_id:
        raise ValueError("workflow identity requires CODEX_THREAD_ID or --workflow-id")
    uuid.UUID(workflow_id)
    for label, value in (("candidate", candidate_id), ("job", job_id)):
        if value is not None and not re.fullmatch(r"[A-Za-z0-9._:-]{1,128}", value):
            raise ValueError(f"{label} identity contains unsupported characters")
    if outcome is not None and outcome not in {"passed", "failed", "canceled", "unknown"}:
        raise ValueError("outcome must be passed, failed, canceled, or unknown")
    root = log_root or perf_log.configured_log_root()
    directory = root / "workflows" / datetime.now(timezone.utc).date().isoformat()
    directory.mkdir(mode=0o700, parents=True, exist_ok=True)
    path = directory / f"{workflow_id}.jsonl"
    context = perf_log.git_metadata(repository)
    context.update(
        task_id=os.environ.get("CODEX_THREAD_ID"),
        turn_id=os.environ.get("CODEX_TURN_ID"),
        root_operation_id=os.environ.get("BATTLEMENT_ROOT_OPERATION_ID"),
    )
    record = {
        "schema": 1,
        "event": "workflow.milestone",
        "milestone": milestone,
        "workflow_id": workflow_id,
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "monotonic_ns": time.monotonic_ns(),
        "clock_domain": {
            "host": process_identity.identity()["host"],
            "boot_process": process_identity.identity(1)["birth"],
        },
        "context": context,
        "candidate_id": candidate_id,
        "job_id": job_id,
        "outcome": outcome,
    }
    encoded = json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n"
    descriptor = os.open(path, os.O_CREAT | os.O_APPEND | os.O_WRONLY, 0o600)
    try:
        with os.fdopen(descriptor, "a", encoding="utf-8") as destination:
            fcntl.flock(destination, fcntl.LOCK_EX)
            destination.write(encoded)
            destination.flush()
            os.fsync(destination.fileno())
    finally:
        os.chmod(path, 0o600)
    return path


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("milestone", choices=MILESTONES)
    parser.add_argument("--workflow-id")
    parser.add_argument("--candidate-id")
    parser.add_argument("--job-id")
    parser.add_argument("--outcome", choices=("passed", "failed", "canceled", "unknown"))
    return parser.parse_args()


if __name__ == "__main__":
    arguments = parse_arguments()
    print(
        record(
            arguments.milestone,
            workflow_id=arguments.workflow_id,
            candidate_id=arguments.candidate_id,
            job_id=arguments.job_id,
            outcome=arguments.outcome,
        )
    )
