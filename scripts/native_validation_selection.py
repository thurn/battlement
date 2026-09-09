#!/usr/bin/env python3

"""Select native sample validation from changed ownership boundaries."""

from __future__ import annotations

import json
from pathlib import Path
import subprocess


GLOBAL_NATIVE_INPUTS = (
    "Cargo.lock",
    "Cargo.toml",
    "rust-toolchain.toml",
    "contracts/",
    "Packages/com.battlement.client/",
    "crates/battlement-ditto/",
    "crates/battlement-tooling/",
    "Assets/DefaultVolumeProfile.asset",
    "Assets/DefaultVolumeProfile.asset.meta",
    "Assets/Settings/",
    "Assets/UniversalRenderPipelineGlobalSettings.asset",
    "Assets/UniversalRenderPipelineGlobalSettings.asset.meta",
    "ProjectSettings/",
    "samples/basic/Assets/AddressableAssetsData/",
    "samples/basic/Assets/AddressableAssetsData.meta",
    "samples/basic/Assets/DefaultVolumeProfile.asset",
    "samples/basic/Assets/DefaultVolumeProfile.asset.meta",
    "samples/basic/Assets/Fonts/",
    "samples/basic/Assets/Fonts.meta",
    "samples/basic/Assets/Resources/",
    "samples/basic/Assets/Resources.meta",
    "samples/basic/Assets/Shaders/",
    "samples/basic/Assets/Shaders.meta",
    "samples/basic/Assets/Settings/",
    "samples/basic/Assets/UniversalRenderPipelineGlobalSettings.asset",
    "samples/basic/Assets/UniversalRenderPipelineGlobalSettings.asset.meta",
    "samples/basic/Packages/",
    "samples/basic/ProjectSettings/",
    "samples/chess/Assets/Settings/",
    "samples/chess/Assets/Shaders/LegalSquare.shader",
    "samples/chess/Assets/Shaders/LegalSquare.shader.meta",
    "samples/ui/Assets/Original/Battlement Emoji.asset",
    "samples/ui/Assets/Original/Battlement Emoji.asset.meta",
    "samples/ui/Assets/Original/Rocket Emoji.png",
    "samples/ui/Assets/Original/Rocket Emoji.png.meta",
    "samples/ui/Assets/Resources/BattlementTextSettings.asset",
    "samples/ui/Assets/Resources/BattlementTextSettings.asset.meta",
)


def select(repository: Path, paths: list[str], samples: list[str]) -> list[str]:
    """Return samples whose assembled native behavior can change."""
    normalized = {path.replace("\\", "/") for path in paths}
    if any(path == prefix or path.startswith(prefix) for path in normalized for prefix in GLOBAL_NATIVE_INPUTS):
        return samples
    selected = {
        sample
        for sample in samples
        if any(path.startswith(f"samples/{sample}/") for path in normalized)
    }
    changed_crates = {
        path.split("/", 2)[1]
        for path in normalized
        if path.startswith("crates/") and path.count("/") >= 2
    }
    if changed_crates:
        for sample in samples:
            if changed_crates & sample_crates(repository, sample):
                selected.add(sample)
    return [sample for sample in samples if sample in selected]


def sample_crates(repository: Path, sample: str) -> set[str]:
    """Return workspace crate directories in one sample's Cargo dependency graph."""
    result = subprocess.run(
        [
            "cargo",
            "metadata",
            "--format-version",
            "1",
            "--manifest-path",
            f"samples/{sample}/rules/Cargo.toml",
        ],
        cwd=repository,
        check=True,
        capture_output=True,
        text=True,
    )
    metadata = json.loads(result.stdout)
    roots = set()
    crates = (repository / "crates").resolve()
    for package in metadata["packages"]:
        manifest = Path(package["manifest_path"]).resolve()
        try:
            relative = manifest.relative_to(crates)
        except ValueError:
            continue
        roots.add(relative.parts[0])
    return roots
