#!/usr/bin/env python3
"""Verify that CI discovers every convention-based sample."""

from __future__ import annotations

from contextlib import nullcontext
import importlib.util
import json
import os
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
    _verify_rust_configuration()
    _verify_active_rust_toolchain_guard()
    with tempfile.TemporaryDirectory(prefix="battlement-ci-test.") as temporary:
        root = Path(temporary)
        _verify_cargo_target_isolation(root)
        _verify_cargo_targets_do_not_cross_checkouts(root)
        _verify_parallel_sample_target_isolation(root)
        _verify_sample_worker_defaults()
        _verify_windows_paths(root)
        _verify_ditto_gate_contract()
        _verify_unity_project_regeneration(root)
        for name in ("tictactoe", "basic", "chess", "chess-ui"):
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
        assert ci.sample_names() == ["basic", "chess", "chess-ui", "tictactoe"]
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
    os.utime(first_root, ns=(1, 1))
    first_access = first_root.stat().st_mtime_ns
    ci.cargo_environment(None)
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
    assert first_root.is_relative_to(cache / "cargo-targets/shared")
    assert second_root.is_relative_to(cache / "cargo-targets/shared")
    assert first_root.stat().st_mtime_ns > first_access


def _verify_cargo_targets_do_not_cross_checkouts(root: Path) -> None:
    cache = root / "cross-checkout-cache"
    old_checkout = root / "old-checkout"
    new_checkout = root / "new-checkout"
    _cargo_test_checkout(old_checkout, "fn main() {}\n")
    _cargo_test_checkout(
        new_checkout,
        """fn main() {}

#[cfg(test)]
mod tests {
    #[test]
    fn new_checkout_test_runs() {
        panic!("new checkout test executed");
    }
}
""",
    )
    ci.CI_CACHE_ROOT = cache
    ci.REPOSITORY_ROOT = old_checkout
    old_environment = ci.cargo_environment(None)
    subprocess.run(
        ["cargo", "test", "--workspace", "--quiet"],
        cwd=old_checkout,
        env=old_environment,
        check=True,
        capture_output=True,
        text=True,
    )
    for path in new_checkout.rglob("*"):
        if path.is_file():
            os.utime(path, ns=(1, 1))

    ci.REPOSITORY_ROOT = new_checkout
    new_environment = ci.cargo_environment(None)
    assert old_environment["CARGO_TARGET_DIR"] != new_environment["CARGO_TARGET_DIR"]
    try:
        subprocess.run(
            ["cargo", "test", "--workspace", "--quiet"],
            cwd=new_checkout,
            env=new_environment,
            check=True,
            capture_output=True,
            text=True,
        )
    except subprocess.CalledProcessError as error:
        output = error.stdout + error.stderr
        assert "new_checkout_test_runs" in output
        assert "new checkout test executed" in output
    else:
        raise AssertionError("new checkout reused an older checkout's test binary")


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


def _verify_sample_worker_defaults() -> None:
    with patch.dict(ci.os.environ, {}, clear=True):
        with patch.object(ci.platform, "system", return_value="Windows"):
            assert ci.standalone_sample_workers() == 1
        with patch.object(ci.platform, "system", return_value="Linux"):
            assert ci.standalone_sample_workers() == 4
    with patch.dict(ci.os.environ, {"BATTLEMENT_CI_SAMPLE_WORKERS": "3"}):
        assert ci.standalone_sample_workers() == 3


def _verify_ditto_gate_contract() -> None:
    config = tomllib.loads(
        (REPOSITORY_ROOT / ".tollgate/config.toml").read_text(encoding="utf-8")
    )
    assert [step["name"] for step in config["step"]] == ["ci"]
    assert config["step"][0]["run"] == (
        "rustup run 1.98.1 python3 scripts/ci.py --full"
    )
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


def _verify_rust_configuration() -> None:
    assert ci.rust_configuration_errors() == []
    original_root = ci.REPOSITORY_ROOT
    with tempfile.TemporaryDirectory(prefix="battlement-rust-config-test.") as temporary:
        root = Path(temporary)
        (root / ".tollgate").mkdir()
        (root / "rust-toolchain.toml").write_text(
            '[toolchain]\nchannel = "1.98.1"\ncomponents = ["clippy", "rustfmt"]\n'
        )
        (root / "Cargo.toml").write_text(
            '[workspace]\n[workspace.package]\nrust-version = "1.98.1"\n'
        )
        fixture = root / "crates/battlement-reactant/tests/fixtures/asset-registry"
        fixture.mkdir(parents=True)
        (fixture / "Cargo.toml").write_text(
            '[workspace]\n[workspace.package]\nrust-version = "1.98.1"\n'
        )
        (root / ".tollgate/config.toml").write_text(
            '[[step]]\nname = "ci"\n'
            'run = "rustup run 1.99.0 python3 scripts/ci.py --full"\n'
        )
        try:
            ci.REPOSITORY_ROOT = root
            errors = ci.rust_configuration_errors()
        finally:
            ci.REPOSITORY_ROOT = original_root
    assert errors == [
        "Tollgate invokes 'rustup run 1.99.0 python3 scripts/ci.py --full'; "
        "expected 'rustup run 1.98.1 python3 scripts/ci.py --full'"
    ]


def _verify_active_rust_toolchain_guard() -> None:
    outputs = {
        ("rustup", "show", "active-toolchain"): "1.99.0-test-target (override)",
        ("rustc", "--version"): "rustc 1.98.1 (48a229cea 2026-09-01)",
        ("cargo", "--version"): "cargo 1.98.1 (797e8a9bc 2026-08-05)",
        ("cargo", "clippy", "--version"): (
            "clippy 0.1.98 (48a229ceae 2026-09-01)"
        ),
        ("cargo", "fmt", "--version"): (
            "rustfmt 1.9.0-stable (48a229ceae 2026-09-01)"
        ),
        ("rustc", "-Vv"): (
            "rustc 1.98.1 (48a229cea 2026-09-01)\n"
            "commit-hash: 48a229ceae2c56c759ab0e8d56ebd5e4d7018d57"
        ),
    }
    with patch.object(
        ci,
        "command_output",
        side_effect=lambda command: outputs[tuple(command)],
    ):
        try:
            ci.check_rust_toolchain()
        except RuntimeError as error:
            assert "active rustup toolchain is '1.99.0-test-target'" in str(error)
        else:
            raise AssertionError("mismatched active Rust toolchain was accepted")


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


def _cargo_test_checkout(root: Path, source: str) -> None:
    _manifest(root, '[workspace]\nmembers = ["app"]\nresolver = "3"\n')
    _manifest(
        root / "app",
        '[package]\nname = "app"\nversion = "0.1.0"\nedition = "2024"\n',
    )
    source_root = root / "app/src"
    source_root.mkdir()
    (source_root / "main.rs").write_text(source)


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
