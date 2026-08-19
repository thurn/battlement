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

check_unity_analyzer_diagnostics() {
    masonry_unity_analyzers=$(sed -n '
        /Library\/PackageCache\/org\.nuget\.microsoft\.unity\.analyzers@.*\/Microsoft\.Unity\.Analyzers\.dll/ {
            s/.*Include="\([^"]*\)".*/\1/
            p
        }
    ' Assembly-CSharp-Editor.csproj)
    masonry_unity_analyzer_count=$(printf '%s\n' "$masonry_unity_analyzers" \
        | awk 'NF { count++ } END { print count + 0 }')
    if [ "$masonry_unity_analyzer_count" -ne 1 ]; then
        printf 'Expected one active Microsoft.Unity.Analyzers package, found %s.\n' \
            "$masonry_unity_analyzer_count" >&2
        return 1
    fi
    if [ ! -f "$masonry_unity_analyzers" ]; then
        printf 'Microsoft.Unity.Analyzers was not found at %s.\n' \
            "$masonry_unity_analyzers" >&2
        return 1
    fi

    env MASONRY_UNITY_ANALYZER_PATH="$masonry_unity_analyzers" \
        dotnet format masonry.slnx analyzers --verify-no-changes --severity info
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

    native_fixture="$repository_root/target/unity-native-fixture/debug"
    native_fixture_link="$repository_root/masonry_rules"
    cargo build --quiet -p masonry-native-export-fixture \
        --target-dir "$repository_root/target/unity-native-fixture"
    case $(uname -s) in
        Darwin) cp "$native_fixture/libmasonry_rules.dylib" "$native_fixture_link" ;;
        Linux) cp "$native_fixture/libmasonry_rules.so" "$native_fixture_link" ;;
        *) cp "$native_fixture/masonry_rules.dll" "$native_fixture_link" ;;
    esac
    trap 'rm -f "$test_log" "$test_results" "$native_fixture_link"' EXIT HUP INT TERM

    if ! env \
        DYLD_LIBRARY_PATH="$native_fixture${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}" \
        LD_LIBRARY_PATH="$native_fixture${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" \
        PATH="$native_fixture:$PATH" \
        "$unity_editor" \
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

    rm -f "$test_log" "$test_results" "$native_fixture_link"
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
run_step "Test visual capture workflow" \
    ./scripts/tests/visual-capture-workflow.test.sh
run_step "Restore local .NET tools" dotnet tool restore
run_step "Check C# formatting" dotnet csharpier check .
run_step "Check C# line lengths" check_csharp_line_lengths
run_step "Check Unity compilation and analyzers" check_unity_compilation
run_step "Check Unity analyzer diagnostics" check_unity_analyzer_diagnostics
run_step "Check C# diagnostics" \
    dotnet format masonry.slnx style --verify-no-changes --diagnostics \
        IDE0004 IDE0005 IDE0010 IDE0035 IDE0043 IDE0059 IDE0079 IDE0080 IDE0240 \
        IDE0241
run_step "Run Unity Edit Mode tests" run_unity_edit_mode_tests
run_step "Refresh tracked file metadata" git update-index --refresh
