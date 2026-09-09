#!/usr/bin/env python3

"""Select expensive CI work from the certified-release dependency boundary."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import native_validation_selection


GLOBAL_RUST_INPUTS = (
    ".cargo/",
    "Cargo.lock",
    "Cargo.toml",
    "rust-toolchain.toml",
)
REACTANT_ASSET_INPUTS = (
    "Cargo.lock",
    "Cargo.toml",
    "rust-toolchain.toml",
    "crates/battlement-cli/",
    "crates/battlement-reactant/",
    "crates/battlement-reactant-asset-macros/",
    "crates/battlement-reactant-asset-syntax/",
    "crates/battlement-reactant-assets/",
    "scripts/reactant_asset_validation.py",
)


@dataclass(frozen=True)
class RustSelection:
    """Root and standalone Rust workspaces affected by changed paths."""

    root: bool
    samples: tuple[Path, ...]
    reasons: tuple[str, ...]

    def report(self) -> dict[str, object]:
        return {
            "reasons": list(self.reasons),
            "root": self.root,
            "samples": [str(path.parent) for path in self.samples],
        }


def select_rust(
    repository: Path,
    paths: list[str],
    sample_workspaces: list[Path],
) -> RustSelection:
    """Return Rust workspaces whose source or dependency closure changed."""
    if not paths:
        return RustSelection(
            True,
            tuple(sample_workspaces),
            ("no candidate diff was available",),
        )
    normalized = {path.replace("\\", "/") for path in paths}
    global_paths = sorted(
        path for path in normalized if _matches(path, GLOBAL_RUST_INPUTS)
    )
    changed_crates = {
        path.split("/", 2)[1]
        for path in normalized
        if path.startswith("crates/") and path.count("/") >= 2
    }
    selected = []
    reasons = [f"global Rust input changed: {path}" for path in global_paths]
    for workspace in sample_workspaces:
        sample = workspace.parts[1]
        own_input = any(path.startswith(f"samples/{sample}/rules/") for path in normalized)
        dependency_input = bool(
            changed_crates
            and changed_crates
            & native_validation_selection.sample_crates(repository, sample)
        )
        if global_paths or own_input or dependency_input:
            selected.append(workspace)
        if own_input:
            reasons.append(f"sample Rust input changed: samples/{sample}/rules")
        if dependency_input:
            reasons.append(f"sample dependency changed: samples/{sample}/rules")
    root = bool(global_paths or changed_crates)
    if changed_crates:
        reasons.append("root workspace crate changed: " + ", ".join(sorted(changed_crates)))
    if not root and not selected:
        reasons.append("no changed path can affect a Rust workspace")
    return RustSelection(root, tuple(selected), tuple(dict.fromkeys(reasons)))


def select_reactant_assets(paths: list[str]) -> tuple[bool, tuple[str, ...]]:
    """Return whether the ignored Reactant CLI/browser scenario is affected."""
    if not paths:
        return True, ("no candidate diff was available",)
    selected = tuple(
        f"Reactant asset input changed: {path}"
        for path in sorted({path.replace("\\", "/") for path in paths})
        if _matches(path, REACTANT_ASSET_INPUTS)
    )
    return bool(selected), selected or (
        "no changed path can affect Reactant asset CLI/browser behavior",
    )


def _matches(path: str, inputs: tuple[str, ...]) -> bool:
    return any(
        path == value.rstrip("/") or path.startswith(value)
        for value in inputs
    )
