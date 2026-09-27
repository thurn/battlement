"""Lease bounded warm Cargo targets with validated checkout handoffs."""

from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import subprocess
import shutil
import uuid

import resource_slots
from platform_support import try_lock_file, unlock_file

LEASE_DESCRIPTOR = "BATTLEMENT_CARGO_LEASE_DESCRIPTOR"
OWN_WRAPPER = "BATTLEMENT_CARGO_WRAPPER"


def process_options(environment=None) -> dict:
    """Retain an inherited target lease through a Python subprocess boundary."""
    env = os.environ if environment is None else environment
    options = {"env": environment}
    if os.name == "nt" or LEASE_DESCRIPTOR not in env:
        return options
    descriptor = int(env[LEASE_DESCRIPTOR])
    target = Path(env["CARGO_TARGET_DIR"])
    lock = target.parents[2] / "locks" / f"cargo-{target.name.removeprefix('pool-')}.lock"
    held, expected = os.fstat(descriptor), lock.stat()
    if (held.st_dev, held.st_ino) != (expected.st_dev, expected.st_ino):
        raise RuntimeError("Inherited Cargo target lease does not match its target")
    options["pass_fds"] = (descriptor,)
    return options


class CargoEnvironment(dict):
    """Cargo settings retaining the target lease through subprocess boundaries."""

    def process_options(self) -> dict:
        return process_options(self)


class _TargetLease(resource_slots.SlotLease):
    """Let the last child descriptor release the mutable target lock."""

    def _release_files(self) -> None:
        for file in reversed(self.files):
            file.close()
        self.files.clear()


def _compiler_environment(repository: Path, target: Path, descriptor=None) -> CargoEnvironment:
    env = CargoEnvironment(resource_slots.capacity_environment())
    env.pop(LEASE_DESCRIPTOR, None)
    env.setdefault("CARGO_BUILD_JOBS", "3")
    if env.get("RUSTC_BOOTSTRAP") == "-1":
        own = env.pop(OWN_WRAPPER, None)
        outer = env.pop("BATTLEMENT_CARGO_OUTER_WRAPPER", None)
        if own and env.get("RUSTC_WRAPPER") == own:
            if outer:
                env["RUSTC_WRAPPER"] = outer
            else:
                env.pop("RUSTC_WRAPPER")
        return env
    source = Path(__file__).with_name("cargo_rustc.rs")
    toolchain = source.parent.parent / "rust-toolchain.toml"
    digest = hashlib.sha256(source.read_bytes() + toolchain.read_bytes()).hexdigest()[:16]
    suffix = ".exe" if os.name == "nt" else ""
    wrapper = target / f"battlement-rustc-{digest}{suffix}"
    if not wrapper.exists():
        temporary = target / f".battlement-rustc-{uuid.uuid4()}{suffix}"
        options = {"env": env}
        if descriptor is not None:
            options["pass_fds"] = (descriptor,)
        try:
            subprocess.run(["rustc", "--edition", "2024", "-Dwarnings", "-O", str(source),
                            "-o", str(temporary)], cwd=source.parent.parent, check=True, **options)
            temporary.replace(wrapper)
        finally:
            temporary.unlink(missing_ok=True)
    outer = env.get("RUSTC_WRAPPER")
    if outer == env.get(OWN_WRAPPER):
        outer = env.get("BATTLEMENT_CARGO_OUTER_WRAPPER")
    env.pop("BATTLEMENT_CARGO_OUTER_WRAPPER", None)
    if outer:
        env["BATTLEMENT_CARGO_OUTER_WRAPPER"] = outer
    env["RUSTC_WRAPPER"] = str(wrapper)
    env[OWN_WRAPPER] = str(wrapper)
    env["BATTLEMENT_CARGO_REPOSITORY"] = str(repository.resolve())
    return env


@contextmanager
def environment(repository: Path, cache: Path, workspace: Path | None, *, scope: str | None = None):
    """Hold compiler capacity and a mutable target until every consumer returns.

    Two writers can run concurrently; sequential checkouts reuse registry builds.
    Cargo's timestamp-based local fingerprints are cleared on checkout handoff.
    Target directories remain inside the existing capacity-protected LRU budget.
    """
    manifest = Path("Cargo.toml") if workspace is None else workspace
    identity = hashlib.sha256(manifest.as_posix().encode()).hexdigest()[:16]
    if os.name == "nt":
        with resource_slots.compiler_capacity_lease():
            # Windows byte-range locks cannot be retained through POSIX pass_fds.
            private = f"{repository.resolve()}:{scope or identity}"
            target = cache / "cargo-targets/shared" / hashlib.sha256(private.encode()).hexdigest()[:16]
            target.mkdir(parents=True, exist_ok=True)
            target.touch()
            env = _compiler_environment(repository, target)
            env["CARGO_TARGET_DIR"] = str(target)
            yield env
            return
    with _TargetLease(cache / "locks", f"cargo-{identity}", 2) as lease:
        with resource_slots.compiler_capacity_lease():
            slot = Path(lease.files[0].name).stem.rsplit("-", 1)[1]
            target = cache / "cargo-targets/shared" / f"pool-{identity}-{slot}"
            target.mkdir(parents=True, exist_ok=True)
            target.touch()
            env = _compiler_environment(repository, target, lease.files[0].fileno())
            env["CARGO_TARGET_DIR"] = str(target)
            env[LEASE_DESCRIPTOR] = str(lease.files[0].fileno())
            _handoff(repository, manifest, target, env)
            yield env


def _source_identity(repository: Path):
    if not (repository / ".git").exists():
        return None
    head = subprocess.run(["git", "rev-parse", "--verify", "HEAD"], cwd=repository,
                          capture_output=True, text=True)
    staged = subprocess.run(["git", "diff", "--cached", "--raw", "--no-abbrev", "-z"],
                            cwd=repository, capture_output=True, check=True)
    return [head.stdout.strip() if head.returncode == 0 else None,
            hashlib.sha256(staged.stdout).hexdigest()]


def _handoff(repository: Path, manifest: Path, target: Path, env: CargoEnvironment) -> None:
    owner = target / ".battlement-owner.json"
    directory = repository.stat()
    wrapper = Path(__file__).with_name("cargo_rustc.rs")
    identity = [str((repository / manifest).resolve()), directory.st_dev, directory.st_ino,
                _source_identity(repository),
                hashlib.sha256(wrapper.read_bytes()).hexdigest(),
                env.get("RUSTC_BOOTSTRAP"), env.get("BATTLEMENT_CARGO_OUTER_WRAPPER")]
    try:
        previous = json.loads(owner.read_text())
    except (FileNotFoundError, json.JSONDecodeError):
        previous = None
    if previous == identity:
        return
    owner.unlink(missing_ok=True)
    result = subprocess.run(
        ["cargo", "metadata", "--locked", "--format-version", "1", "--manifest-path", str(manifest)],
        cwd=repository, **env.process_options(), capture_output=True, text=True, check=True,
    )
    packages = [package for package in json.loads(result.stdout)["packages"]
                if package["source"] is None]
    names = {package["name"] for package in packages}
    # Retain incremental data and link inputs, but force Cargo to rebuild every
    # local package. rustc then validates reusable incremental state itself.
    fingerprints = [*target.glob("*/.fingerprint"), *target.glob("*/*/.fingerprint")]
    for directory in fingerprints:
        for entry in directory.iterdir():
            if entry.name.rsplit("-", 1)[0] in names:
                shutil.rmtree(entry)
    owner.write_text(json.dumps(identity))


def idle(target: Path, cache: Path) -> bool:
    """Check for a child-retained lease while maintenance owns compiler capacity."""
    if not target.name.startswith("pool-"):
        return True
    identity = target.name.removeprefix("pool-")
    lock = cache / "locks" / f"cargo-{identity}.lock"
    lock.parent.mkdir(parents=True, exist_ok=True)
    with lock.open("a+") as file:
        if not try_lock_file(file):
            return False
        unlock_file(file)
        return True
