"""Resident memory of processes, on POSIX and Windows alike.

POSIX reads it from ``ps`` and ``resource``; Windows from the process memory
counters (``K32GetProcessMemoryInfo``), whose working set is its resident
memory. Child scripts started by ``conftest.WatchedProcess`` can import this
module too.
"""

from __future__ import annotations

import subprocess
import sys

BYTES_PER_MB = 2**20

if sys.platform == "win32":
    import ctypes
    from ctypes import wintypes

    class _MemoryCounters(ctypes.Structure):
        """``PROCESS_MEMORY_COUNTERS`` from ``psapi.h``."""

        _fields_ = [
            ("cb", wintypes.DWORD),
            ("PageFaultCount", wintypes.DWORD),
            ("PeakWorkingSetSize", ctypes.c_size_t),
            ("WorkingSetSize", ctypes.c_size_t),
            ("QuotaPeakPagedPoolUsage", ctypes.c_size_t),
            ("QuotaPagedPoolUsage", ctypes.c_size_t),
            ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
            ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
            ("PagefileUsage", ctypes.c_size_t),
            ("PeakPagefileUsage", ctypes.c_size_t),
        ]

    _PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
    _PROCESS_VM_READ = 0x0010
    _kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    _kernel32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    _kernel32.OpenProcess.restype = wintypes.HANDLE
    _kernel32.CloseHandle.argtypes = [wintypes.HANDLE]
    _kernel32.CloseHandle.restype = wintypes.BOOL
    _kernel32.GetCurrentProcess.argtypes = []
    _kernel32.GetCurrentProcess.restype = wintypes.HANDLE
    _kernel32.K32GetProcessMemoryInfo.argtypes = [
        wintypes.HANDLE,
        ctypes.POINTER(_MemoryCounters),
        wintypes.DWORD,
    ]
    _kernel32.K32GetProcessMemoryInfo.restype = wintypes.BOOL

    def _counters(handle: int) -> _MemoryCounters | None:
        counters = _MemoryCounters()
        counters.cb = ctypes.sizeof(counters)
        if not _kernel32.K32GetProcessMemoryInfo(
            handle, ctypes.byref(counters), counters.cb
        ):
            return None
        return counters

    def resident_mb(pid: int) -> float:
        """Resident memory of process ``pid`` in MB (0 once it has exited)."""
        handle = _kernel32.OpenProcess(
            _PROCESS_QUERY_LIMITED_INFORMATION | _PROCESS_VM_READ, False, pid
        )
        if not handle:
            return 0.0
        try:
            counters = _counters(handle)
        finally:
            _kernel32.CloseHandle(handle)
        return counters.WorkingSetSize / BYTES_PER_MB if counters else 0.0

    def peak_resident_mb() -> float:
        """Peak resident memory of this process in MB."""
        counters = _counters(_kernel32.GetCurrentProcess())
        if counters is None:
            raise ctypes.WinError(ctypes.get_last_error())
        return counters.PeakWorkingSetSize / BYTES_PER_MB

else:
    import resource

    def resident_mb(pid: int) -> float:
        """Resident memory of process ``pid`` in MB (0 once it has exited)."""
        result = subprocess.run(
            ["ps", "-o", "rss=", "-p", str(pid)],
            capture_output=True,
            text=True,
            check=False,
        )
        return int(result.stdout.strip() or 0) / 1024

    def peak_resident_mb() -> float:
        """Peak resident memory of this process in MB."""
        peak = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
        # Bytes on macOS, kilobytes elsewhere.
        return peak / BYTES_PER_MB if sys.platform == "darwin" else peak / 1024
