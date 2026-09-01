#!/usr/bin/env python3
"""Verify that CI discovers every convention-based sample."""

from __future__ import annotations

from contextlib import nullcontext
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
from threading import Barrier, Lock
import tomllib
from unittest.mock import patch


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY_ROOT / "scripts"))
SPEC = importlib.util.spec_from_file_location("ci", REPOSITORY_ROOT / "scripts/ci.py")
assert SPEC and SPEC.loader
ci = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ci)


def main() -> None:
    assert "samples" in ci.ROOT_RUST_INPUTS
    with tempfile.TemporaryDirectory(prefix="battlement-ci-test.") as temporary:
        root = Path(temporary)
        _verify_cargo_target_isolation(root)
        _verify_parallel_sample_target_isolation(root)
        _verify_windows_paths(root)
        _verify_ditto_gate_contract()
        _verify_unity_project_regeneration(root)
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


def _verify_cargo_target_isolation(root: Path) -> None:
    cache = root / "cache"
    first_checkout = root / "checkout-a"
    second_checkout = root / "checkout-b"
    ci.CI_CACHE_ROOT = cache

    ci.REPOSITORY_ROOT = first_checkout
    first_root = Path(ci.cargo_environment(None)["CARGO_TARGET_DIR"])
    first_sample = Path(
        ci.cargo_environment(None, "standalone-basic")["CARGO_TARGET_DIR"]
    )
    first_sample_again = Path(
        ci.cargo_environment(None, "standalone-basic")["CARGO_TARGET_DIR"]
    )
    other_sample = Path(
        ci.cargo_environment(None, "standalone-chess")["CARGO_TARGET_DIR"]
    )

    ci.REPOSITORY_ROOT = second_checkout
    second_root = Path(ci.cargo_environment(None)["CARGO_TARGET_DIR"])
    second_sample = Path(
        ci.cargo_environment(None, "standalone-basic")["CARGO_TARGET_DIR"]
    )

    assert first_root != second_root
    assert first_sample != second_sample
    assert first_sample != first_root
    assert first_sample != other_sample
    assert first_sample == first_sample_again
    assert first_root.is_relative_to(cache / "cargo-targets")


def _verify_parallel_sample_target_isolation(root: Path) -> None:
    ci.REPOSITORY_ROOT = root / "checkout"
    ci.CI_CACHE_ROOT = root / "cache"
    barrier = Barrier(2)
    target_lock = Lock()
    targets: list[str] = []

    def run(command: list[str], **options: object) -> subprocess.CompletedProcess[str]:
        if command[0] == "cargo":
            environment = options["env"]
            assert isinstance(environment, dict)
            with target_lock:
                targets.append(environment["CARGO_TARGET_DIR"])
            barrier.wait(timeout=5)
            return subprocess.CompletedProcess(command, 0)
        return subprocess.CompletedProcess(command, 0, stdout="")

    class ImmediateCache:
        def run(self, _step: str, _inputs: tuple[str, ...], function: object) -> bool:
            assert callable(function)
            function()
            return True

    original_run = ci.subprocess.run
    original_lease = ci.unity_editor_lease
    original_workers = ci.standalone_sample_workers
    try:
        ci.subprocess.run = run
        ci.unity_editor_lease = nullcontext
        ci.standalone_sample_workers = lambda: 2
        with patch.object(ci.platform, "system", return_value="Windows"):
            ci.build_standalone_samples(["basic", "chess"], ImmediateCache())
    finally:
        ci.subprocess.run = original_run
        ci.unity_editor_lease = original_lease
        ci.standalone_sample_workers = original_workers

    assert len(targets) == 2
    assert targets[0] != targets[1]


def _verify_windows_paths(root: Path) -> None:
    program_files = root / "Program Files"
    with patch.dict(ci.os.environ, {"PROGRAMFILES": str(program_files)}):
        with patch.object(ci.platform, "system", return_value="Windows"):
            assert ci.unity_editor() == (
                program_files / "Unity/Hub/Editor/6000.5.8f1/Editor/Unity.exe"
            )
    expected = "fixture.exe" if ci.os.name == "nt" else "fixture"
    assert ci.executable_name("fixture") == expected


def _verify_ditto_gate_contract() -> None:
    config = tomllib.loads(
        (REPOSITORY_ROOT / ".tollgate/config.toml").read_text(encoding="utf-8")
    )
    assert [step["name"] for step in config["step"]] == ["ci"]
    assert config["step"][0]["run"] == "python3 scripts/ci.py --full"
    with patch.object(sys, "argv", ["ci.py", "--full"]):
        assert ci.parse_arguments().ditto is False
    with patch.object(sys, "argv", ["ci.py", "--ditto"]):
        assert ci.parse_arguments().ditto is True

    steps: list[tuple[str, list[str], dict[str, str]]] = []

    def record(
        name: str,
        command: list[str] | None = None,
        environment: dict[str, str] | None = None,
        **_options: object,
    ) -> None:
        assert command is not None
        assert environment is not None
        steps.append((name, command, environment))

    with patch.object(ci, "run_step", side_effect=record):
        ci.run_ditto_validation(1.25)

    commands = [command for _name, command, _environment in steps]
    assert commands == [[sys.executable, "scripts/ditto_ci.py", "gate"]]
    assert steps[0][2]["DITTO_CI_REUSABLE_BUILD_SECONDS"] == "1.25"


def _verify_unity_project_regeneration(root: Path) -> None:
    ci.REPOSITORY_ROOT = root
    project = root / "Assembly-CSharp-Editor.csproj"
    with patch.object(ci, "run_with_unity_lease") as run:
        ci.ensure_unity_project_files()
        run.assert_called_once_with(ci.run_unity_edit_mode_tests)
        project.touch()
        ci.ensure_unity_project_files()
        run.assert_called_once_with(ci.run_unity_edit_mode_tests)


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
