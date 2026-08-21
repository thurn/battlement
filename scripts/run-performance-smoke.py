#!/usr/bin/env python3

"""Build and run Task 38's diagnostic Unity development-player scenario."""

from __future__ import annotations

import os
from pathlib import Path
import platform
import subprocess
import tempfile


REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
UNITY_VERSION = "6000.5.8f1"


def unity_editor() -> Path:
    if configured := os.environ.get("UNITY_EDITOR"):
        return Path(configured)
    return Path(
        f"/Applications/Unity/Hub/Editor/{UNITY_VERSION}/Unity.app/Contents/MacOS/Unity"
    )


def main() -> None:
    if platform.system() != "Darwin":
        raise RuntimeError("The performance smoke development player currently requires macOS.")
    editor = unity_editor()
    if not os.access(editor, os.X_OK):
        raise RuntimeError(f"Unity {UNITY_VERSION} was not found at {editor}.")

    with tempfile.TemporaryDirectory(prefix="masonry-performance-smoke.") as temporary:
        root = Path(temporary)
        application = root / "MasonryPerformanceSmoke.app"
        build_log = root / "build.log"
        player_log = root / "player.log"
        report = root / "report.txt"
        environment = os.environ.copy()
        environment["MASONRY_PERFORMANCE_BUILD_PATH"] = str(application)
        build = subprocess.run(
            [
                str(editor),
                "-batchmode",
                "-nographics",
                "--burst-disable-compilation",
                "-quit",
                "-projectPath",
                str(REPOSITORY_ROOT),
                "-executeMethod",
                "Masonry.Editor.PerformanceSmokeBuild.Build",
                "-logFile",
                str(build_log),
            ],
            cwd=REPOSITORY_ROOT,
            env=environment,
        )
        require_success(build, build_log, "Unity development-player build")

        executables = [
            path
            for path in (application / "Contents/MacOS").iterdir()
            if path.is_file() and os.access(path, os.X_OK)
        ]
        if len(executables) != 1:
            raise RuntimeError(
                f"Expected one player executable, found {len(executables)}: {executables}"
            )
        executable = executables[0]
        player = subprocess.run(
            [
                str(executable),
                "-batchmode",
                "-screen-width",
                "640",
                "-screen-height",
                "480",
                "-logFile",
                str(player_log),
                "--masonry-performance-report",
                str(report),
            ],
            cwd=REPOSITORY_ROOT,
            timeout=60,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )
        if player.returncode != 0:
            raise RuntimeError(
                "performance smoke player failed with exit "
                f"{player.returncode}.\n{player.stdout}\n{tail(player_log)}"
            )
        if not report.is_file():
            raise RuntimeError(f"The player did not write a report.\n{tail(player_log)}")
        print(report.read_text(), end="")


def require_success(result: subprocess.CompletedProcess[bytes], log: Path, phase: str) -> None:
    if result.returncode != 0:
        raise RuntimeError(f"{phase} failed with exit {result.returncode}.\n{tail(log)}")


def tail(path: Path, lines: int = 100) -> str:
    return "\n".join(path.read_text(errors="replace").splitlines()[-lines:])


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        raise SystemExit(str(error)) from error
