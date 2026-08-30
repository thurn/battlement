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

from resource_slots import SlotLease  # noqa: E402


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

    print("Resource slot tests passed.")


if __name__ == "__main__":
    main()
