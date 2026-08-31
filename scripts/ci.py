#!/usr/bin/env python3

"""Run Battlement's complete local continuous-integration suite."""

from __future__ import annotations

import argparse
from collections.abc import Callable
from concurrent.futures import as_completed, ThreadPoolExecutor
import hashlib
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

from ci_cache import CiCache
from platform_support import (
    executable_name,
    readline_with_timeout,
    resolve_executable,
    user_cache_path,
)
from sample_validation import validate_runtime_ui_package, validate_sample_input_backend
from resource_slots import unity_editor_lease


REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
UNITY_VERSION = "6000.5.8f1"
CI_CACHE_ROOT = Path(
    os.environ.get(
        "BATTLEMENT_CI_CACHE",
        user_cache_path("Battlement", "ci-cache"),
    )
)
DEFAULT_STANDALONE_SAMPLE_WORKERS = 4
RUST_WORKSPACE_WORKERS = 2
DEFAULT_CARGO_JOBS = 3
ROOT_RUST_INPUTS = (
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "crates",
    "scripts/ci.py",
    "scripts/ci_cache.py",
)
UNITY_TEST_INPUTS = (
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "Assets",
    "Packages",
    "ProjectSettings",
    "crates",
    "scripts/ci.py",
    "scripts/ci_cache.py",
)
DOTNET_DIAGNOSTIC_INPUTS = (
    ".config/dotnet-tools.json",
    "Assets",
    "Packages",
    "ProjectSettings",
    "battlement-ci.slnx",
    "scripts/ci.py",
    "scripts/ci_cache.py",
)
SAMPLE_SHARED_INPUTS = (
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "Packages/com.battlement.client",
    "crates",
    "scripts/ci.py",
    "scripts/ci_cache.py",
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
DITTO_SAMPLES = ("basic", "tictactoe", "reactant", "chess", "ui")
DITTO_ADAPTERS = ("webgl", "ios")


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


def run_step(
    name: str,
    command: list[str] | None = None,
    function: Callable[[], None] | None = None,
    environment: dict[str, str] | None = None,
) -> None:
    print(f"\n==> {name}", flush=True)
    started = time.monotonic()
    try:
        if function is not None:
            function()
        else:
            subprocess.run(
                command,
                cwd=REPOSITORY_ROOT,
                env=environment,
                check=True,
            )
    finally:
        print(f"<== {name} ({time.monotonic() - started:.1f}s)", flush=True)


def run_parallel_steps(
    steps: list[tuple[str, Callable[[], None]]],
    workers: int = 2,
) -> None:
    def execute(function: Callable[[], None]) -> tuple[float, Exception | None]:
        started = time.monotonic()
        try:
            function()
        except Exception as error:
            return time.monotonic() - started, error
        return time.monotonic() - started, None

    failures: list[Exception] = []
    with ThreadPoolExecutor(max_workers=min(workers, len(steps))) as executor:
        futures = {
            executor.submit(execute, function): name
            for name, function in steps
        }
        for future in as_completed(futures):
            name = futures[future]
            elapsed, error = future.result()
            if error is not None:
                failures.append(error)
                outcome = "failed"
            else:
                outcome = "passed"
            print(
                f"    {name}: {outcome} ({elapsed:.1f}s)",
                flush=True,
            )
    if failures:
        raise failures[0]


def cargo_environment(
    workspace: Path | None,
    concurrent_scope: str | None = None,
) -> dict[str, str]:
    """Return bounded Cargo settings isolated by checkout and concurrent writer."""
    checkout = hashlib.sha256(REPOSITORY_ROOT.resolve().as_posix().encode()).hexdigest()[:16]
    workspace_identity = "root" if workspace is None else workspace.parent.as_posix()
    target_identity = workspace_identity if concurrent_scope is None else concurrent_scope
    target = hashlib.sha256(target_identity.encode()).hexdigest()[:16]
    environment = os.environ.copy()
    environment.setdefault("CARGO_BUILD_JOBS", str(DEFAULT_CARGO_JOBS))
    environment["CARGO_TARGET_DIR"] = str(
        CI_CACHE_ROOT / "cargo-targets" / checkout / target
    )
    return environment


def rust_workspace_inputs(workspace: Path | None) -> tuple[str, ...]:
    """Return staged inputs that can change one Rust workspace result."""
    if workspace is None:
        return ROOT_RUST_INPUTS
    return (*SAMPLE_SHARED_INPUTS, str(workspace.parent))


def lint_rust_workspaces(sample_workspaces: list[Path], ci_cache: CiCache) -> None:
    steps: list[tuple[str, Callable[[], None]]] = [
        (
            "root workspace",
            lambda: ci_cache.run(
                "rust-lint-root",
                rust_workspace_inputs(None),
                lambda: subprocess.run(
                    ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
                    cwd=REPOSITORY_ROOT,
                    env=cargo_environment(None),
                    check=True,
                ),
            ),
        )
    ]
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
        for workspace in sample_workspaces
    )
    run_parallel_steps(steps, workers=RUST_WORKSPACE_WORKERS)


def test_rust_workspaces(sample_workspaces: list[Path], ci_cache: CiCache) -> None:
    steps: list[tuple[str, Callable[[], None]]] = [
        (
            "root workspace",
            lambda: ci_cache.run(
                "rust-test-root",
                rust_workspace_inputs(None),
                lambda: subprocess.run(
                    ["cargo", "test", "--workspace"],
                    cwd=REPOSITORY_ROOT,
                    env=cargo_environment(None),
                    check=True,
                ),
            ),
        )
    ]
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
        for workspace in sample_workspaces
    )
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
        "ffmpeg": [os.environ.get("BATTLEMENT_FFMPEG", "ffmpeg"), "-version"],
        "rustc": ["rustc", "-Vv"],
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


def check_dotnet_diagnostics() -> None:
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


def run_unity_edit_mode_tests() -> None:
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
    mutable_project_files = tuple(
        REPOSITORY_ROOT / relative
        for relative in (
            "ProjectSettings/ProjectAuditorSettings.asset",
            "ProjectSettings/TimeManager.asset",
        )
    )
    project_file_state = {
        path: path.read_bytes() if path.is_file() else None for path in mutable_project_files
    }
    http_fixture: subprocess.Popen[str] | None = None
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
        http_fixture = subprocess.Popen(
            [str(native_fixture / executable_name("battlement-release-http-fixture"))],
            cwd=REPOSITORY_ROOT,
            stdout=subprocess.PIPE,
            text=True,
        )
        if http_fixture.stdout is None:
            raise RuntimeError("The release HTTP fixture did not expose stdout.")
        fixture_url = readline_with_timeout(http_fixture.stdout, 5)
        if fixture_url is None:
            raise RuntimeError("The release HTTP fixture did not start within five seconds.")
        environment["BATTLEMENT_RELEASE_FIXTURE_URL"] = fixture_url.strip()
        if not environment["BATTLEMENT_RELEASE_FIXTURE_URL"].startswith("http://127.0.0.1:"):
            raise RuntimeError("The release HTTP fixture reported an invalid loopback URL.")
        assembly_names = (
            "Battlement.Integration.EditorTests;Battlement.EditorTests;"
            "Battlement.HostEditorTests"
        )
        result = subprocess.run(
            [
                str(editor), "-batchmode", "-nographics", "--burst-disable-compilation",
                "-projectPath", str(REPOSITORY_ROOT), "-runTests", "-testPlatform",
                "EditMode", "-assemblyNames", assembly_names, "-testResults",
                str(test_results), "-logFile", str(test_log),
            ],
            cwd=REPOSITORY_ROOT,
            env=environment,
        )
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
        preparing = unity_log.find("Preparing fixture connect panic")
        triggering = unity_log.find("Triggering fixture connect panic")
        panic = unity_log.find(
            "panicked at crates/battlement-native/tests/fixtures/exported-engine"
        )
        ordered_tracing = preparing >= 0 and triggering >= 0 and preparing < triggering
        panic_captured = platform.system() == "Windows" or panic >= 0
        if not ordered_tracing or not panic_captured:
            print_tail(test_log, 120)
            raise RuntimeError(
                "Unity's log did not preserve the expected Rust failure diagnostics."
            )
    finally:
        if http_fixture is not None:
            http_fixture.terminate()
            try:
                http_fixture.wait(timeout=5)
            except subprocess.TimeoutExpired:
                http_fixture.kill()
                http_fixture.wait(timeout=5)
        test_log.unlink(missing_ok=True)
        test_results.unlink(missing_ok=True)
        native_fixture_link.unlink(missing_ok=True)
        for path, contents in project_file_state.items():
            if contents is None:
                path.unlink(missing_ok=True)
            else:
                path.write_bytes(contents)


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


def build_standalone_samples(samples: list[str], ci_cache: CiCache) -> float:
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
            with unity_editor_lease():
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
        environment = os.environ.copy()
        environment["DITTO_CACHE_ROOT"] = os.environ.get(
            "DITTO_CI_CACHE_ROOT",
            str(Path.home() / "Library/Caches/Battlement/ditto-ci"),
        )
        subprocess.run(
            [
                str(REPOSITORY_ROOT / "target/debug/ditto"),
                "--config", f"samples/{name}/ditto.toml", "build",
                "--profile", "macos", "--json",
            ],
            cwd=REPOSITORY_ROOT,
            env=environment,
            check=True,
        )
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


def run_ditto_validation(preparation_seconds: float) -> None:
    """Run the bounded screenshot gate against prebuilt players."""
    environment = os.environ.copy()
    environment["DITTO_CI_PREPARATION_SECONDS"] = str(preparation_seconds)
    run_step(
        "Run Ditto screenshot gate",
        [sys.executable, "scripts/ditto_ci.py", "gate"],
        environment=environment,
    )


def main(full: bool, use_ci_cache: bool, ditto: bool) -> None:
    samples = sample_names()
    sample_workspaces = sample_rust_workspaces()
    ci_cache = CiCache(
        REPOSITORY_ROOT,
        CI_CACHE_ROOT,
        ci_environment(),
        enabled=use_ci_cache,
    )
    run_step(
        "Check rust-analyzer projects",
        [sys.executable, "scripts/update-rust-analyzer-projects.py", "--check"],
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
        function=lambda: lint_rust_workspaces(sample_workspaces, ci_cache),
    )
    run_step(
        "Test Rust workspaces",
        function=lambda: test_rust_workspaces(sample_workspaces, ci_cache),
    )
    run_step(
        "Test resource slots",
        [sys.executable, "scripts/tests/resource-slots.test.py"],
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
        "Test CI sample discovery",
        [sys.executable, "scripts/tests/ci.test.py"],
    )
    run_step(
        "Test CI Cache",
        [sys.executable, "scripts/tests/ci-cache.test.py"],
    )
    run_step(
        "Test Ditto CI",
        [sys.executable, "scripts/tests/ditto-ci.test.py"],
    )
    if ditto:
        run_step(
            "Test Ditto performance benchmark",
            [sys.executable, "scripts/tests/ditto-benchmark.test.py"],
        )
        run_step(
            "Test Ditto cutover",
            [sys.executable, "scripts/tests/ditto-cutover.test.py"],
        )
    run_step("Restore local .NET tools", ["dotnet", "tool", "restore"])
    run_step("Check C# formatting", ["dotnet", "csharpier", "check", "."])
    run_step("Check C# line lengths", function=lambda: check_csharp_line_lengths(samples))
    run_step("Check sample runtime preflight", function=lambda: check_sample_runtime_preflight(samples))
    run_step("Check samples have no C#", function=lambda: check_samples_have_no_csharp(samples))
    run_step(
        "Run Unity Edit Mode tests",
        function=lambda: ci_cache.run(
            "unity-edit-mode",
            UNITY_TEST_INPUTS,
            lambda: run_with_unity_lease(run_unity_edit_mode_tests),
        ),
    )
    run_step(
        "Check .NET diagnostics",
        function=lambda: ci_cache.run(
            "dotnet-diagnostics",
            DOTNET_DIAGNOSTIC_INPUTS,
            check_dotnet_diagnostics,
        ),
    )
    ditto_preparation_seconds = [0.0]
    if full and platform.system() in {"Darwin", "Windows"}:
        def build_samples() -> None:
            ditto_preparation_seconds[0] = build_standalone_samples(samples, ci_cache)

        run_step(
            "Build standalone samples",
            function=build_samples,
        )
    elif full:
        run_step("Skip desktop full validation", function=skip_desktop_full_validation)
    if full and platform.system() == "Darwin":
        run_ditto_validation(ditto_preparation_seconds[0])
    run_step("Refresh tracked file metadata", function=refresh_tracked_file_metadata)


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
        help="also run Ditto screenshot and performance validation",
    )
    parser.add_argument(
        "--no-ci-cache",
        action="store_true",
        help="execute expensive validation without reading or publishing CI Cache entries",
    )
    return parser.parse_args()


def interrupted(_signal_number, _frame) -> None:
    raise KeyboardInterrupt


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGINT, interrupted)
    try:
        arguments = parse_arguments()
        main(arguments.full, not arguments.no_ci_cache, arguments.ditto)
    except KeyboardInterrupt:
        raise SystemExit(130) from None
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        raise SystemExit(1) from error
