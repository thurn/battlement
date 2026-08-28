#!/usr/bin/env python3

"""Cross-platform process, path, and file-lock helpers for local tooling."""

from __future__ import annotations

import os
from pathlib import Path
from queue import Empty, Queue
import shutil
import sys
import threading
import time
from typing import TextIO


def user_cache_path(*parts: str) -> Path:
    """Return a conventional per-user cache path for the current host."""
    if os.name == "nt":
        root = Path(os.environ.get("LOCALAPPDATA", Path.home() / "AppData/Local"))
    elif sys.platform == "darwin":
        root = Path.home() / "Library/Caches"
    else:
        root = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache"))
    return root.joinpath(*parts)


def executable_name(name: str) -> str:
    """Return a host-native executable filename."""
    return f"{name}.exe" if os.name == "nt" else name


def configure_windows_tool_path() -> None:
    """Expose conventional user tools and GitHub Desktop's bundled Git."""
    if os.name != "nt":
        return
    candidates = [Path.home() / ".cargo/bin"]
    desktop = Path(os.environ.get("LOCALAPPDATA", Path.home() / "AppData/Local"))
    installations = desktop / "GitHubDesktop"
    if installations.is_dir():
        candidates.extend(
            path / "resources/app/git/cmd"
            for path in sorted(installations.glob("app-*"), reverse=True)
        )
    current = os.environ.get("PATH", "").split(os.pathsep)
    known = {os.path.normcase(path) for path in current if path}
    additions = [
        str(path)
        for path in candidates
        if path.is_dir() and os.path.normcase(str(path)) not in known
    ]
    if additions:
        os.environ["PATH"] = os.pathsep.join((*additions, *current))


def resolve_executable(name: str) -> str:
    """Resolve an executable after applying supported local-tool conventions."""
    configure_windows_tool_path()
    return shutil.which(name) or name


def readline_with_timeout(stream: TextIO, timeout: float) -> str | None:
    """Read one line from a process pipe, returning None after a timeout."""
    results: Queue[str] = Queue(maxsize=1)
    threading.Thread(target=lambda: results.put(stream.readline()), daemon=True).start()
    try:
        return results.get(timeout=timeout)
    except Empty:
        return None


def try_lock_file(file: TextIO) -> bool:
    """Try to acquire an exclusive lock on an open file."""
    _prepare_lock_file(file)
    if os.name == "nt":
        import msvcrt

        try:
            msvcrt.locking(file.fileno(), msvcrt.LK_NBLCK, 1)
        except OSError:
            return False
        return True
    import fcntl

    try:
        fcntl.flock(file, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        return False
    return True


def lock_file(file: TextIO) -> None:
    """Wait until an exclusive lock can be acquired on an open file."""
    if os.name != "nt":
        import fcntl

        fcntl.flock(file, fcntl.LOCK_EX)
        return
    while not try_lock_file(file):
        time.sleep(0.05)


def unlock_file(file: TextIO) -> None:
    """Release an exclusive lock held on an open file."""
    if os.name == "nt":
        import msvcrt

        file.seek(0)
        msvcrt.locking(file.fileno(), msvcrt.LK_UNLCK, 1)
        return
    import fcntl

    fcntl.flock(file, fcntl.LOCK_UN)


def _prepare_lock_file(file: TextIO) -> None:
    if os.name != "nt":
        return
    file.seek(0, os.SEEK_END)
    if file.tell() == 0:
        file.write("\0")
        file.flush()
    file.seek(0)


configure_windows_tool_path()
