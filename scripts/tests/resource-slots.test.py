#!/usr/bin/env python3

"""Exercise shared capacity leases without launching Unity."""

from __future__ import annotations

import multiprocessing
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import time


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY_ROOT / "scripts"))

import resource_slots  # noqa: E402
from resource_slots import LeaseGroup, SlotLease  # noqa: E402


def nested_compiler_process(locks: str, ready, start) -> None:
    """Hold outer compiler capacity while a descendant reuses its admission."""
    resource_slots.GLOBAL_RESOURCE_ROOT = Path(locks)
    with resource_slots.compiler_capacity_lease():
        ready.put(os.getpid())
        if not start.wait(2):
            raise TimeoutError("nested compiler fixture was not started")
        source = """
from resource_slots import compiler_capacity_lease
with compiler_capacity_lease():
    pass
"""
        environment = resource_slots.capacity_environment()
        environment["PYTHONPATH"] = os.pathsep.join(
            value for value in (
                str(REPOSITORY_ROOT / "scripts"), environment.get("PYTHONPATH")
            ) if value
        )
        subprocess.run(
            [sys.executable, "-c", source],
            check=True,
            env=environment,
            timeout=2,
        )


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

        occupied = SlotLease(locks, "weighted-heavy", 6, 4).acquire()
        order = []
        head = threading.Thread(
            target=lambda: _record_lease(locks, "weighted-heavy", 6, 3, "head", order)
        )
        follower = threading.Thread(
            target=lambda: _record_lease(locks, "weighted-heavy", 6, 1, "follower", order)
        )
        head.start()
        _wait_for_tickets(locks, "weighted-heavy", 1)
        follower.start()
        _wait_for_tickets(locks, "weighted-heavy", 2)
        time.sleep(0.2)
        assert order == [], "a smaller follower bypassed the weighted FIFO head"
        occupied.close()
        head.join(timeout=2)
        follower.join(timeout=2)
        assert order == ["head", "follower"], f"weighted requests were not FIFO: {order}"

        for attempt in range(20):
            held = SlotLease(locks, "fair-heavy", 1).acquire()
            order = []

            def acquire_in_order(label: str) -> None:
                with SlotLease(locks, "fair-heavy", 1):
                    order.append(label)
                    time.sleep(0.005)

            first_waiter = threading.Thread(target=acquire_in_order, args=("first",))
            second_waiter = threading.Thread(target=acquire_in_order, args=("second",))
            first_waiter.start()
            while len(list(locks.glob(".fair-heavy.queue.*.lock"))) < 1:
                time.sleep(0.001)
            second_waiter.start()
            while len(list(locks.glob(".fair-heavy.queue.*.lock"))) < 2:
                time.sleep(0.001)
            held.close()
            first_waiter.join(timeout=5)
            second_waiter.join(timeout=5)
            assert not first_waiter.is_alive() and not second_waiter.is_alive(), (
                f"queued leases did not finish on attempt {attempt}"
            )
            assert order == ["first", "second"], (
                f"queued leases were not FIFO on attempt {attempt}: {order}"
            )

        stale = locks / ".stale-heavy.queue.00000000000000000000.0000000000.0000000000.lock"
        stale.touch()
        old = time.time() - 10
        os.utime(stale, (old, old))
        with SlotLease(locks, "stale-heavy", 1):
            assert not stale.exists(), "a stale admission ticket blocked live work"

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

        browser = SlotLease(locks, "browser", 1).acquire()
        group_acquired = threading.Event()

        def acquire_blocked_group() -> None:
            with LeaseGroup(
                SlotLease(locks, "machine-heavy", 6, 3),
                SlotLease(locks, "browser", 1),
            ):
                group_acquired.set()

        group = threading.Thread(target=acquire_blocked_group)
        group.start()
        _wait_for_tickets(locks, "machine-heavy", 1)
        assert not group_acquired.is_set(), "compound lease ignored its secondary capacity"
        machine_probe = SlotLease(locks, "machine-heavy", 6, 6)
        assert machine_probe._try_acquire(), "compound wait retained partial machine capacity"
        machine_probe._release_files()
        browser.close()
        group.join(timeout=2)
        assert group_acquired.is_set(), "compound lease did not resume after secondary release"

        context = multiprocessing.get_context("spawn")
        ready = context.Queue()
        start = context.Event()
        nested = [
            context.Process(target=nested_compiler_process, args=(str(locks), ready, start))
            for _ in range(2)
        ]
        for process in nested:
            process.start()
        assert len({ready.get(timeout=2), ready.get(timeout=2)}) == 2
        start.set()
        for process in nested:
            process.join(timeout=5)
        assert not any(process.is_alive() for process in nested), (
            "nested compiler admissions deadlocked with two outer three-unit leases"
        )
        assert [process.exitcode for process in nested] == [0, 0]

    print("Resource slot tests passed.")


def _record_lease(
    locks: Path, name: str, count: int, units: int, label: str, order: list[str]
) -> None:
    with SlotLease(locks, name, count, units):
        order.append(label)
        time.sleep(0.05)


def _wait_for_tickets(locks: Path, name: str, expected: int) -> None:
    deadline = time.monotonic() + 2
    while time.monotonic() < deadline:
        if len(list(locks.glob(f".{name}.queue.*.lock"))) == expected:
            return
        time.sleep(0.01)
    raise TimeoutError(f"expected {expected} {name} admission tickets")


if __name__ == "__main__":
    main()
