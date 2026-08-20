#!/usr/bin/env python3

"""Run Masonry's complete local continuous-integration suite."""

from __future__ import annotations

import os
from pathlib import Path
import platform
import re
import shutil
import signal
import subprocess
import sys
import tempfile


REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
UNITY_VERSION = "6000.5.8f1"


def run_step(name: str, command: list[str] | None = None, function=None) -> None:
    print(f"\n==> {name}", flush=True)
    if function is not None:
        function()
    else:
        subprocess.run(command, cwd=REPOSITORY_ROOT, check=True)


def unity_editor() -> Path:
    if configured := os.environ.get("UNITY_EDITOR"):
        return Path(configured)
    if platform.system() == "Darwin":
        return Path(f"/Applications/Unity/Hub/Editor/{UNITY_VERSION}/Unity.app/Contents/MacOS/Unity")
    if platform.system() == "Linux":
        return Path.home() / f"Unity/Hub/Editor/{UNITY_VERSION}/Editor/Unity"
    raise RuntimeError("Unity is unsupported on this operating system.")


def project_unity_version() -> str:
    version_file = REPOSITORY_ROOT / "ProjectSettings/ProjectVersion.txt"
    return next(
        line.removeprefix("m_EditorVersion: ")
        for line in version_file.read_text().splitlines()
        if line.startswith("m_EditorVersion: ")
    )


def print_tail(path: Path, count: int) -> None:
    print("\n".join(path.read_text(errors="replace").splitlines()[-count:]), file=sys.stderr)


def check_unity_compilation() -> None:
    editor = unity_editor()
    if not os.access(editor, os.X_OK):
        raise RuntimeError(
            f"Unity {project_unity_version()} was not found at {editor}. "
            "Set UNITY_EDITOR to its executable."
        )
    with tempfile.NamedTemporaryFile(prefix="masonry-unity-ci.", delete=False) as temporary:
        unity_log = Path(temporary.name)
    try:
        result = subprocess.run(
            [
                str(editor), "-batchmode", "-nographics", "--burst-disable-compilation", "-quit",
                "-projectPath", str(REPOSITORY_ROOT), "-executeMethod", "Masonry.Editor.Ci.Run",
                "-logFile", str(unity_log),
            ],
            cwd=REPOSITORY_ROOT,
        )
        if result.returncode != 0:
            pattern = re.compile(
                r"^(?:Assets|Packages)/.*: error |Aborting batchmode|Scripts have compiler errors"
            )
            errors = list(dict.fromkeys(line for line in unity_log.read_text(errors="replace").splitlines() if pattern.search(line)))
            if errors:
                print("\n".join(errors), file=sys.stderr)
            else:
                print_tail(unity_log, 80)
            raise RuntimeError("Unity compilation failed.")
        if "CI Unity compilation check passed." not in unity_log.read_text(errors="replace"):
            print_tail(unity_log, 200)
            raise RuntimeError("Unity exited without completing the compilation check.")
    finally:
        unity_log.unlink(missing_ok=True)


def check_unity_analyzer_diagnostics() -> None:
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
    environment["MASONRY_UNITY_ANALYZER_PATH"] = str(analyzer)
    subprocess.run(
        ["dotnet", "format", "masonry.slnx", "analyzers", "--verify-no-changes", "--severity", "info"],
        cwd=REPOSITORY_ROOT,
        env=environment,
        check=True,
    )


def run_unity_edit_mode_tests() -> None:
    editor = unity_editor()
    if not os.access(editor, os.X_OK):
        raise RuntimeError(f"Unity executable was not found at {editor}. Set UNITY_EDITOR to its executable.")
    with tempfile.NamedTemporaryFile(prefix="masonry-unity-tests-log.", delete=False) as log_file:
        test_log = Path(log_file.name)
    with tempfile.NamedTemporaryFile(prefix="masonry-unity-tests-results.", delete=False) as result_file:
        test_results = Path(result_file.name)
    native_fixture = REPOSITORY_ROOT / "target/unity-native-fixture/debug"
    native_fixture_link = REPOSITORY_ROOT / "masonry_rules"
    try:
        subprocess.run(
            [
                "cargo", "build", "--quiet", "-p", "masonry-native-export-fixture",
                "--target-dir", str(REPOSITORY_ROOT / "target/unity-native-fixture"),
            ],
            cwd=REPOSITORY_ROOT,
            check=True,
        )
        library_name = {
            "Darwin": "libmasonry_rules.dylib",
            "Linux": "libmasonry_rules.so",
        }.get(platform.system(), "masonry_rules.dll")
        shutil.copy2(native_fixture / library_name, native_fixture_link)
        environment = os.environ.copy()
        for variable in ("DYLD_LIBRARY_PATH", "LD_LIBRARY_PATH"):
            environment[variable] = os.pathsep.join(
                value for value in (str(native_fixture), environment.get(variable)) if value
            )
        environment["PATH"] = os.pathsep.join((str(native_fixture), environment["PATH"]))
        result = subprocess.run(
            [
                str(editor), "-batchmode", "-nographics", "--burst-disable-compilation",
                "-projectPath", str(REPOSITORY_ROOT), "-runTests", "-testPlatform", "EditMode",
                "-testResults", str(test_results), "-logFile", str(test_log),
            ],
            cwd=REPOSITORY_ROOT,
            env=environment,
        )
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
        if re.search(r'<test-run[^>]*testcasecount="[1-9][0-9]*"[^>]*result="Passed"', results) is None:
            print(results, file=sys.stderr)
            raise RuntimeError("Unity did not report a passing Edit Mode test run.")
    finally:
        test_log.unlink(missing_ok=True)
        test_results.unlink(missing_ok=True)
        native_fixture_link.unlink(missing_ok=True)


def check_csharp_line_lengths() -> None:
    violations = []
    for root in (REPOSITORY_ROOT / "Assets", REPOSITORY_ROOT / "Packages/com.masonry.client"):
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


def main() -> None:
    run_step("Check Rust formatting", ["cargo", "fmt", "--all", "--", "--check"])
    run_step("Lint Rust crates", ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"])
    run_step("Test Rust crates", ["cargo", "test", "--workspace"])
    run_step(
        "Test visual capture workflow",
        [sys.executable, "scripts/tests/visual-capture-workflow.test.py"],
    )
    run_step("Restore local .NET tools", ["dotnet", "tool", "restore"])
    run_step("Check C# formatting", ["dotnet", "csharpier", "check", "."])
    run_step("Check C# line lengths", function=check_csharp_line_lengths)
    run_step("Check Unity compilation and analyzers", function=check_unity_compilation)
    run_step("Check Unity analyzer diagnostics", function=check_unity_analyzer_diagnostics)
    run_step(
        "Check C# diagnostics",
        [
            "dotnet", "format", "masonry.slnx", "style", "--verify-no-changes", "--diagnostics",
            "IDE0004", "IDE0005", "IDE0010", "IDE0035", "IDE0043", "IDE0059", "IDE0079",
            "IDE0080", "IDE0240", "IDE0241",
        ],
    )
    run_step("Run Unity Edit Mode tests", function=run_unity_edit_mode_tests)
    run_step("Refresh tracked file metadata", ["git", "update-index", "--refresh"])


def interrupted(_signal_number, _frame) -> None:
    raise KeyboardInterrupt


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGINT, interrupted)
    try:
        main()
    except KeyboardInterrupt:
        raise SystemExit(130) from None
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        raise SystemExit(1) from error
