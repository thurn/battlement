"""Conservative reuse boundary for isolated repository-tool fixtures."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys

from ci_cache import CiCache
import perf_log


# These fixtures use tracked inputs, temporary repositories and fake external
# programs. New checks default to execution until their input boundary is audited.
REUSABLE = frozenset(f"scripts/tests/{name}" for name in {
    "ditto-ci.test.py", "wire-contracts.test.py",
    "unity-transaction.test.py", "unity-metadata.test.py",
    "deploy.test.py", "browser-console.test.py", "web-selection.test.py",
    "stylon-validation.test.py", "native-validation-selection.test.py",
    "perf-report.test.py", "perf-candidate.test.py", "tollgate-evidence.test.py",
    "prose-validation.test.py", "ditto-replay.test.py", "ditto-benchmark.test.py",
    "ditto-cutover.test.py",
})

# Compiler ownership, real sockets, process priority/accounting, admission and
# cache/CI supervisors remain live checks. Validation/web preparation and Unity
# selection consult the installed Cargo toolchain/configuration and remain live.
_CONTEXT_PREFIXES = ("BATTLEMENT_INHERITED_",)
_CONTEXT_VARIABLES = {
    "BATTLEMENT_PARENT_OPERATION_ID", "BATTLEMENT_PYTHON", "BATTLEMENT_CI_JOB_ID",
    "BATTLEMENT_ROOT_OPERATION_ID", "BATTLEMENT_CARGO_LEASE_DESCRIPTOR",
}
_SYSTEM_VARIABLES = {
    "HOME", "USERPROFILE", "HOMEDRIVE", "HOMEPATH", "SYSTEMROOT", "SystemRoot",
    "WINDIR", "COMSPEC", "ComSpec", "PATHEXT", "TMPDIR", "TEMP", "TMP",
    "LANG", "LC_ALL", "LC_CTYPE", "TZ", "NODE_OPTIONS", "LD_PRELOAD",
    "DYLD_INSERT_LIBRARIES", "GIT_INDEX_FILE", "GIT_DIR", "GIT_WORK_TREE", "GIT_TEMPLATE_DIR",
}
_TOOLS = ("python3", "git", "node") + (("taskkill",) if os.name == "nt" else ())


def fixture_environment(inherited: dict[str, str] | None = None) -> dict[str, str]:
    """Bound synthetic fixtures to explicit settings and the selected real tools."""
    inherited = os.environ if inherited is None else inherited
    environment = {
        key: value for key, value in inherited.items()
        if key in _SYSTEM_VARIABLES or key.startswith(("BATTLEMENT_", "DITTO_", "PYTHON"))
    }
    environment = {key: value for key, value in environment.items()
                   if key not in _CONTEXT_VARIABLES and not key.startswith(_CONTEXT_PREFIXES)}
    # Preserve resolution order for every real fixture tool; unrelated app PATH
    # entries and caller context cannot change the effective fixture environment.
    path = inherited.get("PATH", os.defpath)
    directories = {str(Path(executable).parent.absolute()) for name in _TOOLS
                   if (executable := shutil.which(name, path=path)) is not None}
    selected = [part for part in path.split(os.pathsep)
                if str(Path(part).absolute()) in directories]
    environment["PATH"] = os.pathsep.join(dict.fromkeys([*selected, *os.defpath.split(os.pathsep)]))
    environment.update(GIT_CONFIG_GLOBAL=os.devnull, GIT_CONFIG_NOSYSTEM="1")
    environment["BATTLEMENT_LOG_ROOT"] = str(perf_log.configured_log_root())
    return environment


def create(repository: Path, shared: CiCache, checks: tuple, logs: Path) -> tuple[CiCache, dict[str, str]]:
    """Bind reuse to the effective fixture environment and complete staged tree."""
    environment = fixture_environment()
    tools = {}
    for label, name in (("interpreter", sys.executable), *((name, name) for name in _TOOLS)):
        executable = shutil.which(name, path=environment.get("PATH"))
        if executable is None:
            tools[label] = None
            continue
        path = Path(executable).resolve()
        tools[label] = [str(path), hashlib.sha256(path.read_bytes()).hexdigest()]
    identity = {
        "environment": environment,
        "tools": tools,
        "python": sys.version,
        "platform": platform.platform(),
        "checks": checks,
        "reusable": sorted(REUSABLE),
    }
    # The staged tree includes sample manifests, Unity packages, fixture crates,
    # generated contracts and every imported script; a scripts-only key is unsafe.
    unsupported = any(os.environ.get(key) for key in (
        "GIT_INDEX_FILE", "GIT_DIR", "GIT_WORK_TREE", "GIT_TEMPLATE_DIR", "PYTHONPATH",
        "PYTHONHOME", "PYTHONUSERBASE", "NODE_OPTIONS", "LD_PRELOAD", "DYLD_INSERT_LIBRARIES",
    ))
    staged = subprocess.check_output(["git", "ls-files", "--stage", "-z"], cwd=repository)
    unsupported |= any(row.startswith((b"120000 ", b"160000 ")) for row in staged.split(b"\0"))
    if unsupported:
        print("    Tooling reuse disabled: external input boundary is not represented", flush=True)
    cache = CiCache(
        repository, shared.cache_root,
        shared.environment | {"tooling": hashlib.sha256(json.dumps(identity, sort_keys=True).encode()).hexdigest()},
        enabled=shared.enabled and not unsupported,
        event=shared.event,
        provenance={"source": perf_log.git_metadata(repository), "fixture_logs": str(logs)},
    )
    return cache, environment
