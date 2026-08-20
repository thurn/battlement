#!/usr/bin/env python3

"""Build a release Unity player and capture deterministic visual evidence."""

from __future__ import annotations

import argparse
import difflib
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time

from visual_capture_lib import (
    is_nonnegative_number,
    now,
    project_fingerprint,
    sha256_file,
    tracked_state,
    verify_png_dimensions,
    wait_for_initial_hold,
)


REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
SAFE_NAME = re.compile(r"[A-Za-z0-9._-]+")


def parse_dimensions(value: str) -> tuple[int, int]:
    try:
        width, height = (int(part) for part in value.split("x", 1))
    except ValueError as error:
        raise argparse.ArgumentTypeError("Dimensions must be WIDTHxHEIGHT.") from error
    if width < 320 or height < 240:
        raise argparse.ArgumentTypeError("Capture dimensions must be at least 320x240.")
    return width, height


def parse_arguments() -> argparse.Namespace:
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
    parser.add_argument("--smoke", action="store_true")
    parser.add_argument("--dimensions", type=parse_dimensions, default=(1280, 720))
    parser.add_argument("--video-seconds", type=int, default=5)
    parser.add_argument("--initial-hold-seconds", default="2")
    parser.add_argument("--interaction-timeout", type=int, default=15)
    parser.add_argument(
        "--run-id", default=time.strftime("%Y%m%dT%H%M%SZ", time.gmtime()) + f"-{os.getpid()}"
    )
    parser.add_argument("--show-overlay", action="store_true")
    return parser.parse_args()


def resolved(path: Path) -> Path:
    return path if path.is_absolute() else REPOSITORY_ROOT / path


def fail(message: str, status: int = 1) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(status)


def run_output(command: list[str], **kwargs) -> str:
    return subprocess.run(command, check=True, capture_output=True, text=True, **kwargs).stdout.strip()


def process_alive(process_id: int | None) -> bool:
    if process_id is None:
        return False
    try:
        os.kill(process_id, 0)
        return True
    except ProcessLookupError:
        return False


def terminate_process(process: subprocess.Popen | None) -> None:
    if process is None or process.poll() is not None:
        return
    process.terminate()
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait()


def terminate_pid(process_id: int | None) -> None:
    if not process_alive(process_id):
        return
    os.kill(process_id, signal.SIGTERM)
    deadline = time.monotonic() + 5
    while process_alive(process_id) and time.monotonic() < deadline:
        time.sleep(0.05)
    if process_alive(process_id):
        os.kill(process_id, signal.SIGKILL)


def read_json(path: Path) -> dict:
    try:
        return json.loads(path.read_text())
    except (FileNotFoundError, json.JSONDecodeError):
        return {}


class CaptureRun:
    """Own the resources and protocol state for one visual capture."""

    def __init__(self, args: argparse.Namespace) -> None:
        self.args = args
        self.width, self.height = args.dimensions
        self.scene_path = REPOSITORY_ROOT / args.scene
        self.artifact_root = resolved(args.artifact_root)
        self.build_cache_root = resolved(args.build_cache) if args.build_cache else self.artifact_root / ".build-cache"
        self.plugin = resolved(args.plugin) if args.plugin else None
        self.unity_version = next(
            line.removeprefix("m_EditorVersion: ")
            for line in (REPOSITORY_ROOT / "ProjectSettings/ProjectVersion.txt").read_text().splitlines()
            if line.startswith("m_EditorVersion: ")
        )
        self.unity_editor = Path(
            os.environ.get(
                "UNITY_EDITOR",
                f"/Applications/Unity/Hub/Editor/{self.unity_version}/Unity.app/Contents/MacOS/Unity",
            )
        )
        self.revision = run_output(["git", "rev-parse", "HEAD"], cwd=REPOSITORY_ROOT)
        plugin_identity = sha256_file(self.plugin) if self.plugin else (
            f"cargo:{args.cargo_package}" if args.cargo_package else ""
        )
        self.content_fingerprint = project_fingerprint(
            REPOSITORY_ROOT, args.scene, args.scenario, args.transport, plugin_identity
        )
        self.identity = f"{self.revision}-{self.content_fingerprint[:12]}"
        self.output_directory = self.artifact_root / self.identity / args.task / args.run_id
        if self.output_directory.exists():
            fail(f"Refusing to overwrite capture run: {self.output_directory}")
        self.output_directory.mkdir(parents=True)
        self.run_log = self.output_directory / f"{args.run_id}.log"
        self.temporary_root = Path(tempfile.mkdtemp(prefix="masonry-capture."))
        self.initial_tracked_state = tracked_state(REPOSITORY_ROOT)
        self.isolated_project = self.temporary_root / "project"
        self.status_path = self.temporary_root / "player-status.json"
        self.unity_log = self.temporary_root / "unity-build.log"
        self.player_log = self.temporary_root / "player.log"
        self.helper = self.temporary_root / "macos-capture"
        build_digest = hashlib.sha256(
            f"{self.content_fingerprint}\0{self.unity_version}\0".encode()
        ).hexdigest()
        self.cache_directory = self.build_cache_root / build_digest
        self.build_path = self.cache_directory / "Masonry Capture.app"
        self.cache_manifest = self.cache_directory / "manifest.json"
        self.player_pid: int | None = None
        self.caffeinate: subprocess.Popen | None = None
        self.recorder: subprocess.Popen | None = None
        self.pointer_button_down = False
        self.last_pointer_x = 0
        self.last_pointer_y = 0

    def log(self, message: str) -> None:
        print(message)
        with self.run_log.open("a") as destination:
            print(message, file=destination)

    def append_file_to_log(self, path: Path, count: int | None = None) -> None:
        lines = path.read_text(errors="replace").splitlines()
        with self.run_log.open("a") as destination:
            print("\n".join(lines[-count:] if count else lines), file=destination)

    def cleanup(self) -> bool:
        terminate_process(self.recorder)
        if self.pointer_button_down and self.helper.exists() and self.player_pid is not None:
            subprocess.run(
                [
                    str(self.helper), "pointer-left-button-up", str(self.player_pid),
                    str(self.last_pointer_x), str(self.last_pointer_y),
                ],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
        terminate_pid(self.player_pid)
        terminate_process(self.caffeinate)
        final_state = tracked_state(REPOSITORY_ROOT)
        clean = self.initial_tracked_state == final_state
        if clean:
            self.log("repository cleanliness check passed")
        else:
            self.log("Unexpected tracked repository changes occurred during capture:")
            difference = "".join(
                difflib.unified_diff(
                    self.initial_tracked_state.splitlines(keepends=True),
                    final_state.splitlines(keepends=True),
                    fromfile="tracked-state.before",
                    tofile="tracked-state.after",
                )
            )
            print(difference, file=sys.stderr)
            with self.run_log.open("a") as destination:
                destination.write(difference)
        shutil.rmtree(self.temporary_root)
        return clean

    def compile_helper(self) -> None:
        subprocess.run(
            [
                "swiftc", "scripts/macos-capture.swift", "-o", str(self.helper),
                "-framework", "AppKit", "-framework", "ApplicationServices",
                "-framework", "AVFoundation", "-framework", "ScreenCaptureKit",
            ],
            cwd=REPOSITORY_ROOT,
            check=True,
        )
        with self.run_log.open("a") as destination:
            subprocess.run(
                [str(self.helper), "preflight-input" if self.args.smoke else "preflight"],
                stdout=destination,
                stderr=subprocess.STDOUT,
                check=True,
            )
        self.caffeinate = subprocess.Popen(["caffeinate", "-dims", "-w", str(os.getpid())])
        subprocess.run(["caffeinate", "-u", "-t", "2"], check=True)

    def cache_is_valid(self) -> bool:
        if not (self.build_path / "Contents/Info.plist").is_file() or not self.cache_manifest.is_file():
            return False
        try:
            manifest = json.loads(self.cache_manifest.read_text())
        except json.JSONDecodeError:
            return False
        return manifest == {
            "fingerprint": self.content_fingerprint,
            "revision": manifest.get("revision"),
            "scene": self.args.scene,
            "scenario": self.args.scenario,
            "unity": self.unity_version,
        }

    def build_player(self) -> None:
        if self.cache_is_valid():
            self.log(f"reusing verified packaged player {self.build_path}")
            return
        if self.cache_directory.exists():
            self.log(f"discarding an incomplete or invalid cache entry {self.cache_directory}")
            shutil.rmtree(self.cache_directory)
        self.log(f"building isolated non-Development macOS player with Unity {self.unity_version}")
        self.isolated_project.mkdir()
        subprocess.run(
            [
                "rsync", "-a", "--exclude", ".git", "--exclude", ".worktrees",
                "--exclude", "Library", "--exclude", "Temp", "--exclude", "Logs",
                "--exclude", "obj", "--exclude", "target", "--exclude", "artifacts",
                f"{REPOSITORY_ROOT}/", f"{self.isolated_project}/",
            ],
            check=True,
        )
        plugin = self.plugin
        if plugin or self.args.cargo_package:
            isolated_plugin_directory = self.isolated_project / "Assets/Plugins/macOS"
            isolated_plugin_directory.mkdir(parents=True)
            if self.args.cargo_package:
                subprocess.run(
                    [
                        "cargo", "build", "--quiet", "--release", "-p", self.args.cargo_package,
                        "--manifest-path", str(self.isolated_project / "Cargo.toml"),
                        "--target-dir", str(self.temporary_root / "rust-target"),
                    ],
                    check=True,
                )
                plugin = self.temporary_root / "rust-target/release/libmasonry_rules.dylib"
            isolated_plugin = isolated_plugin_directory / "libmasonry_rules.dylib"
            shutil.copy2(plugin, isolated_plugin)
            architectures = run_output(["lipo", "-archs", str(isolated_plugin)]).split()
            if platform.machine() not in architectures:
                fail(f"The native plugin lacks host architecture {platform.machine()}.")
        uncached_build = self.temporary_root / "Masonry Capture.app"
        environment = os.environ.copy()
        environment.update(
            MASONRY_CAPTURE_BUILD_PATH=str(uncached_build),
            MASONRY_CAPTURE_SCENE_PATH=self.args.scene,
            MASONRY_CAPTURE_SCENARIO=self.args.scenario,
        )
        result = subprocess.run(
            [
                str(self.unity_editor), "-batchmode", "-nographics", "--burst-disable-compilation",
                "-quit", "-projectPath", str(self.isolated_project), "-executeMethod",
                "Masonry.Editor.VisualCaptureBuild.Build", "-logFile", str(self.unity_log),
            ],
            env=environment,
        )
        if result.returncode != 0:
            self.append_file_to_log(self.unity_log, 120)
            fail("Unity release player build failed.")
        if f"MASONRY_CAPTURE_BUILD_OK:{uncached_build}" not in self.unity_log.read_text(errors="replace"):
            self.append_file_to_log(self.unity_log, 120)
            fail("Unity omitted the build success marker.")
        cache_staging = self.temporary_root / "cache"
        cache_staging.mkdir()
        subprocess.run(
            ["ditto", str(uncached_build), str(cache_staging / "Masonry Capture.app")], check=True
        )
        (cache_staging / "manifest.json").write_text(
            json.dumps(
                {
                    "fingerprint": self.content_fingerprint,
                    "revision": self.revision,
                    "scene": self.args.scene,
                    "scenario": self.args.scenario,
                    "unity": self.unity_version,
                },
                indent=2,
            )
            + "\n"
        )
        self.build_cache_root.mkdir(parents=True, exist_ok=True)
        if not self.cache_directory.exists():
            cache_staging.rename(self.cache_directory)
        self.log(f"cached packaged player {self.build_path}")

    def launch_player(self) -> tuple[Path, str]:
        executable_name = run_output(
            ["plutil", "-extract", "CFBundleExecutable", "raw", str(self.build_path / "Contents/Info.plist")]
        )
        player_executable = self.build_path / f"Contents/MacOS/{executable_name}"
        packaged_plugin = self.build_path / "Contents/PlugIns/libmasonry_rules.dylib"
        if not os.access(player_executable, os.X_OK):
            fail("The player executable is missing.")
        if self.args.transport == "native":
            if not packaged_plugin.is_file():
                fail("The bundled native plugin is missing.")
            if platform.machine() not in run_output(["lipo", "-archs", str(packaged_plugin)]).split():
                fail(f"The packaged dylib lacks host architecture {platform.machine()}.")
        self.log("launching packaged player without Editor library search paths")
        environment = os.environ.copy()
        environment.pop("DYLD_LIBRARY_PATH", None)
        environment.pop("DYLD_FRAMEWORK_PATH", None)
        command = [
            "open", "-n", str(self.build_path), "--args", "-popupwindow", "-screen-fullscreen", "0",
            "-screen-width", str(self.width), "-screen-height", str(self.height),
            "-masonryCaptureScenario", self.args.scenario, "-masonryCaptureStatus",
            str(self.status_path), "-logFile", str(self.player_log),
        ]
        if self.args.show_overlay:
            command.append("-masonryCaptureOverlay")
        subprocess.run(command, env=environment, check=True)
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            matches = subprocess.run(
                ["pgrep", "-f", str(player_executable)], capture_output=True, text=True
            ).stdout.splitlines()
            if matches:
                self.player_pid = int(matches[0])
                break
            time.sleep(0.1)
        if self.player_pid is None:
            fail("The launched player process could not be found.")
        return player_executable, executable_name

    def wait_until_ready(self) -> None:
        deadline = time.monotonic() + 45
        phase = ""
        while time.monotonic() < deadline:
            if not process_alive(self.player_pid):
                self.append_file_to_log(self.player_log, 120)
                fail("Player exited before the ready signal.")
            status = read_json(self.status_path)
            phase = status.get("phase", "")
            if phase == "ready":
                break
            if phase == "failed":
                self.log(status.get("failure", "unknown player failure"))
                fail("Player failed before the ready signal.")
            time.sleep(0.1)
        if phase != "ready":
            self.append_file_to_log(self.player_log, 120)
            fail("Ready signal timed out.")

    def find_window(self) -> tuple[str, float, float, float, float]:
        deadline = time.monotonic() + 10
        window_data = ""
        while time.monotonic() < deadline:
            result = subprocess.run(
                [str(self.helper), "window", str(self.player_pid)], capture_output=True, text=True
            )
            window_data = result.stdout.strip()
            if window_data:
                break
            time.sleep(0.1)
        if not window_data:
            fail("Could not locate the player content window.")
        window_id, window_x, window_y, window_width, window_height = window_data.split()
        subprocess.run([str(self.helper), "focus", str(self.player_pid)], check=True)
        return window_id, float(window_x), float(window_y), float(window_width), float(window_height)

    def start_recording(self, window_id: str, video_path: Path) -> float | None:
        if self.args.smoke or self.args.capture not in {"video", "both"}:
            return None
        self.log(
            f"recording H.264 interaction video with {self.args.initial_hold_seconds}s initial hold"
        )
        recording_ready = self.temporary_root / "recording-ready"
        self.recorder = subprocess.Popen(
            [
                str(self.helper), "record-window", window_id, str(video_path),
                str(self.args.video_seconds), str(self.width), str(self.height), str(recording_ready),
            ]
        )
        deadline = time.monotonic() + 10
        while not recording_ready.exists() and time.monotonic() < deadline:
            if self.recorder.poll() is not None:
                self.recorder = None
                fail("Video capture failed before recording began.")
            time.sleep(0.05)
        if not recording_ready.exists():
            fail("Timed out waiting for recording.")
        return now()

    def capture_png(self, label: str, window_id: str, path: Path) -> None:
        subprocess.run(["/usr/sbin/screencapture", "-x", "-o", "-l", window_id, str(path)], check=True)
        if not verify_png_dimensions(path, self.width, self.height):
            fail(f"{label.capitalize()} PNG dimensions did not match {self.width}x{self.height}.")
        self.log(f"{label} PNG {path}")

    def drive_scenario(
        self,
        window_x: float,
        window_y: float,
        window_width: float,
        window_height: float,
        hold_started_at: float | None,
    ) -> dict:
        deadline = time.monotonic() + self.args.interaction_timeout
        phase = ""
        last_request = 0
        first_dispatch_at: float | None = None
        while time.monotonic() < deadline:
            if not process_alive(self.player_pid):
                self.append_file_to_log(self.player_log, 120)
                fail("Player crashed before assertions.")
            if self.recorder is not None and self.recorder.poll() is not None:
                self.recorder = None
                fail("Video ended before the scenario completed.")
            status = read_json(self.status_path)
            phase = status.get("phase", "")
            if phase in {"passed", "failed"}:
                break
            if phase == "ready" and status.get("requestId", -1) != last_request:
                request_id = status.get("requestId")
                capture_input = status.get("input", "")
                pointer_x = status.get("pointerX", -1)
                pointer_y = status.get("pointerY", -1)
                if not isinstance(request_id, int):
                    fail("Input request ID was invalid.")
                if request_id != last_request + 1:
                    fail("Input request IDs must be consecutive.")
                if capture_input not in {
                    "pointer-move", "pointer-left-button-down", "pointer-left-button-up"
                }:
                    fail(f"Unsupported capture input: {capture_input}")
                if not all(isinstance(value, (int, float)) and 0 <= value <= 1 for value in (pointer_x, pointer_y)):
                    fail("Input request lacks normalized coordinates.")
                self.last_pointer_x = int(window_x + window_width * pointer_x)
                self.last_pointer_y = int(window_y + window_height * pointer_y)
                if capture_input == "pointer-left-button-down" and self.pointer_button_down:
                    fail("Primary pointer button was already pressed.")
                if capture_input == "pointer-left-button-up" and not self.pointer_button_down:
                    fail("Primary pointer button was not pressed.")
                dispatched_input = (
                    "pointer-left-drag"
                    if capture_input == "pointer-move" and self.pointer_button_down
                    else capture_input
                )
                if first_dispatch_at is None:
                    first_dispatch_at = now()
                    if hold_started_at is not None:
                        self.log(
                            f"first input dispatched {first_dispatch_at - hold_started_at:.3f}s "
                            "after recording started"
                        )
                subprocess.run(
                    [
                        str(self.helper), dispatched_input, str(self.player_pid),
                        str(self.last_pointer_x), str(self.last_pointer_y),
                    ],
                    check=True,
                )
                if capture_input == "pointer-left-button-down":
                    self.pointer_button_down = True
                elif capture_input == "pointer-left-button-up":
                    self.pointer_button_down = False
                last_request = request_id
            time.sleep(0.1)
        if phase != "passed":
            self.log(status.get("failure", "capture assertions timed out"))
            fail("Player assertions did not pass.")
        if self.pointer_button_down:
            fail("Scenario passed with the primary pointer button pressed.")
        return status

    def run(self) -> None:
        mode = "smoke" if self.args.smoke else "capture"
        self.log(f"capture run {self.args.run_id}")
        self.log(f"source commit {self.revision}")
        self.log(f"content fingerprint {self.content_fingerprint}")
        self.log(f"artifact identity {self.identity}")
        self.log(
            f"task {self.args.task} scenario {self.args.scenario} mode {mode} "
            f"dimensions {self.width}x{self.height}"
        )
        self.log(f"scene {self.args.scene} transport {self.args.transport}")
        self.compile_helper()
        self.build_player()
        self.launch_player()
        self.wait_until_ready()
        window_id, window_x, window_y, window_width, window_height = self.find_window()
        video_path = self.output_directory / f"{self.args.run_id}.mp4"
        before_png = self.output_directory / f"{self.args.run_id}-before.png"
        after_png = self.output_directory / f"{self.args.run_id}-after.png"
        hold_started_at = self.start_recording(window_id, video_path)
        if not self.args.smoke and self.args.capture in {"png", "both"}:
            self.capture_png("before", window_id, before_png)
        if hold_started_at is not None:
            wait_for_initial_hold(hold_started_at, float(self.args.initial_hold_seconds))
        status = self.drive_scenario(
            window_x, window_y, window_width, window_height, hold_started_at
        )
        if not self.args.smoke and self.args.capture in {"png", "both"}:
            self.capture_png("after", window_id, after_png)
        if self.recorder is not None:
            if self.recorder.wait() != 0:
                self.recorder = None
                fail("Video capture failed.")
            self.recorder = None
        if not self.args.smoke and self.args.capture in {"video", "both"}:
            video_details = run_output([str(self.helper), "inspect-video", str(video_path)]).split()
            if (
                len(video_details) != 4
                or int(video_details[0]) != self.width
                or int(video_details[1]) != self.height
                or video_details[3] != "avc1"
            ):
                fail(f"MP4 inspection failed: {' '.join(video_details)}")
            frame_rate = float(video_details[2])
            if not 28 <= frame_rate <= 31:
                fail(f"MP4 frame rate was {frame_rate} rather than 30 fps.")
            self.log(f"video {video_path}")
        self.log(f"assertions passed: {', '.join(status['assertions'])}")
        if self.args.smoke:
            self.log("smoke validation passed; no media produced")
        else:
            self.log(f"evidence retained at {self.output_directory}")


def validate_arguments(args: argparse.Namespace) -> None:
    if SAFE_NAME.fullmatch(args.task) is None:
        fail("A safe --task ID is required.", 2)
    if SAFE_NAME.fullmatch(args.scenario) is None:
        fail("A safe --scenario name is required.", 2)
    if args.cargo_package and SAFE_NAME.fullmatch(args.cargo_package) is None:
        fail("Invalid --cargo-package name.", 2)
    if SAFE_NAME.fullmatch(args.run_id) is None:
        fail("A safe --run-id is required.", 2)
    if not args.scene.startswith("Assets/") or not args.scene.endswith(".unity"):
        fail("--scene must name an Assets/*.unity file.", 2)
    if ".." in Path(args.scene).parts:
        fail("--scene may not traverse parent directories.", 2)
    if not (REPOSITORY_ROOT / args.scene).is_file():
        fail(f"Capture scene was not found: {args.scene}", 2)
    if (args.plugin or args.cargo_package) and args.transport != "native":
        fail("A native plugin requires --transport native.", 2)
    if args.transport == "native" and not (args.plugin or args.cargo_package):
        fail("--transport native requires --plugin or --cargo-package.", 2)
    if not 1 <= args.video_seconds <= 60:
        fail("Video duration must be between 1 and 60 seconds.", 2)
    if not is_nonnegative_number(args.initial_hold_seconds):
        fail("Initial hold must be a nonnegative number.", 2)
    if not 1 <= args.interaction_timeout <= 120:
        fail("Interaction timeout must be between 1 and 120 seconds.", 2)
    if platform.system() != "Darwin":
        fail("Release-player capture is supported on macOS only.")
    version = next(
        line.removeprefix("m_EditorVersion: ")
        for line in (REPOSITORY_ROOT / "ProjectSettings/ProjectVersion.txt").read_text().splitlines()
        if line.startswith("m_EditorVersion: ")
    )
    editor = Path(
        os.environ.get(
            "UNITY_EDITOR", f"/Applications/Unity/Hub/Editor/{version}/Unity.app/Contents/MacOS/Unity"
        )
    )
    if not os.access(editor, os.X_OK):
        fail(f"Unity {version} was not found at {editor}.")
    if args.plugin and not resolved(args.plugin).is_file():
        fail(f"Native plugin was not found: {args.plugin}", 2)


def interrupted(_signal_number, _frame) -> None:
    raise KeyboardInterrupt


def main() -> None:
    args = parse_arguments()
    validate_arguments(args)
    capture = CaptureRun(args)
    succeeded = False
    try:
        capture.run()
        succeeded = True
    finally:
        if not capture.cleanup():
            succeeded = False
    if not succeeded:
        raise SystemExit(1)


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGINT, interrupted)
    try:
        main()
    except KeyboardInterrupt:
        raise SystemExit(130) from None
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        raise SystemExit(1) from error
