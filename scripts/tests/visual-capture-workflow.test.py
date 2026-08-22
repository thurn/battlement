#!/usr/bin/env python3

"""Exercise visual-capture helpers without launching Unity."""

from __future__ import annotations

import json
from pathlib import Path
import sys
import tempfile
import threading
import time


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY_ROOT / "scripts"))

import visual_capture_lib  # noqa: E402


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
    test_atomic_capture_protocol()
    test_slot_limit()
    print("Visual capture workflow tests passed.")


if __name__ == "__main__":
    main()
