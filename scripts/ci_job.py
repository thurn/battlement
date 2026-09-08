#!/usr/bin/env python3

"""Start, inspect, wait for, or cancel one durable local CI operation."""

from __future__ import annotations

import argparse
from contextlib import contextmanager
from datetime import datetime, timezone
import json
import hashlib
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import uuid

import perf_log
import operation_log
from platform_support import lock_file, unlock_file
import process_identity
import workflow_event


REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
TERMINAL_STATES = {"passed", "failed", "canceled", "interrupted", "inputs-invalidated"}


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="milliseconds").replace("+00:00", "Z")


def job_root(repository: Path = REPOSITORY_ROOT) -> Path:
    configured = os.environ.get("BATTLEMENT_CI_JOB_ROOT")
    return Path(configured) if configured else repository / ".logs/ci-jobs"


@contextmanager
def registry_lock(root: Path):
    root.mkdir(mode=0o700, parents=True, exist_ok=True)
    with (root / "registry.lock").open("a+", encoding="utf-8") as handle:
        lock_file(handle)
        try:
            yield
        finally:
            unlock_file(handle)


def atomic_write(path: Path, value: dict) -> None:
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as handle:
            json.dump(value, handle, sort_keys=True)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    finally:
        try:
            os.unlink(temporary)
        except FileNotFoundError:
            pass


def read_job(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def observe_progress(job: dict) -> dict:
    """Add a bounded current-step and log-progress view without rewriting the handle."""
    result = dict(job)
    path = Path(job["log_path"])
    try:
        stat = path.stat()
        with path.open("rb") as handle:
            handle.seek(max(0, stat.st_size - 4 * 1024 * 1024))
            lines = handle.read().decode("utf-8", errors="replace").splitlines()
    except OSError:
        lines = []
        stat = None
    active = []
    waiting_for = None
    for line in lines:
        if line.startswith("==> "):
            active.append(line[4:])
        elif line.startswith("<== "):
            completed = line[4:].partition(" (")[0]
            for index in range(len(active) - 1, -1, -1):
                if active[index] == completed:
                    active.pop(index)
                    break
        if "waiting for " in line.casefold() or " queued for " in line.casefold():
            waiting_for = line.strip()[-500:]
    result["current_step"] = None if job["state"] in TERMINAL_STATES else (
        active[-1] if active else None
    )
    result["waiting_for"] = waiting_for
    result["last_progress"] = None if stat is None else {
        "bytes": stat.st_size,
        "modified_at": datetime.fromtimestamp(stat.st_mtime, timezone.utc).isoformat(
            timespec="milliseconds"
        ).replace("+00:00", "Z"),
    }
    return result


def source_identity(repository: Path) -> dict:
    source = perf_log.git_metadata(repository)
    digest = hashlib.sha256()
    diff = subprocess.run(["git", "diff", "--binary", "HEAD"], cwd=repository,
                          check=True, capture_output=True).stdout
    digest.update(diff)
    untracked = subprocess.run(
        ["git", "ls-files", "--others", "--exclude-standard", "-z"], cwd=repository,
        check=True, capture_output=True,
    ).stdout.split(b"\0")
    for encoded in sorted(path for path in untracked if path):
        path = repository / os.fsdecode(encoded)
        digest.update(encoded + b"\0")
        if path.is_file() and not path.is_symlink():
            digest.update(path.read_bytes())
    result = {key: source[key] for key in (
        "repository_url", "worktree_path", "head_oid", "staged_tree_oid"
    )}
    result["working_tree_digest"] = digest.hexdigest()
    return result


def job_key(repository: Path, arguments: list[str], purpose: str) -> dict:
    return {
        "source": source_identity(repository),
        "arguments": arguments,
        "purpose": purpose,
        "task_id": os.environ.get("CODEX_THREAD_ID"),
    }


def update(path: Path, **changes) -> dict:
    with registry_lock(path.parent.parent):
        job = read_job(path)
        job.update(changes)
        job["revision"] = int(job.get("revision", 0)) + 1
        job["updated_at"] = utc_now()
        atomic_write(path, job)
        return job


def refresh(path: Path) -> dict:
    job = read_job(path)
    if job["state"] in TERMINAL_STATES:
        return observe_progress(job)
    if not process_identity.matches(job.get("process", {})):
        return observe_progress(update(path, state="interrupted", finished_at=utc_now(),
                                       failure="supervisor-exited-without-terminal-state"))
    return observe_progress(job)


def resolve_job(root: Path, value: str) -> Path:
    candidate = Path(value)
    if candidate.is_file():
        return candidate.resolve()
    path = root / value / "job.json"
    if not path.is_file():
        raise FileNotFoundError(f"unknown CI job {value!r}")
    return path


def start_job(repository: Path, arguments: list[str], purpose: str = "local-validation",
              *, command: list[str] | None = None) -> dict:
    root = job_root(repository)
    key = job_key(repository, arguments, purpose)
    with registry_lock(root):
        for path in sorted(root.glob("*/job.json"), reverse=True):
            job = read_job(path)
            if job.get("key") == key and job.get("state") not in TERMINAL_STATES:
                if process_identity.matches(job.get("process", {})):
                    return observe_progress(job) | {"attached": True}
            same_owner = (
                job.get("state") not in TERMINAL_STATES
                and job.get("key", {}).get("purpose") == purpose
                and job.get("key", {}).get("task_id") == key["task_id"]
                and job.get("key", {}).get("source", {}).get("worktree_path")
                == key["source"]["worktree_path"]
            )
            if same_owner:
                job.update(state="inputs-invalidated", finished_at=utc_now(),
                           failure="working-source-identity-changed",
                           revision=int(job.get("revision", 0)) + 1, updated_at=utc_now())
                atomic_write(path, job)
                process = job.get("process", {})
                if process_identity.matches(process):
                    if job.get("owns_process_group"):
                        os.killpg(process["pid"], signal.SIGTERM)
                    else:
                        os.kill(process["pid"], signal.SIGTERM)
        identifier = str(uuid.uuid4())
        directory = root / identifier
        directory.mkdir(mode=0o700)
        path = directory / "job.json"
        job = {
            "schema": 1, "job_id": identifier, "state": "queued", "revision": 1,
            "created_at": utc_now(), "updated_at": utc_now(), "key": key,
            "log_path": str((directory / "output.log").resolve()),
            "handle_path": str(path.resolve()), "process": {}, "child_process": None,
            "exit_code": None, "owns_process_group": "TOLLGATE_BUILDSET_ID" not in os.environ,
        }
        atomic_write(path, job)
        supervise_command = [sys.executable, str(Path(__file__).resolve()), "_supervise", str(path)]
        environment = dict(os.environ)
        if command is not None:
            environment["BATTLEMENT_CI_JOB_TEST_COMMAND"] = json.dumps(command)
        child = subprocess.Popen(supervise_command, cwd=repository, env=environment,
                                 stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                                 stderr=subprocess.DEVNULL,
                                 start_new_session=job["owns_process_group"])
        job["process"] = process_identity.identity(child.pid)
        job["revision"] += 1
        job["updated_at"] = utc_now()
        atomic_write(path, job)
        return observe_progress(job) | {"attached": False}


def supervise(path: Path) -> int:
    job = read_job(path)
    repository = Path(job["key"]["source"]["worktree_path"])
    test_command = os.environ.pop("BATTLEMENT_CI_JOB_TEST_COMMAND", None)
    command = json.loads(test_command) if test_command else [
        sys.executable, str(repository / "scripts/ci.py"), *job["key"]["arguments"]
    ]
    canceled = False
    child: subprocess.Popen | None = None

    def stop(_number, _frame):
        nonlocal canceled
        canceled = True
        if child is not None and child.poll() is None:
            child.terminate()

    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)
    operation = operation_log.Operation(repository, "ci-job", operation_id=job["job_id"],
                                        metadata={"purpose": job["key"]["purpose"]})
    with operation, Path(job["log_path"]).open("ab", buffering=0) as output:
        environment = operation_log.child_environment()
        environment["BATTLEMENT_CI_JOB_ID"] = job["job_id"]
        child = subprocess.Popen(command, cwd=repository, env=environment,
                                 stdin=subprocess.DEVNULL, stdout=output, stderr=subprocess.STDOUT)
        update(path, state="running", started_at=utc_now(),
               child_process=process_identity.identity(child.pid), operation_id=operation.id)
        result = child.wait()
        retained_state = read_job(path)["state"]
        invalidated = retained_state == "inputs-invalidated"
        if not invalidated:
            invalidated = source_identity(repository) != job["key"]["source"]
        state = "inputs-invalidated" if invalidated else (
            "canceled" if canceled else "passed" if result == 0 else "failed")
        update(path, state=state, exit_code=result, finished_at=utc_now(), child_process=None)
        operation.finish(state, result, handle_path=str(path))
        task_id = job["key"].get("task_id")
        if task_id:
            try:
                workflow_event.record(
                    "jobs.finished",
                    repository=repository,
                    workflow_id=task_id,
                    job_id=job["job_id"],
                    outcome=state if state in {"passed", "failed", "canceled"} else "failed",
                )
            except (OSError, ValueError):
                pass
    return result


def cancel(path: Path) -> dict:
    job = refresh(path)
    if job["state"] in TERMINAL_STATES:
        return job
    process = job["process"]
    if not process_identity.matches(process):
        return refresh(path)
    if job.get("owns_process_group"):
        os.killpg(process["pid"], signal.SIGTERM)
    else:
        os.kill(process["pid"], signal.SIGTERM)
    deadline = time.monotonic() + 15
    while time.monotonic() < deadline:
        time.sleep(0.05)
        job = refresh(path)
        if job["state"] in TERMINAL_STATES:
            return job
    raise TimeoutError("CI supervisor did not stop within 15 seconds")


def wait_for(path: Path, after: int | None, timeout_seconds: float,
             repository: Path | None = None) -> dict:
    deadline = time.monotonic() + timeout_seconds
    while True:
        job = refresh(path)
        changed = after is not None and job["revision"] > after
        if changed or job["state"] in TERMINAL_STATES:
            return job | {"timed_out": False}
        if time.monotonic() >= deadline:
            return job | {"timed_out": True}
        time.sleep(0.1)


def print_job(job: dict, json_output: bool) -> None:
    if json_output:
        print(json.dumps(job, sort_keys=True))
        return
    print(f"{job['job_id']}  {job['state']}  revision {job['revision']}")
    print(f"  log  {job['log_path']}")


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true", help="return stable JSON")
    commands = parser.add_subparsers(dest="command", required=True)
    start = commands.add_parser("start", help="start or attach to matching active CI")
    start.add_argument("--purpose", default="local-validation")
    start.add_argument("--full", action="store_true")
    start.add_argument("--ditto", action="store_true")
    start.add_argument("--no-ci-cache", action="store_true")
    for name in ("status", "cancel"):
        command = commands.add_parser(name)
        command.add_argument("job")
    wait = commands.add_parser("wait")
    wait.add_argument("job")
    wait.add_argument("--after", type=int)
    wait.add_argument("--timeout", type=float, default=60)
    internal = commands.add_parser("_supervise", help=argparse.SUPPRESS)
    internal.add_argument("job")
    return parser.parse_args()


def main() -> int:
    arguments = parse_arguments()
    root = job_root()
    if arguments.command == "_supervise":
        return supervise(Path(arguments.job))
    if arguments.command == "start":
        ci_arguments = [name for selected, name in (
            (arguments.full, "--full"), (arguments.ditto, "--ditto"),
            (arguments.no_ci_cache, "--no-ci-cache"),
        ) if selected]
        job = start_job(REPOSITORY_ROOT, ci_arguments, arguments.purpose)
    else:
        path = resolve_job(root, arguments.job)
        if arguments.command == "status":
            job = refresh(path)
        elif arguments.command == "cancel":
            job = cancel(path)
        else:
            job = wait_for(path, arguments.after, arguments.timeout, REPOSITORY_ROOT)
    print_job(job, arguments.json)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
