#!/usr/bin/env python3

"""Exercise visual-capture helpers without launching Unity."""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import time


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY_ROOT / "scripts"))

import visual_capture_lib  # noqa: E402
import visual_capture_slots  # noqa: E402


def fail(message: str) -> None:
    raise AssertionError(f"visual capture workflow test failed: {message}")


def test_default_hold_timing() -> None:
    fake_now = 100.0
    slept: list[float] = []
    original_now = visual_capture_lib.now
    original_sleep = visual_capture_lib.time.sleep

    def current_time() -> float:
        return fake_now

    def sleep(duration: float) -> None:
        nonlocal fake_now
        slept.append(duration)
        fake_now += duration

    try:
        visual_capture_lib.now = current_time
        visual_capture_lib.time.sleep = sleep
        visual_capture_lib.wait_for_initial_hold(100, 2)
    finally:
        visual_capture_lib.now = original_now
        visual_capture_lib.time.sleep = original_sleep
    if slept != [2.0]:
        fail(f"expected one 2-second sleep, got {slept}")
    if fake_now != 102:
        fail(f"expected time 102, got {fake_now}")


def test_zero_hold_override() -> None:
    original_now = visual_capture_lib.now
    original_sleep = visual_capture_lib.time.sleep
    try:
        visual_capture_lib.now = lambda: 100
        visual_capture_lib.time.sleep = lambda duration: fail(
            f"zero-second hold slept for {duration} seconds"
        )
        visual_capture_lib.wait_for_initial_hold(100, 0)
    finally:
        visual_capture_lib.now = original_now
        visual_capture_lib.time.sleep = original_sleep


def test_fingerprint_invalidation() -> None:
    with tempfile.TemporaryDirectory(prefix="battlement-capture-test.") as temporary_directory:
        fixture = Path(temporary_directory)
        for directory in ("Assets", "Packages", "ProjectSettings", "scripts", "crates"):
            (fixture / directory).mkdir()
        scenario = fixture / "Assets/Scenario.cs"
        scenario.write_text("initial\n")
        initial = visual_capture_lib.project_fingerprint(
            fixture, "Assets/Scenario.unity", "demo", "none"
        )
        scenario.write_text("changed\n")
        changed = visual_capture_lib.project_fingerprint(
            fixture, "Assets/Scenario.unity", "demo", "none"
        )
        if initial == changed:
            fail("relevant input change did not invalidate build")


def test_sample_rust_fingerprint_invalidation() -> None:
    with tempfile.TemporaryDirectory(prefix="battlement-sample-fingerprint.") as temporary:
        root = Path(temporary)
        sample = root / "sample"
        harness = root / "harness"
        create_project(sample)
        (sample / "rules/src").mkdir(parents=True)
        rust = sample / "rules/src/lib.rs"
        rust.write_text("pub fn value() -> u8 { 1 }\n")
        for directory in (
            "Assets/VisualCapture",
            "Packages/com.battlement.client",
            "crates/example/src",
            "Assets/Editor",
        ):
            (harness / directory).mkdir(parents=True)
        (harness / "Assets/VisualCapture/Harness.cs").write_text("class Harness {}\n")
        (harness / "Packages/com.battlement.client/package.json").write_text("{}\n")
        (harness / "crates/example/src/lib.rs").write_text("pub struct Example;\n")
        for name in ("Cargo.toml", "Cargo.lock"):
            (harness / name).write_text("\n")
        for suffix in ("", ".meta"):
            (harness / f"Assets/Editor/SampleVisualCaptureBuild.cs{suffix}").write_text("\n")
        initial = visual_capture_lib.sample_project_fingerprint(
            sample, harness, "Assets/Scenario.unity", "sample", "native", "cargo:rules"
        )
        rust.write_text("pub fn value() -> u8 { 2 }\n")
        changed = visual_capture_lib.sample_project_fingerprint(
            sample, harness, "Assets/Scenario.unity", "sample", "native", "cargo:rules"
        )
        if initial == changed:
            fail("sample Rust source edit did not invalidate packaged player")


def test_atomic_capture_protocol() -> None:
    with tempfile.TemporaryDirectory(prefix="battlement-capture-protocol.") as temporary_directory:
        control = Path(temporary_directory)
        first = visual_capture_lib.write_capture_command(
            control, 1, {"kind": "dispatch-input", "requestId": 7}
        )
        second = visual_capture_lib.write_capture_command(
            control, 2, {"kind": "capture-png", "outputPath": "/tmp/frame.png"}
        )
        if first.name != "000001.json" or second.name != "000002.json":
            fail("capture commands were not consecutively named")
        if list((control / "commands").glob("*.new")):
            fail("capture command staging files remained after publication")
        if json.loads(first.read_text()) != {
            "commandId": 1,
            "kind": "dispatch-input",
            "requestId": 7,
        }:
            fail("capture command payload changed during publication")

        acknowledgement_directory = control / "acks"
        acknowledgement_directory.mkdir()
        (acknowledgement_directory / "000001.json").write_text(
            json.dumps({"commandId": 1, "success": True, "encoderPid": 123})
        )
        acknowledgement = visual_capture_lib.wait_for_capture_ack(control, 1, 0.1)
        if acknowledgement.get("encoderPid") != 123:
            fail("capture acknowledgement payload was not returned")


def test_player_log_failure_diagnostics() -> None:
    with tempfile.TemporaryDirectory(prefix="battlement-player-log.") as temporary_directory:
        log = Path(temporary_directory) / "player.log"
        log.write_text(
            "Player initialized\n"
            "InvalidOperationException: PanelSettings theme is missing.\n"
            "  at Battlement.BattlementUiDocuments.Create ()\n"
            "  at Battlement.BattlementWorld.Replace ()\n"
            "Unrelated final line\n"
        )
        diagnostics = visual_capture_lib.player_log_diagnostics(log)
        if "InvalidOperationException: PanelSettings theme is missing." not in diagnostics:
            fail("player exception was omitted from diagnostics")
        if "at Battlement.BattlementUiDocuments.Create" not in diagnostics:
            fail("player exception stack was omitted from diagnostics")


def test_slot_limit() -> None:
    with tempfile.TemporaryDirectory(prefix="battlement-capture-slots.") as temporary_directory:
        locks = Path(temporary_directory)
        first = visual_capture_lib.SlotLease(locks, "capture", 2).acquire()
        second = visual_capture_lib.SlotLease(locks, "capture", 2).acquire()
        acquired = threading.Event()

        def acquire_third() -> None:
            with visual_capture_lib.SlotLease(locks, "capture", 2):
                acquired.set()

        waiter = threading.Thread(target=acquire_third)
        waiter.start()
        time.sleep(0.2)
        if acquired.is_set():
            fail("a third consumer exceeded the two-slot limit")
        first.close()
        waiter.join(timeout=2)
        second.close()
        if not acquired.is_set():
            fail("a released slot did not admit the waiting consumer")


def create_project(root: Path) -> None:
    """Create the smallest source tree accepted by the slot synchronizer."""
    for directory in ("Assets", "Packages", "ProjectSettings", "scripts", "crates"):
        (root / directory).mkdir(parents=True)
    (root / "Assets/Scenario.cs").write_text("initial\n")
    (root / "Packages/manifest.json").write_text('{"dependencies": {}}\n')
    (root / "ProjectSettings/ProjectVersion.txt").write_text(
        "m_EditorVersion: 6000.5.8f1\n"
    )
    (root / "Cargo.toml").write_text("[workspace]\nmembers = []\n")


def test_durable_project_synchronization() -> None:
    with tempfile.TemporaryDirectory(prefix="battlement-slot-sync.") as temporary_directory:
        root = Path(temporary_directory)
        source = root / "source"
        destination = root / "slot/project"
        create_project(source)
        visual_capture_slots.sync_standard_project(source, destination)
        (destination / "Library").mkdir()
        (destination / "Library/imported.marker").write_text("warm\n")
        (destination / "target").mkdir()
        (destination / "target/incremental.marker").write_text("warm\n")
        (destination / "Temp").mkdir()
        (destination / "Temp/UnityLockfile").write_text("stale\n")
        (source / "Assets/Scenario.cs").write_text("changed\n")
        (source / "Assets/Deleted.cs").write_text("remove me\n")
        visual_capture_slots.sync_standard_project(source, destination)
        (source / "Assets/Deleted.cs").unlink()
        visual_capture_slots.sync_standard_project(source, destination)
        if (destination / "Assets/Scenario.cs").read_text() != "changed\n":
            fail("changed source was not synchronized")
        if (destination / "Assets/Deleted.cs").exists():
            fail("deleted source survived synchronization")
        if (destination / "Temp").exists():
            fail("stale Unity transient state survived synchronization")
        for marker in ("Library/imported.marker", "target/incremental.marker"):
            if not (destination / marker).is_file():
                fail(f"accelerator was removed during synchronization: {marker}")


def test_sample_overlay_synchronization() -> None:
    with tempfile.TemporaryDirectory(prefix="battlement-sample-sync.") as temporary_directory:
        root = Path(temporary_directory)
        sample = root / "sample"
        harness = root / "harness"
        materialized_repository = root / "slot/source"
        destination = materialized_repository / "samples/example"
        create_project(sample)
        (sample / "rules").mkdir()
        cargo_manifest = Path("rules/Cargo.toml")
        (sample / cargo_manifest).write_text(
            '[dependencies]\nbattlement = { path = "../../../crates/battlement" }\n'
        )
        for directory in (
            "Assets/VisualCapture",
            "Assets/Editor",
            "Packages/com.battlement.client",
            "crates/battlement/src",
        ):
            (harness / directory).mkdir(parents=True)
        (harness / "Assets/VisualCapture/Harness.cs").write_text("class Harness {}\n")
        (harness / "Packages/com.battlement.client/package.json").write_text("{}\n")
        (harness / "crates/battlement/src/lib.rs").write_text("pub struct Engine;\n")
        for suffix in ("", ".meta"):
            (harness / f"Assets/Editor/SampleVisualCaptureBuild.cs{suffix}").write_text(
                f"harness{suffix}\n"
            )
        for name in ("Cargo.toml", "Cargo.lock"):
            (harness / name).write_text(f"root {name}\n")
        (destination / "Assets/Editor").mkdir(parents=True)
        (destination / "Assets/Editor/DeletedHarness.cs").write_text("stale\n")
        visual_capture_slots.sync_sample_project(
            sample, harness, destination, materialized_repository
        )
        if (destination / "Assets/Editor/DeletedHarness.cs").exists():
            fail("deleted sample harness file survived synchronization")
        if not (destination / "Assets/VisualCapture/Harness.cs").is_file():
            fail("visual capture harness was not materialized")
        if not (materialized_repository / "crates/battlement/src/lib.rs").is_file():
            fail("sample Cargo dependency sources were not materialized")
        if (destination / cargo_manifest).read_text() != (sample / cargo_manifest).read_text():
            fail("sample Cargo manifest paths changed during topology-preserving sync")


def test_slot_reuse_seeding_and_compatibility() -> None:
    with tempfile.TemporaryDirectory(prefix="battlement-slot-pool.") as temporary_directory:
        cache = Path(temporary_directory)
        project = cache / "source"
        create_project(project)
        compatibility = visual_capture_slots.compatibility_manifest(
            project, "6000.5.8f1", "repository"
        )
        pool = visual_capture_slots.BuildSlotPool(cache / "cache", compatibility, count=2)
        first = pool.acquire()
        visual_capture_slots.sync_standard_project(project, first.project)
        (first.project / "Library").mkdir()
        (first.project / "Library/imported.marker").write_text("warm\n")
        pool.publish_seed(first)
        first.close()
        reused = pool.acquire()
        if reused.disposition != "reused":
            fail(f"existing slot was not reused: {reused.disposition}")
        seeded = pool.acquire()
        if not seeded.disposition.startswith("seeded"):
            fail(f"additional slot was not seeded: {seeded.disposition}")
        if not (seeded.project / "Library/imported.marker").is_file():
            fail("seeded slot lost imported Unity state")
        reused.close()
        seeded.close()
        changed = visual_capture_slots.compatibility_manifest(
            project, "6000.5.9f1", "repository"
        )
        invalidated = visual_capture_slots.BuildSlotPool(
            cache / "cache", changed, count=1
        ).acquire()
        if invalidated.path.parent == first.path.parent:
            fail("Unity version change did not invalidate slot compatibility")
        invalidated.close()


def write_fake_build_tools(directory: Path) -> tuple[list[str], list[str]]:
    """Create fake Cargo and Unity executables that expose incremental state."""
    cargo = directory / "fake-cargo.py"
    cargo.write_text(
        "#!/usr/bin/env python3\n"
        "from pathlib import Path\n"
        "import sys\n"
        "target = Path(sys.argv[sys.argv.index('--target-dir') + 1])\n"
        "(target / 'release').mkdir(parents=True, exist_ok=True)\n"
        "(target / 'incremental.marker').write_text('warm\\n')\n"
        "(target / 'release/libbattlement_rules.dylib').write_text('fake\\n')\n"
    )
    unity = directory / "fake-unity.py"
    unity.write_text(
        "#!/usr/bin/env python3\n"
        "from pathlib import Path\n"
        "import os, sys\n"
        "project = Path(sys.argv[sys.argv.index('-projectPath') + 1])\n"
        "library = project / 'Library'\n"
        "state = 'warm' if (library / 'imported.marker').is_file() else 'cold'\n"
        "library.mkdir(parents=True, exist_ok=True)\n"
        "(library / 'imported.marker').write_text('warm\\n')\n"
        "with (project / 'unity-states.log').open('a') as log: log.write(state + '\\n')\n"
        "raise SystemExit(7 if os.environ.get('FAKE_UNITY_FAIL') else 0)\n"
    )
    return [sys.executable, str(cargo)], [sys.executable, str(unity)]


def test_failed_build_recovery_with_fake_tools() -> None:
    with tempfile.TemporaryDirectory(prefix="battlement-slot-recovery.") as temporary_directory:
        root = Path(temporary_directory)
        source = root / "source"
        create_project(source)
        cargo, unity = write_fake_build_tools(root)
        compatibility = visual_capture_slots.compatibility_manifest(
            source, "6000.5.8f1", "repository"
        )
        pool = visual_capture_slots.BuildSlotPool(root / "cache", compatibility, count=1)
        first = pool.acquire()
        visual_capture_slots.sync_standard_project(source, first.project)
        subprocess.run(
            [*cargo, "build", "--target-dir", str(first.project / "target")],
            check=True,
        )
        failed_environment = os.environ.copy()
        failed_environment["FAKE_UNITY_FAIL"] = "1"
        failure = subprocess.run(
            [*unity, "-projectPath", str(first.project)], env=failed_environment
        )
        if failure.returncode != 7:
            fail("fake Unity failure was not observed")
        slot_path = first.path
        first.close()
        recovered = pool.acquire()
        if recovered.path != slot_path or recovered.disposition != "reused":
            fail("failed build slot was not safely reused")
        subprocess.run(
            [*cargo, "build", "--target-dir", str(recovered.project / "target")],
            check=True,
        )
        subprocess.run([*unity, "-projectPath", str(recovered.project)], check=True)
        states = (recovered.project / "unity-states.log").read_text().splitlines()
        if states != ["cold", "warm"]:
            fail(f"Unity did not observe retained Library state: {states}")
        if not (recovered.project / "target/incremental.marker").is_file():
            fail("Cargo target did not survive the failed build")
        recovered.close()


def test_cross_process_slot_locking() -> None:
    with tempfile.TemporaryDirectory(prefix="battlement-slot-lock.") as temporary_directory:
        root = Path(temporary_directory)
        project = root / "source"
        create_project(project)
        compatibility = visual_capture_slots.compatibility_manifest(
            project, "6000.5.8f1", "repository"
        )
        pool = visual_capture_slots.BuildSlotPool(root / "cache", compatibility, count=1)
        held = pool.acquire()
        script = (
            "from pathlib import Path; import json, sys; "
            "from visual_capture_slots import BuildSlotPool; "
            "pool=BuildSlotPool(Path(sys.argv[1]), json.loads(sys.argv[2]), count=1); "
            "slot=pool.acquire(); print(slot.path, flush=True); slot.close()"
        )
        environment = os.environ.copy()
        environment["PYTHONPATH"] = str(REPOSITORY_ROOT / "scripts")
        waiter = subprocess.Popen(
            [sys.executable, "-c", script, str(root / "cache"), json.dumps(compatibility)],
            stdout=subprocess.PIPE,
            text=True,
            env=environment,
        )
        if waiter.stdout is None:
            fail("slot waiter did not expose stdout")
        time.sleep(0.25)
        if waiter.poll() is not None:
            fail("concurrent process acquired an already leased writable slot")
        held.close()
        output = waiter.communicate(timeout=3)[0].strip()
        if waiter.returncode != 0 or output != str(held.path):
            fail("waiting process did not acquire the released slot")


def test_safe_cleanup_validation() -> None:
    with tempfile.TemporaryDirectory(prefix="battlement-safe-cleanup.") as temporary_directory:
        root = Path(temporary_directory)
        child = root / "players/exact-key"
        child.mkdir(parents=True)
        visual_capture_slots.remove_owned_path(child, root / "players")
        if child.exists():
            fail("validated exact cache child was not removed")
        try:
            visual_capture_slots.remove_owned_path(root, root)
        except ValueError:
            return
        fail("broad cache-root deletion was accepted")


def main() -> None:
    if not visual_capture_lib.is_nonnegative_number("0"):
        fail("zero should be accepted")
    if not visual_capture_lib.is_nonnegative_number("2.5"):
        fail("decimal should be accepted")
    if visual_capture_lib.is_nonnegative_number("-1"):
        fail("negative hold should be rejected")
    test_default_hold_timing()
    test_zero_hold_override()
    test_fingerprint_invalidation()
    test_sample_rust_fingerprint_invalidation()
    test_atomic_capture_protocol()
    test_player_log_failure_diagnostics()
    test_slot_limit()
    test_durable_project_synchronization()
    test_sample_overlay_synchronization()
    test_slot_reuse_seeding_and_compatibility()
    test_failed_build_recovery_with_fake_tools()
    test_cross_process_slot_locking()
    test_safe_cleanup_validation()
    print("Visual capture workflow tests passed.")


if __name__ == "__main__":
    main()
