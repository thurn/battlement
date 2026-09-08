"""Read operating-system process birth identities without trusting a PID alone."""

from __future__ import annotations

import ctypes
from ctypes import wintypes
import os
from pathlib import Path
import platform


def identity(pid: int | None = None) -> dict:
    """Return a verifiable birth identity, or explicitly unknown when unavailable."""
    pid = os.getpid() if pid is None else pid
    birth = None
    try:
        if platform.system() == "Darwin":
            birth = _mac_birth(pid)
        elif platform.system() == "Linux":
            fields = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
            boot = Path('/proc/sys/kernel/random/boot_id').read_text().strip()
            birth = f"linux:{boot}:{fields[19]}"
        elif platform.system() == "Windows":
            birth = _windows_birth(pid)
    except (OSError, ValueError, IndexError):
        pass
    return {"pid": pid, "birth": birth, "host": platform.node()}


def matches(expected: dict) -> bool:
    """Refuse unknown or recycled process identities."""
    return bool(expected.get('birth')) and identity(expected['pid']) == expected


def _mac_birth(pid: int) -> str | None:
    # sys/proc_info.h: proc_bsdinfo, PROC_PIDTBSDINFO.
    class BsdInfo(ctypes.Structure):
        _fields_ = [('prefix', ctypes.c_uint32 * 12), ('names', ctypes.c_char * 48),
                    ('details', ctypes.c_uint32 * 6), ('seconds', ctypes.c_uint64),
                    ('microseconds', ctypes.c_uint64)]
    library = ctypes.CDLL('/usr/lib/libproc.dylib', use_errno=True)
    library.proc_pidinfo.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.c_uint64,
                                    ctypes.c_void_p, ctypes.c_int]
    library.proc_pidinfo.restype = ctypes.c_int
    info = BsdInfo()
    size = library.proc_pidinfo(pid, 3, 0, ctypes.byref(info), ctypes.sizeof(info))
    if size != ctypes.sizeof(info):
        return None
    return f"darwin:{info.seconds}:{info.microseconds}"


def _windows_birth(pid: int) -> str | None:
    kernel = ctypes.WinDLL('kernel32', use_last_error=True)
    kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    kernel.OpenProcess.restype = wintypes.HANDLE
    kernel.CloseHandle.argtypes = [wintypes.HANDLE]
    kernel.GetProcessTimes.argtypes = [wintypes.HANDLE, *([ctypes.POINTER(wintypes.FILETIME)] * 4)]
    handle = kernel.OpenProcess(0x1000, False, pid)
    if not handle:
        return None
    try:
        times = [wintypes.FILETIME() for _ in range(4)]
        if not kernel.GetProcessTimes(handle, *(ctypes.byref(value) for value in times)):
            return None
        return f"windows:{times[0].dwHighDateTime}:{times[0].dwLowDateTime}"
    finally:
        kernel.CloseHandle(handle)
