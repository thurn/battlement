#!/usr/bin/env python3
"""Verify that CI discovers every convention-based sample."""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import sys
import tempfile


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY_ROOT / "scripts"))
SPEC = importlib.util.spec_from_file_location("ci", REPOSITORY_ROOT / "scripts/ci.py")
assert SPEC and SPEC.loader
ci = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ci)


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="battlement-ci-test.") as temporary:
        root = Path(temporary)
        for name in ("tictactoe", "basic", "chess"):
            sample = root / "samples" / name
            sample.mkdir(parents=True)
            (sample / "sample.toml").write_text(f'executable = "{name}"\n')
        (root / "samples/incomplete").mkdir()

        _workspace(root / "samples/basic/rules")
        _workspace(root / "samples/standalone/engine")
        _manifest(root / "samples/standalone/engine/member", "[package]\nname = \"member\"\n")
        _workspace(root / "samples/ignored/target/generated")
        _workspace(root / "samples/ignored/Build/generated")
        _manifest(root / "samples/not-standalone", "[package]\nname = \"member\"\n")
        _manifest(root / "samples/workspace-fields", "[workspace.package]\nedition = \"2024\"\n")
        _manifest(root / "samples/commented-workspace", "# [workspace]\n")
        _manifest(root / "samples/quoted-workspace", "['workspace'] # standalone\n")

        ci.REPOSITORY_ROOT = root
        assert ci.sample_names() == ["basic", "chess", "tictactoe"]
        assert ci.sample_rust_workspaces() == [
            Path("samples/basic/rules/Cargo.toml"),
            Path("samples/quoted-workspace/Cargo.toml"),
            Path("samples/standalone/engine/Cargo.toml"),
        ]

        package = root / "Packages/com.battlement.client"
        _runtime_ui_package(package)
        ci.validate_runtime_ui_package(package)
        runtime_assembly = package / "Runtime/Battlement.Runtime.asmdef"
        runtime_assembly.write_text(json.dumps({"references": ["Battlement.Protocol"]}))
        try:
            ci.validate_runtime_ui_package(package)
        except RuntimeError as error:
            assert "Battlement.UI" in str(error)
        else:
            raise AssertionError("missing runtime UI assembly edge was accepted")


def _workspace(root: Path) -> None:
    _manifest(root, "[workspace]\n")


def _manifest(root: Path, contents: str) -> None:
    root.mkdir(parents=True)
    (root / "Cargo.toml").write_text(contents)


def _runtime_ui_package(package: Path) -> None:
    resources = package / "Runtime/UI/Resources"
    resources.mkdir(parents=True)
    (resources / "BattlementPanelSettingsTemplate.asset").write_text(
        "  themeUss: {fileID: -4733365628477956816, "
        "guid: 11111111111111111111111111111111, type: 3}\n"
        "  m_ScaleMode: 0\n"
    )
    (resources / "BattlementPanelSettingsTemplate.asset.meta").write_text(
        "guid: 22222222222222222222222222222222\n"
    )
    (resources / "BattlementRuntimeTheme.tss").write_text("")
    (resources / "BattlementRuntimeTheme.tss.meta").write_text(
        "guid: 11111111111111111111111111111111\n"
    )
    (package / "Runtime/Battlement.Runtime.asmdef").write_text(
        json.dumps({"references": ["Battlement.Protocol", "Battlement.UI"]})
    )
    (package / "Runtime/UI/Battlement.UI.asmdef").write_text(
        json.dumps(
            {
                "rootNamespace": "Battlement.UI",
                "references": ["Battlement.Protocol"],
            }
        )
    )


if __name__ == "__main__":
    main()
