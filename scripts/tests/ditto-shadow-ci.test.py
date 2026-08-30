#!/usr/bin/env python3

"""Black-box checks for the Ditto shadow-CI runner."""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
RUNNER = REPOSITORY_ROOT / "scripts/ditto_shadow_ci.py"


FAKE_DITTO = r'''#!/usr/bin/env python3
import json
import os
from pathlib import Path
import sys

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
status = os.environ.get("FAKE_STATUS", "passed")
disposition = "reused" if "--no-build" in arguments else "created"
result = {
    "run_id": "0197b35f-6e24-75d8-9482-aa6c22a15133",
    "status": status,
    "build": {"disposition": disposition},
    "scenarios": [{"name": "fixture", "status": status}],
}
output.parent.mkdir(parents=True, exist_ok=True)
output.write_text(json.dumps(result))
print(f"DITTO_RUN_DIR={run}", file=sys.stderr)
raise SystemExit(0 if status == "passed" else 1)
'''


def run(arguments: list[str], environment: dict[str, str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(RUNNER), *arguments],
        cwd=REPOSITORY_ROOT,
        env=environment,
        capture_output=True,
        text=True,
    )


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="ditto-shadow-ci-test.") as temporary:
        root = Path(temporary)
        fake = root / "ditto"
        fake.write_text(FAKE_DITTO)
        fake.chmod(0o755)
        environment = os.environ.copy()
        environment.update({
            "DITTO_SHADOW_BINARY": str(fake),
            "FAKE_RUN_ROOT": str(root / "runs"),
            "FAKE_PUBLISH_LOG": str(root / "published"),
        })

        passed = run(["sample", "basic"], environment)
        assert passed.returncode == 0, passed.stderr
        artifact = REPOSITORY_ROOT / "artifacts/ditto-ci/basic/run"
        assert (artifact / "logs/events.jsonl").is_file()
        assert (artifact / "diagnostics.txt").read_text() == "private failure diagnostics\n"
        assert not (root / "published").exists()

        environment["FAKE_STATUS"] = "failed"
        failed = run(["sample", "chess"], environment)
        assert failed.returncode == 1
        failed_artifact = REPOSITORY_ROOT / "artifacts/ditto-ci/chess/run"
        assert (failed_artifact / "diagnostics.txt").is_file()
        assert not (root / "published").exists()

        environment.pop("FAKE_STATUS")
        branch = subprocess.run(
            ["git", "branch", "--show-current"], cwd=REPOSITORY_ROOT,
            check=True, capture_output=True, text=True,
        ).stdout.strip()
        environment["DITTO_DEFAULT_BRANCH"] = branch
        published = run(["publish"], environment)
        assert published.returncode == 0, published.stderr
        assert len((root / "published").read_text().splitlines()) == 5

        environment["DITTO_DEFAULT_BRANCH"] = "a-different-branch"
        skipped = run(["publish"], environment)
        assert skipped.returncode == 0
        assert "publication skipped" in skipped.stdout

    print("Ditto shadow CI tests passed.")


if __name__ == "__main__":
    main()
