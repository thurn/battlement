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


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
RUNNER = REPOSITORY_ROOT / "scripts/ditto_ci.py"


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
run = Path(os.environ["FAKE_RUN_ROOT"]) / output.parent.name
run.mkdir(parents=True, exist_ok=True)
(run / "logs").mkdir(exist_ok=True)
(run / "logs/events.jsonl").write_text('{"sequence":1}\n')
(run / "diagnostics.txt").write_text("private failure diagnostics\n")
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
if "--no-build" not in arguments:
    names = [arguments[-1]]
result = {
    "run_id": "0197b35f-6e24-75d8-9482-aa6c22a15133",
    "status": status,
    "build": {"disposition": disposition},
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


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="ditto-ci-test.") as temporary:
        root = Path(temporary)
        fake = root / "ditto"
        fake.write_text(FAKE_DITTO)
        fake.chmod(0o755)
        environment = os.environ.copy()
        environment.update({
            "DITTO_CI_BINARY": str(fake),
            "DITTO_CI_CACHE_ROOT": str(root / "cache"),
            "FAKE_EXPECTED_CACHE": str(root / "cache"),
            "FAKE_RUN_ROOT": str(root / "runs"),
            "FAKE_PUBLISH_LOG": str(root / "published"),
        })

        passed = run(["sample", "basic"], environment)
        assert passed.returncode == 0, passed.stderr
        artifact = REPOSITORY_ROOT / "artifacts/ditto-ci/basic/run.tar.gz"
        with tarfile.open(artifact) as retained:
            assert "run/logs/events.jsonl" in retained.getnames()
            diagnostics = retained.extractfile("run/diagnostics.txt")
            assert diagnostics is not None
            assert diagnostics.read() == b"private failure diagnostics\n"
        assert not (root / "published").exists()

        gated = run(["gate"], environment)
        assert gated.returncode == 0, gated.stderr
        gate = json.loads(
            (REPOSITORY_ROOT / "artifacts/ditto-ci/gate.json").read_text()
        )
        assert gate["status"] == "passed"
        assert len(gate["samples"]) == 5
        assert gate["budget_seconds"] == 110
        assert gate["added_budget_seconds"] == 110

        environment["FAKE_SLEEP"] = "0.2"
        gated = run(["gate"], environment)
        assert gated.returncode == 0, gated.stderr
        gate = json.loads(
            (REPOSITORY_ROOT / "artifacts/ditto-ci/gate.json").read_text()
        )
        assert gate["duration_seconds"] < 0.7

        environment["DITTO_CI_GATE_BUDGET_SECONDS"] = "0.05"
        over_budget = run(["gate"], environment)
        assert over_budget.returncode == 1
        gate = json.loads(
            (REPOSITORY_ROOT / "artifacts/ditto-ci/gate.json").read_text()
        )
        assert gate["status"] == "failed"
        assert any("gate budget" in failure for failure in gate["failures"])
        environment.pop("DITTO_CI_GATE_BUDGET_SECONDS")
        environment.pop("FAKE_SLEEP")

        environment["DITTO_CI_REUSABLE_BUILD_SECONDS"] = "61"
        reusable_build = run(["gate"], environment)
        assert reusable_build.returncode == 0
        gate = json.loads(
            (REPOSITORY_ROOT / "artifacts/ditto-ci/gate.json").read_text()
        )
        assert gate["reusable_build_seconds"] == 61
        assert gate["added_duration_seconds"] == gate["scenario_execution_seconds"]
        environment.pop("DITTO_CI_REUSABLE_BUILD_SECONDS")

        environment["FAKE_STATUS"] = "failed"
        failed = run(["sample", "chess"], environment)
        assert failed.returncode == 1
        failed_artifact = REPOSITORY_ROOT / "artifacts/ditto-ci/chess/run.tar.gz"
        with tarfile.open(failed_artifact) as retained:
            assert "run/diagnostics.txt" in retained.getnames()
        assert not (root / "published").exists()

        for result_mode in ("missing", "malformed"):
            environment.pop("FAKE_STATUS", None)
            environment["FAKE_RESULT"] = result_mode
            invalid_result = run(["sample", "chess"], environment)
            assert invalid_result.returncode == 1
            with tarfile.open(failed_artifact) as retained:
                assert "run/diagnostics.txt" in retained.getnames()
        environment.pop("FAKE_RESULT")

        marker = root / "leaked-child"
        environment["DITTO_CI_SAMPLE_TIMEOUT_SECONDS"] = "0.1"
        environment["FAKE_CHILD_MARKER"] = str(marker)
        environment["FAKE_SLEEP"] = "10"
        timed_out = run(["sample", "chess"], environment)
        assert timed_out.returncode == 1
        time.sleep(0.8)
        assert not marker.exists()
        with tarfile.open(failed_artifact) as retained:
            assert "run/diagnostics.txt" in retained.getnames()
        environment.pop("DITTO_CI_SAMPLE_TIMEOUT_SECONDS")
        environment.pop("FAKE_CHILD_MARKER")
        environment.pop("FAKE_SLEEP")

        environment.pop("FAKE_STATUS", None)
        environment["DITTO_CI_BRANCH"] = "master"
        published = run(["publish"], environment)
        assert published.returncode == 0, published.stderr
        assert len((root / "published").read_text().splitlines()) == 5

        environment["DITTO_CI_BRANCH"] = "feature"
        skipped = run(["publish"], environment)
        assert skipped.returncode == 0
        assert "publication skipped" in skipped.stdout

    print("Ditto CI tests passed.")


if __name__ == "__main__":
    main()
