#!/usr/bin/env python3

"""Stable, inexpensive contracts shared by sample CI and visual capture."""

from __future__ import annotations

import json
from pathlib import Path
import re
import subprocess


RUNTIME_UI_ASSETS = (
    "Runtime/UI/Resources/BattlementPanelSettingsTemplate.asset",
    "Runtime/UI/Resources/BattlementPanelSettingsTemplate.asset.meta",
    "Runtime/UI/Resources/BattlementRuntimeTheme.tss",
    "Runtime/UI/Resources/BattlementRuntimeTheme.tss.meta",
)
REQUIRED_ASSEMBLY_REFERENCES = {
    "Runtime/Battlement.Runtime.asmdef": {"Battlement.Protocol", "Battlement.UI"},
    "Runtime/UI/Battlement.UI.asmdef": {"Battlement.Protocol"},
}


def validate_sample_input_backend(project: Path) -> None:
    """Require the Input System backend used by Battlement and capture."""
    settings_path = project / "ProjectSettings/ProjectSettings.asset"
    settings = settings_path.read_text(encoding="utf-8")
    if "  activeInputHandler: 1\n" not in settings:
        raise RuntimeError(
            f"{project.name}: {settings_path} must enable Unity's Input System backend."
        )


def validate_runtime_ui_package(package: Path, repository_root: Path | None = None) -> None:
    """Require Battlement's committed runtime UI assets and assembly edges."""
    missing = [relative for relative in RUNTIME_UI_ASSETS if not (package / relative).is_file()]
    if missing:
        raise RuntimeError("Runtime UI package assets are missing:\n" + "\n".join(missing))

    panel_path = package / RUNTIME_UI_ASSETS[0]
    theme_meta_path = package / RUNTIME_UI_ASSETS[3]
    panel = panel_path.read_text(encoding="utf-8")
    theme_meta = theme_meta_path.read_text(encoding="utf-8")
    theme_guid = _required_match(theme_meta, r"^guid: ([0-9a-f]{32})$", theme_meta_path)
    if f"themeUss: {{fileID: -4733365628477956816, guid: {theme_guid}, type: 3}}" not in panel:
        raise RuntimeError(f"{panel_path} must reference the committed runtime theme.")
    if "  m_ScaleMode: 0\n" not in panel:
        raise RuntimeError(f"{panel_path} must use deterministic ConstantPixelSize scaling.")

    for relative, required in REQUIRED_ASSEMBLY_REFERENCES.items():
        path = package / relative
        assembly = json.loads(path.read_text(encoding="utf-8"))
        references = set(assembly.get("references", []))
        if missing_references := sorted(required - references):
            raise RuntimeError(
                f"{path} is missing required assembly references: "
                + ", ".join(missing_references)
            )
        if relative == "Runtime/UI/Battlement.UI.asmdef":
            if assembly.get("rootNamespace") != "Battlement.UI":
                raise RuntimeError(f"{path} must use the Battlement.UI root namespace.")

    if repository_root is not None:
        tracked = [
            str(path.relative_to(repository_root))
            for relative in (*RUNTIME_UI_ASSETS, *REQUIRED_ASSEMBLY_REFERENCES)
            for path in (package / relative,)
        ]
        result = subprocess.run(
            ["git", "ls-files", "--error-unmatch", "--", *tracked],
            cwd=repository_root,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        if result.returncode != 0:
            raise RuntimeError("Runtime UI assets and assembly definitions must be committed.")


def _required_match(contents: str, pattern: str, path: Path) -> str:
    match = re.search(pattern, contents, re.MULTILINE)
    if match is None:
        raise RuntimeError(f"{path} does not contain the required metadata.")
    return match.group(1)
