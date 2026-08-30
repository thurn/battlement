#!/usr/bin/env python3

"""Machine-wide capacity leases shared by repository tooling."""

from __future__ import annotations

import os
from pathlib import Path
import time

from platform_support import try_lock_file, unlock_file, user_cache_path


GLOBAL_RESOURCE_ROOT = Path(
    os.environ.get(
        "BATTLEMENT_RESOURCE_SLOTS",
        user_cache_path("Battlement", "resource-slots"),
    )
)


class SlotLease:
    """Hold one cross-process slot until the lease is closed."""

    def __init__(self, directory: Path, name: str, count: int) -> None:
        self.directory = directory
        self.name = name
        self.count = count
        self.file = None

    def acquire(self) -> "SlotLease":
        """Wait for and exclusively lock one named slot."""
        self.directory.mkdir(parents=True, exist_ok=True)
        while self.file is None:
            for index in range(self.count):
                candidate = (self.directory / f"{self.name}-{index}.lock").open("a+")
                try:
                    if not try_lock_file(candidate):
                        candidate.close()
                        continue
                    self.file = candidate
                    break
                except OSError:
                    candidate.close()
            if self.file is None:
                time.sleep(0.1)
        return self

    def close(self) -> None:
        """Release the held slot."""
        if self.file is not None:
            unlock_file(self.file)
            self.file.close()
            self.file = None

    def __enter__(self) -> "SlotLease":
        return self.acquire()

    def __exit__(self, _type, _value, _traceback) -> None:
        self.close()


def unity_editor_lease() -> SlotLease:
    """Return one of two machine-wide Unity Editor capacity leases."""
    return SlotLease(GLOBAL_RESOURCE_ROOT, "unity-editor", 2)
