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
import shutil
import signal
import subprocess
import sys
import tempfile
import time

from visual_capture_lib import (
    SlotLease,
    inspect_video,
    now,
    player_log_diagnostics,
    project_fingerprint,
    sample_project_fingerprint,
    sha256_file,
    tracked_state,
    verify_png_dimensions,
    wait_for_initial_hold,
    wait_for_capture_ack,
    write_capture_command,
    unity_editor_lease,
)
from visual_capture_options import parse_arguments, validate_arguments
from visual_capture_slots import (
    BuildSlotPool,
    accelerator_state,
    compatibility_manifest,
    remove_owned_path,
    sync_sample_project,
    sync_standard_project,
)


REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
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
        self.project_root = resolved(args.project_root).resolve()
        self.width, self.height = args.dimensions
        self.scene_path = self.project_root / args.scene
        self.artifact_root = resolved(args.artifact_root)
        self.build_cache_root = (
            resolved(args.build_cache)
            if args.build_cache
            else Path.home() / "Library/Caches/Battlement/visual-capture"
        )
        self.sample_harness_root = (
            resolved(args.sample_harness_root).resolve() if args.sample_harness_root else None
        )
        self.cargo_manifest = (
            resolved(args.cargo_manifest).resolve() if args.cargo_manifest else None
        )
        self.plugin = resolved(args.plugin) if args.plugin else None
        self.unity_version = next(
            line.removeprefix("m_EditorVersion: ")
            for line in (self.project_root / "ProjectSettings/ProjectVersion.txt").read_text().splitlines()
            if line.startswith("m_EditorVersion: ")
        )
        self.unity_editor = Path(
            os.environ.get(
                "UNITY_EDITOR",
                f"/Applications/Unity/Hub/Editor/{self.unity_version}/Unity.app/Contents/MacOS/Unity",
            )
        )
        self.revision = run_output(["git", "rev-parse", "HEAD"], cwd=REPOSITORY_ROOT)
        cargo_identity = args.cargo_package or (
            self.cargo_manifest.relative_to(self.project_root).as_posix()
            if self.cargo_manifest
            else ""
        )
        plugin_identity = (
            sha256_file(self.plugin) if self.plugin else f"cargo:{cargo_identity}"
            if cargo_identity else ""
        )
        if self.sample_harness_root:
            self.content_fingerprint = sample_project_fingerprint(
                self.project_root,
                self.sample_harness_root,
                args.scene,
                args.scenario,
                args.transport,
                plugin_identity,
            )
        else:
            self.content_fingerprint = project_fingerprint(
                self.project_root, args.scene, args.scenario, args.transport, plugin_identity
            )
        self.identity = f"{self.revision}-{self.content_fingerprint[:12]}"
        self.output_directory = self.artifact_root / self.identity / args.task / args.run_id
        if self.output_directory.exists():
            fail(f"Refusing to overwrite capture run: {self.output_directory}")
        self.output_directory.mkdir(parents=True)
        self.run_log = self.output_directory / f"{args.run_id}.log"
        self.temporary_root = Path(tempfile.mkdtemp(prefix="battlement-capture."))
        self.control_directory = self.temporary_root / "control"
        self.control_directory.mkdir()
        self.initial_tracked_state = tracked_state(REPOSITORY_ROOT)
        self.isolated_project = self.temporary_root / "unselected-project"
        self.materialized_repository = self.isolated_project
        self.status_path = self.temporary_root / "player-status.json"
        self.unity_log = self.temporary_root / "unity-build.log"
        self.player_log = self.temporary_root / "player.log"
        self.helper = self.temporary_root / "macos-capture"
        build_digest = hashlib.sha256(
            f"{self.content_fingerprint}\0{self.unity_version}\0".encode()
        ).hexdigest()
        self.cache_directory = self.build_cache_root / "players" / build_digest
        self.build_path = self.cache_directory / "Battlement Capture.app"
        self.cache_manifest = self.cache_directory / "manifest.json"
        self.lock_directory = self.build_cache_root / "locks"
        self.capture_slot = SlotLease(self.lock_directory, "capture", 5)
        self.legacy_slot = SlotLease(self.lock_directory, "legacy", 1)
        self.command_id = 0
        self.encoder_pid: int | None = None
        ffmpeg = resolved(args.ffmpeg) if args.ffmpeg else shutil.which("ffmpeg")
        self.ffmpeg = Path(ffmpeg) if ffmpeg else None
        self.ffprobe = (
            self.ffmpeg.with_name("ffprobe")
            if self.ffmpeg and self.ffmpeg.with_name("ffprobe").is_file()
            else Path(shutil.which("ffprobe") or "")
        )
        self.player_pid: int | None = None
        self.caffeinate: subprocess.Popen | None = None
        self.recorder: subprocess.Popen | None = None
        self.pointer_button_down = False
        self.last_pointer_x = 0
        self.last_pointer_y = 0
        self.failure_artifacts_preserved = False

    def log(self, message: str) -> None:
        print(message)
        with self.run_log.open("a") as destination:
            print(message, file=destination)

    def append_file_to_log(self, path: Path, count: int | None = None) -> None:
        lines = path.read_text(errors="replace").splitlines()
        with self.run_log.open("a") as destination:
            print("\n".join(lines[-count:] if count else lines), file=destination)

    def preserve_failure_artifacts(self) -> None:
        """Retain transient Unity logs and surface player failures."""
        if self.failure_artifacts_preserved:
            return
        self.failure_artifacts_preserved = True
        for source, name in (
            (self.player_log, f"{self.args.run_id}-player.log"),
            (self.unity_log, f"{self.args.run_id}-unity-build.log"),
        ):
            if not source.is_file():
                continue
            destination = self.output_directory / name
            shutil.copy2(source, destination)
            message = f"retained failure log {destination}"
            self.log(message)
            print(message, file=sys.stderr)
            if source == self.player_log:
                diagnostics = player_log_diagnostics(destination)
                if diagnostics:
                    heading = "Relevant player-log errors/exceptions:"
                    print(f"{heading}\n{diagnostics}", file=sys.stderr)
                    with self.run_log.open("a") as run_log:
                        print(f"{heading}\n{diagnostics}", file=run_log)

    def cleanup(self) -> bool:
        terminate_process(self.recorder)
        if (
            self.args.input_driver == "macos-hid"
            and self.pointer_button_down
            and self.helper.exists()
            and self.player_pid is not None
        ):
            subprocess.run(
                [
                    str(self.helper), "pointer-left-button-up", str(self.player_pid),
                    str(self.last_pointer_x), str(self.last_pointer_y),
                ],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
        terminate_pid(self.player_pid)
        terminate_pid(self.encoder_pid)
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
        remove_owned_path(self.temporary_root, self.temporary_root.parent)
        self.capture_slot.close()
        self.legacy_slot.close()
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
        legacy_input = self.args.input_driver == "macos-hid"
        legacy_media = not self.args.smoke and self.args.media_driver == "screen-capture-kit"
        if legacy_input or legacy_media:
            preflight = "preflight" if legacy_media else "preflight-input"
            with self.run_log.open("a") as destination:
                subprocess.run(
                    [str(self.helper), preflight],
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
            self.log("packaged-player cache hit")
            self.log("selected build slot: none (packaged player already complete)")
            self.log(f"reusing verified packaged player {self.build_path}")
            return
        self.log("packaged-player cache miss")
        cache_lock = SlotLease(self.lock_directory, f"cache-{self.cache_directory.name}", 1)
        with cache_lock:
            if self.cache_is_valid():
                self.log("packaged-player cache hit after waiting for publisher")
                self.log("selected build slot: none (packaged player already complete)")
                self.log(f"reusing verified packaged player {self.build_path}")
                return
            with unity_editor_lease():
                self._build_player_locked()

    def _build_player_locked(self) -> None:
        if self.cache_is_valid():
            self.log("packaged-player cache hit after waiting for build capacity")
            self.log("selected build slot: none (packaged player already complete)")
            self.log(f"reusing verified packaged player {self.build_path}")
            return
        if self.cache_directory.exists():
            self.log(f"discarding an incomplete or invalid cache entry {self.cache_directory}")
            remove_owned_path(self.cache_directory, self.build_cache_root / "players")
        self.log(f"building isolated non-Development macOS player with Unity {self.unity_version}")
        layout = "sample-overlay" if self.sample_harness_root else "repository"
        compatibility = compatibility_manifest(
            self.project_root, self.unity_version, layout, self.sample_harness_root
        )
        slot_pool = BuildSlotPool(self.build_cache_root, compatibility)
        with slot_pool.acquire() as slot:
            materialized_repository = slot.path / "source"
            self.materialized_repository = materialized_repository
            self.isolated_project = (
                materialized_repository
                / self.project_root.relative_to(self.sample_harness_root)
                if self.sample_harness_root
                else slot.project
            )
            self.log(f"selected build slot {slot.path} ({slot.disposition})")
            sync_started = time.monotonic()
            try:
                if self.sample_harness_root:
                    sync_sample_project(
                        self.project_root,
                        self.sample_harness_root,
                        self.isolated_project,
                        materialized_repository,
                    )
                else:
                    sync_standard_project(self.project_root, self.isolated_project)
            finally:
                self.log(
                    f"source synchronization time {time.monotonic() - sync_started:.2f}s"
                )
            self.log(f"incremental state before build: {accelerator_state(self.isolated_project)}")
            plugin = self._build_plugin()
            if plugin:
                isolated_plugin_directory = self.isolated_project / "Assets/Plugins/macOS"
                isolated_plugin_directory.mkdir(parents=True, exist_ok=True)
                isolated_plugin = isolated_plugin_directory / "libbattlement_rules.dylib"
                shutil.copy2(plugin, isolated_plugin)
                architectures = run_output(["lipo", "-archs", str(isolated_plugin)]).split()
                if platform.machine() not in architectures:
                    fail(f"The native plugin lacks host architecture {platform.machine()}.")
            uncached_build = self.temporary_root / "Battlement Capture.app"
            environment = os.environ.copy()
            environment.update(
                BATTLEMENT_CAPTURE_BUILD_PATH=str(uncached_build),
                BATTLEMENT_CAPTURE_SCENE_PATH=self.args.scene,
                BATTLEMENT_CAPTURE_SCENARIO=self.args.scenario,
            )
            unity_started = time.monotonic()
            result = subprocess.run(
                [
                    str(self.unity_editor), "-batchmode", "-nographics",
                    "--burst-disable-compilation", "-quit", "-projectPath",
                    str(self.isolated_project), "-executeMethod", self.args.build_method,
                    "-logFile", str(self.unity_log),
                ],
                env=environment,
            )
            self.log(f"Unity build time {time.monotonic() - unity_started:.2f}s")
            if result.returncode != 0:
                self.append_file_to_log(self.unity_log, 120)
                fail("Unity release player build failed.")
            if f"BATTLEMENT_CAPTURE_BUILD_OK:{uncached_build}" not in self.unity_log.read_text(errors="replace"):
                self.append_file_to_log(self.unity_log, 120)
                fail("Unity omitted the build success marker.")
            try:
                seed_outcome = slot_pool.publish_seed(slot)
                self.log(f"published disposable slot seed ({seed_outcome})")
            except (OSError, subprocess.CalledProcessError, ValueError) as error:
                self.log(f"slot seed publication skipped: {error}")
        cache_staging = self.temporary_root / "cache"
        cache_staging.mkdir()
        subprocess.run(
            ["ditto", str(uncached_build), str(cache_staging / "Battlement Capture.app")], check=True
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
        self.cache_directory.parent.mkdir(parents=True, exist_ok=True)
        if not self.cache_directory.exists():
            cache_staging.rename(self.cache_directory)
        self.log(f"cached packaged player {self.build_path}")

    def _build_plugin(self) -> Path | None:
        if self.plugin:
            self.log("Cargo build time 0.00s (prebuilt plugin)")
            return self.plugin
        if not self.args.cargo_package and not self.cargo_manifest:
            self.log("Cargo build time 0.00s (not requested)")
            return None
        manifest = (
            self.isolated_project / self.cargo_manifest.relative_to(self.project_root)
            if self.cargo_manifest
            else self.isolated_project / "Cargo.toml"
        )
        command = [
            "cargo", "build", "--quiet", "--release", "--manifest-path", str(manifest),
            "--target-dir", str(self.isolated_project / "target"),
        ]
        if self.args.cargo_package:
            command.extend(("-p", self.args.cargo_package))
        self._refresh_cargo_inputs()
        cargo_started = time.monotonic()
        result = subprocess.run(command)
        self.log(f"Cargo build time {time.monotonic() - cargo_started:.2f}s")
        result.check_returncode()
        return self.isolated_project / "target/release/libbattlement_rules.dylib"

    def _refresh_cargo_inputs(self) -> None:
        for path in self.materialized_repository.rglob("*"):
            if "target" in path.relative_to(self.materialized_repository).parts:
                continue
            is_cargo_input = path.suffix == ".rs" or path.name in {
                "Cargo.toml",
                "Cargo.lock",
            }
            if path.is_file() and is_cargo_input:
                path.touch()

    def launch_player(self) -> tuple[Path, str]:
        executable_name = run_output(
            ["plutil", "-extract", "CFBundleExecutable", "raw", str(self.build_path / "Contents/Info.plist")]
        )
        player_executable = self.build_path / f"Contents/MacOS/{executable_name}"
        packaged_plugin = self.build_path / "Contents/PlugIns/libbattlement_rules.dylib"
        if not os.access(player_executable, os.X_OK):
            fail("The player executable is missing.")
        if self.args.transport == "native":
            if not packaged_plugin.is_file():
                fail("The bundled native plugin is missing.")
            if platform.machine() not in run_output(["lipo", "-archs", str(packaged_plugin)]).split():
                fail(f"The packaged dylib lacks host architecture {platform.machine()}.")
        self.log("launching packaged player without Editor library search paths")
        command = [
            str(self.helper), "launch-background", str(self.build_path),
            "-popupwindow", "-screen-fullscreen", "0",
            "-screen-width", str(self.width), "-screen-height", str(self.height),
            "-battlementCaptureScenario", self.args.scenario, "-battlementCaptureStatus",
            str(self.status_path), "-battlementCaptureControl", str(self.control_directory),
            "-battlementCaptureInputDriver", self.args.input_driver,
            "-logFile", str(self.player_log),
        ]
        if self.args.show_overlay:
            command.append("-battlementCaptureOverlay")
        self.player_pid = int(run_output(command))
        self.log(f"player PID {self.player_pid}")
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
        if self.args.input_driver == "macos-hid":
            subprocess.run([str(self.helper), "focus", str(self.player_pid)], check=True)
        return window_id, float(window_x), float(window_y), float(window_width), float(window_height)

    def start_recording(self, window_id: str, video_path: Path) -> float | None:
        if self.args.smoke or self.args.capture not in {"video", "both"}:
            return None
        self.log(
            f"recording H.264 interaction video with {self.args.initial_hold_seconds}s initial hold"
        )
        if self.args.media_driver == "in-player":
            acknowledgement = self.send_command(
                {
                    "kind": "start-video",
                    "outputPath": str(video_path),
                    "ffmpegPath": str(self.ffmpeg),
                    "width": self.width,
                    "height": self.height,
                    "frameRate": 30,
                    "durationSeconds": self.args.video_seconds,
                },
                15,
            )
            self.encoder_pid = acknowledgement.get("encoderPid") or None
            self.log(f"encoder PID {self.encoder_pid}")
            return now()
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
        if self.args.media_driver == "in-player":
            self.send_command({"kind": "capture-png", "outputPath": str(path)}, 15)
        else:
            subprocess.run(
                ["/usr/sbin/screencapture", "-x", "-o", "-l", window_id, str(path)],
                check=True,
            )
        if not verify_png_dimensions(path, self.width, self.height):
            fail(f"{label.capitalize()} PNG dimensions did not match {self.width}x{self.height}.")
        self.log(f"{label} PNG {path}")

    def send_command(self, command: dict, timeout: float = 10) -> dict:
        self.command_id += 1
        write_capture_command(self.control_directory, self.command_id, command)
        return wait_for_capture_ack(
            self.control_directory, self.command_id, timeout, self.player_pid
        )

    def wait_for_in_player_video(self, video_path: Path) -> None:
        completion_path = Path(str(video_path) + ".capture.json")
        deadline = time.monotonic() + self.args.video_seconds + 15
        completion = {}
        while time.monotonic() < deadline:
            completion = read_json(completion_path)
            if completion:
                break
            if not process_alive(self.player_pid):
                fail("Player exited before video encoding completed.")
            time.sleep(0.1)
        if not completion.get("success"):
            fail(completion.get("error", "In-player video encoding timed out."))
        if completion.get("frames") != self.args.video_seconds * 30:
            fail("In-player video emitted the wrong frame count.")
        self.encoder_pid = None

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
            if (
                self.args.media_driver == "in-player"
                and hold_started_at is not None
                and Path(
                    str(self.output_directory / f"{self.args.run_id}.mp4")
                    + ".capture.json"
                ).exists()
            ):
                fail("Video ended before the scenario completed.")
            status = read_json(self.status_path)
            phase = status.get("phase", "")
            if phase in {"passed", "failed"}:
                break
            if phase == "ready" and status.get("requestId", -1) != last_request:
                request_id = status.get("requestId")
                input_device = status.get("inputDevice", "pointer")
                capture_input = status.get("input", "")
                pointer_x = status.get("pointerX", -1)
                pointer_y = status.get("pointerY", -1)
                if not isinstance(request_id, int):
                    fail("Input request ID was invalid.")
                if request_id != last_request + 1:
                    fail("Input request IDs must be consecutive.")
                if capture_input not in {
                    "pointer-move", "pointer-left-button-down", "pointer-left-button-up",
                    "key-down", "key-up",
                }:
                    fail(f"Unsupported capture input: {capture_input}")
                if input_device == "pointer" and not all(
                    isinstance(value, (int, float)) and 0 <= value <= 1
                    for value in (pointer_x, pointer_y)
                ):
                    fail("Input request lacks normalized coordinates.")
                if input_device == "pointer":
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
                if self.args.input_driver == "in-player":
                    self.send_command(
                        {"kind": "dispatch-input", "requestId": request_id},
                        self.args.interaction_timeout,
                    )
                else:
                    if input_device != "pointer":
                        fail("The macOS HID driver does not support keyboard capture requests.")
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
        self.capture_slot.acquire()
        if self.args.input_driver == "macos-hid" or self.args.media_driver == "screen-capture-kit":
            self.legacy_slot.acquire()
        mode = "smoke" if self.args.smoke else "capture"
        self.log(f"capture run {self.args.run_id}")
        self.log(f"run log {self.run_log}")
        self.log(f"source commit {self.revision}")
        self.log(f"content fingerprint {self.content_fingerprint}")
        self.log(f"artifact identity {self.identity}")
        self.log(
            f"task {self.args.task} scenario {self.args.scenario} mode {mode} "
            f"dimensions {self.width}x{self.height}"
        )
        self.log(f"scene {self.args.scene} transport {self.args.transport}")
        if (
            not self.args.smoke
            and self.args.media_driver == "in-player"
            and self.args.capture in {"video", "both"}
        ):
            version = run_output([str(self.ffmpeg), "-version"]).splitlines()[0]
            self.log(f"FFmpeg {version}; sha256 {sha256_file(self.ffmpeg)}")
        self.compile_helper()
        self.build_player()
        self.launch_player()
        self.wait_until_ready()
        if (
            self.args.input_driver == "macos-hid"
            or (not self.args.smoke and self.args.media_driver == "screen-capture-kit")
        ):
            window_id, window_x, window_y, window_width, window_height = self.find_window()
        else:
            window_id, window_x, window_y = "", 0.0, 0.0
            window_width, window_height = float(self.width), float(self.height)
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
            if self.args.media_driver == "in-player":
                self.wait_for_in_player_video(video_path)
                details = inspect_video(self.ffprobe, video_path)
                stream = details.get("streams", [{}])[0]
                if (
                    stream.get("codec_name") != "h264"
                    or stream.get("width") != self.width
                    or stream.get("height") != self.height
                    or stream.get("r_frame_rate") != "30/1"
                    or int(stream.get("nb_read_frames", 0)) != self.args.video_seconds * 30
                    or abs(
                        float(details.get("format", {}).get("duration", 0))
                        - self.args.video_seconds
                    ) > 0.02
                ):
                    fail(f"MP4 inspection failed: {json.dumps(details, sort_keys=True)}")
            else:
                video_details = run_output(
                    [str(self.helper), "inspect-video", str(video_path)]
                ).split()
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
        for evidence in status.get("evidence", []):
            self.log(f"evidence: {evidence}")
        if self.args.smoke:
            self.log("smoke validation passed; no media produced")
        else:
            self.log(f"evidence retained at {self.output_directory}")


def interrupted(_signal_number, _frame) -> None:
    raise KeyboardInterrupt


def main() -> None:
    args = parse_arguments()
    validate_arguments(args, REPOSITORY_ROOT)
    capture = CaptureRun(args)
    succeeded = False
    try:
        capture.run()
        succeeded = True
    except BaseException:
        capture.preserve_failure_artifacts()
        raise
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
