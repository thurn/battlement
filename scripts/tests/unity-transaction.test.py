#!/usr/bin/env python3

"""Exercise Unity source restoration without launching the editor."""

from __future__ import annotations

import json
import os
from pathlib import Path
import signal
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


def verify_parallel_transactions_use_private_indexes() -> None:
    with tempfile.TemporaryDirectory(prefix="unity-transaction-parallel-test.") as temporary:
        repository = Path(temporary)
        (repository / ".gitignore").write_text(".logs\n", encoding="utf-8")
        projects = []
        for name in ("sample-a", "sample-b"):
            project = repository / name
            projects.append(project)
            for directory in ("Assets", "Packages", "ProjectSettings"):
                (project / directory).mkdir(parents=True)
            (project / "Assets/tracked.txt").write_text("committed\n", encoding="utf-8")
            (project / "Packages/manifest.json").write_text("{}\n", encoding="utf-8")
            (project / "ProjectSettings/settings.txt").write_text(
                "settings\n", encoding="utf-8"
            )
        git(repository, "init", "--quiet")
        git(repository, "add", ".")
        git(
            repository,
            "-c",
            "user.name=CI Fixture",
            "-c",
            "user.email=ci@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "fixture",
        )
        before = git(repository, "status", "--porcelain=v1", "-z", "--untracked-files=all")
        probe = """
from pathlib import Path
import os
import time
root = Path.cwd().parent
name = Path.cwd().name
logs = root / '.logs'
logs.mkdir(exist_ok=True)
(logs / f'{name}.index').write_text(os.environ['GIT_INDEX_FILE'])
(logs / f'{name}.ready').write_text('ready')
deadline = time.monotonic() + 5
peer = 'sample-b' if name == 'sample-a' else 'sample-a'
while not (logs / f'{peer}.ready').exists() and time.monotonic() < deadline:
    time.sleep(0.01)
if not (logs / f'{peer}.ready').exists():
    raise SystemExit(9)
"""
        processes = [
            subprocess.Popen(
                [
                    sys.executable,
                    str(RUNNER),
                    "--project",
                    str(project),
                    "--label",
                    "parallel",
                    "--",
                    sys.executable,
                    "-c",
                    probe,
                ],
                cwd=repository,
            )
            for project in projects
        ]
        assert [process.wait(timeout=10) for process in processes] == [0, 0]
        indexes = [
            Path((repository / f".logs/{project.name}.index").read_text())
            for project in projects
        ]
        assert indexes[0] != indexes[1]
        assert all(
            index.parent.parent.resolve() == journal_root(repository).resolve()
            for index in indexes
        )
        assert git(
            repository, "status", "--porcelain=v1", "-z", "--untracked-files=all"
        ) == before


def verify_terminal_input(repository: Path, project: Path, before: bytes) -> None:
    """Batch descendants must reach EOF while their caller's terminal stays open."""
    if os.name == "nt":
        return
    import pty
    import select

    started = time.monotonic()
    # Exercise a descendant's read after modifying source, so both completion and
    # transaction restoration are observable without starting Unity or FFmpeg.
    script = mutation_script().replace("raise SystemExit(7)", "") + """
import subprocess
import sys
subprocess.run(
    [sys.executable, '-c', 'import os; assert os.read(0, 1) == b""'],
    check=True,
)
raise SystemExit(7)
"""
    # An open PTY reproduces inherited terminal input without creating a new
    # session, so the probe remains inside CI's process containment boundary.
    terminal, slave = pty.openpty()
    try:
        wrapper = subprocess.Popen([
            sys.executable, str(RUNNER), "--project", str(project),
            "--label", "terminal-input", "--", sys.executable, "-c", script,
        ], stdin=slave, stdout=slave, stderr=slave, process_group=0)
    except BaseException:
        os.close(terminal)
        raise
    finally:
        os.close(slave)
    status = None
    output = bytearray()
    try:
        deadline = started + 5
        while time.monotonic() < deadline:
            if select.select([terminal], [], [], 0.01)[0]:
                try:
                    output.extend(os.read(terminal, 8192))
                except OSError:
                    pass  # A closed PTY reports EIO on Linux.
            status = wrapper.poll()
            if status is not None:
                break
        assert status is not None, "terminal-launched Unity transaction froze (5s deadline)"
        assert status == 7, output.decode(errors="replace")
        assert_restored(repository, project, before)
        journals = [
            json.loads(path.read_text())
            for path in journal_root(repository).glob("*/journal.json")
        ]
        terminal_journal, = [item for item in journals if item["label"] == "terminal-input"]
        assert terminal_journal["state"] == "verified"
    finally:
        # A suspended group cannot handle SIGTERM. Kill only this probe's
        # journaled group, then its wrapper, without the production cleanup wait.
        for path in journal_root(repository).glob("*/journal.json"):
            journal = json.loads(path.read_text())
            if journal["label"] != "terminal-input" or journal.get("unity_stopped"):
                continue
            unity_pid = journal.get("unity_pid")
            if unity_pid is not None:
                try:
                    os.killpg(unity_pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
        if status is None:
            wrapper.kill()
            wrapper.wait(timeout=1)
        os.close(terminal)
    elapsed = time.monotonic() - started
    assert elapsed < 10, f"terminal regression exceeded its CI budget: {elapsed:.3f}s"
    print(f"Unity terminal regression passed ({elapsed:.3f}s; 5s deadline).")


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
session = root.parent / '.logs/child-session'
temporary_session = session.with_suffix('.tmp')
temporary_session.write_text(str(os.getsid(0)))
temporary_session.replace(session)
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
        session = repository / ".logs/child-session"
        deadline = time.monotonic() + 5
        while not session.exists() and time.monotonic() < deadline:
            time.sleep(0.02)
        assert session.exists()
        assert int(session.read_text()) == os.getsid(wrapper.pid)
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
        verify_terminal_input(repository, project, before)

    verify_parallel_transactions_use_private_indexes()
    print("Unity transaction tests passed.")


if __name__ == "__main__":
    main()
