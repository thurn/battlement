#!/usr/bin/env python3

"""Build or reuse a content-addressed Web sample for demos and screenshots."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile
import time
from typing import NamedTuple

import operation_log
from platform_support import lock_file, user_cache_path


REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_CACHE_ROOT = Path(
    os.environ.get(
        "BATTLEMENT_WEB_DEMO_CACHE",
        user_cache_path("Battlement", "web-demos"),
    )
)
WEB_SHARED_INPUTS = ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml")


class BuildIdentity(NamedTuple):
    """Exact staged input manifest for one reusable player build."""

    key: str
    manifest: dict[str, object]


def staged_fingerprint(sample: str, release: bool) -> str:
    """Fingerprint staged Web build inputs and the local build toolchain."""
    return web_build_identity(sample, release).key


def web_build_identity(sample: str, release: bool) -> BuildIdentity:
    """Resolve and fingerprint only inputs that can change player bytes."""
    editor = unity_editor(sample)
    editor_metadata = editor.stat()
    groups = {
        "toolchain": WEB_SHARED_INPUTS,
        "build-tool": dependency_pathspecs(sample),
        "client-runtime": (
            "Packages/com.battlement.client/Editor",
            "Packages/com.battlement.client/Editor.meta",
            "Packages/com.battlement.client/Runtime",
            "Packages/com.battlement.client/Runtime.meta",
            "Packages/com.battlement.client/package.json",
            "Packages/com.battlement.client/package.json.meta",
        ),
        "sample-project": (
            f"samples/{sample}/Assets",
            f"samples/{sample}/Packages",
            f"samples/{sample}/ProjectSettings",
            f"samples/{sample}/sample.toml",
        ),
        "sample-rules": (f"samples/{sample}/rules",),
        "web-bootstrap": ("web/init.js",),
        "preparation-tool": ("scripts/prepare-web-demo.py",),
    }
    pathspecs = tuple(dict.fromkeys(path for paths in groups.values() for path in paths))
    status = subprocess.run(
        [
            "git", "status", "--porcelain=v1", "-z", "--untracked-files=all",
            "--", *pathspecs,
        ],
        cwd=REPOSITORY_ROOT,
        check=True,
        capture_output=True,
    ).stdout
    if any(
        record.startswith(b"??") or len(record) < 2 or record[1:2] != b" "
        for record in status.split(b"\0")
        if record
    ):
        raise RuntimeError("Stage all Web demo inputs before preparing a reusable build.")
    inputs = {
        category: staged_digest(paths)
        for category, paths in groups.items()
    }
    manifest = {
        "schema": 2,
        "sample": sample,
        "release": release,
        "inputs": inputs,
        "host": [platform.system(), platform.machine()],
        "editor": [str(editor.resolve()), editor_metadata.st_mtime_ns, editor_metadata.st_size],
        "cargo": command_version(["cargo", "--version"]),
        "rustc": command_version(["rustc", "-Vv"]),
    }
    encoded = json.dumps(manifest, sort_keys=True, separators=(",", ":")).encode()
    return BuildIdentity(hashlib.sha256(encoded).hexdigest(), manifest)


def dependency_pathspecs(sample: str) -> tuple[str, ...]:
    """Return the transitive local packages used by the builder and sample rules."""
    root_packages = cargo_packages(REPOSITORY_ROOT / "Cargo.toml")
    sample_packages = cargo_packages(REPOSITORY_ROOT / f"samples/{sample}/rules/Cargo.toml")
    by_directory = {
        str(Path(package["manifest_path"]).resolve().parent): package
        for package in root_packages
    }
    queue = [
        directory
        for directory, package in by_directory.items()
        if package["name"] == "battlement-cli"
    ]
    if not queue:
        raise RuntimeError("Cargo metadata omitted the battlement-cli build package")
    for package in sample_packages:
        queue.extend(
            str(Path(dependency["path"]).resolve())
            for dependency in package["dependencies"]
            if dependency.get("path")
        )
    selected = set()
    while queue:
        directory = queue.pop()
        if directory in selected:
            continue
        selected.add(directory)
        package = by_directory.get(directory)
        if package:
            queue.extend(
                str(Path(dependency["path"]).resolve())
                for dependency in package["dependencies"]
                if dependency.get("path")
            )
    repository = REPOSITORY_ROOT.resolve()
    try:
        return tuple(sorted(str(Path(directory).relative_to(repository)) for directory in selected))
    except ValueError as error:
        raise RuntimeError("Web build depends on a local package outside the repository") from error


def cargo_packages(manifest: Path) -> list[dict[str, object]]:
    output = subprocess.run(
        [
            "cargo", "metadata", "--format-version", "1", "--locked", "--no-deps",
            "--manifest-path", str(manifest),
        ],
        cwd=REPOSITORY_ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return json.loads(output)["packages"]


def staged_digest(pathspecs: tuple[str, ...]) -> str:
    staged = subprocess.run(
        ["git", "ls-files", "--stage", "--", *pathspecs],
        cwd=REPOSITORY_ROOT,
        check=True,
        capture_output=True,
    ).stdout
    return hashlib.sha256(staged).hexdigest()


def prepare(sample: str, release: bool, cache_root: Path) -> Path:
    """Materialize one exact cached Web build and return its local path."""
    profile = "release" if release else "debug"
    with operation_log.Operation(
        REPOSITORY_ROOT,
        "Web preparation",
        metadata={"sample": sample, "build_profile": profile, "cache_root": str(cache_root)},
    ) as operation:
        return _prepare(sample, release, cache_root, operation)


def _prepare(
    sample: str, release: bool, cache_root: Path, operation: operation_log.Operation,
) -> Path:
    validate_sample(sample)
    identity = web_build_identity(sample, release)
    key = identity.key
    profile = "release" if release else "debug"
    operation.event(
        "preparation.inputs",
        source_manifest_sha256=key,
        build_profile=profile,
        manifest=identity.manifest,
    )
    output_name = "WebThreads"
    output = REPOSITORY_ROOT / "samples" / sample / "Build" / profile / output_name
    cached = cache_root / "entries" / sample / key / output_name
    manifest_path = cached.parent / "manifest.json"
    lock = cache_root / "locks" / sample / f"{key}.lock"
    lock.parent.mkdir(parents=True, exist_ok=True)
    started = time.monotonic()
    cache_hit = False
    with lock.open("a+") as lease:
        lock_file(lease)
        if not valid_web_build(cached) or read_manifest(manifest_path) != identity.manifest:
            changed = changed_categories(cached.parent.parent, identity.manifest)
            operation.event(
                "cache.lookup", result="miss", reason=changed, cache_key=key,
                producer_run=operation.id,
            )
            print(f"Web demo cache miss {key[:12]} ({changed}); building {sample}", flush=True)
            operation_log.run(build_command(sample, release), cwd=REPOSITORY_ROOT)
            if not valid_web_build(output):
                raise RuntimeError(f"Web build is incomplete: {output}")
            publish_directory(output, cached, cache_root / "entries")
            write_manifest(manifest_path, identity.manifest)
        else:
            cache_hit = True
            operation.event(
                "cache.lookup", result="hit", reason="inputs unchanged", cache_key=key,
                producer_run=operation.id,
            )
            print(f"Web demo cache hit {key[:12]} (inputs unchanged)", flush=True)
        materialize_directory(cached, output)
    operation.event(
        "artifact.published", artifact_kind="web-player", path=str(output.resolve()),
        source_manifest_sha256=key,
    )
    if cache_hit:
        operation.finish(
            "reused", 0, artifact_path=str(output.resolve()),
            source_manifest_sha256=key,
        )
    print(f"Prepared {output} in {time.monotonic() - started:.1f}s", flush=True)
    return output


def read_manifest(path: Path) -> dict[str, object] | None:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (FileNotFoundError, json.JSONDecodeError):
        return None


def write_manifest(path: Path, manifest: dict[str, object]) -> None:
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(manifest, sort_keys=True) + "\n", encoding="utf-8")
    temporary.replace(path)


def changed_categories(entries: Path, manifest: dict[str, object]) -> str:
    previous = [read_manifest(path) for path in entries.glob("*/manifest.json")]
    previous = [candidate for candidate in previous if candidate]
    if not previous:
        return "no prior reusable build"
    latest = max(
        (path for path in entries.glob("*/manifest.json") if read_manifest(path)),
        key=lambda path: path.stat().st_mtime_ns,
    )
    before = read_manifest(latest) or {}
    categories = [
        category
        for category, digest in manifest["inputs"].items()
        if before.get("inputs", {}).get(category) != digest
    ]
    for category in ("host", "editor", "cargo", "rustc", "release"):
        if before.get(category) != manifest.get(category):
            categories.append(category)
    return ", ".join(categories) or "prior artifact invalid"


def build_command(sample: str, release: bool) -> list[str]:
    command = [
        "cargo", "run", "--quiet", "-p", "battlement-cli", "--",
        "sample", "build", sample, "--web",
    ]
    if release:
        command.append("--release")
    return command


def publish_directory(source: Path, destination: Path, entries_root: Path) -> None:
    entries_root.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix="web-demo.", dir=entries_root))
    try:
        clone_directory(source, staging / destination.name)
        destination.parent.mkdir(parents=True, exist_ok=True)
        if not destination.exists():
            (staging / destination.name).rename(destination)
    finally:
        shutil.rmtree(staging, ignore_errors=True)


def materialize_directory(source: Path, destination: Path) -> None:
    if destination.exists():
        shutil.rmtree(destination)
    destination.parent.mkdir(parents=True, exist_ok=True)
    clone_directory(source, destination)


def clone_directory(source: Path, destination: Path) -> None:
    if platform.system() == "Darwin":
        subprocess.run(["cp", "-cR", str(source), str(destination)], check=True)
    else:
        shutil.copytree(source, destination)


def valid_web_build(path: Path) -> bool:
    wasm = tuple((path / "Build").glob("*.wasm")) + tuple(
        (path / "Build").glob("*.wasm.unityweb")
    )
    return (path / "index.html").is_file() and bool(wasm)


def unity_editor(sample: str) -> Path:
    version_file = REPOSITORY_ROOT / "samples" / sample / "ProjectSettings/ProjectVersion.txt"
    version = next(
        line.removeprefix("m_EditorVersion: ")
        for line in version_file.read_text(encoding="utf-8").splitlines()
        if line.startswith("m_EditorVersion: ")
    )
    if configured := os.environ.get("UNITY_EDITOR"):
        return Path(configured)
    if platform.system() == "Windows":
        program_files = Path(os.environ.get("PROGRAMFILES", "C:/Program Files"))
        return program_files / f"Unity/Hub/Editor/{version}/Editor/Unity.exe"
    return Path(f"/Applications/Unity/Hub/Editor/{version}/Unity.app/Contents/MacOS/Unity")


def command_version(command: list[str]) -> str:
    return subprocess.run(
        command,
        cwd=REPOSITORY_ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def validate_sample(sample: str) -> None:
    if not sample or any(character not in "abcdefghijklmnopqrstuvwxyz0123456789-" for character in sample):
        raise RuntimeError("Sample names may contain lowercase letters, numbers, and hyphens.")
    if not (REPOSITORY_ROOT / "samples" / sample / "sample.toml").is_file():
        raise RuntimeError(f"Unknown sample: {sample}")


def parse_arguments(arguments: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("sample")
    parser.add_argument(
        "--development",
        action="store_true",
        help="build an uncompressed development player instead of the deployable release profile",
    )
    parser.add_argument("--cache-root", type=Path, default=DEFAULT_CACHE_ROOT)
    return parser.parse_args(arguments)


if __name__ == "__main__":
    arguments = parse_arguments()
    try:
        prepare(
            arguments.sample,
            not arguments.development,
            arguments.cache_root,
        )
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(error)
        raise SystemExit(1) from error
