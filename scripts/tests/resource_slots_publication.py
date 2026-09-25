"""Force a queue observer to race a creator before its ticket is locked."""

from pathlib import Path
import tempfile
import threading

import platform_support
import resource_slots


def verify_publication() -> None:
    ready = threading.Event()
    observer_entered = threading.Event()
    publish = threading.Event()
    created = threading.Event()
    release_probe = threading.Event()
    finish = threading.Event()
    failures = []
    observations = []
    original_try = resource_slots.try_lock_file
    original_lock = getattr(resource_slots, "lock_file", None)

    def before_creator_lock(file):
        if threading.current_thread().name == "creator" and file.mode == "x+":
            ready.set()
            if not publish.wait(5):
                raise TimeoutError("creator was not released")

    def try_lock(file):
        before_creator_lock(file)
        locked = original_try(file)
        if threading.current_thread().name == "observer" and file.mode == "r+" and locked:
            observer_entered.set()
            if not release_probe.wait(5):
                raise TimeoutError("observer probe was not released")
        return locked

    def lock(file):
        before_creator_lock(file)
        if threading.current_thread().name == "observer":
            observer_entered.set()
        platform_support.lock_file(file)

    with tempfile.TemporaryDirectory(prefix="resource-ticket-publication.") as temporary:
        directory = Path(temporary)

        def creator():
            ticket = None
            try:
                ticket = resource_slots.AdmissionTicket(directory, "publication", 1)
            except BaseException as error:
                failures.append(error)
            finally:
                created.set()
            try:
                if not finish.wait(5):
                    failures.append(TimeoutError("creator cleanup was not released"))
            finally:
                if ticket is not None:
                    ticket.close()

        def observer():
            ticket = None
            try:
                ticket = resource_slots.AdmissionTicket(directory, "publication", 1)
                observations.append(ticket.is_first())
            except BaseException as error:
                failures.append(error)
            finally:
                if ticket is not None:
                    ticket.close()

        resource_slots.try_lock_file = try_lock
        resource_slots.lock_file = lock
        writer = threading.Thread(target=creator, name="creator", daemon=True)
        reader = threading.Thread(target=observer, name="observer", daemon=True)
        try:
            writer.start()
            assert ready.wait(5), "creator did not reach its unpublished ticket"
            reader.start()
            assert observer_entered.wait(5), "observer did not reach queue admission"
            publish.set()
            assert created.wait(5), "ticket publication did not finish"
            release_probe.set()
            reader.join(timeout=5)
            assert not reader.is_alive(), "observer remained blocked after publication"
            assert not failures, f"queue inspection interfered with publication: {failures}"
            assert observations == [False], "observer bypassed the older published ticket"
        finally:
            publish.set()
            release_probe.set()
            finish.set()
            writer.join(timeout=5)
            if reader.ident is not None:
                reader.join(timeout=5)
            resource_slots.try_lock_file = original_try
            if original_lock is None:
                del resource_slots.lock_file
            else:
                resource_slots.lock_file = original_lock


if __name__ == "__main__":
    verify_publication()
