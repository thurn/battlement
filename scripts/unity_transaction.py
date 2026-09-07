#!/usr/bin/env python3

"""Restore a Unity project's source directories after one editor invocation."""

from __future__ import annotations

import argparse
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import signal
import shutil
import subprocess
import sys
import time
import uuid
from collections.abc import Iterator
from typing import Any


SOURCE_DIRECTORIES = ("Assets", "Packages", "ProjectSettings")


def git(
    repository: Path,
    arguments: list[str],
    *,
    input_bytes: bytes | None = None,
    check: bool = True,
) -> subprocess.CompletedProcess[bytes]:
    """Run a Git command without decoding path-bearing output."""
    return subprocess.run(
        ["git", *arguments],
        cwd=repository,
        input=input_bytes,
        capture_output=True,
        check=check,
    )


def repository_root(project: Path) -> Path:
    return Path(
        git(project, ["rev-parse", "--show-toplevel"]).stdout.decode().strip()
    ).resolve()


def source_pathspecs(repository: Path, project: Path) -> tuple[str, ...]:
    relative = project.resolve().relative_to(repository)
    return tuple((relative / name).as_posix() for name in SOURCE_DIRECTORIES)


def listed_paths(repository: Path, arguments: list[str]) -> set[str]:
    separator = arguments.index("--")
    output = git(
        repository,
        [*arguments[:separator], "-z", *arguments[separator:]],
    ).stdout
    return {value.decode("utf-8", "surrogateescape") for value in output.split(b"\0") if value}


def untracked_paths(repository: Path, pathspecs: tuple[str, ...]) -> set[str]:
    ordinary = listed_paths(
        repository,
        ["ls-files", "--others", "--exclude-standard", "--", *pathspecs],
    )
    ignored = listed_paths(
        repository,
        ["ls-files", "--others", "--ignored", "--exclude-standard", "--", *pathspecs],
    )
    return ordinary | ignored


def path_digest(path: Path) -> str:
    digest = hashlib.sha256()
    if path.is_symlink():
        digest.update(b"symlink\0")
        digest.update(os.readlink(path).encode("utf-8", "surrogateescape"))
    else:
        digest.update(b"file\0")
        with path.open("rb") as source:
            while block := source.read(1024 * 1024):
                digest.update(block)
    return digest.hexdigest()


def copy_file(source: Path, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    if source.is_symlink():
        destination.symlink_to(os.readlink(source))
    else:
        shutil.copy2(source, destination)


def remove_file(path: Path) -> None:
    if path.is_symlink() or path.is_file():
        path.unlink()
    elif path.exists():
        shutil.rmtree(path)


def process_exists(pid: int) -> bool:
    try:
        os.kill(pid, 0)
    except (OSError, ValueError):
        return False
    return True


def process_group_exists(pid: int) -> bool:
    if os.name == "nt":
        return process_exists(pid)
    try:
        os.killpg(pid, 0)
    except (OSError, ValueError):
        return False
    return True


def stop_process_group(
    pid: int, process: subprocess.Popen[Any] | None = None
) -> None:
    if not process_group_exists(pid):
        return
    if os.name == "nt":
        subprocess.run(
            ["taskkill", "/PID", str(pid), "/T", "/F"],
            capture_output=True,
            check=False,
        )
    else:
        os.killpg(pid, signal.SIGTERM)
        if process is not None:
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                os.killpg(pid, signal.SIGKILL)
                process.wait(timeout=5)
                return
        else:
            deadline = time.monotonic() + 10
            while process_group_exists(pid) and time.monotonic() < deadline:
                time.sleep(0.05)
        if process_group_exists(pid):
            os.killpg(pid, signal.SIGKILL)
    if process is not None:
        process.wait(timeout=5)
        return
    deadline = time.monotonic() + 5
    while process_group_exists(pid) and time.monotonic() < deadline:
        time.sleep(0.05)
    if process_group_exists(pid):
        raise RuntimeError(f"Unity process group {pid} did not stop")


def project_directories(repository: Path, pathspecs: tuple[str, ...]) -> set[str]:
    directories = set()
    for pathspec in pathspecs:
        root = repository / pathspec
        if not root.is_dir():
            continue
        directories.add(pathspec)
        directories.update(
            path.relative_to(repository).as_posix()
            for path in root.rglob("*")
            if path.is_dir() and not path.is_symlink()
        )
    return directories


def recover_unity_transactions(repository: Path, project: Path | None = None) -> None:
    """Recover dead journal entries before CI observes their working trees."""
    repository = repository.resolve()
    selected_project = project.resolve() if project is not None else None
    journal_root = repository / ".logs/ci/unity-transactions"
    if not journal_root.is_dir():
        return
    for path in sorted(journal_root.glob("*/journal.json")):
        journal = json.loads(path.read_text(encoding="utf-8"))
        if selected_project is not None and journal.get("project") != str(selected_project):
            continue
        if journal.get("state") in {"verified", "failed"}:
            continue
        if process_exists(int(journal.get("pid", -1))):
            if selected_project is not None:
                raise RuntimeError(f"Unity transaction is already active for {selected_project}")
            continue
        UnityProjectTransaction.stop_journaled_unity(path.parent, journal)
        state = journal.get("state")
        if state not in {"diagnostics_retained", "restored"}:
            UnityProjectTransaction.retain_diff_at(path.parent, journal)
            journal["state"] = "diagnostics_retained"
            UnityProjectTransaction.write_journal_at(path.parent, journal)
        if journal.get("state") == "restored":
            UnityProjectTransaction.verify(path.parent, journal)
        else:
            UnityProjectTransaction.restore_journal(path.parent, journal)


class UnityProjectTransaction:
    """Journal and restore Git-managed Unity source state for one invocation."""

    def __init__(self, project: Path, label: str = "unity") -> None:
        self.project = project.resolve()
        self.repository = repository_root(self.project)
        self.pathspecs = source_pathspecs(self.repository, self.project)
        self.journal_root = self.repository / ".logs/ci/unity-transactions"
        self.directory = self.journal_root / f"{time.time_ns()}-{uuid.uuid4()}"
        self.label = label
        self.journal: dict[str, Any] = {}

    def prepare(self) -> None:
        recover_unity_transactions(self.repository, self.project)
        unstaged = git(
            self.repository,
            ["diff-files", "--quiet", "--", *self.pathspecs],
            check=False,
        )
        if unstaged.returncode != 0:
            raise RuntimeError(
                "Unity transaction requires tracked project files to match the staged Git index"
            )
        self.directory.mkdir(parents=True)
        before_untracked = sorted(untracked_paths(self.repository, self.pathspecs))
        for relative in before_untracked:
            copy_file(self.repository / relative, self.directory / "untracked-backup" / relative)
        before_status = git(
            self.repository,
            ["status", "--porcelain=v1", "-z", "--untracked-files=all", "--", *self.pathspecs],
        ).stdout
        (self.directory / "before-status.bin").write_bytes(before_status)
        self.journal = {
            "schema": 1,
            "state": "prepared",
            "pid": os.getpid(),
            "label": self.label,
            "repository": str(self.repository),
            "project": str(self.project),
            "pathspecs": self.pathspecs,
            "directories": sorted(project_directories(self.repository, self.pathspecs)),
            "index_tree": git(self.repository, ["write-tree"]).stdout.decode().strip(),
            "untracked": before_untracked,
            "untracked_digests": {
                relative: path_digest(self.repository / relative)
                for relative in before_untracked
            },
        }
        self.write_journal()

    def mark_running(self) -> None:
        self.journal["state"] = "running"
        self.write_journal()

    def run(self, command: list[str], **options: Any) -> subprocess.CompletedProcess[Any]:
        """Run one Unity process and journal enough identity to stop an orphan."""
        check = options.pop("check", False)
        input_value = options.pop("input", None)
        timeout = options.pop("timeout", None)
        if options.pop("capture_output", False):
            if options.get("stdout") is not None or options.get("stderr") is not None:
                raise ValueError("stdout and stderr may not be used with capture_output")
            options["stdout"] = subprocess.PIPE
            options["stderr"] = subprocess.PIPE
        if os.name == "nt":
            options["creationflags"] = options.get("creationflags", 0) | subprocess.CREATE_NEW_PROCESS_GROUP
        else:
            options["process_group"] = 0
        process = subprocess.Popen(command, **options)
        self.journal["unity_pid"] = process.pid
        self.journal["unity_stopped"] = False
        self.write_journal()
        try:
            stdout, stderr = process.communicate(input=input_value, timeout=timeout)
        finally:
            if process.poll() is None:
                stop_process_group(process.pid, process)
            self.journal["unity_stopped"] = True
            self.write_journal()
        completed = subprocess.CompletedProcess(command, process.returncode, stdout, stderr)
        if check:
            completed.check_returncode()
        return completed

    def restore(self) -> None:
        self.journal["state"] = "recovering"
        self.write_journal()
        self.retain_diff()
        self.journal["state"] = "diagnostics_retained"
        self.write_journal()
        self.restore_journal(self.directory, self.journal)

    def retain_diff(self) -> None:
        self.retain_diff_at(self.directory, self.journal)

    @staticmethod
    def retain_diff_at(directory: Path, journal: dict[str, Any]) -> None:
        repository = Path(journal["repository"])
        pathspecs = tuple(journal["pathspecs"])
        patch = git(
            repository,
            ["diff", "--binary", "--no-ext-diff", "--", *pathspecs],
        ).stdout
        (directory / "discarded.patch").write_bytes(patch)
        after = git(
            repository,
            ["status", "--porcelain=v1", "-z", "--untracked-files=all", "--", *pathspecs],
        ).stdout
        (directory / "after-unity-status.bin").write_bytes(after)
        created = sorted(untracked_paths(repository, pathspecs) - set(journal["untracked"]))
        (directory / "created-untracked.json").write_text(
            json.dumps(created, indent=2) + "\n", encoding="utf-8"
        )
        changed = []
        for relative, before_digest in journal["untracked_digests"].items():
            path = repository / relative
            if not path.exists() and not path.is_symlink():
                changed.append(relative)
            elif path_digest(path) != before_digest:
                changed.append(relative)
        (directory / "changed-preexisting-untracked.json").write_text(
            json.dumps(changed, indent=2) + "\n", encoding="utf-8"
        )
        if patch or created or changed:
            print(f"Unity source diff retained: {directory}", file=sys.stderr)

    @staticmethod
    def stop_journaled_unity(directory: Path, journal: dict[str, Any]) -> None:
        pid = journal.get("unity_pid")
        if pid is None or journal.get("unity_stopped"):
            return
        stop_process_group(int(pid))
        journal["unity_stopped"] = True
        UnityProjectTransaction.write_journal_at(directory, journal)

    @staticmethod
    def restore_journal(directory: Path, journal: dict[str, Any]) -> None:
        repository = Path(journal["repository"])
        pathspecs = tuple(journal["pathspecs"])
        if git(repository, ["write-tree"]).stdout.decode().strip() != journal["index_tree"]:
            raise RuntimeError("Unity changed the staged Git index")
        tracked = git(repository, ["ls-files", "-z", "--", *pathspecs]).stdout
        if tracked:
            git(repository, ["checkout-index", "--force", "--stdin", "-z"], input_bytes=tracked)
        before_untracked = set(journal["untracked"])
        for relative in sorted(
            untracked_paths(repository, pathspecs) - before_untracked,
            key=lambda value: (value.count("/"), value),
            reverse=True,
        ):
            remove_file(repository / relative)
        before_directories = set(journal.get("directories", []))
        for relative in sorted(
            project_directories(repository, pathspecs) - before_directories,
            key=lambda value: (value.count("/"), value),
            reverse=True,
        ):
            try:
                (repository / relative).rmdir()
            except OSError:
                pass
        for relative in before_untracked:
            destination = repository / relative
            remove_file(destination)
            copy_file(directory / "untracked-backup" / relative, destination)
        journal["state"] = "restored"
        UnityProjectTransaction.write_journal_at(directory, journal)
        UnityProjectTransaction.verify(directory, journal)

    @staticmethod
    def verify(directory: Path, journal: dict[str, Any]) -> None:
        repository = Path(journal["repository"])
        if git(repository, ["write-tree"]).stdout.decode().strip() != journal["index_tree"]:
            raise RuntimeError("Unity changed the staged Git index")
        after_status = git(
            repository,
            ["status", "--porcelain=v1", "-z", "--untracked-files=all", "--", *journal["pathspecs"]],
        ).stdout
        if after_status != (directory / "before-status.bin").read_bytes():
            raise RuntimeError("Unity project did not return to its pre-run Git state")
        for relative, expected in journal["untracked_digests"].items():
            if path_digest(repository / relative) != expected:
                raise RuntimeError(f"Unity transaction did not preserve {relative}")
        journal["state"] = "verified"
        UnityProjectTransaction.write_journal_at(directory, journal)

    def write_journal(self) -> None:
        self.write_journal_at(self.directory, self.journal)

    @staticmethod
    def write_journal_at(directory: Path, journal: dict[str, Any]) -> None:
        temporary = directory / "journal.json.tmp"
        temporary.write_text(json.dumps(journal, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        temporary.replace(directory / "journal.json")


@contextmanager
def unity_project_transaction(
    project: Path, label: str = "unity"
) -> Iterator[UnityProjectTransaction]:
    transaction = UnityProjectTransaction(project, label)
    transaction.prepare()
    try:
        transaction.mark_running()
        yield transaction
    finally:
        transaction.restore()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--project", required=True, type=Path)
    parser.add_argument("--label", default="unity")
    parser.add_argument("command", nargs=argparse.REMAINDER)
    arguments = parser.parse_args()
    command = arguments.command
    if command[:1] == ["--"]:
        command = command[1:]
    if not command:
        parser.error("a Unity command is required after --")
    with unity_project_transaction(arguments.project, arguments.label) as transaction:
        return transaction.run(command, cwd=arguments.project).returncode


if __name__ == "__main__":
    raise SystemExit(main())
