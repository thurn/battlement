#!/usr/bin/env python3
"""Build and run the release worker proof on an iOS Simulator or Android emulator."""

from __future__ import annotations

import argparse
import contextlib
import hashlib
import json
import os
from pathlib import Path
import plistlib
import selectors
import shutil
import subprocess
import sys
import tempfile
import time
import zipfile


ROOT = Path(__file__).resolve().parents[1]
UNITY = Path("/Applications/Unity/Hub/Editor/6000.5.8f1/Unity.app/Contents/MacOS/Unity")
ANDROID_MODULE = UNITY.parents[3] / "PlaybackEngines/AndroidPlayer"
UNITY_SDK = ANDROID_MODULE / "SDK"
NDK = ANDROID_MODULE / "NDK"
JAVA_HOME = ANDROID_MODULE / "OpenJDK"
CONFIG = ROOT / "fixtures/mobile-worker-proof/ditto.toml"
SCENE = "Assets/BattlementWebglWorkerProof/BattlementWebglWorkerProof.unity"
MANIFEST = ROOT / "crates/battlement-native/tests/fixtures/exported-engine/Cargo.toml"
PACKAGE = "com.battlement.ditto.mobileworkerproof"
MARKERS = (
    "BATTLEMENT_WEBGL_WORKER_STARTED_OFF_UI",
    "BATTLEMENT_WEBGL_WORKER_CANCELLED_CLEANED_STOPPED",
    "BATTLEMENT_WEBGL_WORKER_LATEST_REPLACEMENT_ONLY",
    "BATTLEMENT_WEBGL_WORKER_REAL_PANIC_FAILED",
    "BATTLEMENT_WEBGL_WORKER_RECOVERED",
    "BATTLEMENT_WEBGL_WORKER_NONJOINING_EXIT_CLEANED",
)
PLUGIN_META = b"""fileFormatVersion: 2
guid: 821c7f6f38454ea0ab770332096066f8
PluginImporter:
  externalObjects: {}
  serializedVersion: 3
  iconMap: {}
  executionOrder: {}
  defineConstraints: []
  isPreloaded: 0
  isOverridable: 0
  isExplicitlyReferenced: 0
  validateReferences: 1
  platformData: []
  userData:
  assetBundleName:
  assetBundleVariant:
"""


def run(command: list[str | Path], **kwargs) -> subprocess.CompletedProcess[str]:
    rendered = [str(value) for value in command]
    check = kwargs.pop("check", True)
    return subprocess.run(rendered, check=check, text=True, **kwargs)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def unity_android_commands() -> Path:
    candidates = list((UNITY_SDK / "cmdline-tools").glob("*/bin"))
    if not candidates:
        raise RuntimeError("Unity's Android command-line tools are required")
    return max(candidates)


def verify_symbols(binary: Path, tool: Path, symbols: tuple[str, ...]) -> None:
    output = run([tool, "-Ws", binary], capture_output=True).stdout
    missing = [symbol for symbol in symbols if symbol not in output]
    if missing:
        raise RuntimeError(f"{binary} does not export: {', '.join(missing)}")


class StagedFile:
    def __init__(self, path: Path, contents: bytes):
        self.path = path
        self.previous = path.read_bytes() if path.is_file() else None
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(contents)

    def restore(self) -> None:
        if self.previous is None:
            self.path.unlink(missing_ok=True)
        else:
            self.path.write_bytes(self.previous)


@contextlib.contextmanager
def android_plugin(plugin: Path):
    destination = ROOT / "Assets/Plugins/Android/arm64-v8a/libbattlement_rules.so"
    staged = [
        StagedFile(destination, plugin.read_bytes()),
        StagedFile(Path(f"{destination}.meta"), PLUGIN_META),
    ]
    try:
        yield
    finally:
        for entry in reversed(staged):
            entry.restore()


def build_ios() -> dict[str, object]:
    completed = run(
        [
            "cargo",
            "run",
            "--quiet",
            "-p",
            "battlement-ditto",
            "--",
            "--config",
            CONFIG,
            "build",
            "--profile",
            "ios-simulator",
            "--json",
        ],
        cwd=ROOT,
        capture_output=True,
    )
    return json.loads(completed.stdout)


def build_android() -> dict[str, object]:
    if not UNITY.is_file() or not NDK.is_dir() or not UNITY_SDK.is_dir():
        raise RuntimeError("Unity Android module, SDK, and NDK are required")
    ndk_bin = NDK / "toolchains/llvm/prebuilt/darwin-x86_64/bin"
    linker = ndk_bin / "aarch64-linux-android26-clang"
    with tempfile.TemporaryDirectory(prefix="battlement-android-") as temporary:
        temporary_path = Path(temporary)
        target = Path.home() / "Library/Caches/Battlement/mobile-native/android"
        environment = os.environ.copy()
        environment.update(
            {
                "RUSTC_BOOTSTRAP": "1",
                "CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER": str(linker),
                "CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS": "-C panic=unwind",
            }
        )
        run(
            [
                "cargo",
                "rustc",
                "--manifest-path",
                MANIFEST,
                "--target",
                "aarch64-linux-android",
                "--target-dir",
                target,
                "--release",
                "--lib",
                "--crate-type",
                "cdylib",
                "-Z",
                "build-std=std,panic_unwind",
                "--config",
                'profile.release.debug="line-tables-only"',
                "--config",
                'profile.release.split-debuginfo="off"',
            ],
            cwd=ROOT,
            env=environment,
        )
        plugin = target / "aarch64-linux-android/release/libbattlement_rules.so"
        verify_symbols(
            plugin,
            ndk_bin / "llvm-readelf",
            (
                "battlement_engine_create",
                "fixture_worker_observation",
                "fixture_worker_release_computation",
            ),
        )
        unsigned_apk = temporary_path / "BattlementDitto.apk"
        unity_log = temporary_path / "unity.log"
        environment.update(
            {
                "BATTLEMENT_DITTO_BUILD_PATH": str(unsigned_apk),
                "BATTLEMENT_DITTO_SCENE_PATH": SCENE,
                "BATTLEMENT_DITTO_DIAGNOSTICS": "1",
                "BATTLEMENT_DITTO_SUITE": "mobile-worker-proof",
            }
        )
        with android_plugin(plugin):
            try:
                run(
                    [
                        sys.executable,
                        ROOT / "scripts/unity_transaction.py",
                        "--project",
                        ROOT,
                        "--",
                        UNITY,
                        "-batchmode",
                        "-nographics",
                        "-quit",
                        "-projectPath",
                        ROOT,
                        "-buildTarget",
                        "Android",
                        "-executeMethod",
                        "Battlement.Editor.BattlementDittoBuild.BuildAndroid",
                        "-logFile",
                        unity_log,
                    ],
                    cwd=ROOT,
                    env=environment,
                )
            except subprocess.CalledProcessError as error:
                details = unity_log.read_text(errors="replace")[-20000:]
                raise RuntimeError(f"Android Unity build failed:\n{details}") from error
        if not unsigned_apk.is_file():
            raise RuntimeError(f"Unity omitted Android artifact; log: {unity_log}")
        with zipfile.ZipFile(unsigned_apk) as archive:
            members = set(archive.namelist())
        required = {
            "lib/arm64-v8a/libbattlement_rules.so",
            "lib/arm64-v8a/libunity.so",
        }
        if not required.issubset(members):
            raise RuntimeError(f"Android artifact omits native libraries: {sorted(required - members)}")
        digest = sha256(unsigned_apk)
        retained = (
            Path.home()
            / "Library/Caches/Battlement/mobile-builds/android"
            / digest
            / "BattlementDitto.apk"
        )
        retained.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(unsigned_apk, retained)
        return {
            "schema": 1,
            "suite": "mobile-worker-proof",
            "profile": "android-emulator",
            "target": "android-emulator",
            "build_fingerprint": digest,
            "application_path": str(retained),
            "architecture": "arm64-v8a",
            "minimum_android_api": 26,
            "threaded": True,
            "panic_runtime": "unwind",
            "physical": False,
        }


def collect_until(process: subprocess.Popen[str], final_marker: str, timeout: float) -> str:
    deadline = time.monotonic() + timeout
    lines: list[str] = []
    assert process.stdout is not None
    ready = selectors.DefaultSelector()
    ready.register(process.stdout, selectors.EVENT_READ)
    while time.monotonic() < deadline:
        if process.poll() is not None:
            break
        for key, _ in ready.select(timeout=min(0.25, deadline - time.monotonic())):
            line = key.fileobj.readline()
            if line:
                lines.append(line)
                if final_marker in line:
                    return "".join(lines)
    raise RuntimeError(f"mobile worker proof did not finish; observed log:\n{''.join(lines[-100:])}")


def validate_log(log: str, final_marker: str) -> None:
    missing = [marker for marker in (*MARKERS, final_marker) if marker not in log]
    if missing:
        raise RuntimeError(f"mobile worker proof omitted markers: {', '.join(missing)}")


def run_ios(build: dict[str, object], timeout: float) -> dict[str, object]:
    application = Path(str(build["application_path"]))
    info = plistlib.loads((application / "Info.plist").read_bytes())
    bundle = str(info["CFBundleIdentifier"])
    native_runtime = application / "Frameworks/UnityFramework.framework/UnityFramework"
    symbols = run(["nm", "-gU", native_runtime], capture_output=True).stdout
    if not all(
        symbol in symbols
        for symbol in (
            "battlement_engine_create",
            "fixture_worker_observation",
            "fixture_worker_release_computation",
        )
    ):
        raise RuntimeError("iOS application does not link the Battlement native exports")
    name = f"Battlement-{os.getpid()}"
    created = run(
        ["xcrun", "simctl", "create", name, "iPhone 17"], capture_output=True
    ).stdout.strip()
    try:
        run(["xcrun", "simctl", "boot", created])
        run(["xcrun", "simctl", "bootstatus", created, "-b"])
        run(["xcrun", "simctl", "install", created, application])
        player = subprocess.Popen(
            [
                "xcrun",
                "simctl",
                "launch",
                "--console-pty",
                created,
                bundle,
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )
        try:
            log = collect_until(
                player,
                "BATTLEMENT_WEBGL_WORKER_NONJOINING_EXIT_CLEANED",
                timeout,
            )
            validate_log(log, "BATTLEMENT_MOBILE_WORKER_TARGET:IPhonePlayer")
        finally:
            player.terminate()
            with contextlib.suppress(subprocess.TimeoutExpired):
                player.wait(timeout=5)
        device = json.loads(
            run(["xcrun", "simctl", "list", "devices", "-j"], capture_output=True).stdout
        )
        identity = next(
            entry
            for entries in device["devices"].values()
            for entry in entries
            if entry.get("udid") == created
        )
        return {"name": identity["name"], "udid": created, "runtime": "iOS Simulator"}
    finally:
        run(["xcrun", "simctl", "shutdown", created], check=False)
        run(["xcrun", "simctl", "delete", created], check=False)


def android_environment() -> tuple[Path, dict[str, str]]:
    sdk = Path.home() / "Library/Caches/Battlement/android-sdk"
    environment = os.environ.copy()
    environment.update(
        {
            "JAVA_HOME": str(JAVA_HOME),
            "ANDROID_SDK_ROOT": str(sdk),
            "ANDROID_HOME": str(sdk),
        }
    )
    return sdk, environment


def install_android_prerequisites() -> None:
    sdk, environment = android_environment()
    sdk.mkdir(parents=True, exist_ok=True)
    commands_source = unity_android_commands()
    sdkmanager = commands_source / "sdkmanager"
    packages = (
        "platform-tools",
        "emulator",
        "platforms;android-36",
        "system-images;android-36;google_apis;arm64-v8a",
    )
    run(
        [sdkmanager, f"--sdk_root={sdk}", *packages],
        env=environment,
        input="y\n" * 40,
    )
    commands = sdk / "cmdline-tools/latest"
    if not commands.exists():
        shutil.copytree(commands_source.parent, commands)


def run_android(build: dict[str, object], timeout: float) -> dict[str, object]:
    sdk, environment = android_environment()
    emulator = sdk / "emulator/emulator"
    adb = sdk / "platform-tools/adb"
    image = sdk / "system-images/android-36/google_apis/arm64-v8a"
    if not emulator.is_file() or not adb.is_file() or not image.is_dir():
        raise RuntimeError(
            "Android emulator prerequisites are missing; rerun with --install-android-prerequisites"
        )
    avd_home = sdk / "avd"
    avd_home.mkdir(parents=True, exist_ok=True)
    environment["ANDROID_AVD_HOME"] = str(avd_home)
    avd = "battlement-mobile-worker"
    avdmanager = sdk / "cmdline-tools/latest/bin/avdmanager"
    run(
        [
            avdmanager,
            "create",
            "avd",
            "--force",
            "--name",
            avd,
            "--package",
            "system-images;android-36;google_apis;arm64-v8a",
            "--device",
            "pixel_8",
        ],
        env=environment,
        input="no\n",
    )
    emulator_process = subprocess.Popen(
        [
            str(emulator),
            "-avd",
            avd,
            "-no-window",
            "-no-audio",
            "-no-boot-anim",
            "-gpu",
            "swiftshader_indirect",
            "-wipe-data",
        ],
        env=environment,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    try:
        run([adb, "wait-for-device"], env=environment, timeout=180)
        deadline = time.monotonic() + 180
        while time.monotonic() < deadline:
            booted = run(
                [adb, "shell", "getprop", "sys.boot_completed"],
                env=environment,
                capture_output=True,
            ).stdout.strip()
            if booted == "1":
                break
            time.sleep(2)
        else:
            raise RuntimeError("Android emulator did not finish booting")
        run([adb, "install", "-r", build["application_path"]], env=environment)
        run([adb, "logcat", "-c"], env=environment)
        resolved = run(
            [adb, "shell", "cmd", "package", "resolve-activity", "--brief", PACKAGE],
            env=environment,
            capture_output=True,
        ).stdout.strip()
        activity = next(
            (line for line in reversed(resolved.splitlines()) if "/" in line),
            "",
        )
        if "/" not in activity:
            raise RuntimeError(f"Android launcher activity was not resolved: {activity}")
        logger = subprocess.Popen(
            [str(adb), "logcat", "Unity:I", "*:S"],
            env=environment,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )
        try:
            run([adb, "shell", "am", "start", "-n", activity], env=environment)
            log = collect_until(
                logger,
                "BATTLEMENT_WEBGL_WORKER_NONJOINING_EXIT_CLEANED",
                timeout,
            )
            validate_log(log, "BATTLEMENT_MOBILE_WORKER_TARGET:Android")
        finally:
            logger.terminate()
            with contextlib.suppress(subprocess.TimeoutExpired):
                logger.wait(timeout=5)
        return {
            "name": run([adb, "shell", "getprop", "ro.product.model"], env=environment, capture_output=True).stdout.strip(),
            "serial": run([adb, "get-serialno"], env=environment, capture_output=True).stdout.strip(),
            "api": run([adb, "shell", "getprop", "ro.build.version.sdk"], env=environment, capture_output=True).stdout.strip(),
            "abi": run([adb, "shell", "getprop", "ro.product.cpu.abi"], env=environment, capture_output=True).stdout.strip(),
        }
    finally:
        run([adb, "emu", "kill"], env=environment, check=False)
        with contextlib.suppress(subprocess.TimeoutExpired):
            emulator_process.wait(timeout=20)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", required=True, choices=("ios-simulator", "android-emulator"))
    parser.add_argument("--build-only", action="store_true")
    parser.add_argument("--install-android-prerequisites", action="store_true")
    parser.add_argument("--timeout", type=float, default=180.0)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()
    if arguments.install_android_prerequisites:
        install_android_prerequisites()
    started = time.time()
    build = build_ios() if arguments.target == "ios-simulator" else build_android()
    result: dict[str, object] = {
        "schema": 1,
        "target": arguments.target,
        "physical": False,
        "artifact": build,
        "result": "built" if arguments.build_only else "pass",
        "started_unix": int(started),
    }
    if not arguments.build_only:
        result["runtime"] = (
            run_ios(build, arguments.timeout)
            if arguments.target == "ios-simulator"
            else run_android(build, arguments.timeout)
        )
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
