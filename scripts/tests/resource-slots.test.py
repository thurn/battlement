#!/usr/bin/env python3

"""Exercise shared capacity leases without launching Unity."""

from __future__ import annotations

from pathlib import Path
import sys
import tempfile
import threading
import time


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY_ROOT / "scripts"))

from resource_slots import LeaseGroup, SlotLease  # noqa: E402


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="battlement-resource-slots.") as temporary:
        locks = Path(temporary)
        first = SlotLease(locks, "unity-editor", 2).acquire()
        second = SlotLease(locks, "unity-editor", 2).acquire()
        acquired = threading.Event()

        def acquire_third() -> None:
            with SlotLease(locks, "unity-editor", 2):
                acquired.set()

        waiter = threading.Thread(target=acquire_third)
        waiter.start()
        time.sleep(0.2)
        assert not acquired.is_set(), "a third consumer exceeded the two-slot limit"
        first.close()
        waiter.join(timeout=2)
        second.close()
        assert acquired.is_set(), "a released slot did not admit the waiting consumer"

        occupied = SlotLease(locks, "atomic-heavy", 6, 4).acquire()
        blocked = SlotLease(locks, "atomic-heavy", 6, 3)
        acquired = threading.Event()

        def acquire_atomic_capacity() -> None:
            with blocked:
                acquired.set()

        waiter = threading.Thread(target=acquire_atomic_capacity)
        waiter.start()
        time.sleep(0.2)
        assert not blocked.files, "a queued multi-unit request retained partial capacity"
        occupied.close()
        waiter.join(timeout=2)
        assert acquired.is_set(), "an atomic request was not admitted after capacity released"

        compiler = SlotLease(locks, "machine-heavy", 6, 3).acquire()
        player = SlotLease(locks, "machine-heavy", 6, 2).acquire()
        browser = SlotLease(locks, "machine-heavy", 6).acquire()
        admitted = threading.Event()

        def acquire_more_capacity() -> None:
            with SlotLease(locks, "machine-heavy", 6):
                admitted.set()

        waiter = threading.Thread(target=acquire_more_capacity)
        waiter.start()
        time.sleep(0.2)
        assert not admitted.is_set(), "heavy children exceeded the machine budget"
        browser.close()
        waiter.join(timeout=2)
        player.close()
        compiler.close()
        assert admitted.is_set(), "released machine capacity did not admit queued work"

        first = SlotLease(locks, "machine-heavy", 6, 3)
        second = SlotLease(locks, "unity-editor", 2)
        with LeaseGroup(first, second):
            assert len(first.files) == 3
            assert len(second.files) == 1
        assert not first.files and not second.files

    print("Resource slot tests passed.")


if __name__ == "__main__":
    main()
