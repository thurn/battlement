#!/usr/bin/env python3

"""Run Battlement's complete local continuous-integration suite."""

from __future__ import annotations

import argparse
from collections.abc import Callable
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import tomllib
import uuid

import ditto_evidence
import tollgate_evidence

from ci_cache import CiCache
import ci_steps
import ci_selection
from ci_steps import run_parallel_steps, run_step
from ditto_build_leases import DittoBuildLeases
from platform_support import (
    executable_name,
    readline_with_timeout,
    resolve_executable,
    user_cache_path,
)
from sample_validation import validate_runtime_ui_package, validate_sample_input_backend
import resource_slots
from resource_slots import unity_editor_lease
from unity_transaction import recover_unity_transactions, unity_project_transaction
import perf_log
import prose_validation
import unity_test_selection
import native_validation_selection


REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
UNITY_VERSION = "6000.5.8f1"
RUST_VERSION = "1.98.1"
CLIPPY_VERSION = "0.1.98"
RUSTFMT_VERSION = "1.9.0-stable"
RUST_COMPONENTS = {"clippy", "rustfmt"}
RUST_VERSION_MANIFESTS = (
    "Cargo.toml",
    "crates/battlement-reactant/tests/fixtures/asset-registry/Cargo.toml",
)
TOLLGATE_CI_COMMAND = f"rustup run {RUST_VERSION} python3 scripts/ci.py --full --tollgate-evidence"
TOLLGATE_PROSE_COMMAND = "python3 scripts/prose_validation.py --tollgate-evidence"
CI_CACHE_ROOT = Path(
    os.environ.get(
        "BATTLEMENT_CI_CACHE",
        user_cache_path("Battlement", "ci-cache"),
    )
)
DEFAULT_STANDALONE_SAMPLE_WORKERS = 4
WINDOWS_STANDALONE_SAMPLE_WORKERS = 1
RUST_WORKSPACE_WORKERS = 2
DEFAULT_CARGO_JOBS = 3
ROOT_RUST_INPUTS = (
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "crates",
    "samples",
    "scripts/ci.py",
    "scripts/ci_cache.py",
    "scripts/ci_steps.py",
    "scripts/perf_log.py",
    "scripts/resource_slots.py",
)
SAMPLE_SHARED_INPUTS = (
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "Packages/com.battlement.client",
    "crates",
    "scripts/ci.py",
    "scripts/ci_cache.py",
    "scripts/ci_steps.py",
    "scripts/perf_log.py",
    "scripts/resource_slots.py",
)
IGNORED_SAMPLE_PROJECT_DIRECTORIES = {
    ".git",
    ".worktrees",
    "Build",
    "Library",
    "Logs",
    "Temp",
    "build",
    "obj",
    "target",
}
CARGO_WORKSPACE_TABLE = re.compile(
    r'''(?m)^[ \t]*\[[ \t]*(?:workspace|"workspace"|'workspace')[ \t]*\]'''
    r"[ \t]*(?:#[^\r\n]*)?$"
)
DITTO_SAMPLES = ("basic", "tictactoe", "reactant", "chess", "ui", "chess-ui")
DITTO_ADAPTERS = ("webgl", "ios")


def check_rust_toolchain() -> None:
    """Require matching repository, Tollgate, and active Rust toolchains."""
    configuration_errors = rust_configuration_errors()
    if configuration_errors:
        raise RuntimeError(
            "Rust toolchain configuration mismatch:\n- "
            + "\n- ".join(configuration_errors)
        )

    commands = {
        "rustc": (["rustc", "--version"], f"rustc {RUST_VERSION} "),
        "Cargo": (["cargo", "--version"], f"cargo {RUST_VERSION} "),
        "Clippy": (["cargo", "clippy", "--version"], f"clippy {CLIPPY_VERSION} "),
        "rustfmt": (["cargo", "fmt", "--version"], f"rustfmt {RUSTFMT_VERSION} "),
    }
    active = command_output(["rustup", "show", "active-toolchain"])
    active_name = active.partition(" ")[0]
    errors = []
    if active_name != RUST_VERSION and not active_name.startswith(f"{RUST_VERSION}-"):
        errors.append(
            f"active rustup toolchain is {active_name!r}; expected {RUST_VERSION!r}"
        )
    for name, (command, expected_prefix) in commands.items():
        output = command_output(command)
        if not output.startswith(expected_prefix):
            errors.append(
                f"{name} reported {output!r}; expected a version starting with "
                f"{expected_prefix!r}"
            )
    rustc_verbose = command_output(["rustc", "-Vv"])
    commit = re.search(r"(?m)^commit-hash: ([0-9a-f]+)$", rustc_verbose)
    if commit is None:
        errors.append("rustc -Vv did not report a commit hash")
    else:
        commit_prefix = commit.group(1)[:10]
        for name, command in (
            ("Clippy", ["cargo", "clippy", "--version"]),
            ("rustfmt", ["cargo", "fmt", "--version"]),
        ):
            output = command_output(command)
            if f"({commit_prefix}" not in output:
                errors.append(
                    f"{name} was not built from Rust {RUST_VERSION} commit "
                    f"{commit_prefix}"
                )
    if errors:
        raise RuntimeError(
            f"Rust {RUST_VERSION} toolchain check failed:\n- " + "\n- ".join(errors)
        )


def rust_configuration_errors() -> list[str]:
    """Return drift between the Rust pin, MSRV, components, and Tollgate."""
    errors = []
    toolchain = tomllib.loads(
        (REPOSITORY_ROOT / "rust-toolchain.toml").read_text(encoding="utf-8")
    )["toolchain"]
    if toolchain.get("channel") != RUST_VERSION:
        errors.append(
            f"rust-toolchain.toml pins {toolchain.get('channel')!r}; "
            f"expected {RUST_VERSION!r}"
        )
    components = set(toolchain.get("components", []))
    if components != RUST_COMPONENTS:
        errors.append(
            "rust-toolchain.toml components are "
            f"{sorted(components)!r}; expected {sorted(RUST_COMPONENTS)!r}"
        )

    for manifest in RUST_VERSION_MANIFESTS:
        cargo = tomllib.loads(
            (REPOSITORY_ROOT / manifest).read_text(encoding="utf-8")
        )
        rust_version = cargo.get("workspace", {}).get("package", {}).get("rust-version")
        if rust_version != RUST_VERSION:
            errors.append(
                f"{manifest} declares rust-version {rust_version!r}; "
                f"expected {RUST_VERSION!r}"
            )

    tollgate = tomllib.loads(
        (REPOSITORY_ROOT / ".tollgate/config.toml").read_text(encoding="utf-8")
    )
    steps = tollgate.get("step", [])
    ci_steps = [step for step in steps if step.get("name") == "ci"]
    if len(ci_steps) != 1:
        errors.append(f"Tollgate defines {len(ci_steps)} CI steps named 'ci'; expected one")
    elif ci_steps[0].get("run") != TOLLGATE_CI_COMMAND:
        errors.append(
            f"Tollgate invokes {ci_steps[0].get('run')!r}; "
            f"expected {TOLLGATE_CI_COMMAND!r}"
        )
    prose_steps = [step for step in steps if step.get("name") == "prose"]
    if len(prose_steps) != 1:
        errors.append(
            f"Tollgate defines {len(prose_steps)} CI steps named 'prose'; expected one"
        )
    elif prose_steps[0].get("run") != TOLLGATE_PROSE_COMMAND:
        errors.append(
            f"Tollgate invokes {prose_steps[0].get('run')!r}; "
            f"expected {TOLLGATE_PROSE_COMMAND!r}"
        )
    expected_paths = sorted(prose_validation.TRUSTED_PATHS)
    if len(prose_steps) == 1:
        prose = prose_steps[0]
        if prose.get("include") != expected_paths or prose.get("include_mode") != "all":
            errors.append("Tollgate prose selection differs from the trusted path allowlist")
    if len(ci_steps) == 1:
        full_ci = ci_steps[0]
        if full_ci.get("exclude") != expected_paths or full_ci.get("exclude_mode") != "all":
            errors.append("Tollgate full CI does not complement the trusted prose selection")
    return errors


def command_output(command: list[str]) -> str:
    """Run a version probe and return its stripped standard output."""
    return subprocess.run(
        command,
        cwd=REPOSITORY_ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def sample_names() -> list[str]:
    """Return declared Unity sample names in stable order."""
    return sorted(
        path.parent.name for path in (REPOSITORY_ROOT / "samples").glob("*/sample.toml")
    )


def sample_rust_workspaces() -> list[Path]:
    """Discover standalone Cargo workspaces below samples in stable order."""
    samples_root = REPOSITORY_ROOT / "samples"
    manifests: list[Path] = []
    for directory, child_directories, files in os.walk(samples_root):
        child_directories[:] = sorted(
            name
            for name in child_directories
            if name not in IGNORED_SAMPLE_PROJECT_DIRECTORIES
        )
        if "Cargo.toml" not in files:
            continue
        manifest = Path(directory) / "Cargo.toml"
        if CARGO_WORKSPACE_TABLE.search(manifest.read_text(encoding="utf-8")) is None:
            continue
        manifests.append(manifest.relative_to(REPOSITORY_ROOT))
        child_directories.clear()
    return sorted(manifests, key=lambda path: path.as_posix())


def cargo_environment(
    workspace: Path | None,
    concurrent_scope: str | None = None,
) -> dict[str, str]:
    """Return bounded Cargo settings isolated by checkout and writer scope."""
    workspace_identity = "root" if workspace is None else workspace.parent.as_posix()
    writer_identity = workspace_identity if concurrent_scope is None else concurrent_scope
    target_identity = f"{REPOSITORY_ROOT.resolve()}\0{writer_identity}"
    target = hashlib.sha256(target_identity.encode()).hexdigest()[:16]
    environment = resource_slots.capacity_environment()
    environment.setdefault("CARGO_BUILD_JOBS", str(DEFAULT_CARGO_JOBS))
    target_directory = CI_CACHE_ROOT / "cargo-targets" / "shared" / target
    target_directory.mkdir(parents=True, exist_ok=True)
    target_directory.touch()
    environment["CARGO_TARGET_DIR"] = str(target_directory)
    return environment


def rust_workspace_inputs(workspace: Path | None) -> tuple[str, ...]:
    """Return staged inputs that can change one Rust workspace result."""
    if workspace is None:
        return ROOT_RUST_INPUTS
    return (*SAMPLE_SHARED_INPUTS, str(workspace.parent))


def lint_rust_workspaces(
    selection: ci_selection.RustSelection, ci_cache: CiCache
) -> None:
    steps: list[tuple[str, Callable[[], None]]] = []
    if selection.root:
        steps.append((
            "root workspace",
            lambda: ci_cache.run(
                "rust-lint-root", rust_workspace_inputs(None),
                lambda: subprocess.run(
                    ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
                    cwd=REPOSITORY_ROOT, env=cargo_environment(None), check=True,
                ),
            ),
        ))
    steps.extend(
        (
            str(workspace.parent),
            lambda workspace=workspace: ci_cache.run(
                f"rust-lint-{workspace.parent.as_posix().replace('/', '-')}",
                rust_workspace_inputs(workspace),
                lambda: subprocess.run(
                    [
                        "cargo", "clippy", "--manifest-path", str(workspace),
                        "--all-targets", "--", "-D", "warnings",
                    ],
                    cwd=REPOSITORY_ROOT,
                    env=cargo_environment(workspace),
                    check=True,
                ),
            ),
        )
        for workspace in selection.samples
    )
    if not steps:
        print("    skipped; no affected Rust workspace", flush=True)
        return
    run_parallel_steps(steps, workers=RUST_WORKSPACE_WORKERS)


def test_rust_workspaces(
    selection: ci_selection.RustSelection, ci_cache: CiCache
) -> None:
    steps: list[tuple[str, Callable[[], None]]] = []
    if selection.root:
        steps.append((
            "root workspace",
            lambda: ci_cache.run(
                "rust-test-root", rust_workspace_inputs(None),
                lambda: subprocess.run(
                    ["cargo", "test", "--workspace"], cwd=REPOSITORY_ROOT,
                    env=cargo_environment(None), check=True,
                ),
            ),
        ))
    steps.extend(
        (
            str(workspace.parent),
            lambda workspace=workspace: ci_cache.run(
                f"rust-test-{workspace.parent.as_posix().replace('/', '-')}",
                rust_workspace_inputs(workspace),
                lambda: subprocess.run(
                    ["cargo", "test", "--manifest-path", str(workspace)],
                    cwd=REPOSITORY_ROOT,
                    env=cargo_environment(workspace),
                    check=True,
                ),
            ),
        )
        for workspace in selection.samples
    )
    if not steps:
        print("    skipped; no affected Rust workspace", flush=True)
        return
    run_parallel_steps(steps, workers=RUST_WORKSPACE_WORKERS)


def unity_editor() -> Path:
    if configured := os.environ.get("UNITY_EDITOR"):
        return Path(configured)
    if platform.system() == "Darwin":
        return Path(f"/Applications/Unity/Hub/Editor/{UNITY_VERSION}/Unity.app/Contents/MacOS/Unity")
    if platform.system() == "Linux":
        return Path.home() / f"Unity/Hub/Editor/{UNITY_VERSION}/Editor/Unity"
    if platform.system() == "Windows":
        program_files = Path(os.environ.get("PROGRAMFILES", "C:/Program Files"))
        return program_files / f"Unity/Hub/Editor/{UNITY_VERSION}/Editor/Unity.exe"
    raise RuntimeError("Unity is unsupported on this operating system.")


def ci_environment() -> dict[str, str | int]:
    """Return the toolchain and host identity that bounds reusable CI results."""
    editor = unity_editor()
    editor_metadata = editor.stat()
    commands = {
        "cargo": ["cargo", "--version"],
        "clippy": ["cargo", "clippy", "--version"],
        "ffmpeg": [os.environ.get("BATTLEMENT_FFMPEG", "ffmpeg"), "-version"],
        "rustc": ["rustc", "-Vv"],
        "rustfmt": ["cargo", "fmt", "--version"],
    }
    identity: dict[str, str | int] = {
        "hostSystem": platform.system(),
        "hostArchitecture": platform.machine(),
        "python": platform.python_version(),
        "unityEditor": str(editor.resolve()),
        "unityEditorMtimeNs": editor_metadata.st_mtime_ns,
        "unityEditorSize": editor_metadata.st_size,
    }
    for name, command in commands.items():
        executable = resolve_executable(command[0])
        try:
            output = subprocess.run(
                [executable, *command[1:]],
                cwd=REPOSITORY_ROOT,
                check=True,
                capture_output=True,
                text=True,
            ).stdout.strip()
        except FileNotFoundError:
            identity[name] = "unavailable"
            identity[f"{name}Path"] = ""
            continue
        identity[name] = output if name == "rustc" else output.partition("\n")[0]
        identity[f"{name}Path"] = str(Path(executable).resolve())
    for variable in (
        "BATTLEMENT_FFMPEG",
        "CARGO_ENCODED_RUSTFLAGS",
        "CFLAGS",
        "MACOSX_DEPLOYMENT_TARGET",
        "RUSTFLAGS",
    ):
        identity[variable] = os.environ.get(variable, "")
    return identity


def standalone_sample_workers() -> int:
    """Return the configured number of concurrent standalone sample builds."""
    configured = os.environ.get("BATTLEMENT_CI_SAMPLE_WORKERS")
    if configured is None:
        if platform.system() == "Windows":
            return WINDOWS_STANDALONE_SAMPLE_WORKERS
        return DEFAULT_STANDALONE_SAMPLE_WORKERS
    try:
        workers = int(configured)
    except ValueError:
        raise RuntimeError("BATTLEMENT_CI_SAMPLE_WORKERS must be an integer.") from None
    if workers < 1:
        raise RuntimeError("BATTLEMENT_CI_SAMPLE_WORKERS must be positive.")
    return workers


def print_tail(path: Path, count: int) -> None:
    print(
        "\n".join(path.read_text(encoding="utf-8", errors="replace").splitlines()[-count:]),
        file=sys.stderr,
    )


def wait_for_unity_project_unlock() -> None:
    lock = REPOSITORY_ROOT / "Temp/UnityLockfile"
    deadline = time.monotonic() + 15
    while lock.exists() and time.monotonic() < deadline:
        time.sleep(0.1)
    if lock.exists():
        raise RuntimeError("Unity did not release the project lock within 15 seconds.")


def run_with_unity_lease(function: Callable[[], None]) -> None:
    """Run one Unity operation within the shared machine-wide capacity."""
    with unity_editor_lease():
        function()


def unity_analyzer_environment() -> dict[str, str]:
    project = (REPOSITORY_ROOT / "Assembly-CSharp-Editor.csproj").read_text(
        encoding="utf-8"
    )
    analyzers = re.findall(
        r'Include="([^"]*Library[\\/]PackageCache[\\/]org\.nuget\.microsoft\.unity'
        r'\.analyzers@[^\"]*[\\/]Microsoft\.Unity\.Analyzers\.dll)"',
        project,
    )
    if len(analyzers) != 1:
        raise RuntimeError(f"Expected one active Microsoft.Unity.Analyzers package, found {len(analyzers)}.")
    analyzer = Path(analyzers[0])
    if not analyzer.is_file():
        raise RuntimeError(f"Microsoft.Unity.Analyzers was not found at {analyzer}.")
    environment = os.environ.copy()
    environment["BATTLEMENT_UNITY_ANALYZER_PATH"] = str(analyzer)
    return environment


def generate_unity_project_files() -> None:
    """Generate current Unity project files without running the test suite."""
    editor = unity_editor()
    if not os.access(editor, os.X_OK):
        raise RuntimeError(
            f"Unity executable was not found at {editor}. Set UNITY_EDITOR to its executable."
        )
    with tempfile.NamedTemporaryFile(
        prefix="battlement-unity-project-files.", delete=False
    ) as log_file:
        unity_log = Path(log_file.name)
    try:
        with unity_project_transaction(REPOSITORY_ROOT, "project-files") as transaction:
            result = transaction.run(
                [
                    str(editor), "-batchmode", "-nographics",
                    "--burst-disable-compilation", "-projectPath",
                    str(REPOSITORY_ROOT), "-quit", "-logFile", str(unity_log),
                ],
                cwd=REPOSITORY_ROOT,
            )
        wait_for_unity_project_unlock()
        if result.returncode != 0:
            print_tail(unity_log, 120)
            raise RuntimeError("Unity project-file generation failed.")
        if not (REPOSITORY_ROOT / "Assembly-CSharp-Editor.csproj").is_file():
            print_tail(unity_log, 120)
            raise RuntimeError("Unity did not generate C# project files.")
    finally:
        unity_log.unlink(missing_ok=True)


def check_dotnet_diagnostics() -> None:
    run_with_unity_lease(generate_unity_project_files)
    environment = unity_analyzer_environment()
    subprocess.run(
        ["dotnet", "restore", "battlement-ci.slnx"],
        cwd=REPOSITORY_ROOT,
        check=True,
    )
    run_parallel_steps(
        [
            (
                "Unity analyzer diagnostics",
                lambda: subprocess.run(
                    [
                        "dotnet", "format", "battlement-ci.slnx", "analyzers",
                        "--no-restore", "--verify-no-changes", "--severity", "info",
                    ],
                    cwd=REPOSITORY_ROOT,
                    env=environment,
                    check=True,
                ),
            ),
            (
                "C# style diagnostics",
                lambda: subprocess.run(
                    [
                        "dotnet", "format", "battlement-ci.slnx", "style",
                        "--no-restore", "--verify-no-changes", "--diagnostics", "IDE0004", "IDE0005",
                        "IDE0010", "IDE0035", "IDE0043", "IDE0059", "IDE0079",
                        "IDE0080", "IDE0240", "IDE0241",
                    ],
                    cwd=REPOSITORY_ROOT,
                    check=True,
                ),
            ),
        ]
    )


def run_unity_edit_mode_tests(assemblies: tuple[str, ...]) -> None:
    """Run the selected non-empty set of Unity Edit Mode test assemblies."""
    if not assemblies:
        raise ValueError("Unity Edit Mode test assemblies cannot be empty")
    editor = unity_editor()
    if not os.access(editor, os.X_OK):
        raise RuntimeError(f"Unity executable was not found at {editor}. Set UNITY_EDITOR to its executable.")
    with tempfile.NamedTemporaryFile(prefix="battlement-unity-tests-log.", delete=False) as log_file:
        test_log = Path(log_file.name)
    with tempfile.NamedTemporaryFile(prefix="battlement-unity-tests-results.", delete=False) as result_file:
        test_results = Path(result_file.name)
    native_fixture = REPOSITORY_ROOT / "target/unity-native-fixture/debug"
    native_fixture_link = REPOSITORY_ROOT / (
        "battlement_rules.dll" if platform.system() == "Windows" else "battlement_rules"
    )
    tests_passed = False
    try:
        subprocess.run(
            [
                "cargo", "build", "--quiet", "-p", "battlement-native-export-fixture",
                "--target-dir", str(REPOSITORY_ROOT / "target/unity-native-fixture"),
            ],
            cwd=REPOSITORY_ROOT,
            check=True,
        )
        library_name = {
            "Darwin": "libbattlement_rules.dylib",
            "Linux": "libbattlement_rules.so",
        }.get(platform.system(), "battlement_rules.dll")
        shutil.copy2(native_fixture / library_name, native_fixture_link)
        environment = os.environ.copy()
        for variable in ("DYLD_LIBRARY_PATH", "LD_LIBRARY_PATH"):
            environment[variable] = os.pathsep.join(
                value for value in (str(native_fixture), environment.get(variable)) if value
            )
        environment["PATH"] = os.pathsep.join((str(native_fixture), environment["PATH"]))
        assembly_names = ";".join(assemblies)
        with unity_project_transaction(REPOSITORY_ROOT, "edit-mode-tests") as transaction:
            result = transaction.run(
                [
                    str(editor), "-batchmode", "-nographics", "--burst-disable-compilation",
                    "-projectPath", str(REPOSITORY_ROOT), "-runTests", "-testPlatform",
                    "EditMode", "-assemblyNames", assembly_names, "-testResults",
                    str(test_results), "-logFile", str(test_log),
                ],
                cwd=REPOSITORY_ROOT,
                env=environment,
            )
        if result.returncode != 0:
            # Unity can leave its empty project lock behind when compilation aborts
            # batch mode before normal editor shutdown. The process above has exited
            # and this operation holds the repository's exclusive editor lease.
            (REPOSITORY_ROOT / "Temp/UnityLockfile").unlink(missing_ok=True)
        wait_for_unity_project_unlock()
        results = test_results.read_text(encoding="utf-8", errors="replace")
        if result.returncode != 0:
            failed_cases = re.findall(
                r'<test-case [^>]*result="Failed"[^>]*>.*?</test-case>',
                results,
                re.DOTALL,
            )
            if failed_cases:
                print("\n".join(failed_cases), file=sys.stderr)
            else:
                print_tail(test_log, 120)
            raise RuntimeError("Unity Edit Mode tests failed.")
        passed = re.search(
            r'<test-run[^>]*testcasecount="[1-9][0-9]*"[^>]*result="Passed"',
            results,
        )
        if passed is None:
            print(results, file=sys.stderr)
            raise RuntimeError("Unity did not report a passing Edit Mode test run.")
        unity_log = test_log.read_text(errors="replace").replace("\\", "/")
        if not native_fixture_diagnostics_passed(assemblies, unity_log):
            print_tail(test_log, 120)
            raise RuntimeError(
                "Unity's log did not preserve the expected Rust failure diagnostics."
            )
        tests_passed = True
    finally:
        if not tests_passed:
            retained = REPOSITORY_ROOT / "artifacts/unity-tests" / test_results.name
            retained.mkdir(parents=True, exist_ok=True)
            shutil.copy2(test_log, retained / "player.log")
            shutil.copy2(test_results, retained / "results.xml")
            print(f"Unity failure evidence retained: {retained}", file=sys.stderr)
        test_log.unlink(missing_ok=True)
        test_results.unlink(missing_ok=True)
        native_fixture_link.unlink(missing_ok=True)


def native_fixture_diagnostics_passed(
    assemblies: tuple[str, ...],
    unity_log: str,
) -> bool:
    """Validate native panic diagnostics only when the host tests emitted them."""
    if unity_test_selection.HOST_ASSEMBLY not in assemblies:
        return True
    preparing = unity_log.find("Preparing fixture connect panic")
    triggering = unity_log.find("Triggering fixture connect panic")
    panic = unity_log.find(
        "panicked at crates/battlement-native/tests/fixtures/exported-engine"
    )
    ordered_tracing = preparing >= 0 and triggering >= 0 and preparing < triggering
    panic_captured = platform.system() == "Windows" or panic >= 0
    return ordered_tracing and panic_captured


def run_selected_unity_tests(
    selection: unity_test_selection.Selection,
    ci_cache: CiCache,
) -> float:
    """Run only the Unity assemblies selected by the candidate dependency boundary."""
    print(
        "Unity Edit Mode selection: " + json.dumps(selection.report(), sort_keys=True),
        flush=True,
    )
    if selection.scope == unity_test_selection.Scope.NONE:
        return 0.0
    return run_step(
        f"Run Unity Edit Mode tests ({selection.scope.value})",
        function=lambda: ci_cache.run(
            f"unity-edit-mode-{selection.scope.value}",
            selection.cache_inputs,
            lambda: run_with_unity_lease(
                lambda: run_unity_edit_mode_tests(selection.assemblies)
            ),
        ),
    )


def skip_desktop_full_validation() -> None:
    """Report full-suite checks whose packaging pipeline needs a supported desktop."""
    print(
        "    skipped standalone sample builds: "
        "the Battlement packaging pipeline currently targets macOS and Windows",
        flush=True,
    )


def refresh_tracked_file_metadata() -> None:
    if platform.system() == "Windows":
        subprocess.run(
            ["git", "diff", "--quiet", "--ignore-cr-at-eol"],
            cwd=REPOSITORY_ROOT,
            check=True,
        )
        return
    subprocess.run(
        ["git", "update-index", "--refresh"],
        cwd=REPOSITORY_ROOT,
        check=True,
    )


def check_csharp_line_lengths(samples: list[str]) -> None:
    violations = []
    for root in (
        REPOSITORY_ROOT / "Assets",
        REPOSITORY_ROOT / "Packages/com.battlement.client",
        *(REPOSITORY_ROOT / f"samples/{name}/Assets" for name in samples),
    ):
        for path in root.rglob("*.cs"):
            for line_number, line in enumerate(
                path.read_text(encoding="utf-8").splitlines(), 1
            ):
                if len(line) > 100:
                    violations.append(
                        f"{path.relative_to(REPOSITORY_ROOT)}:{line_number}: "
                        f"line is {len(line)} characters; maximum is 100"
                    )
    if violations:
        print("\n".join(violations), file=sys.stderr)
        raise RuntimeError("C# line-length check failed.")


def check_sample_runtime_preflight(samples: list[str]) -> None:
    validate_runtime_ui_package(
        REPOSITORY_ROOT / "Packages/com.battlement.client",
        REPOSITORY_ROOT,
    )
    for name in samples:
        validate_sample_input_backend(REPOSITORY_ROOT / f"samples/{name}")


def check_samples_have_no_csharp(samples: list[str]) -> None:
    for name in samples:
        result = subprocess.run(
            ["git", "ls-files", f"samples/{name}/**/*.cs"],
            cwd=REPOSITORY_ROOT,
            check=True,
            capture_output=True,
            text=True,
        )
        files = result.stdout.splitlines()
        if files:
            formatted = "\n".join(files)
            raise RuntimeError(
                f"The {name} sample must be authored without C#:\n{formatted}"
            )


def run_csharp_preflight(
    samples: list[str],
    selection: unity_test_selection.Selection,
    ci_cache: CiCache,
) -> None:
    """Run the complete authoritative C# gate before expensive validation."""
    run_step("Restore local .NET tools", ["dotnet", "tool", "restore"])
    run_step("Check C# formatting", ["dotnet", "csharpier", "check", "."])
    run_step(
        "Check C# line lengths",
        function=lambda: check_csharp_line_lengths(samples),
    )
    run_step(
        "Check sample runtime preflight",
        function=lambda: check_sample_runtime_preflight(samples),
    )
    run_step(
        "Check samples have no C#",
        function=lambda: check_samples_have_no_csharp(samples),
    )
    if selection.dotnet_diagnostics:
        run_step(
            "Check .NET diagnostics",
            function=lambda: ci_cache.run(
                "dotnet-diagnostics",
                unity_test_selection.DOTNET_DIAGNOSTIC_INPUTS,
                check_dotnet_diagnostics,
            ),
        )
    else:
        print(".NET diagnostics selection: skipped; no C# input changed", flush=True)


def build_standalone_samples(
    samples: list[str], ci_cache: CiCache,
    ditto_builds: DittoBuildLeases | None = None,
) -> float:
    if not samples:
        return 0.0

    def build(name: str) -> None:
        if platform.system() == "Darwin":
            build_uncached(name)
            return
        ci_cache.run(
            f"standalone-{name}",
            (*SAMPLE_SHARED_INPUTS, f"samples/{name}"),
            lambda: build_uncached(name),
        )

    def build_uncached(name: str) -> None:
        if platform.system() != "Darwin":
            subprocess.run(
                [
                    "cargo", "run", "--quiet", "-p", "battlement-cli", "--",
                    "sample", "build", name,
                ],
                cwd=REPOSITORY_ROOT,
                env=cargo_environment(None, f"standalone-{name}"),
                check=True,
            )
            return
        if ditto_builds is None:
            raise RuntimeError("macOS standalone builds require retained cache leases")
        ditto_builds.prepare(name)
        changed = subprocess.run(
            ["git", "diff", "--name-only", "--", f"samples/{name}"],
            cwd=REPOSITORY_ROOT,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.splitlines()
        if changed:
            raise RuntimeError(
                f"The {name} sample build modified tracked files:\n" + "\n".join(changed)
            )

    ditto_preparation_seconds = 0.0
    if platform.system() == "Darwin":
        started = time.monotonic()
        with ci_steps.span("Prepare standalone sample builder"):
            subprocess.run(
                ["cargo", "build", "-p", "battlement-ditto"],
                cwd=REPOSITORY_ROOT,
                check=True,
            )
        ditto_preparation_seconds = time.monotonic() - started
    run_parallel_steps(
        [(f"{name} standalone build", lambda name=name: build(name)) for name in samples],
        workers=standalone_sample_workers(),
    )
    return ditto_preparation_seconds


def run_ditto_validation(
    reusable_build_seconds: float,
    ditto_builds: DittoBuildLeases | None = None,
    invocation_id: str | None = None,
    evidence_export: tollgate_evidence.Export | None = None,
    samples: list[str] | None = None,
) -> None:
    """Run every canonical Ditto scenario against prebuilt players."""
    explicit_samples = samples is not None
    samples = list(DITTO_SAMPLES) if samples is None else samples
    environment = os.environ.copy()
    environment["DITTO_CI_REUSABLE_BUILD_SECONDS"] = str(reusable_build_seconds)
    invocation_id = invocation_id or environment.get(
        "DITTO_CI_INVOCATION_ID", str(uuid.uuid4())
    )
    environment["DITTO_CI_INVOCATION_ID"] = invocation_id
    root = ditto_evidence.invocation_root(REPOSITORY_ROOT, invocation_id)
    environment["DITTO_CI_ARTIFACT_ROOT"] = str(root)
    ci_steps.record_event("ditto.invocation", {
        "invocation_id": invocation_id, "artifact_root": str(root),
        "evidence_path": str(root / "evidence.json"),
    })
    if ditto_builds is not None:
        ditto_builds.assert_healthy()
        environment["DITTO_CI_CACHE_ROOT"] = str(ditto_builds.cache_root)
    command = [sys.executable, "scripts/ditto_ci.py", "gate"]
    if explicit_samples:
        for sample in samples:
            command.extend(["--sample", sample])
    if platform.system() == "Darwin":
        command = ["/usr/bin/caffeinate", "-u", "-d", "-i", "--", *command]
    failure = None
    try:
        run_step("Run Ditto full suite", command, environment=environment)
    except BaseException as error:
        failure = error
        raise
    finally:
        if evidence_export is not None:
            try:
                bundle = evidence_export.publish(root / "evidence.json", invocation_id)
                ci_steps.record_event("ditto.tollgate_evidence", {
                    "invocation_id": invocation_id,
                    "buildset_id": evidence_export.buildset_id,
                    "evidence_path": str(bundle),
                    "sha256": ditto_evidence.digest(bundle),
                })
            except Exception as error:
                if failure is None:
                    raise
                failure.add_note(f"Tollgate evidence export failed: {error}")
                print(f"Tollgate evidence export failed: {error}", file=sys.stderr)


def publish_empty_ditto_validation(
    evidence_export: tollgate_evidence.Export,
    invocation_id: str,
    paths: list[str],
) -> None:
    """Publish a canonical empty gate when no native sample can be affected."""
    root = ditto_evidence.invocation_root(REPOSITORY_ROOT, invocation_id)
    identity = ditto_evidence.begin(root, invocation_id, REPOSITORY_ROOT, "gate")
    (root / "gate.json").write_text(
        json.dumps(
            {
                "artifact_root": str(root),
                "expected_samples": [],
                "failures": [],
                "invocation_id": invocation_id,
                "samples": [],
                "selected_paths": sorted(paths),
                "status": "passed",
            },
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )
    manifest = ditto_evidence.finish(root, identity, "passed")
    bundle = evidence_export.publish(manifest, invocation_id)
    ci_steps.record_event(
        "ditto.tollgate_evidence",
        {
            "buildset_id": evidence_export.buildset_id,
            "evidence_path": str(bundle),
            "samples": [],
            "sha256": ditto_evidence.digest(bundle),
        },
    )


def run_reactant_asset_fast_lane() -> None:
    """Run the fast tier's single consolidated CLI and browser process lane."""
    run_step(
        "Run Reactant asset CLI/browser scenario",
        [
            sys.executable,
            "scripts/reactant_asset_validation.py",
            "fast",
            "--portion",
            "cli/browser",
        ],
    )


def select_native_samples(paths: list[str], samples: list[str]) -> list[str]:
    """Select affected samples; Tollgate can seal an exact empty selection."""
    return native_validation_selection.select(REPOSITORY_ROOT, paths, samples)


def run_ci(
    full: bool, use_ci_cache: bool, ditto: bool,
    evidence_export: tollgate_evidence.Export | None = None,
) -> None:
    samples = sample_names()
    sample_workspaces = sample_rust_workspaces()
    ci_cache = CiCache(
        REPOSITORY_ROOT,
        CI_CACHE_ROOT,
        ci_environment(),
        enabled=use_ci_cache,
        event=ci_steps.record_cache_event,
    )
    from web_selection import changed_paths
    _revision, paths = changed_paths(REPOSITORY_ROOT)
    rust_selection = ci_selection.select_rust(
        REPOSITORY_ROOT, paths, sample_workspaces
    )
    print(
        "Rust workspace selection: "
        + json.dumps(rust_selection.report(), sort_keys=True),
        flush=True,
    )
    native_samples = select_native_samples(paths, samples)
    print(
        "Native sample selection: "
        + (", ".join(native_samples) if native_samples else "none"),
        flush=True,
    )
    unity_selection = unity_test_selection.select(REPOSITORY_ROOT, paths)
    run_csharp_preflight(samples, unity_selection, ci_cache)
    run_step(
        "Check rust-analyzer projects",
        [sys.executable, "scripts/update-rust-analyzer-projects.py", "--check"],
    )
    run_step(
        "Check Trox localization artifacts",
        [sys.executable, "scripts/trox_validation.py"],
    )
    run_step(
        "Prepare validation inputs",
        [sys.executable, "scripts/prepare_validation.py", "check"],
    )
    run_step("Check Rust formatting", ["cargo", "fmt", "--all", "--", "--check"])
    for workspace in sample_workspaces:
        run_step(
            f"Check {workspace.parent} Rust formatting",
            [
                "cargo", "fmt", "--manifest-path", str(workspace),
                "--", "--check",
            ],
        )
    run_step(
        "Lint Rust workspaces",
        function=lambda: lint_rust_workspaces(rust_selection, ci_cache),
    )
    rust_test_seconds = run_step(
        "Test Rust workspaces",
        function=lambda: test_rust_workspaces(rust_selection, ci_cache),
    )
    reactant_cli_seconds = 0.0
    if full:
        reactant_selected, reactant_reasons = ci_selection.select_reactant_assets(paths)
        print(
            "Reactant asset selection: "
            + json.dumps({"selected": reactant_selected, "reasons": reactant_reasons}),
            flush=True,
        )
        if reactant_selected:
            reactant_cli_started = time.monotonic()
            run_reactant_asset_fast_lane()
            reactant_cli_seconds = time.monotonic() - reactant_cli_started
    run_step(
        "Test operation telemetry",
        [sys.executable, "scripts/tests/operation-log.test.py"],
    )
    run_step("Test CI job handles", [sys.executable, "scripts/tests/ci-job.test.py"])
    run_step(
        "Test validation preparation",
        [sys.executable, "scripts/tests/prepare-validation.test.py"],
    )
    run_step(
        "Test resource slots",
        [sys.executable, "scripts/tests/resource-slots.test.py"],
    )
    run_step(
        "Test Unity transactions",
        [sys.executable, "scripts/tests/unity-transaction.test.py"],
    )
    run_step(
        "Test Web sample server",
        [sys.executable, "scripts/tests/serve-web.test.py"],
    )
    run_step(
        "Test Web demo cache",
        [sys.executable, "scripts/tests/prepare-web-demo.test.py"],
    )
    run_step(
        "Test sample deployment workflow",
        [sys.executable, "scripts/tests/deploy.test.py"],
    )
    run_step(
        "Test isolated Playwright transport",
        [sys.executable, "scripts/tests/playwright-mcp.test.py"],
    )
    run_step(
        "Test browser risk selection",
        [sys.executable, "scripts/tests/web-selection.test.py"],
    )
    run_step(
        "Test CI sample discovery",
        [sys.executable, "scripts/tests/ci.test.py"],
    )
    run_step(
        "Test affected CI selection",
        [sys.executable, "scripts/tests/ci-selection.test.py"],
    )
    run_step(
        "Test CI Cache",
        [sys.executable, "scripts/tests/ci-cache.test.py"],
    )
    run_step(
        "Test Unity affected-test selection",
        [sys.executable, "scripts/tests/unity-test-selection.test.py"],
    )
    run_step(
        "Test native sample selection",
        [sys.executable, "scripts/tests/native-validation-selection.test.py"],
    )
    run_step(
        "Test performance reporting",
        [sys.executable, "scripts/tests/perf-report.test.py"],
    )
    run_step(
        "Test candidate performance reporting",
        [sys.executable, "scripts/tests/perf-candidate.test.py"],
    )
    run_step(
        "Test Tollgate evidence collection",
        [sys.executable, "scripts/tests/tollgate-evidence.test.py"],
    )
    run_step(
        "Test trusted prose validation",
        [sys.executable, "scripts/tests/prose-validation.test.py"],
    )
    run_step(
        "Test Ditto CI",
        [sys.executable, "scripts/tests/ditto-ci.test.py"],
    )
    run_step(
        "Test Ditto replay",
        [sys.executable, "scripts/tests/ditto-replay.test.py"],
    )
    run_step(
        "Test Ditto build-cache lifetime",
        [sys.executable, "scripts/tests/ditto-cache-lifetime.test.py"],
    )
    if full and ditto:
        run_step(
            "Test Ditto performance benchmark",
            [sys.executable, "scripts/tests/ditto-benchmark.test.py"],
        )
        run_step(
            "Test Ditto cutover",
            [sys.executable, "scripts/tests/ditto-cutover.test.py"],
        )
    unity_seconds = run_selected_unity_tests(unity_selection, ci_cache)
    if full:
        print(
            "Reactant asset fast-tier timing "
            f"in-process+compile={rust_test_seconds:.3f}s "
            f"cli/browser={reactant_cli_seconds:.3f}s "
            f"Unity={unity_seconds:.3f}s "
            f"total={rust_test_seconds + reactant_cli_seconds + unity_seconds:.3f}s",
            flush=True,
        )
    ditto_preparation_seconds = [0.0]
    invocation_id = os.environ.get("DITTO_CI_INVOCATION_ID", str(uuid.uuid4()))
    ditto_builds = None
    if full and platform.system() == "Darwin":
        cache_root = Path(os.environ.get(
            "DITTO_CI_CACHE_ROOT",
            Path.home() / "Library/Caches/Battlement/ditto-ci",
        ))
        invocation_root = ditto_evidence.invocation_root(
            REPOSITORY_ROOT, invocation_id
        )
        lease_evidence = invocation_root.parent / f"{invocation_id}.prepared-builds"
        ditto_builds = DittoBuildLeases(
            REPOSITORY_ROOT,
            REPOSITORY_ROOT / "target/debug/ditto",
            cache_root,
            lease_evidence,
        )
    try:
        if full and platform.system() in {"Darwin", "Windows"}:
            def build_samples() -> None:
                if platform.system() == "Windows":
                    ditto_preparation_seconds[0] = build_standalone_samples(
                        native_samples, ci_cache
                    )
                    return
                ditto_preparation_seconds[0] = build_standalone_samples(
                    native_samples, ci_cache, ditto_builds
                )

            run_step(
                "Build standalone samples",
                function=build_samples,
            )
        elif full:
            run_step("Skip desktop full validation", function=skip_desktop_full_validation)
        if full and platform.system() == "Darwin":
            if native_samples:
                run_ditto_validation(
                    ditto_preparation_seconds[0], ditto_builds, invocation_id,
                    evidence_export, samples=native_samples
                )
            else:
                print("Ditto validation selection: skipped; no native sample input changed")
                if evidence_export is not None:
                    run_step(
                        "Publish empty Ditto selection evidence",
                        function=lambda: publish_empty_ditto_validation(
                            evidence_export, invocation_id, paths
                        ),
                    )
    finally:
        if ditto_builds is not None:
            ditto_builds.close()
    from web_selection import validate_affected
    run_step("Validate affected browser contracts", function=lambda: validate_affected(REPOSITORY_ROOT))
    run_step("Refresh tracked file metadata", function=refresh_tracked_file_metadata)


def main(full: bool, use_ci_cache: bool, ditto: bool, export_evidence: bool = False) -> None:
    """Run the configured continuous-integration suite."""
    evidence_export = None
    if export_evidence:
        if not full or platform.system() != "Darwin":
            raise ValueError("Tollgate evidence requires the full native macOS gate")
        evidence_export = tollgate_evidence.Export.begin(REPOSITORY_ROOT)
    if not full and not export_evidence:
        paths = prose_validation.changed_paths(REPOSITORY_ROOT)
        if prose_validation.selected(paths):
            run_step(
                "Validate trusted prose",
                function=lambda: prose_validation.run(REPOSITORY_ROOT),
            )
            return
    recover_unity_transactions(REPOSITORY_ROOT)
    run_step("Check Rust toolchain", function=check_rust_toolchain)
    run_ci(full, use_ci_cache, ditto, evidence_export)


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--full",
        action="store_true",
        help="also run slow integration validation and standalone sample build",
    )
    parser.add_argument(
        "--ditto",
        action="store_true",
        help="with --full, also run Ditto performance validation",
    )
    parser.add_argument(
        "--no-ci-cache",
        action="store_true",
        help="execute expensive validation without reading or publishing CI Cache entries",
    )
    parser.add_argument("--tollgate-evidence", action="store_true",
                        help="export exact native gate evidence for the current Tollgate buildset")
    parser.add_argument("--test-trace-outcome", choices=("passed", "failed", "interrupted"), help=argparse.SUPPRESS)
    return parser.parse_args()


if __name__ == "__main__":
    os.environ.setdefault("BATTLEMENT_PYTHON", sys.executable)
    signal.signal(signal.SIGTERM, ci_steps.interrupted)
    signal.signal(signal.SIGINT, ci_steps.interrupted)
    arguments = parse_arguments()
    trace = perf_log.CiTrace(
        REPOSITORY_ROOT,
        {
            "argv": sys.argv[1:],
            "full": arguments.full,
            "ditto": arguments.ditto,
            "ci_cache_enabled": not arguments.no_ci_cache,
            "platform": platform.system(),
        },
    )
    ci_steps.configure(REPOSITORY_ROOT, trace)
    outcome = "passed"
    exit_code = 0
    try:
        if arguments.test_trace_outcome:
            ci_steps.trace_smoke_test(arguments.test_trace_outcome)
        else:
            main(arguments.full, not arguments.no_ci_cache, arguments.ditto, arguments.tollgate_evidence)
    except KeyboardInterrupt:
        outcome = "interrupted"
        exit_code = 130
        raise SystemExit(130) from None
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        outcome = "failed"
        exit_code = 1
        print(error, file=sys.stderr)
        raise SystemExit(1) from error
    except BaseException:
        outcome = "failed"
        exit_code = 1
        raise
    finally:
        trace.finish(outcome, exit_code)
        try:
            with perf_log.retention_guard(trace.log_root):
                perf_log.enforce_retention(
                    trace.log_root,
                    perf_log.configured_max_log_bytes(),
                    {trace.path} if trace.path is not None else set(),
                )
        except OSError as error:
            print(f"CI performance retention skipped: {error}", file=sys.stderr)
