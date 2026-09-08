#!/usr/bin/env python3

"""Black-box checks for the Ditto CI runner."""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import time
import tomllib
import uuid


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
RUNNER = REPOSITORY_ROOT / "scripts/ditto_ci.py"
sys.path.insert(0, str(REPOSITORY_ROOT / "scripts"))
import ditto_evidence


FAKE_DITTO = r'''#!/usr/bin/env python3
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import tomllib

arguments = sys.argv[1:]
if "storage" in arguments:
    with Path(os.environ["FAKE_PUBLISH_LOG"]).open("a") as output:
        output.write(arguments[arguments.index("--config") + 1] + "\n")
    raise SystemExit(0)
output = Path(arguments[arguments.index("--output") + 1])
run = Path(os.environ["FAKE_RUN_ROOT"]) / output.parent.parent.name / output.parent.name
run.mkdir(parents=True, exist_ok=True)
(run / "logs").mkdir(exist_ok=True)
(run / "logs/events.jsonl").write_text('{"sequence":1}\n')
(run / "diagnostics.txt").write_bytes(b"private failure diagnostics\n")
print(f"DITTO_RUN_DIR={run}", file=sys.stderr, flush=True)
if expected_cache := os.environ.get("FAKE_EXPECTED_CACHE"):
    assert os.environ["DITTO_CACHE_ROOT"] == expected_cache
if marker := os.environ.get("FAKE_CHILD_MARKER"):
    subprocess.Popen([
        sys.executable,
        "-c",
        "import os,time; time.sleep(0.5); open(os.environ['FAKE_CHILD_MARKER'], 'w').write('leaked')",
    ])
time.sleep(float(os.environ.get("FAKE_SLEEP", "0")))
status = os.environ.get("FAKE_STATUS", "passed")
disposition = "reused" if "--no-build" in arguments else "created"
config = Path(arguments[arguments.index("--config") + 1])
suite = tomllib.loads(config.read_text())
names = [scenario["name"] for scenario in suite["scenarios"]]
selected = [argument for argument in arguments if argument in names]
if selected:
    names = selected
result = {
    "run_id": "0197b35f-6e24-75d8-9482-aa6c22a15133",
    "status": status,
    "build": {"disposition": disposition, "fingerprint": "a" * 64},
    "player_sessions": [{
        "startup_report": {"capture_adapter": "native-screen-capture"},
    }],
    "scenarios": [{"name": name, "status": status} for name in names],
}
output.parent.mkdir(parents=True, exist_ok=True)
result_mode = os.environ.get("FAKE_RESULT", "complete")
if result_mode == "complete":
    output.write_text(json.dumps(result))
elif result_mode == "malformed":
    output.write_text("{")
if os.environ.get("DITTO_REPLAY_BUILD_FINGERPRINT"):
    assert os.environ["DITTO_REPLAY_BUILD_FINGERPRINT"] == "a" * 64
    Path(os.environ["FAKE_REPLAY_MARKER"]).write_text("replayed")
time.sleep(float(os.environ.get("FAKE_SLEEP_AFTER_RESULT", "0")))
raise SystemExit(0 if status == "passed" else 1)
'''


def run(
    arguments: list[str],
    environment: dict[str, str],
    runner: Path = RUNNER,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(runner), *arguments],
        cwd=REPOSITORY_ROOT,
        env=environment,
        capture_output=True,
        text=True,
    )


def artifact_root(completed: subprocess.CompletedProcess[str]) -> Path:
    values = [
        line.removeprefix("DITTO_CI_ARTIFACT_ROOT=")
        for line in completed.stdout.splitlines()
        if line.startswith("DITTO_CI_ARTIFACT_ROOT=")
    ]
    assert len(values) == 1, completed.stdout
    return Path(values[0])


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="ditto-ci-test.") as temporary:
        root = Path(temporary)
        fake = root / "ditto.py"
        fake.write_text(FAKE_DITTO)
        if os.name == "nt":
            launcher = root / "ditto.cmd"
            launcher.write_text(f'@"{sys.executable}" "{fake}" %*\n')
        else:
            fake.chmod(0o755)
            launcher = fake
        environment = os.environ.copy()
        environment.update({
            "DITTO_CI_BINARY": str(launcher),
            "DITTO_ODIFF_PATH": str(launcher),
            "DITTO_CI_CACHE_ROOT": str(root / "cache"),
            "DITTO_CI_TEST_HOST": "macos-arm64",
            "FAKE_EXPECTED_CACHE": str(root / "cache"),
            "FAKE_RUN_ROOT": str(root / "runs"),
            "FAKE_PUBLISH_LOG": str(root / "published"),
            "BATTLEMENT_LOG_ROOT": str(root / "operation-logs"),
        })

        passed = run(["sample", "basic"], environment)
        assert passed.returncode == 0, passed.stderr
        artifact = artifact_root(passed) / "basic/run.tar.gz"
        with tarfile.open(artifact) as retained:
            assert "run/logs/events.jsonl" in retained.getnames()
            diagnostics = retained.extractfile("run/diagnostics.txt")
            assert diagnostics is not None
            assert diagnostics.read() == b"private failure diagnostics\n"
        assert not (root / "published").exists()
        owned = artifact_root(passed)
        manifest = ditto_evidence.read(owned / "evidence.json", owned.name)
        assert manifest["status"] == "passed"
        assert "basic/run.tar.gz" in {item["path"] for item in manifest["files"]}
        duplicate = run(["sample", "basic"], {**environment, "DITTO_CI_INVOCATION_ID": owned.name})
        assert duplicate.returncode != 0
        assert ditto_evidence.read(owned / "evidence.json", owned.name) == manifest
        external_id = f"external-{uuid.uuid4()}"
        external_root = root / "external" / external_id
        external = run(["sample", "basic"], {**environment,
                       "DITTO_CI_INVOCATION_ID": external_id,
                       "DITTO_CI_ARTIFACT_ROOT": str(external_root)})
        assert external.returncode == 0, external.stderr
        assert artifact_root(external) == external_root
        assert ditto_evidence.read(external_root / "evidence.json", external_id)["status"] == "passed"

        gated = run(["gate"], environment)
        assert gated.returncode == 0, gated.stderr
        gate = json.loads((artifact_root(gated) / "gate.json").read_text())
        assert gate["status"] == "passed"
        assert len(gate["samples"]) == 6
        assert gate["budget_seconds"] == 120
        suites = [
            tomllib.loads(path.read_text())
            for path in sorted((REPOSITORY_ROOT / "samples").glob("*/ditto.toml"))
        ]
        assert gate["scenario_count"] == sum(
            len(suite["scenarios"]) for suite in suites
        )
        assert gate["screenshot_count"] == sum(
            "screenshot" in step
            for suite in suites
            for scenario in suite["scenarios"]
            for step in scenario["steps"]
        )
        gate_invocation = artifact_root(gated).name
        operation_events = [
            json.loads(line)
            for path in (root / "operation-logs/operations").glob("**/*.jsonl")
            for line in path.read_text().splitlines()
        ]
        native_start = next(
            event for event in operation_events
            if event.get("event") == "operation.started"
            and event.get("metadata", {}).get("ditto_ci_invocation_id") == gate_invocation
        )
        child_processes = [
            event for event in operation_events
            if event.get("event") == "process.started"
            and event.get("operation_id") == native_start["operation_id"]
        ]
        assert len(child_processes) == 6

        environment["FAKE_SLEEP"] = "0.2"
        gated = run(["gate"], environment)
        assert gated.returncode == 0, gated.stderr
        gate = json.loads((artifact_root(gated) / "gate.json").read_text())
        assert 0.2 <= gate["duration_seconds"] < 0.8

        parallel_environment_a = environment.copy()
        parallel_environment_b = environment.copy()
        parallel_environment_a["DITTO_CI_INVOCATION_ID"] = f"parallel-a-{uuid.uuid4()}"
        parallel_environment_b["DITTO_CI_INVOCATION_ID"] = f"parallel-b-{uuid.uuid4()}"
        started = time.monotonic()
        parallel_a = subprocess.Popen(
            [sys.executable, str(RUNNER), "gate"], cwd=REPOSITORY_ROOT,
            env=parallel_environment_a, stdout=subprocess.PIPE,
            stderr=subprocess.PIPE, text=True,
        )
        parallel_b = subprocess.Popen(
            [sys.executable, str(RUNNER), "gate"], cwd=REPOSITORY_ROOT,
            env=parallel_environment_b, stdout=subprocess.PIPE,
            stderr=subprocess.PIPE, text=True,
        )
        stdout_a, stderr_a = parallel_a.communicate()
        stdout_b, stderr_b = parallel_b.communicate()
        elapsed = time.monotonic() - started
        assert parallel_a.returncode == 0, stderr_a
        assert parallel_b.returncode == 0, stderr_b
        assert elapsed < 0.8, elapsed
        root_a = artifact_root(subprocess.CompletedProcess([], 0, stdout_a, stderr_a))
        root_b = artifact_root(subprocess.CompletedProcess([], 0, stdout_b, stderr_b))
        assert root_a != root_b
        assert json.loads((root_a / "gate.json").read_text())["status"] == "passed"
        assert json.loads((root_b / "gate.json").read_text())["status"] == "passed"

        for invocation_root in (root_a, root_b):
            verified = ditto_evidence.read(invocation_root / "evidence.json", invocation_root.name)
            assert verified["status"] == "passed"
            assert len([item for item in verified["files"] if item["path"].endswith("/result.json")]) == 6
        # Both failures remain independently discoverable after the latest alias changes.
        failed_runs = [subprocess.Popen(
            [sys.executable, str(RUNNER), "gate"], cwd=REPOSITORY_ROOT,
            env={**environment, "FAKE_STATUS": "failed"},
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
        ) for _ in range(2)]
        for process in failed_runs:
            stdout, stderr = process.communicate()
            assert process.returncode == 1, stderr
            failed_root = artifact_root(subprocess.CompletedProcess([], 1, stdout, stderr))
            verified = ditto_evidence.read(failed_root / "evidence.json", failed_root.name)
            assert verified["status"] == "failed"
            assert len([item for item in verified["files"] if item["path"].endswith("/run.tar.gz")]) == 6
        result_path = root_a / "basic/result.json"
        original_result = result_path.read_bytes()
        for mutation in ("remove", "alter"):
            if mutation == "remove":
                result_path.unlink()
            else:
                result_path.write_text("corrupted")
            try:
                ditto_evidence.read(root_a / "evidence.json", root_a.name)
                raise AssertionError("Missing or altered evidence was accepted")
            except ValueError as error:
                assert "evidence" in str(error).lower()
            result_path.write_bytes(original_result)
        try:
            ditto_evidence.read(root_a / "evidence.json", root_b.name)
            raise AssertionError("Wrong invocation was accepted")
        except ValueError:
            pass

        environment["DITTO_CI_GATE_BUDGET_SECONDS"] = "0.05"
        over_budget = run(["gate"], environment)
        assert over_budget.returncode == 0
        gate = json.loads((artifact_root(over_budget) / "gate.json").read_text())
        assert gate["status"] == "passed"
        assert any("exceeded" in warning for warning in gate["warnings"])
        environment.pop("DITTO_CI_GATE_BUDGET_SECONDS")
        environment.pop("FAKE_SLEEP")

        environment["DITTO_CI_REUSABLE_BUILD_SECONDS"] = "61"
        reusable_build = run(["gate"], environment)
        assert reusable_build.returncode == 0
        gate = json.loads((artifact_root(reusable_build) / "gate.json").read_text())
        assert gate["reusable_build_seconds"] == 61
        assert gate["added_duration_seconds"] == gate["scenario_execution_seconds"]
        environment.pop("DITTO_CI_REUSABLE_BUILD_SECONDS")

        environment["FAKE_STATUS"] = "failed"
        failed = run(["sample", "chess"], environment)
        assert failed.returncode == 1
        failed_artifact = artifact_root(failed) / "chess/run.tar.gz"
        with tarfile.open(failed_artifact) as retained:
            assert "run/diagnostics.txt" in retained.getnames()
        failed_bytes = failed_artifact.read_bytes()
        environment.pop("FAKE_STATUS")
        succeeding = run(["sample", "chess"], environment)
        assert succeeding.returncode == 0
        assert failed_artifact.read_bytes() == failed_bytes
        assert artifact_root(succeeding) != failed_artifact.parents[1]
        assert not (root / "published").exists()

        for result_mode in ("missing", "malformed"):
            environment.pop("FAKE_STATUS", None)
            environment["FAKE_RESULT"] = result_mode
            invalid_result = run(["sample", "chess"], environment)
            assert invalid_result.returncode == 1
            failed_artifact = artifact_root(invalid_result) / "chess/run.tar.gz"
            with tarfile.open(failed_artifact) as retained:
                assert "run/diagnostics.txt" in retained.getnames()
        environment.pop("FAKE_RESULT")

        marker = root / "leaked-child"
        environment["DITTO_CI_SAMPLE_TIMEOUT_SECONDS"] = "0.1"
        environment["FAKE_CHILD_MARKER"] = str(marker)
        environment["FAKE_SLEEP"] = "10"
        timed_out = run(["sample", "chess"], environment)
        assert timed_out.returncode == 1
        failed_artifact = artifact_root(timed_out) / "chess/run.tar.gz"
        time.sleep(0.8)
        assert not marker.exists()
        with tarfile.open(failed_artifact) as retained:
            assert "run/diagnostics.txt" in retained.getnames()
        environment.pop("DITTO_CI_SAMPLE_TIMEOUT_SECONDS")
        environment.pop("FAKE_CHILD_MARKER")
        environment.pop("FAKE_SLEEP")

        environment["DITTO_CI_SAMPLE_TIMEOUT_SECONDS"] = "0.3"
        environment["FAKE_SLEEP_AFTER_RESULT"] = "10"
        environment["FAKE_STATUS"] = "infrastructureError"
        timeout_with_result = run(["sample", "chess"], environment)
        assert timeout_with_result.returncode == 1
        evidence = artifact_root(timeout_with_result) / "chess"
        recipe_path = evidence / "replay.json"
        recipe = json.loads(recipe_path.read_text())
        assert recipe["source_status"] == "infrastructureError"
        assert recipe["build"]["fingerprint"] == "a" * 64
        assert "DITTO_RUN_DIR=" in (evidence / "stderr.log").read_text()
        assert json.loads((evidence / "timeout.json").read_text())["seconds"] == 0.3
        recipe_bytes = recipe_path.read_bytes()
        environment.pop("DITTO_CI_SAMPLE_TIMEOUT_SECONDS")
        environment.pop("FAKE_SLEEP_AFTER_RESULT")
        environment.pop("FAKE_STATUS")
        replay_marker = root / "replayed-timeout"
        environment["FAKE_REPLAY_MARKER"] = str(replay_marker)
        replayed = run(["replay", str(recipe_path)], environment)
        assert replayed.returncode == 0, replayed.stderr + replayed.stdout
        assert replay_marker.read_text() == "replayed"
        assert recipe_path.read_bytes() == recipe_bytes
        environment.pop("FAKE_REPLAY_MARKER")

        environment.pop("FAKE_STATUS", None)
        environment["DITTO_CI_BRANCH"] = "master"
        published = run(["publish"], environment)
        assert published.returncode == 0, published.stderr
        assert len((root / "published").read_text().splitlines()) == 6

        environment["DITTO_CI_BRANCH"] = "feature"
        skipped = run(["publish"], environment)
        assert skipped.returncode == 0
        assert "publication skipped" in skipped.stdout

    print("Ditto CI tests passed.")


if __name__ == "__main__":
    main()
