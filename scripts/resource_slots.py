#!/usr/bin/env python3

"""Machine-wide capacity leases shared by repository tooling."""

from __future__ import annotations

import os
from pathlib import Path
import time

import operation_log

from platform_support import try_lock_file, unlock_file


GLOBAL_RESOURCE_ROOT = Path(
    os.environ.get(
        "BATTLEMENT_RESOURCE_SLOTS",
        (
            Path(os.environ.get("PROGRAMDATA", "C:/ProgramData"))
            / "Battlement/resource-slots"
            if os.name == "nt"
            else Path("/tmp/Battlement/resource-slots")
        ),
    )
)
MACHINE_CAPACITY = 6


class SlotLease:
    """Hold one cross-process slot until the lease is closed."""

    def __init__(self, directory: Path, name: str, count: int, units: int = 1) -> None:
        if units < 1 or units > count:
            raise ValueError("slot lease units must be between one and the capacity")
        self.directory = directory
        self.name = name
        self.count = count
        self.units = units
        self.files = []
        self.operation = operation_log.current()
        self.acquired_ns = None

    def acquire(self) -> "SlotLease":
        """Wait for and exclusively lock one named slot."""
        self.directory.mkdir(parents=True, exist_ok=True)
        started = time.monotonic_ns()
        self._event("resource.queued")
        while not self._try_acquire():
            time.sleep(0.1)
        self.acquired_ns = time.monotonic_ns()
        self._event(
            "resource.acquired",
            slots=[Path(file.name).stem.rsplit("-", 1)[1] for file in self.files],
            units=self.units,
            queue_duration_ms=round((self.acquired_ns - started) / 1_000_000),
        )
        return self

    def _try_acquire(self) -> bool:
        """Acquire every requested unit atomically or release the partial set."""
        acquired = []
        try:
            for index in range(self.count):
                candidate = (self.directory / f"{self.name}-{index}.lock").open("a+")
                try:
                    if not try_lock_file(candidate):
                        candidate.close()
                        continue
                    acquired.append(candidate)
                    if len(acquired) == self.units:
                        self.files = acquired
                        return True
                except OSError:
                    candidate.close()
        finally:
            if len(acquired) < self.units:
                for file in reversed(acquired):
                    unlock_file(file)
                    file.close()
        return False

    def close(self) -> None:
        """Release the held slot."""
        if self.files:
            for file in reversed(self.files):
                unlock_file(file)
                file.close()
            self.files.clear()
            self._event(
                "resource.released",
                units=self.units,
                held_duration_ms=round((time.monotonic_ns() - self.acquired_ns) / 1_000_000),
            )

    def _event(self, event: str, **attributes) -> None:
        if self.operation:
            self.operation.event(event, resource=self.name, capacity=self.count,
                                 owner=self.operation.process, **attributes)

    def __enter__(self) -> "SlotLease":
        return self.acquire()

    def __exit__(self, _type, _value, _traceback) -> None:
        self.close()


class LeaseGroup:
    """Acquire related leases in a stable order and release them together."""

    def __init__(self, *leases: SlotLease) -> None:
        self.leases = leases
        self.acquired = []

    def acquire(self) -> "LeaseGroup":
        try:
            for lease in self.leases:
                self.acquired.append(lease.acquire())
        except BaseException:
            self.close()
            raise
        return self

    def close(self) -> None:
        for lease in reversed(self.acquired):
            lease.close()
        self.acquired.clear()

    def __enter__(self) -> "LeaseGroup":
        return self.acquire()

    def __exit__(self, _type, _value, _traceback) -> None:
        self.close()


def compiler_capacity_lease() -> SlotLease:
    """Reserve the machine capacity used by one three-job Cargo writer."""
    return SlotLease(GLOBAL_RESOURCE_ROOT, "machine-heavy", MACHINE_CAPACITY, 3)


def browser_capacity_lease() -> LeaseGroup:
    """Reserve one bounded browser session and one machine unit."""
    return LeaseGroup(
        SlotLease(GLOBAL_RESOURCE_ROOT, "machine-heavy", MACHINE_CAPACITY),
        SlotLease(GLOBAL_RESOURCE_ROOT, "browser", 2),
    )


def native_player_capacity_lease() -> LeaseGroup:
    """Reserve one bounded native player session and two machine units."""
    return LeaseGroup(
        SlotLease(GLOBAL_RESOURCE_ROOT, "machine-heavy", MACHINE_CAPACITY, 2),
        SlotLease(GLOBAL_RESOURCE_ROOT, "native-player", 3),
    )


def unity_editor_lease() -> LeaseGroup:
    """Reserve one editor and its share of the machine-wide heavy-work budget."""
    return LeaseGroup(
        SlotLease(GLOBAL_RESOURCE_ROOT, "machine-heavy", MACHINE_CAPACITY, 3),
        SlotLease(GLOBAL_RESOURCE_ROOT, "unity-editor", 2),
    )
