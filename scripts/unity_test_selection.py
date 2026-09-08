#!/usr/bin/env python3

"""Select Unity Edit Mode validation from the candidate's actual dependencies."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
import json
from pathlib import Path
import subprocess


FIXTURE_MANIFEST = Path(
    "crates/battlement-native/tests/fixtures/exported-engine/Cargo.toml"
)
FULL_ASSEMBLIES = (
    "Battlement.Integration.EditorTests",
    "Battlement.EditorTests",
    "Battlement.HostEditorTests",
)
HOST_ASSEMBLY = "Battlement.HostEditorTests"
INTEGRATION_ASSEMBLIES = ("Battlement.Integration.EditorTests",)
NATIVE_ASSEMBLIES = (
    "Battlement.Integration.EditorTests",
    "Battlement.HostEditorTests",
)
UNITY_PROJECT_ROOTS = ("Assets", "Packages", "ProjectSettings")
UNITY_INTEGRATION_INPUTS = (
    "samples/ui/Assets/Original/Signal Texture.png",
)
UNITY_RUNNER_INPUTS = (
    "scripts/ci.py",
    "scripts/ci_cache.py",
    "scripts/ci_steps.py",
    "scripts/platform_support.py",
    "scripts/resource_slots.py",
    "scripts/unity_test_selection.py",
    "scripts/unity_transaction.py",
    "scripts/web_selection.py",
)
RUST_BUILD_INPUTS = (
    ".cargo",
    "Cargo.lock",
    "Cargo.toml",
    "rust-toolchain.toml",
)
DOTNET_DIAGNOSTIC_INPUTS = (
    ".config/dotnet-tools.json",
    ".editorconfig",
    "Assets",
    "Directory.Build.targets",
    "Packages",
    "ProjectSettings",
    "battlement-ci.slnx",
    "scripts/ci.py",
    "scripts/ci_cache.py",
    "scripts/ci_steps.py",
    "scripts/perf_log.py",
    "scripts/resource_slots.py",
    "scripts/unity_test_selection.py",
)


class Scope(Enum):
    """Unity Edit Mode assemblies required by one candidate."""

    NONE = "none"
    INTEGRATION = "integration"
    NATIVE = "native-integration"
    ALL = "all"


@dataclass(frozen=True)
class Selection:
    """Selected Unity scope with auditable path reasons and cache inputs."""

    scope: Scope
    assemblies: tuple[str, ...]
    reasons: tuple[str, ...]
    cache_inputs: tuple[str, ...]
    dotnet_diagnostics: bool

    def report(self) -> dict[str, object]:
        """Return a stable machine-readable selection explanation."""
        return {
            "assemblies": list(self.assemblies),
            "dotnet_diagnostics": self.dotnet_diagnostics,
            "reasons": list(self.reasons),
            "scope": self.scope.value,
        }


def select(repository: Path, paths: list[str]) -> Selection:
    """Select only Unity tests whose source or native fixture inputs changed."""
    dependency_roots = fixture_dependency_roots(repository)
    cache_inputs = tuple(
        dict.fromkeys(
            (
                *UNITY_PROJECT_ROOTS,
                *UNITY_INTEGRATION_INPUTS,
                *RUST_BUILD_INPUTS,
                *dependency_roots,
                *UNITY_RUNNER_INPUTS,
            )
        )
    )
    if not paths:
        return Selection(
            Scope.ALL,
            FULL_ASSEMBLIES,
            ("no candidate diff was available",),
            cache_inputs,
            True,
        )

    full = sorted(path for path in paths if full_test_input(path))
    native = sorted(
        path for path in paths if native_test_input(path, dependency_roots)
    )
    integration = sorted(
        path for path in paths if matches(path, UNITY_INTEGRATION_INPUTS)
    )
    dotnet = any(matches(path, DOTNET_DIAGNOSTIC_INPUTS) for path in paths)
    if full:
        return Selection(
            Scope.ALL,
            FULL_ASSEMBLIES,
            tuple(f"Unity project or test harness changed: {path}" for path in full),
            cache_inputs,
            dotnet,
        )
    if native:
        return Selection(
            Scope.NATIVE,
            NATIVE_ASSEMBLIES,
            tuple(f"native fixture dependency changed: {path}" for path in native),
            cache_inputs,
            dotnet,
        )
    if integration:
        return Selection(
            Scope.INTEGRATION,
            INTEGRATION_ASSEMBLIES,
            tuple(f"Unity integration fixture input changed: {path}" for path in integration),
            cache_inputs,
            dotnet,
        )
    return Selection(
        Scope.NONE,
        (),
        ("no changed path can affect the Unity project or native fixture",),
        cache_inputs,
        dotnet,
    )


def fixture_dependency_roots(repository: Path) -> tuple[str, ...]:
    """Return repository package roots in the native fixture's non-dev closure."""
    completed = subprocess.run(
        [
            "cargo",
            "metadata",
            "--format-version",
            "1",
            "--manifest-path",
            str(repository / FIXTURE_MANIFEST),
        ],
        cwd=repository,
        check=True,
        capture_output=True,
        text=True,
    )
    metadata = json.loads(completed.stdout)
    packages = {package["id"]: package for package in metadata["packages"]}
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    fixture = next(
        package["id"]
        for package in metadata["packages"]
        if Path(package["manifest_path"]).resolve() == (repository / FIXTURE_MANIFEST).resolve()
    )
    pending = [fixture]
    selected: set[str] = set()
    while pending:
        package_id = pending.pop()
        if package_id in selected:
            continue
        selected.add(package_id)
        for dependency in nodes[package_id]["deps"]:
            if any(kind["kind"] != "dev" for kind in dependency["dep_kinds"]):
                pending.append(dependency["pkg"])

    roots = []
    repository = repository.resolve()
    for package_id in selected:
        manifest = Path(packages[package_id]["manifest_path"]).resolve()
        if manifest.is_relative_to(repository):
            roots.append(manifest.parent.relative_to(repository).as_posix())
    roots.sort(key=lambda root: (root.count("/"), root))
    minimal = []
    for root in roots:
        if not any(root == parent or root.startswith(f"{parent}/") for parent in minimal):
            minimal.append(root)
    return tuple(minimal)


def full_test_input(path: str) -> bool:
    """Return whether a path can change compiled Unity code or its test harness."""
    return matches(path, UNITY_PROJECT_ROOTS) or path in UNITY_RUNNER_INPUTS


def native_test_input(path: str, dependency_roots: tuple[str, ...]) -> bool:
    """Return whether a path can change the native fixture loaded by Unity."""
    return matches(path, RUST_BUILD_INPUTS) or matches(path, dependency_roots)


def matches(path: str, inputs: tuple[str, ...]) -> bool:
    """Match an exact file or any descendant of a declared input directory."""
    return any(path == value or path.startswith(f"{value}/") for value in inputs)
