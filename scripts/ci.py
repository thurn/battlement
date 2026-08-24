#!/usr/bin/env python3

"""Run Battlement's complete local continuous-integration suite."""

from __future__ import annotations

import argparse
from collections.abc import Callable
from concurrent.futures import as_completed, ThreadPoolExecutor
import os
from pathlib import Path
import platform
import re
import select
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import tomllib

from sample_validation import validate_runtime_ui_package, validate_sample_input_backend


REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
UNITY_VERSION = "6000.5.8f1"
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
        if "workspace" not in tomllib.loads(manifest.read_text()):
            continue
        manifests.append(manifest.relative_to(REPOSITORY_ROOT))
        child_directories.clear()
    return sorted(manifests, key=lambda path: path.as_posix())


def run_step(
    name: str,
    command: list[str] | None = None,
    function: Callable[[], None] | None = None,
) -> None:
    print(f"\n==> {name}", flush=True)
    started = time.monotonic()
    try:
        if function is not None:
            function()
        else:
            subprocess.run(command, cwd=REPOSITORY_ROOT, check=True)
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


def unity_editor() -> Path:
    if configured := os.environ.get("UNITY_EDITOR"):
        return Path(configured)
    if platform.system() == "Darwin":
        return Path(f"/Applications/Unity/Hub/Editor/{UNITY_VERSION}/Unity.app/Contents/MacOS/Unity")
    if platform.system() == "Linux":
        return Path.home() / f"Unity/Hub/Editor/{UNITY_VERSION}/Editor/Unity"
    raise RuntimeError("Unity is unsupported on this operating system.")


def print_tail(path: Path, count: int) -> None:
    print("\n".join(path.read_text(errors="replace").splitlines()[-count:]), file=sys.stderr)


def wait_for_unity_project_unlock() -> None:
    lock = REPOSITORY_ROOT / "Temp/UnityLockfile"
    deadline = time.monotonic() + 15
    while lock.exists() and time.monotonic() < deadline:
        time.sleep(0.1)
    if lock.exists():
        raise RuntimeError("Unity did not release the project lock within 15 seconds.")


def unity_analyzer_environment() -> dict[str, str]:
    project = (REPOSITORY_ROOT / "Assembly-CSharp-Editor.csproj").read_text()
    analyzers = re.findall(
        r'Include="([^"]*Library/PackageCache/org\.nuget\.microsoft\.unity\.analyzers@[^\"]*/Microsoft\.Unity\.Analyzers\.dll)"',
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
    run_parallel_steps(
        [
            (
                "Unity analyzer diagnostics",
                lambda: subprocess.run(
                    [
                        "dotnet", "format", "battlement-ci.slnx", "analyzers",
                        "--verify-no-changes", "--severity", "info",
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
                        "--verify-no-changes", "--diagnostics", "IDE0004", "IDE0005",
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
    native_fixture_link = REPOSITORY_ROOT / "battlement_rules"
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
            [str(native_fixture / "battlement-release-http-fixture")],
            cwd=REPOSITORY_ROOT,
            stdout=subprocess.PIPE,
            text=True,
        )
        if http_fixture.stdout is None:
            raise RuntimeError("The release HTTP fixture did not expose stdout.")
        ready, _, _ = select.select([http_fixture.stdout], [], [], 5)
        if not ready:
            raise RuntimeError("The release HTTP fixture did not start within five seconds.")
        environment["BATTLEMENT_RELEASE_FIXTURE_URL"] = http_fixture.stdout.readline().strip()
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
        results = test_results.read_text(errors="replace")
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


def run_integration_player_smoke() -> None:
    if platform.system() != "Darwin":
        raise RuntimeError("The Battlement Integration Fixture player check requires macOS.")
    with tempfile.TemporaryDirectory(prefix="battlement-integration-player.") as artifact_root:
        subprocess.run(
            [
                sys.executable,
                "scripts/capture-visual-evidence.py",
                "--task",
                "37",
                "--scenario",
                "battlement-integration-fixture",
                "--scene",
                "Assets/BattlementIntegration/BattlementIntegrationFixture.unity",
                "--transport",
                "native",
                "--cargo-package",
                "battlement-native-export-fixture",
                "--artifact-root",
                artifact_root,
                "--run-id",
                "ci-player-smoke",
                "--smoke",
                "--dimensions",
                "1280x720",
            ],
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
            for line_number, line in enumerate(path.read_text().splitlines(), 1):
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


def build_standalone_samples(samples: list[str]) -> None:
    def build(name: str) -> None:
        subprocess.run(
            [
                "cargo", "run", "--quiet", "-p", "battlement-cli", "--",
                "sample", "build", name,
            ],
            cwd=REPOSITORY_ROOT,
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

    run_parallel_steps(
        [(f"{name} standalone build", lambda name=name: build(name)) for name in samples]
    )


def main(full: bool) -> None:
    samples = sample_names()
    sample_workspaces = sample_rust_workspaces()
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
    run_step("Lint Rust crates", ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"])
    for workspace in sample_workspaces:
        run_step(
            f"Lint {workspace.parent} Rust workspace",
            [
                "cargo", "clippy", "--manifest-path", str(workspace),
                "--all-targets", "--", "-D", "warnings",
            ],
        )
    run_step("Test Rust crates", ["cargo", "test", "--workspace"])
    for workspace in sample_workspaces:
        run_step(
            f"Test {workspace.parent} Rust workspace",
            ["cargo", "test", "--manifest-path", str(workspace)],
        )
    run_step(
        "Test visual capture workflow",
        [sys.executable, "scripts/tests/visual-capture-workflow.test.py"],
    )
    run_step(
        "Test Web sample server",
        [sys.executable, "scripts/tests/serve-web.test.py"],
    )
    run_step(
        "Test sample deployment workflow",
        [sys.executable, "scripts/tests/deploy.test.py"],
    )
    run_step(
        "Test CI sample discovery",
        [sys.executable, "scripts/tests/ci.test.py"],
    )
    run_step("Restore local .NET tools", ["dotnet", "tool", "restore"])
    run_step("Check C# formatting", ["dotnet", "csharpier", "check", "."])
    run_step("Check C# line lengths", function=lambda: check_csharp_line_lengths(samples))
    run_step("Check sample runtime preflight", function=lambda: check_sample_runtime_preflight(samples))
    run_step("Check samples have no C#", function=lambda: check_samples_have_no_csharp(samples))
    run_step("Run Unity Edit Mode tests", function=run_unity_edit_mode_tests)
    run_step("Check .NET diagnostics", function=check_dotnet_diagnostics)
    if full:
        run_step(
            "Run packaged Battlement Integration Fixture",
            function=run_integration_player_smoke,
        )
        run_step("Build standalone samples", function=lambda: build_standalone_samples(samples))
    run_step("Refresh tracked file metadata", ["git", "update-index", "--refresh"])


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--full",
        action="store_true",
        help="also run slow integration validation and standalone sample build",
    )
    return parser.parse_args()


def interrupted(_signal_number, _frame) -> None:
    raise KeyboardInterrupt


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGINT, interrupted)
    try:
        main(parse_arguments().full)
    except KeyboardInterrupt:
        raise SystemExit(130) from None
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        raise SystemExit(1) from error
