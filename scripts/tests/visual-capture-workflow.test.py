#!/usr/bin/env python3

"""Exercise visual-capture helpers without launching Unity."""

from __future__ import annotations

from pathlib import Path
import sys
import tempfile


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
    with tempfile.TemporaryDirectory(prefix="masonry-capture-test.") as temporary_directory:
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
    print("Visual capture workflow tests passed.")


if __name__ == "__main__":
    main()
