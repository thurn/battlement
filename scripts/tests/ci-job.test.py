#!/usr/bin/env python3

"""Exercise durable CI job attachment, waiting, invalidation, and cancellation."""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY_ROOT / "scripts"))

import ci_job  # noqa: E402
import perf_ci  # noqa: E402
import platform_support  # noqa: E402
import process_identity  # noqa: E402


def await_state(path: Path, state: str, timeout: float = 5) -> dict:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        job = ci_job.refresh(path)
        if job["state"] == state:
            return job
        time.sleep(0.02)
    raise AssertionError(f"job did not reach {state}: {ci_job.read_job(path)}")


def scratch_repository(path: Path) -> Path:
    """Create a one-commit repository so job source identity stays cheap to compute."""
    path.mkdir()
    subprocess.run(["git", "init", "-q"], cwd=path, check=True)
    subprocess.run(
        ["git", "-c", "user.name=ci-job-test", "-c", "user.email=ci-job-test@invalid",
         "-c", "commit.gpgsign=false", "commit", "-q", "--allow-empty", "-m", "fixture"],
        cwd=path, check=True,
    )
    return path


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="battlement-ci-job-test.") as temporary:
        repository = scratch_repository(Path(temporary) / "repository")
        os.environ["BATTLEMENT_CI_JOB_ROOT"] = temporary
        os.environ["BATTLEMENT_LOG_ROOT"] = str(Path(temporary) / "performance")
        os.environ["CODEX_THREAD_ID"] = "11111111-2222-4333-8444-555555555555"
        release_path = Path(temporary) / "fixture.release"
        release_source = """
import pathlib
import sys
import time
release_path = pathlib.Path(sys.argv[1])
print('==> Fixture', flush=True)
print('==> Run Unity Edit Mode tests (native-integration)', flush=True)
print('<== Run Unity Edit Mode tests (native-integration) (95.9s)', flush=True)
print('==> Later check', flush=True)
print('<== Later check (0.1s)', flush=True)
deadline = time.monotonic() + 30
while not release_path.exists():
    if time.monotonic() > deadline:
        sys.exit('fixture was never released')
    time.sleep(.02)
"""
        command = [sys.executable, "-c", release_source, str(release_path)]
        first = ci_job.start_job(repository, ["--full"], command=command)
        second = ci_job.start_job(repository, ["--full"], command=command)
        assert second["attached"] is True
        assert first["job_id"] == second["job_id"]
        path = Path(first["handle_path"])
        running = await_state(path, "running")
        deadline = time.monotonic() + 2
        while (
            running["current_step"] != "Fixture"
            or not running["last_progress"]
            or running["last_progress"]["bytes"] == 0
        ) and time.monotonic() < deadline:
            time.sleep(.02)
            running = ci_job.refresh(path)
        assert running["current_step"] == "Fixture"
        assert running["last_progress"]["bytes"] > 0
        status = subprocess.run(
            [sys.executable, str(REPOSITORY_ROOT / "scripts/ci_job.py"),
             "--json", "status", first["job_id"]],
            check=True, capture_output=True, text=True,
        )
        assert json.loads(status.stdout)["current_step"] == "Fixture"
        unchanged = ci_job.wait_for(path, running["revision"], .05)
        assert unchanged["timed_out"] is True
        assert unchanged["job_id"] == first["job_id"]
        release_path.write_text("release")
        passed = ci_job.wait_for(path, running["revision"], 5)
        assert passed["state"] == "passed"
        assert passed["current_step"] is None
        assert passed["timed_out"] is False
        milestones, warnings = perf_ci.read_workflow_milestones(
            Path(os.environ["BATTLEMENT_LOG_ROOT"])
        )
        assert not warnings
        completed = next(span for span in milestones if span.attributes["job_id"] == first["job_id"])
        assert completed.name == "jobs.finished" and completed.status == "passed"

        lock_path = Path(temporary) / "cancel.lock"
        descendant_path = Path(temporary) / "descendant.pid"
        ready_path = Path(temporary) / "cancel.ready"
        descendant_source = """
import pathlib
import os
import signal
import sys
import threading
sys.path.insert(0, sys.argv[1])
from platform_support import lock_file
lock_path, descendant_path, ready_path = map(pathlib.Path, sys.argv[2:])
signal.signal(signal.SIGTERM, signal.SIG_IGN)
descendant_path.write_text(str(os.getpid()))
with lock_path.open('a+') as lease:
    lock_file(lease)
    ready_path.write_text('ready')
    threading.Event().wait()
"""
        cancel_source = """
import subprocess
import sys
import threading
descendant = subprocess.Popen([
    sys.executable, '-c', sys.argv[1], *sys.argv[2:]
])
threading.Event().wait()
"""
        cancel_command = [
            sys.executable,
            "-c",
            cancel_source,
            descendant_source,
            str(REPOSITORY_ROOT / "scripts"),
            str(lock_path),
            str(descendant_path),
            str(ready_path),
        ]
        cancelable = ci_job.start_job(
            repository,
            ["--ditto"],
            command=cancel_command,
        )
        cancel_path = Path(cancelable["handle_path"])
        await_state(cancel_path, "running")
        blocking_command = [
            sys.executable,
            "-c",
            "import threading; threading.Event().wait()",
        ]
        unrelated = ci_job.start_job(
            repository,
            [],
            purpose="unrelated-job",
            command=blocking_command,
        )
        unrelated_path = Path(unrelated["handle_path"])
        await_state(unrelated_path, "running")
        deadline = time.monotonic() + 2
        while not ready_path.is_file() and time.monotonic() < deadline:
            time.sleep(0.02)
        assert ready_path.is_file(), "cancel fixture did not acquire its cache lock"
        descendant = process_identity.identity(int(descendant_path.read_text()))
        canceled = ci_job.cancel(cancel_path)
        assert canceled["state"] == "canceled"
        assert ci_job.refresh(unrelated_path)["state"] == "running"
        with lock_path.open("a+") as lease:
            assert platform_support.try_lock_file(lease), "canceled job retained its lock"
            platform_support.unlock_file(lease)
        deadline = time.monotonic() + 2
        while process_identity.matches(descendant) and time.monotonic() < deadline:
            time.sleep(0.02)
        assert not process_identity.matches(descendant), "canceled job retained a subprocess"
        assert ci_job.cancel(unrelated_path)["state"] == "canceled"

        invalid = ci_job.start_job(
            repository,
            [],
            purpose="source-test",
            command=blocking_command,
        )
        invalid_path = Path(invalid["handle_path"])
        await_state(invalid_path, "running")
        changed_source = dict(ci_job.read_job(invalid_path)["key"]["source"])
        changed_source["staged_tree_oid"] = "changed"
        original = ci_job.source_identity
        ci_job.source_identity = lambda _repository: changed_source
        try:
            replacement = ci_job.start_job(
                repository, [], purpose="source-test", command=[sys.executable, "-c", "pass"]
            )
            assert replacement["job_id"] != invalid["job_id"]
            assert ci_job.refresh(invalid_path)["state"] == "inputs-invalidated"
        finally:
            ci_job.source_identity = original
        deadline = time.monotonic() + 5
        process = ci_job.read_job(invalid_path)["process"]
        while ci_job.process_identity.matches(process) and time.monotonic() < deadline:
            time.sleep(.02)
        assert not ci_job.process_identity.matches(process)
        replacement_final = ci_job.wait_for(Path(replacement["handle_path"]), None, 5)
        assert replacement_final["state"] == "inputs-invalidated"

    print("CI job handle tests passed.")


if __name__ == "__main__":
    main()
