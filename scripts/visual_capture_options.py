#!/usr/bin/env python3

"""Command-line options and validation for packaged-player visual capture."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import time

from visual_capture_lib import is_nonnegative_number


SAFE_NAME = re.compile(r"[A-Za-z0-9._-]+")


def parse_dimensions(value: str) -> tuple[int, int]:
    """Parse and validate a WIDTHxHEIGHT player size."""
    try:
        width, height = (int(part) for part in value.split("x", 1))
    except ValueError as error:
        raise argparse.ArgumentTypeError("Dimensions must be WIDTHxHEIGHT.") from error
    if width < 320 or height < 240:
        raise argparse.ArgumentTypeError("Capture dimensions must be at least 320x240.")
    return width, height


def parse_arguments() -> argparse.Namespace:
    """Parse visual-capture command-line arguments."""
    parser = argparse.ArgumentParser(
        description="Build a packaged Unity scenario and capture its visual evidence."
    )
    parser.add_argument("--task", required=True)
    parser.add_argument("--scenario", required=True)
    parser.add_argument("--scene", required=True)
    plugin = parser.add_mutually_exclusive_group()
    plugin.add_argument("--plugin", type=Path)
    plugin.add_argument("--cargo-package")
    parser.add_argument("--transport", choices=("native", "http", "none"), default="none")
    parser.add_argument("--artifact-root", type=Path, default=Path("artifacts/visual-evidence"))
    parser.add_argument("--build-cache", type=Path)
    parser.add_argument("--capture", choices=("png", "video", "both"), default="png")
    parser.add_argument(
        "--input-driver", choices=("in-player", "macos-hid"), default="in-player"
    )
    parser.add_argument(
        "--media-driver", choices=("in-player", "screen-capture-kit"), default="in-player"
    )
    parser.add_argument("--ffmpeg", type=Path)
    parser.add_argument("--smoke", action="store_true")
    parser.add_argument("--dimensions", type=parse_dimensions, default=(1280, 720))
    parser.add_argument("--video-seconds", type=int, default=5)
    parser.add_argument("--initial-hold-seconds", default="2")
    parser.add_argument("--interaction-timeout", type=int, default=15)
    parser.add_argument(
        "--run-id",
        default=time.strftime("%Y%m%dT%H%M%SZ", time.gmtime()) + f"-{os.getpid()}",
    )
    parser.add_argument("--show-overlay", action="store_true")
    return parser.parse_args()


def validate_arguments(args: argparse.Namespace, repository_root: Path) -> None:
    """Fail with a usage error when capture options cannot run safely."""
    if SAFE_NAME.fullmatch(args.task) is None:
        _fail("A safe --task ID is required.")
    if SAFE_NAME.fullmatch(args.scenario) is None:
        _fail("A safe --scenario name is required.")
    if args.cargo_package and SAFE_NAME.fullmatch(args.cargo_package) is None:
        _fail("Invalid --cargo-package name.")
    if SAFE_NAME.fullmatch(args.run_id) is None:
        _fail("A safe --run-id is required.")
    if not args.scene.startswith("Assets/") or not args.scene.endswith(".unity"):
        _fail("--scene must name an Assets/*.unity file.")
    if ".." in Path(args.scene).parts:
        _fail("--scene may not traverse parent directories.")
    if not (repository_root / args.scene).is_file():
        _fail(f"Capture scene was not found: {args.scene}")
    if (args.plugin or args.cargo_package) and args.transport != "native":
        _fail("A native plugin requires --transport native.")
    if args.transport == "native" and not (args.plugin or args.cargo_package):
        _fail("--transport native requires --plugin or --cargo-package.")
    if not 1 <= args.video_seconds <= 60:
        _fail("Video duration must be between 1 and 60 seconds.")
    if not is_nonnegative_number(args.initial_hold_seconds):
        _fail("Initial hold must be a nonnegative number.")
    if not 1 <= args.interaction_timeout <= 120:
        _fail("Interaction timeout must be between 1 and 120 seconds.")
    if platform.system() != "Darwin":
        _fail("Release-player capture is supported on macOS only.", 1)

    version = next(
        line.removeprefix("m_EditorVersion: ")
        for line in (repository_root / "ProjectSettings/ProjectVersion.txt").read_text().splitlines()
        if line.startswith("m_EditorVersion: ")
    )
    editor = Path(
        os.environ.get(
            "UNITY_EDITOR",
            f"/Applications/Unity/Hub/Editor/{version}/Unity.app/Contents/MacOS/Unity",
        )
    )
    if not os.access(editor, os.X_OK):
        _fail(f"Unity {version} was not found at {editor}.", 1)
    if args.plugin and not _resolved(args.plugin, repository_root).is_file():
        _fail(f"Native plugin was not found: {args.plugin}")
    if not args.smoke and args.media_driver == "in-player" and args.capture in {"video", "both"}:
        ffmpeg = (
            _resolved(args.ffmpeg, repository_root)
            if args.ffmpeg
            else Path(shutil.which("ffmpeg") or "")
        )
        ffprobe = (
            ffmpeg.with_name("ffprobe")
            if ffmpeg.name and ffmpeg.with_name("ffprobe").is_file()
            else Path(shutil.which("ffprobe") or "")
        )
        if not ffmpeg.is_file() or not os.access(ffmpeg, os.X_OK):
            _fail("In-player video capture requires FFmpeg on PATH or via --ffmpeg.")
        if not ffprobe.is_file() or not os.access(ffprobe, os.X_OK):
            _fail(f"FFprobe was not found alongside FFmpeg or on PATH: {ffprobe}")
        encoders = subprocess.run(
            [str(ffmpeg), "-hide_banner", "-encoders"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout
        if "h264_videotoolbox" not in encoders:
            _fail("FFmpeg does not provide the required h264_videotoolbox encoder.")


def _resolved(path: Path, repository_root: Path) -> Path:
    return path if path.is_absolute() else repository_root / path


def _fail(message: str, status: int = 2) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(status)
