#!/usr/bin/env python3

"""Process-level regressions for Ditto build-cache preparation lifetimes."""

from __future__ import annotations

import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import uuid


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
RUNNER = REPOSITORY_ROOT / "scripts/ditto_ci.py"
sys.path.insert(0, str(REPOSITORY_ROOT / "scripts"))
SPEC = importlib.util.spec_from_file_location(
    "ditto_build_leases", REPOSITORY_ROOT / "scripts/ditto_build_leases.py"
)
assert SPEC and SPEC.loader
LEASE_MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = LEASE_MODULE
SPEC.loader.exec_module(LEASE_MODULE)
DittoBuildLeases = LEASE_MODULE.DittoBuildLeases


FAKE_DITTO = r'''#!/usr/bin/env python3
import fcntl
import hashlib
import json
import os
from pathlib import Path
import sys
import tomllib
import uuid

arguments = sys.argv[1:]
config = arguments[arguments.index("--config") + 1]
sample = Path(config).parent.name
fingerprint = hashlib.sha256(config.encode()).hexdigest()
cache = Path(os.environ["DITTO_CACHE_ROOT"])
entry = cache / "builds/entries" / fingerprint
settings = entry / "player.app/Contents/Resources/Data/StreamingAssets/aa/settings.json"

if "build" in arguments:
    lock_path = cache / "builds/locks" / f"{fingerprint}.active"
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    lock = lock_path.open("a+")
    fcntl.flock(lock, fcntl.LOCK_SH)
    if sample in {"failing", "cancelled"} and sample == "failing":
        raise SystemExit(7)
    existed = entry.is_dir()
    settings.parent.mkdir(parents=True, exist_ok=True)
    settings.write_text(os.environ.get("FAKE_BUILD_TOKEN", sample))
    result = {
        "schema": 1,
        "suite": sample,
        "profile": "macos",
        "source_fingerprint": "f" * 64,
        "build_fingerprint": fingerprint,
        "disposition": "reused" if existed else "created",
        "player_path": str(entry),
    }
    encoded = json.dumps(result)
    Path(arguments[arguments.index("--output") + 1]).write_text(encoded)
    print(encoded, flush=True)
    if "--retain-until-fd-closed" in arguments:
        descriptor = int(arguments[arguments.index("--retain-until-fd-closed") + 1])
        with os.fdopen(descriptor, "rb") as control:
            control.read()
    raise SystemExit(0)

if "--no-build" in arguments:
    if not settings.is_file():
        print(f"missing prepared build: {settings}", file=sys.stderr)
        raise SystemExit(2)
    disposition = "reused"
else:
    settings.parent.mkdir(parents=True, exist_ok=True)
    settings.write_text(os.environ.get("FAKE_BUILD_TOKEN", sample))
    disposition = "created"

output = Path(arguments[arguments.index("--output") + 1])
suite = tomllib.loads(Path(config).read_text())
names = [scenario["name"] for scenario in suite["scenarios"]]
selected = [argument for argument in arguments if argument in names]
if selected:
    names = selected
run = Path(os.environ["FAKE_RUN_ROOT"]) / str(uuid.uuid4())
(run / "logs").mkdir(parents=True)
(run / "logs/events.jsonl").write_text('{"sequence":1}\n')
print(f"DITTO_RUN_DIR={run}", file=sys.stderr, flush=True)
result = {
    "run_id": str(uuid.uuid4()),
    "status": "passed",
    "build": {"disposition": disposition, "fingerprint": fingerprint},
    "player_sessions": [{
        "startup_report": {"capture_adapter": "native-screen-capture"},
    }],
    "scenarios": [{"name": name, "status": "passed"} for name in names],
}
output.parent.mkdir(parents=True, exist_ok=True)
output.write_text(json.dumps(result))
'''


LOCK_HOLDER = r'''#!/usr/bin/env python3
import fcntl
import json
import os
from pathlib import Path
import sys
import time

active, build, staging, published, ready, release = map(Path, sys.argv[1:])
active_file = active.open("a+")
build_file = build.open("a+")
fcntl.flock(active_file, fcntl.LOCK_SH)
fcntl.flock(build_file, fcntl.LOCK_EX)
ready.write_text(json.dumps({"active": active.stat().st_ino, "build": build.stat().st_ino}))
while not release.exists():
    time.sleep(0.01)
published.parent.mkdir(parents=True, exist_ok=True)
os.rename(staging, published)
'''


CONSUMER = r'''#!/usr/bin/env python3
import fcntl
from pathlib import Path
import sys
import time

lock_path, settings, ready, release = map(Path, sys.argv[1:])
lock = lock_path.open("a+")
fcntl.flock(lock, fcntl.LOCK_SH)
assert settings.read_text()
ready.write_text("ready")
while not release.exists():
    time.sleep(0.01)
'''


def main() -> None:
    if os.name == "nt":
        print("Ditto cache lifetime tests require Unix advisory locks.")
        return
    with tempfile.TemporaryDirectory(prefix="ditto-cache-lifetime.") as temporary:
        root = Path(temporary)
        binary, environment = fixture(root)
        verify_cold_preparation_is_owned(root, binary, environment)
        verify_prepared_build_lease_lifetime(root, binary)
    print("Ditto cache lifetime tests passed.")


def fixture(root: Path) -> tuple[Path, dict[str, str]]:
    binary = root / "ditto.py"
    binary.write_text(FAKE_DITTO)
    binary.chmod(0o755)
    tools = root / "bin"
    tools.mkdir()
    cargo = tools / "cargo"
    cargo.write_text("#!/bin/sh\nexit 0\n")
    cargo.chmod(0o755)
    environment = os.environ.copy()
    environment.update({
        "DITTO_CI_BINARY": str(binary),
        "DITTO_ODIFF_PATH": str(binary),
        "DITTO_CI_TEST_HOST": "macos-arm64",
        "FAKE_RUN_ROOT": str(root / "runs"),
        "FAKE_BUILD_TOKEN": "cold-new",
        "PATH": f"{tools}{os.pathsep}{environment['PATH']}",
    })
    return binary, environment


def verify_cold_preparation_is_owned(
    root: Path, _binary: Path, environment: dict[str, str],
) -> None:
    shared = root / "shared-cache"
    config = "samples/basic/ditto.toml"
    active_fingerprint = hashlib.sha256(config.encode()).hexdigest()
    staged_fingerprint = "b" * 64
    settings = (
        shared / "builds/entries" / active_fingerprint
        / "player.app/Contents/Resources/Data/StreamingAssets/aa/settings.json"
    )
    settings.parent.mkdir(parents=True)
    settings.write_text("shared-old")
    staging = shared / "builds/staging" / staged_fingerprint
    staging.mkdir(parents=True)
    (staging / "partial").write_text("complete after cold")
    locks = shared / "builds/locks"
    locks.mkdir(parents=True)
    journal = shared / "builds/journal.jsonl"
    journal.write_text('{"event":"created"}\n')
    journal_bytes = journal.read_bytes()
    active_lock = locks / f"{active_fingerprint}.active"
    build_lock = locks / f"{staged_fingerprint}.build"
    active_lock.write_text("active")
    build_lock.write_text("build")
    ready = root / "locks-ready.json"
    release = root / "release-locks"
    published = shared / "builds/entries" / staged_fingerprint
    holder = subprocess.Popen([
        sys.executable, "-c", LOCK_HOLDER,
        str(active_lock), str(build_lock), str(staging), str(published),
        str(ready), str(release),
    ])
    wait_for(ready)
    original_inodes = json.loads(ready.read_text())

    cold_environment = environment.copy()
    cold_environment["DITTO_CI_CACHE_ROOT"] = str(shared)
    cold_environment["DITTO_CI_INVOCATION_ID"] = f"cold-{uuid.uuid4()}"
    cold_environment["DITTO_CI_ARTIFACT_ROOT"] = str(
        root / "evidence" / cold_environment["DITTO_CI_INVOCATION_ID"]
    )
    cold = run(["prepare", "cold"], cold_environment)
    assert cold.returncode == 0, cold.stderr
    report_path = artifact_root(cold) / "preparation-cold.json"
    report = json.loads(report_path.read_text())
    owned = Path(report["cache_root"])
    assert owned == report_path.resolve().parent / "prepared-cache"
    assert owned != shared
    assert settings.read_text() == "shared-old"
    assert staging.is_dir()
    assert journal.read_bytes() == journal_bytes
    assert active_lock.stat().st_ino == original_inodes["active"]
    assert build_lock.stat().st_ino == original_inodes["build"]
    assert not try_exclusive(active_lock)
    assert not try_exclusive(build_lock)
    owned_settings = (
        owned / "builds/entries" / active_fingerprint
        / "player.app/Contents/Resources/Data/StreamingAssets/aa/settings.json"
    )
    assert owned_settings.read_text() == "cold-new"
    assert all(sample["build"] == "created" for sample in report["samples"])
    for sample in report["samples"]:
        recipe = report_path.parent / f"prepare-cold-{sample['sample']}/replay.json"
        assert json.loads(recipe.read_text())["environment"]["DITTO_CACHE_ROOT"] == str(owned)

    release.touch()
    assert holder.wait(timeout=5) == 0
    assert published.joinpath("partial").read_text() == "complete after cold"

    warm_environment = environment.copy()
    warm_environment["DITTO_CI_CACHE_ROOT"] = str(shared)
    warm_environment["DITTO_CI_INVOCATION_ID"] = f"warm-missing-{uuid.uuid4()}"
    warm_environment["DITTO_CI_ARTIFACT_ROOT"] = str(
        root / "evidence" / warm_environment["DITTO_CI_INVOCATION_ID"]
    )
    ambiguous_warm = run(["prepare", "warm"], warm_environment)
    assert ambiguous_warm.returncode == 1
    assert "requires --prepared" in ambiguous_warm.stderr
    warm_environment["DITTO_CI_INVOCATION_ID"] = f"warm-{uuid.uuid4()}"
    warm_environment["DITTO_CI_ARTIFACT_ROOT"] = str(
        root / "evidence" / warm_environment["DITTO_CI_INVOCATION_ID"]
    )
    warm = run(["prepare", "warm", "--prepared", str(report_path)], warm_environment)
    assert warm.returncode == 0, warm.stderr
    warm_report = json.loads((artifact_root(warm) / "preparation-warm.json").read_text())
    assert warm_report["cache_root"] == str(owned)
    assert all(sample["build"] == "reused" for sample in warm_report["samples"])
    assert [sample["fingerprint"] for sample in warm_report["samples"]] == [
        sample["fingerprint"] for sample in report["samples"]
    ]


def verify_prepared_build_lease_lifetime(root: Path, binary: Path) -> None:
    cache = root / "lease-cache"
    manager = DittoBuildLeases(
        REPOSITORY_ROOT, binary, cache, root / "lease-evidence"
    )
    result = manager.prepare("basic")
    with manager._lock:
        producer = manager._builds[0].process
    assert os.getsid(producer.pid) == os.getsid(0)
    assert os.getpgid(producer.pid) == producer.pid
    fingerprint = str(result["build_fingerprint"])
    entry = cache / "builds/entries" / fingerprint
    active_lock = cache / "builds/locks" / f"{fingerprint}.active"
    settings = entry / "player.app/Contents/Resources/Data/StreamingAssets/aa/settings.json"
    lock_inode = active_lock.stat().st_ino
    assert not cooperative_cleanup(active_lock, entry)
    assert settings.is_file()
    assert active_lock.stat().st_ino == lock_inode

    ready = root / "consumer-ready"
    release = root / "consumer-release"
    consumer = subprocess.Popen([
        sys.executable, "-c", CONSUMER,
        str(active_lock), str(settings), str(ready), str(release),
    ])
    wait_for(ready)
    manager.close()
    assert not cooperative_cleanup(active_lock, entry)
    assert settings.is_file()
    release.touch()
    assert consumer.wait(timeout=5) == 0
    assert cooperative_cleanup(active_lock, entry)
    assert not entry.exists()
    assert active_lock.stat().st_ino == lock_inode

    protected = DittoBuildLeases(
        REPOSITORY_ROOT, binary, cache, root / "protected-evidence"
    )
    protected_result = protected.prepare("basic")
    protected_fingerprint = str(protected_result["build_fingerprint"])
    protected_entry = cache / "builds/entries" / protected_fingerprint
    protected_lock = cache / "builds/locks" / f"{protected_fingerprint}.active"
    failed = DittoBuildLeases(
        REPOSITORY_ROOT, binary, cache, root / "failed-evidence"
    )
    try:
        failed.prepare("failing")
    except subprocess.CalledProcessError as error:
        assert error.returncode == 7
    else:
        raise AssertionError("failed preparation was accepted")
    failed.close()
    assert not cooperative_cleanup(protected_lock, protected_entry)
    protected.close()
    assert cooperative_cleanup(protected_lock, protected_entry)

    cancelled = DittoBuildLeases(
        REPOSITORY_ROOT, binary, cache, root / "cancelled-evidence"
    )
    cancelled_result = cancelled.prepare("cancelled")
    cancelled_fingerprint = str(cancelled_result["build_fingerprint"])
    cancelled_entry = cache / "builds/entries" / cancelled_fingerprint
    cancelled_lock = cache / "builds/locks" / f"{cancelled_fingerprint}.active"
    with cancelled._lock:
        process = cancelled._builds[0].process
    process.terminate()
    process.wait(timeout=5)
    try:
        cancelled.assert_healthy()
    except RuntimeError as error:
        assert "ended before execution" in str(error)
    else:
        raise AssertionError("canceled build lease was accepted")
    try:
        cancelled.close()
    except RuntimeError as error:
        assert "retained build process exited" in str(error)
    else:
        raise AssertionError("canceled build lease closed as successful")
    assert cooperative_cleanup(cancelled_lock, cancelled_entry)


def run(arguments: list[str], environment: dict[str, str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(RUNNER), *arguments], cwd=REPOSITORY_ROOT,
        env=environment, capture_output=True, text=True,
    )


def artifact_root(completed: subprocess.CompletedProcess[str]) -> Path:
    roots = [
        Path(line.removeprefix("DITTO_CI_ARTIFACT_ROOT="))
        for line in completed.stdout.splitlines()
        if line.startswith("DITTO_CI_ARTIFACT_ROOT=")
    ]
    assert len(roots) == 1, completed.stdout
    return roots[0]


def wait_for(path: Path) -> None:
    deadline = time.monotonic() + 5
    while not path.exists():
        if time.monotonic() >= deadline:
            raise AssertionError(f"process did not publish {path}")
        time.sleep(0.01)


def try_exclusive(path: Path) -> bool:
    with path.open("a+") as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            return False
        return True


def cooperative_cleanup(lock_path: Path, entry: Path) -> bool:
    with lock_path.open("a+") as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            return False
        if entry.is_dir():
            shutil.rmtree(entry)
        return True


if __name__ == "__main__":
    main()
