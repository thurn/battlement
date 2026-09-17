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
import web_selection

SPEC = importlib.util.spec_from_file_location("ci", REPOSITORY_ROOT / "scripts/ci.py")
assert SPEC and SPEC.loader
ci = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ci)


def main() -> None:
    _verify_rust_configuration()
    _verify_active_rust_toolchain_guard()
    with tempfile.TemporaryDirectory(prefix="battlement-ci-test.") as temporary:
        root = Path(temporary)
        _verify_focused_cargo_target(root)
        _verify_lockfile_preflight(root)
        _verify_cargo_target_isolation(root)
        _verify_cargo_targets_do_not_cross_checkouts(root)
        _verify_parallel_sample_target_isolation(root)
        _verify_sample_worker_defaults()
        _verify_windows_paths(root)
        _verify_ditto_gate_contract()
        _verify_selected_native_execution()
        _verify_ditto_build_leases_span_gate(root)
        assert ci.build_standalone_samples([], object()) == 0
        assert ci.select_native_samples(["scripts/ci.py"], ["basic"]) == []
        _verify_unity_project_generation(root)
        _verify_csharp_preflight()
        _verify_csharp_line_length_exclusions(root)
        _verify_unity_execution_selection(root)
        _verify_runtime_checks_overlap()
        _verify_unity_native_diagnostics_selection()
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


def _verify_focused_cargo_target(root: Path) -> None:
    environment = {**os.environ, "BATTLEMENT_CI_CACHE": str(root / "focused-cache")}
    command = [sys.executable, str(REPOSITORY_ROOT / "scripts/cargo_target.py")]
    targets = []
    for arguments in ([], ["samples/reactant/rules/Cargo.toml"]):
        result = subprocess.run(
            [*command, *arguments], env=environment, check=True, capture_output=True, text=True,
        )
        target = Path(result.stdout.strip())
        assert target.is_dir() and target.is_relative_to(root / "focused-cache")
        targets.append(target)
    assert targets[0] != targets[1]
    for manifest in (str(root / "Cargo.toml"), "crates/reactant-core/Cargo.toml"):
        result = subprocess.run(
            [*command, manifest], env=environment, capture_output=True, text=True,
        )
        assert result.returncode == 2 and not result.stdout


def _verify_lockfile_preflight(root: Path) -> None:
    checkout = root / "lockfile-preflight"
    sample = Path("samples/fixture/rules/Cargo.toml")
    _cargo_test_checkout(checkout, "fn main() {}\n")
    _cargo_test_checkout(checkout / sample.parent, "fn main() {}\n")
    for manifest in [Path("Cargo.toml"), sample]:
        subprocess.run(
            ["cargo", "generate-lockfile", "--manifest-path", str(manifest)],
            cwd=checkout, check=True, capture_output=True,
        )
    dependency = checkout / sample.parent / "dependency"
    _manifest(dependency, '[package]\nname = "dependency"\nversion = "0.1.0"\nedition = "2024"\n')
    (dependency / "src").mkdir()
    (dependency / "src/lib.rs").write_text("pub fn value() -> u32 { 1 }\n")
    manifest = checkout / sample.parent / "app/Cargo.toml"
    manifest.write_text(manifest.read_text() + '\n[dependencies]\ndependency = { path = "../dependency" }\n')
    lock = checkout / sample.parent / "Cargo.lock"
    original = lock.read_bytes()
    with patch.object(ci, "REPOSITORY_ROOT", checkout):
        try:
            ci.check_cargo_lockfiles([sample])
        except RuntimeError as error:
            assert str(sample) in str(error)
        else:
            raise AssertionError("stale sample lock passed preflight")
        assert lock.read_bytes() == original
        assert not list(checkout.rglob("target"))
        subprocess.run(
            ["cargo", "generate-lockfile", "--manifest-path", str(sample)],
            cwd=checkout, check=True, capture_output=True,
        )
        ci.check_cargo_lockfiles([sample])


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
    original_transaction = ci.unity_project_transaction
    original_workers = ci.standalone_sample_workers
    try:
        ci.subprocess.run = run
        ci.unity_editor_lease = nullcontext
        ci.unity_project_transaction = lambda *_arguments: nullcontext()
        ci.standalone_sample_workers = lambda: 2
        with patch.object(ci.platform, "system", return_value="Windows"):
            ci.build_standalone_samples(["basic", "chess"], ImmediateCache())
    finally:
        ci.subprocess.run = original_run
        ci.unity_editor_lease = original_lease
        ci.unity_project_transaction = original_transaction
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
    assert [step["name"] for step in config["step"]] == ["prose", "ci"]
    prose, full_ci = config["step"]
    assert prose["run"] == "python3 scripts/prose_validation.py --tollgate-evidence"
    assert prose["include"] == ["plans/workflow-performance.md"]
    assert prose["include_mode"] == "all"
    assert full_ci["run"] == (
        "rustup run 1.98.1 python3 scripts/ci.py --full --tollgate-evidence"
    )
    assert full_ci["exclude"] == prose["include"]
    assert full_ci["exclude_mode"] == "all"
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

    with (
        patch.object(ci.platform, "system", return_value="Darwin"),
        patch.object(ci, "run_step", side_effect=record),
    ):
        ci.run_ditto_validation(1.25)

    commands = [command for _name, command, _environment in steps]
    assert commands == [[
        "/usr/bin/caffeinate", "-u", "-d", "-i", "--",
        sys.executable, "scripts/ditto_ci.py", "gate",
    ]]
    assert steps[0][2]["DITTO_CI_REUSABLE_BUILD_SECONDS"] == "1.25"


def _verify_ditto_build_leases_span_gate(root: Path) -> None:
    class Leases:
        cache_root = root / "ditto-cache"

        def __init__(self) -> None:
            self.prepared: list[str] = []
            self.checked = False

        def prepare(self, sample: str) -> None:
            self.prepared.append(sample)

        def assert_healthy(self) -> None:
            self.checked = True

    leases = Leases()
    commands: list[list[str]] = []

    def completed(command: list[str], **_options: object) -> subprocess.CompletedProcess[str]:
        commands.append(command)
        return subprocess.CompletedProcess(command, 0, stdout="")

    with (
        patch.object(ci.platform, "system", return_value="Darwin"),
        patch.object(ci.subprocess, "run", side_effect=completed),
    ):
        ci.build_standalone_samples(["basic", "chess"], object(), leases)
    assert commands[0] == ["cargo", "build", "-p", "rt"]
    assert sorted(leases.prepared) == ["basic", "chess"]

    steps: list[tuple[list[str], dict[str, str]]] = []

    def record(
        _name: str,
        command: list[str] | None = None,
        environment: dict[str, str] | None = None,
        **_options: object,
    ) -> None:
        assert command is not None and environment is not None
        steps.append((command, environment))

    with patch.object(ci, "run_step", side_effect=record):
        ci.run_ditto_validation(2.5, leases, "retained-invocation")
    assert leases.checked
    assert steps[0][1]["DITTO_CI_CACHE_ROOT"] == str(leases.cache_root)
    assert steps[0][1]["DITTO_CI_BINARY"] == str(leases.binary)
    assert steps[0][1]["DITTO_CI_INVOCATION_ID"] == "retained-invocation"


def _verify_selected_native_execution() -> None:
    class RustSelection:
        root = False
        samples: tuple[Path, ...] = ()

        def report(self) -> dict[str, object]:
            return {"root": False, "samples": []}

    class Cache:
        def __init__(self, *_arguments: object, **_options: object) -> None:
            pass

        def run(self, _step: str, _inputs: tuple[str, ...], function: object) -> bool:
            assert callable(function)
            function()
            return True

    class DittoLeases:
        cache_root = Path("/tmp/battlement-ci-test-ditto")

        def __init__(self, *_arguments: object) -> None:
            pass

        def close(self) -> None:
            pass

    native_runs: list[tuple[str, ...]] = []

    def record_step(
        _name: str,
        _command: list[str] | None = None,
        *,
        function: object | None = None,
        **_options: object,
    ) -> float:
        if function is not None:
            assert callable(function)
            function()
        return 0.0

    def record_ditto(*_arguments: object, **options: object) -> None:
        samples = options.get("samples")
        assert isinstance(samples, list)
        native_runs.append(tuple(samples))

    with (
        patch.object(ci, "CiCache", Cache),
        patch.object(ci, "sample_names", return_value=["ui"]),
        patch.object(ci, "sample_rust_workspaces", return_value=[]),
        patch.object(ci.ci_selection, "select_rust", return_value=RustSelection()),
        patch.object(ci.ci_selection, "select_reactant_assets", return_value=([], [])),
        patch.object(
            ci.unity_test_selection,
            "select",
            return_value=ci.unity_test_selection.Selection(
                ci.unity_test_selection.Scope.NONE,
                (),
                ("no Unity input changed",),
                (),
                False,
            ),
        ),
        patch.object(
            web_selection,
            "changed_paths",
            return_value=("HEAD", ["samples/ui/rules/src/lib.rs"]),
        ),
        patch.object(web_selection, "validate_affected"),
        patch.object(ci, "run_csharp_preflight"),
        patch.object(ci, "check_cargo_lockfiles"),
        patch.object(ci, "lint_rust_workspaces"),
        patch.object(ci, "test_rust_workspaces", return_value=0.0),
        patch.object(ci.ci_tooling, "run") as tooling,
        patch.object(ci, "run_selected_unity_tests", return_value=0.0),
        patch.object(ci, "build_standalone_samples", return_value=0.0),
        patch.object(ci, "run_ditto_validation", side_effect=record_ditto),
        patch.object(ci, "DittoBuildLeases", DittoLeases),
        patch.object(ci.ditto_evidence, "invocation_root", return_value=Path("/tmp")),
        patch.object(ci, "refresh_tracked_file_metadata"),
        patch.object(ci, "run_step", side_effect=record_step),
        patch.object(ci.platform, "system", return_value="Darwin"),
    ):
        ci.run_ci(full=True, use_ci_cache=False, ditto=True)

    assert native_runs == [("ui",)]
    tooling.assert_called_once_with(ci.REPOSITORY_ROOT, performance=True)


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
        fixture = root / "crates/reactant-core/tests/fixtures/asset-registry"
        fixture.mkdir(parents=True)
        (fixture / "Cargo.toml").write_text(
            '[workspace]\n[workspace.package]\nrust-version = "1.98.1"\n'
        )
        (root / ".tollgate/config.toml").write_text(
            '[[step]]\nname = "prose"\n'
            'run = "python3 scripts/prose_validation.py --tollgate-evidence"\n'
            'include = ["plans/workflow-performance.md"]\ninclude_mode = "all"\n'
            '[[step]]\nname = "ci"\n'
            'run = "rustup run 1.99.0 python3 scripts/ci.py --full --tollgate-evidence"\n'
            'exclude = ["plans/workflow-performance.md"]\nexclude_mode = "all"\n'
        )
        try:
            ci.REPOSITORY_ROOT = root
            errors = ci.rust_configuration_errors()
        finally:
            ci.REPOSITORY_ROOT = original_root
    assert errors == [
        "Tollgate invokes 'rustup run 1.99.0 python3 scripts/ci.py --full --tollgate-evidence'; "
        "expected 'rustup run 1.98.1 python3 scripts/ci.py --full --tollgate-evidence'"
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


def _verify_unity_project_generation(root: Path) -> None:
    ci.REPOSITORY_ROOT = root
    project = root / "Assembly-CSharp-Editor.csproj"
    project.unlink(missing_ok=True)

    class Transaction:
        def run(
            self, command: list[str], **_options: object
        ) -> subprocess.CompletedProcess[str]:
            assert "-runTests" not in command
            assert "-quit" in command
            project.touch()
            return subprocess.CompletedProcess(command, 0)

    with (
        patch.object(ci, "unity_editor", return_value=Path(sys.executable)),
        patch.object(
            ci,
            "unity_project_transaction",
            return_value=nullcontext(Transaction()),
        ),
        patch.object(ci, "wait_for_unity_project_unlock"),
    ):
        ci.generate_unity_project_files()

    lock = root / "Temp/UnityLockfile"

    class FailedTransaction:
        def run(
            self, command: list[str], **_options: object
        ) -> subprocess.CompletedProcess[str]:
            lock.parent.mkdir(parents=True, exist_ok=True)
            lock.touch()
            Path(command[command.index("-logFile") + 1]).write_text(
                "compiler failure\n", encoding="utf-8"
            )
            return subprocess.CompletedProcess(command, 1)

    with (
        patch.object(ci, "unity_editor", return_value=Path(sys.executable)),
        patch.object(
            ci,
            "unity_project_transaction",
            return_value=nullcontext(FailedTransaction()),
        ),
    ):
        try:
            ci.generate_unity_project_files()
        except RuntimeError as error:
            assert "project-file generation failed" in str(error)
        else:
            raise AssertionError("failed Unity project generation was accepted")
    assert not lock.exists()


def _verify_csharp_preflight() -> None:
    selection = ci.unity_test_selection.Selection(
        ci.unity_test_selection.Scope.ALL,
        ci.unity_test_selection.FULL_ASSEMBLIES,
        ("C# changed",),
        ("Packages",),
        True,
    )

    class Cache:
        def run(self, step: str, inputs: tuple[str, ...], function: object) -> bool:
            assert step == "dotnet-diagnostics"
            assert inputs == ci.unity_test_selection.DOTNET_DIAGNOSTIC_INPUTS
            assert function is ci.check_dotnet_diagnostics
            return True

    steps: list[str] = []
    with patch.object(
        ci,
        "run_step",
        side_effect=lambda name, *_arguments, **_options: steps.append(name),
    ):
        ci.run_csharp_preflight([], selection, Cache())
    assert steps == [
        "Restore local .NET tools",
        "Check C# formatting",
        "Check C# line lengths",
        "Check sample runtime preflight",
        "Check samples have no C#",
        "Check .NET diagnostics",
    ]


def _verify_csharp_line_length_exclusions(root: Path) -> None:
    package = root / "Packages/com.battlement.client"
    generated = package / "Runtime/FlatBuffers/Generated/value_generated.cs"
    vendored = package / "Runtime/FlatBuffers/Google/Table.cs"
    generated.parent.mkdir(parents=True, exist_ok=True)
    vendored.parent.mkdir(parents=True, exist_ok=True)
    generated.write_text("x" * 101, encoding="utf-8")
    vendored.write_text("x" * 101, encoding="utf-8")
    with patch.object(ci, "REPOSITORY_ROOT", root):
        ci.check_csharp_line_lengths([])


def _verify_unity_execution_selection(root: Path) -> None:
    class Cache:
        def __init__(self) -> None:
            self.calls: list[tuple[str, tuple[str, ...]]] = []

        def run(
            self, step: str, inputs: tuple[str, ...], function: object, *, lease: object,
        ) -> bool:
            assert lease is ci.unity_editor_lease
            self.calls.append((step, inputs))
            assert callable(function)
            function()
            return True

    skipped = ci.unity_test_selection.Selection(
        ci.unity_test_selection.Scope.NONE,
        (),
        ("irrelevant",),
        ("Packages",),
        False,
    )
    cache = Cache()
    with patch.object(ci, "run_step") as run:
        assert ci.run_selected_unity_tests(skipped, cache) == 0
        run.assert_not_called()
    assert cache.calls == []

    native = ci.unity_test_selection.Selection(
        ci.unity_test_selection.Scope.NATIVE,
        ci.unity_test_selection.NATIVE_ASSEMBLIES,
        ("native",),
        ("Packages", "crates/battlement-native"),
        False,
    )
    executed: list[tuple[str, ...]] = []

    def step(_name: str, *, function: object) -> float:
        assert callable(function)
        function()
        return 1.5

    with (
        patch.object(ci, "run_step", side_effect=step),
        patch.object(ci, "run_with_unity_lease", side_effect=lambda function: function()),
        patch.object(ci, "run_unity_edit_mode_tests", side_effect=executed.append),
    ):
        assert ci.run_selected_unity_tests(native, cache) == 1.5
    assert executed == [ci.unity_test_selection.NATIVE_ASSEMBLIES]
    assert cache.calls[-1][0] == "unity-edit-mode-native-integration"


def _verify_runtime_checks_overlap() -> None:
    for failure in (None, "rust", "unity"):
        barrier = Barrier(2, timeout=5)
        completed: list[str] = []

        def execute(name: str) -> float:
            barrier.wait()
            completed.append(name)
            if failure == name:
                raise RuntimeError(name)
            return 1.5

        with (
            patch.object(ci, "test_rust_workspaces", side_effect=lambda *_args: execute("rust")),
            patch.object(ci, "run_selected_unity_tests", side_effect=lambda *_args: execute("unity")),
        ):
            try:
                _rust, unity = ci.test_runtime_integrations(object(), object(), object())
                assert failure is None
                assert unity == 1.5
            except RuntimeError as error:
                assert failure is not None and str(error) == failure
        assert sorted(completed) == ["rust", "unity"]


def _verify_unity_native_diagnostics_selection() -> None:
    assert ci.native_fixture_diagnostics_passed(
        ci.unity_test_selection.INTEGRATION_ASSEMBLIES,
        "",
    )
    assert not ci.native_fixture_diagnostics_passed(
        ci.unity_test_selection.NATIVE_ASSEMBLIES,
        "",
    )
    assert ci.native_fixture_diagnostics_passed(
        ci.unity_test_selection.NATIVE_ASSEMBLIES,
        "Preparing fixture connect panic\n"
        "Triggering fixture connect panic\n"
        "panicked at crates/battlement-native/tests/fixtures/exported-engine",
    )


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
