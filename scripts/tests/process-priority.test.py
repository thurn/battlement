#!/usr/bin/env python3
"""Observe child priority, inheritance, nesting, streams and exit status."""

import os
from pathlib import Path
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import process_priority


def main() -> None:
    if os.name == "nt":
        probe = """import ctypes
kernel = ctypes.windll.kernel32
kernel.GetCurrentProcess.restype = ctypes.c_void_p
kernel.GetPriorityClass.argtypes = [ctypes.c_void_p]
print(kernel.GetPriorityClass(kernel.GetCurrentProcess()))
"""
        expected = 0x4000
    else:
        probe = "import os; print(os.getpriority(os.PRIO_PROCESS, 0))"
        expected = max(10, os.getpriority(os.PRIO_PROCESS, 0))
    parent_before = subprocess.check_output([sys.executable, "-c", probe])
    child = process_priority.run([sys.executable, "-c", probe], capture_output=True, text=True, check=True)
    assert int(child.stdout) == expected, child.stdout
    nested = f"""import subprocess, sys
sys.path.insert(0, {str(Path(__file__).resolve().parents[1])!r})
import process_priority
subprocess.run([sys.executable, '-c', {probe!r}], check=True)
process_priority.run([sys.executable, '-c', {probe!r}], check=True)
"""
    output = process_priority.run([sys.executable, "-c", nested], capture_output=True, text=True, check=True)
    assert [int(line) for line in output.stdout.splitlines()] == [expected, expected], output.stdout
    assert subprocess.check_output([sys.executable, "-c", probe]) == parent_before
    failed = process_priority.run(
        [sys.executable, "-c", "import sys; print(input()); print('failure', file=sys.stderr); sys.exit(7)"],
        input="payload\n", capture_output=True, text=True,
    )
    assert (failed.returncode, failed.stdout, failed.stderr) == (7, "payload\n", "failure\n")
    print(f"Priority tests passed: children={expected}, parent unchanged.")


if __name__ == "__main__":
    main()
