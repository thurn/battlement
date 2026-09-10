#!/usr/bin/env python3

"""Machine-wide capacity leases shared by repository tooling."""

from __future__ import annotations

from contextvars import ContextVar
import itertools
import os
from pathlib import Path
import threading
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
STALE_TICKET_SECONDS = 5
WAIT_DIAGNOSTIC_SECONDS = 30
INHERITED_COMPILER_CAPACITY = "BATTLEMENT_INHERITED_COMPILER_CAPACITY"
INHERITED_COMPILER_CAPACITY_ROOT = "BATTLEMENT_INHERITED_COMPILER_CAPACITY_ROOT"
_TICKET_SEQUENCE = itertools.count()
_ACTIVE_TICKETS = set()
_ACTIVE_TICKETS_LOCK = threading.Lock()
_COMPILER_CAPACITY: ContextVar[tuple[str, int] | None] = ContextVar(
    "compiler_capacity", default=None
)


def _inherited_compiler_capacity() -> tuple[str, int] | None:
    active = _COMPILER_CAPACITY.get()
    if active is not None:
        return active
    try:
        inherited = int(os.environ.get(INHERITED_COMPILER_CAPACITY, "0"))
    except ValueError:
        inherited = 0
    root = os.environ.get(INHERITED_COMPILER_CAPACITY_ROOT)
    return (root, inherited) if root and inherited > 0 else None


class AdmissionTicket:
    """One FIFO position for a named cross-process resource."""

    def __init__(self, directory: Path, name: str, units: int) -> None:
        self.directory = directory
        self.prefix = f".{name}.queue."
        self.path = directory / (
            f"{self.prefix}{time.time_ns():020d}.{os.getpid():010d}."
            f"{next(_TICKET_SEQUENCE):010d}.lock"
        )
        self.file = self.path.open("x+")
        self.file.write(f"units={units}\n")
        self.file.flush()
        if not try_lock_file(self.file):
            self.file.close()
            self.path.unlink(missing_ok=True)
            raise RuntimeError("new admission ticket could not be locked")
        with _ACTIVE_TICKETS_LOCK:
            _ACTIVE_TICKETS.add(self.path)

    def is_first(self) -> bool:
        """Return whether no older live ticket precedes this one."""
        for path in sorted(self.directory.glob(f"{self.prefix}*.lock")):
            if path == self.path:
                return True
            with _ACTIVE_TICKETS_LOCK:
                if path in _ACTIVE_TICKETS:
                    return False
            try:
                candidate = path.open("r+")
            except FileNotFoundError:
                continue
            try:
                if try_lock_file(candidate):
                    try:
                        age = time.time() - path.stat().st_mtime
                    except FileNotFoundError:
                        unlock_file(candidate)
                        continue
                    unlock_file(candidate)
                    if age >= STALE_TICKET_SECONDS:
                        candidate.close()
                        path.unlink(missing_ok=True)
                        continue
                return False
            finally:
                if not candidate.closed:
                    candidate.close()
        raise RuntimeError("admission ticket disappeared while waiting")

    def close(self) -> None:
        """Remove this queue position without affecting another waiter."""
        if not self.file.closed:
            with _ACTIVE_TICKETS_LOCK:
                _ACTIVE_TICKETS.discard(self.path)
            unlock_file(self.file)
            self.file.close()
            self.path.unlink(missing_ok=True)

    def status(self) -> tuple[int, int, list[str]]:
        """Return this ticket's queue position, queue depth, and older owners."""
        tickets = sorted(self.directory.glob(f"{self.prefix}*.lock"))
        try:
            position = tickets.index(self.path) + 1
        except ValueError:
            position = 0
        owners = [path.name.split(".")[-3].lstrip("0") or "0" for path in tickets[:4]]
        return position, len(tickets), owners


class SlotLease:
    """Hold one cross-process slot until the lease is closed."""

    def __init__(
        self, directory: Path, name: str, count: int, units: int = 1,
        *, inherit_compiler_capacity: bool = False,
    ) -> None:
        if units < 1 or units > count:
            raise ValueError("slot lease units must be between one and the capacity")
        self.directory = directory
        self.name = name
        self.count = count
        self.units = units
        self.files = []
        self.operation = operation_log.current()
        self.acquired_ns = None
        self.inherit_compiler_capacity = inherit_compiler_capacity
        self.inherited = False
        self._capacity_token = None

    def acquire(self) -> "SlotLease":
        """Wait for and exclusively lock one named slot."""
        self.directory.mkdir(parents=True, exist_ok=True)
        started = time.monotonic_ns()
        self._event("resource.queued")
        inherited = _inherited_compiler_capacity()
        if (
            self.inherit_compiler_capacity
            and inherited is not None
            and inherited[0] == str(self.directory.resolve())
            and inherited[1] >= self.units
        ):
            self.inherited = True
            self.acquired_ns = started
            self._event("resource.acquired", units=self.units, inherited=True, queue_duration_ms=0)
            return self
        ticket = AdmissionTicket(self.directory, self.name, self.units)
        announced = False
        next_diagnostic = 1_000_000_000
        try:
            while not ticket.is_first() or not self._try_acquire():
                waited = time.monotonic_ns() - started
                if waited >= next_diagnostic:
                    position, depth, owners = ticket.status()
                    held = self._held_slots()
                    print(
                        f"    Resource capacity: waiting for {self.name} "
                        f"({self.units} of {self.count} units; {held} held; "
                        f"queue {position}/{depth}; older pids {','.join(owners) or 'none'})",
                        flush=True,
                    )
                    self._event(
                        "resource.waiting", units=self.units, held_units=held,
                        queue_position=position, queue_depth=depth, older_pids=owners,
                    )
                    announced = True
                    next_diagnostic += WAIT_DIAGNOSTIC_SECONDS * 1_000_000_000
                time.sleep(0.1)
        finally:
            ticket.close()
        self.acquired_ns = time.monotonic_ns()
        if self.inherit_compiler_capacity:
            self._capacity_token = _COMPILER_CAPACITY.set(
                (str(self.directory.resolve()), self.units)
            )
        self._event(
            "resource.acquired",
            slots=[Path(file.name).stem.rsplit("-", 1)[1] for file in self.files],
            units=self.units,
            queue_duration_ms=round((self.acquired_ns - started) / 1_000_000),
        )
        if announced:
            waited_ms = round((self.acquired_ns - started) / 1_000_000)
            print(
                f"    Resource capacity: acquired {self.name} after "
                f"{waited_ms / 1000:.1f}s",
                flush=True,
            )
        return self

    def _held_slots(self) -> int:
        held = 0
        for index in range(self.count):
            with (self.directory / f"{self.name}-{index}.lock").open("a+") as candidate:
                if not try_lock_file(candidate):
                    held += 1
                    continue
                unlock_file(candidate)
        return held

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
        if self.inherited:
            self.inherited = False
            self._event("resource.released", units=self.units, inherited=True, held_duration_ms=0)
            return
        if self.files:
            self._release_files()
            self._event(
                "resource.released",
                units=self.units,
                held_duration_ms=round((time.monotonic_ns() - self.acquired_ns) / 1_000_000),
            )
        if self._capacity_token is not None:
            _COMPILER_CAPACITY.reset(self._capacity_token)
            self._capacity_token = None

    def _release_files(self) -> None:
        for file in reversed(self.files):
            unlock_file(file)
            file.close()
        self.files.clear()

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
        if not self.leases:
            return self
        first = self.leases[0]
        first.directory.mkdir(parents=True, exist_ok=True)
        ticket = AdmissionTicket(first.directory, first.name, first.units)
        started = time.monotonic_ns()
        next_diagnostic = 1_000_000_000
        try:
            while True:
                if ticket.is_first() and self._try_acquire():
                    acquired = time.monotonic_ns()
                    for lease in self.acquired:
                        lease.acquired_ns = acquired
                        lease._event(
                            "resource.acquired",
                            slots=[
                                Path(file.name).stem.rsplit("-", 1)[1]
                                for file in lease.files
                            ],
                            units=lease.units,
                            queue_duration_ms=round((acquired - started) / 1_000_000),
                        )
                    return self
                self._release_partial()
                waited = time.monotonic_ns() - started
                if waited >= next_diagnostic:
                    position, depth, owners = ticket.status()
                    print(
                        f"    Resource capacity: waiting for compound {first.name} "
                        f"lease (queue {position}/{depth}; "
                        f"older pids {','.join(owners) or 'none'})",
                        flush=True,
                    )
                    next_diagnostic += WAIT_DIAGNOSTIC_SECONDS * 1_000_000_000
                time.sleep(0.1)
        except BaseException:
            self.close()
            raise
        finally:
            ticket.close()

    def _try_acquire(self) -> bool:
        for lease in self.leases:
            if not lease._try_acquire():
                return False
            self.acquired.append(lease)
        return True

    def _release_partial(self) -> None:
        for lease in reversed(self.acquired):
            lease._release_files()
        self.acquired.clear()

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
    return SlotLease(
        GLOBAL_RESOURCE_ROOT, "machine-heavy", MACHINE_CAPACITY, 3,
        inherit_compiler_capacity=True,
    )


def capacity_environment(environment: dict[str, str] | None = None) -> dict[str, str]:
    """Propagate a live compiler admission to one descendant process tree."""
    result = dict(os.environ if environment is None else environment)
    if inherited := _inherited_compiler_capacity():
        root, units = inherited
        result[INHERITED_COMPILER_CAPACITY] = str(units)
        result[INHERITED_COMPILER_CAPACITY_ROOT] = root
    else:
        result.pop(INHERITED_COMPILER_CAPACITY, None)
        result.pop(INHERITED_COMPILER_CAPACITY_ROOT, None)
    return result


def compiler_maintenance_lease() -> SlotLease:
    """Pause compiler writers while shared target directories are pruned."""
    return SlotLease(
        GLOBAL_RESOURCE_ROOT,
        "machine-heavy",
        MACHINE_CAPACITY,
        MACHINE_CAPACITY,
    )


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
