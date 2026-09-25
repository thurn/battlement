"""CPU and peak resident bytes for reaped processes, without background sampling."""

import os
import sys
import threading

try:
    import resource
except ImportError:
    resource = None


FIELDS = ("cpu_user_ms", "cpu_system_ms", "max_rss", "cpu_scope")


def unknown() -> dict:
    return dict.fromkeys(FIELDS)


def _rss(usage) -> int:
    return int(usage.ru_maxrss) * (1 if sys.platform == "darwin" else 1024)


def snapshot():
    """Sample process-wide waited-child counters only on the serial controller."""
    if resource is None or threading.current_thread() is not threading.main_thread():
        return None
    return resource.getrusage(resource.RUSAGE_CHILDREN)


def elapsed(started) -> dict:
    """Include descendants reaped in this interval, not still-running children.

    CPU is inclusive of nested spans. A cumulative RSS high-water mark identifies
    this interval's peak only when it exceeds the earlier high-water mark.
    """
    finished = snapshot() if started is not None else None
    if finished is None:
        return unknown()
    return {
        "cpu_user_ms": round((finished.ru_utime - started.ru_utime) * 1000, 3),
        "cpu_system_ms": round((finished.ru_stime - started.ru_stime) * 1000, 3),
        "max_rss": _rss(finished) if finished.ru_maxrss > started.ru_maxrss else None,
        "cpu_scope": "waited_children",
    }


def wait(child) -> dict:
    """Reap an exclusively owned Popen child, retaining its exact resource usage."""
    if not hasattr(os, "wait4"):
        child.wait()
        return unknown()
    _, status, usage = os.wait4(child.pid, 0)
    child.returncode = os.waitstatus_to_exitcode(status)
    return {
        "cpu_user_ms": round(usage.ru_utime * 1000, 3),
        "cpu_system_ms": round(usage.ru_stime * 1000, 3),
        "max_rss": _rss(usage),
        "cpu_scope": "process_and_waited_descendants",
    }
