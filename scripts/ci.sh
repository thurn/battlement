#!/bin/sh

set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repository_root"

run_step() {
    step_name=$1
    shift

    printf '\n==> %s\n' "$step_name"
    "$@"
}

find_unity_editor() {
    if [ -n "${UNITY_EDITOR:-}" ]; then
        printf '%s\n' "$UNITY_EDITOR"
        return
    fi

    unity_version=6000.5.8f1

    case $(uname -s) in
        Darwin)
            printf '/Applications/Unity/Hub/Editor/%s/Unity.app/Contents/MacOS/Unity\n' "$unity_version"
            ;;
        Linux)
            printf '%s/Unity/Hub/Editor/%s/Editor/Unity\n' "$HOME" "$unity_version"
            ;;
        *)
            return 1
            ;;
    esac
}

check_unity_compilation() {
    unity_editor=$(find_unity_editor)
    if [ ! -x "$unity_editor" ]; then
        printf 'Unity %s was not found at %s. Set UNITY_EDITOR to its executable.\n' \
            "$(sed -n 's/^m_EditorVersion: //p' ProjectSettings/ProjectVersion.txt)" \
            "$unity_editor" >&2
        return 1
    fi

    unity_log=$(mktemp "${TMPDIR:-/tmp}/masonry-unity-ci.XXXXXX")
    trap 'rm -f "$unity_log"' EXIT HUP INT TERM

    if ! "$unity_editor" \
        -batchmode \
        -nographics \
        --burst-disable-compilation \
        -quit \
        -projectPath "$repository_root" \
        -executeMethod Masonry.Editor.Ci.Run \
        -logFile "$unity_log"; then
        if ! awk '
            /^(Assets|Packages)\/.*: error |Aborting batchmode|Scripts have compiler errors/ {
                if (!seen[$0]++) print
                found = 1
            }
            END { exit found ? 0 : 1 }
        ' "$unity_log" >&2; then
            tail -n 80 "$unity_log" >&2
        fi
        return 1
    fi

    if ! grep -q "CI Unity compilation check passed." "$unity_log"; then
        tail -n 200 "$unity_log" >&2
        printf 'Unity exited without completing the compilation check.\n' >&2
        return 1
    fi

    rm -f "$unity_log"
    trap - EXIT HUP INT TERM
}

run_unity_edit_mode_tests() {
    unity_editor=$(find_unity_editor)
    if [ ! -x "$unity_editor" ]; then
        printf 'Unity executable was not found at %s. Set UNITY_EDITOR to its executable.\n' \
            "$unity_editor" >&2
        return 1
    fi

    test_log=$(mktemp "${TMPDIR:-/tmp}/masonry-unity-tests-log.XXXXXX")
    test_results=$(mktemp "${TMPDIR:-/tmp}/masonry-unity-tests-results.XXXXXX")
    trap 'rm -f "$test_log" "$test_results"' EXIT HUP INT TERM

    if ! "$unity_editor" \
        -batchmode \
        -nographics \
        --burst-disable-compilation \
        -projectPath "$repository_root" \
        -runTests \
        -testPlatform EditMode \
        -testResults "$test_results" \
        -logFile "$test_log"; then
        if [ -s "$test_results" ]; then
            awk '/<test-case .*result="Failed"/,/<\/test-case>/' "$test_results" >&2
        else
            tail -n 120 "$test_log" >&2
        fi
        return 1
    fi

    if ! grep -Eq '<test-run[^>]*testcasecount="[1-9][0-9]*"[^>]*result="Passed"' \
        "$test_results"; then
        cat "$test_results" >&2
        printf 'Unity did not report a passing Edit Mode test run.\n' >&2
        return 1
    fi

    rm -f "$test_log" "$test_results"
    trap - EXIT HUP INT TERM
}

check_csharp_line_lengths() {
    if ! find Assets Packages/com.masonry.client -type f -name '*.cs' -print0 \
        | xargs -0 awk -v maximum=100 '
        length($0) > maximum {
            printf "%s:%d: line is %d characters; maximum is %d\n", FILENAME, FNR, length($0), maximum
            found = 1
        }
        END { exit found ? 1 : 0 }
    '; then
        return 1
    fi
}

run_step "Check Rust formatting" \
    cargo fmt --all -- --check
run_step "Lint Rust crates" \
    cargo clippy --workspace --all-targets -- -D warnings
run_step "Test Rust crates" \
    cargo test --workspace
run_step "Restore local .NET tools" dotnet tool restore
run_step "Check C# formatting" dotnet csharpier check .
run_step "Check C# line lengths" check_csharp_line_lengths
run_step "Check Unity compilation and analyzers" check_unity_compilation
run_step "Run Unity Edit Mode tests" run_unity_edit_mode_tests
run_step "Refresh tracked file metadata" git update-index --refresh
