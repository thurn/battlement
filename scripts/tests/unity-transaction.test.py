#!/usr/bin/env python3

"""Exercise Unity source restoration without launching the editor."""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
RUNNER = REPOSITORY_ROOT / "scripts/unity_transaction.py"


def command(repository: Path, arguments: list[str]) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        arguments, cwd=repository, check=True, capture_output=True
    )


def git(repository: Path, *arguments: str) -> bytes:
    return command(repository, ["git", *arguments]).stdout


def create_repository(root: Path) -> Path:
    project = root / "sample"
    for name in ("Assets", "Packages", "ProjectSettings"):
        (project / name).mkdir(parents=True)
    (root / ".gitignore").write_text("*.ignored\n.logs\n", encoding="utf-8")
    (project / "Assets/tracked.txt").write_text("committed\n", encoding="utf-8")
    (project / "Packages/manifest.json").write_text("{}\n", encoding="utf-8")
    (project / "ProjectSettings/settings.txt").write_text("settings\n", encoding="utf-8")
    git(root, "init", "--quiet")
    git(root, "add", ".")
    git(
        root, "-c", "user.name=CI Fixture", "-c", "user.email=ci@example.invalid",
        "commit", "--quiet", "-m", "fixture",
    )
    (project / "Assets/tracked.txt").write_text("staged\n", encoding="utf-8")
    git(root, "add", "sample/Assets/tracked.txt")
    (project / "Assets/preexisting.txt").write_text("mine\n", encoding="utf-8")
    (project / "Assets/preexisting.ignored").write_text("ignored\n", encoding="utf-8")
    return project


def mutation_script() -> str:
    return """
from pathlib import Path
root = Path.cwd()
(root / 'Assets/tracked.txt').write_text('unity mutation\\n')
(root / 'Packages/manifest.json').unlink()
(root / 'ProjectSettings/generated.txt').write_text('generated\\n')
(root / 'Assets/generated.ignored').write_text('generated ignored\\n')
(root / 'Assets/preexisting.txt').write_text('overwritten\\n')
(root / 'Assets/preexisting.ignored').unlink()
raise SystemExit(7)
"""


def assert_restored(repository: Path, project: Path, before: bytes) -> None:
    assert (project / "Assets/tracked.txt").read_text() == "staged\n"
    assert (project / "Packages/manifest.json").read_text() == "{}\n"
    assert (project / "Assets/preexisting.txt").read_text() == "mine\n"
    assert (project / "Assets/preexisting.ignored").read_text() == "ignored\n"
    assert not (project / "ProjectSettings/generated.txt").exists()
    assert not (project / "Assets/generated.ignored").exists()
    assert git(repository, "status", "--porcelain=v1", "-z", "--untracked-files=all") == before


def journal_root(repository: Path) -> Path:
    return repository / ".logs/ci/unity-transactions"


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="unity-transaction-test.") as temporary:
        repository = Path(temporary)
        project = create_repository(repository)
        before = git(repository, "status", "--porcelain=v1", "-z", "--untracked-files=all")
        result = subprocess.run(
            [
                sys.executable, str(RUNNER), "--project", str(project), "--label", "failure",
                "--", sys.executable, "-c", mutation_script(),
            ],
            cwd=repository,
        )
        assert result.returncode == 7
        assert_restored(repository, project, before)
        journals = sorted(journal_root(repository).glob("*/journal.json"))
        assert len(journals) == 1
        assert json.loads(journals[0].read_text())["state"] == "verified"
        assert b"unity mutation" in (journals[0].parent / "discarded.patch").read_bytes()
        created = json.loads((journals[0].parent / "created-untracked.json").read_text())
        assert "sample/Assets/generated.ignored" in created

        interrupted = """
from pathlib import Path
import os
import time
root = Path.cwd()
(root.parent / '.logs/child-started').write_text('started')
(root.parent / '.logs/child-session').write_text(str(os.getsid(0)))
while True:
    (root / 'Assets/tracked.txt').write_text('interrupted\\n')
    (root / 'Assets/interrupted.txt').write_text('new\\n')
    time.sleep(0.05)
"""
        wrapper = subprocess.Popen(
            [
                sys.executable, str(RUNNER), "--project", str(project),
                "--label", "interrupted", "--", sys.executable, "-c", interrupted,
            ],
            cwd=repository,
        )
        started = repository / ".logs/child-started"
        deadline = time.monotonic() + 5
        while not started.exists() and time.monotonic() < deadline:
            time.sleep(0.02)
        assert started.exists()
        assert int((repository / ".logs/child-session").read_text()) == os.getsid(wrapper.pid)
        wrapper.kill()
        assert wrapper.wait() != 0
        subprocess.run(
            [
                sys.executable, str(RUNNER), "--project", str(project),
                "--label", "recovery", "--", sys.executable, "-c", "pass",
            ],
            cwd=repository,
            check=True,
        )
        assert_restored(repository, project, before)
        time.sleep(0.2)
        assert_restored(repository, project, before)
        journals = sorted(journal_root(repository).glob("*/journal.json"))
        assert len(journals) == 3
        assert all(json.loads(path.read_text())["state"] == "verified" for path in journals)

        retained = journals[0].parent / "discarded.patch"
        retained.write_text("retained diagnostics\n", encoding="utf-8")
        first = json.loads(journals[0].read_text())
        first["state"] = "restored"
        journals[0].write_text(json.dumps(first), encoding="utf-8")
        subprocess.run(
            [
                sys.executable, str(RUNNER), "--project", str(project),
                "--label", "diagnostic-recovery", "--", sys.executable, "-c", "pass",
            ],
            cwd=repository,
            check=True,
        )
        assert retained.read_text(encoding="utf-8") == "retained diagnostics\n"

    print("Unity transaction tests passed.")


if __name__ == "__main__":
    main()
