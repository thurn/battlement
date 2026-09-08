#!/usr/bin/env python3

"""Exercise durable CI job attachment, waiting, invalidation, and cancellation."""

from __future__ import annotations

import os
from pathlib import Path
import sys
import tempfile
import time


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY_ROOT / "scripts"))

import ci_job  # noqa: E402


def await_state(path: Path, state: str, timeout: float = 5) -> dict:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        job = ci_job.refresh(path)
        if job["state"] == state:
            return job
        time.sleep(0.02)
    raise AssertionError(f"job did not reach {state}: {ci_job.read_job(path)}")


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="battlement-ci-job-test.") as temporary:
        os.environ["BATTLEMENT_CI_JOB_ROOT"] = temporary
        command = [sys.executable, "-c",
                   "import time; print('==> Fixture', flush=True); time.sleep(.4)"]
        first = ci_job.start_job(REPOSITORY_ROOT, ["--full"], command=command)
        second = ci_job.start_job(REPOSITORY_ROOT, ["--full"], command=command)
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
        unchanged = ci_job.wait_for(path, running["revision"], .05)
        assert unchanged["timed_out"] is True
        assert unchanged["job_id"] == first["job_id"]
        passed = ci_job.wait_for(path, running["revision"], 5)
        assert passed["state"] == "passed"
        assert passed["timed_out"] is False

        long_command = [sys.executable, "-c", "import time; time.sleep(30)"]
        cancelable = ci_job.start_job(REPOSITORY_ROOT, ["--ditto"], command=long_command)
        cancel_path = Path(cancelable["handle_path"])
        await_state(cancel_path, "running")
        canceled = ci_job.cancel(cancel_path)
        assert canceled["state"] == "canceled"

        invalid = ci_job.start_job(REPOSITORY_ROOT, [], purpose="source-test", command=long_command)
        invalid_path = Path(invalid["handle_path"])
        await_state(invalid_path, "running")
        changed_source = dict(ci_job.read_job(invalid_path)["key"]["source"])
        changed_source["staged_tree_oid"] = "changed"
        original = ci_job.source_identity
        ci_job.source_identity = lambda _repository: changed_source
        try:
            replacement = ci_job.start_job(
                REPOSITORY_ROOT, [], purpose="source-test", command=[sys.executable, "-c", "pass"]
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
