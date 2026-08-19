#!/bin/sh

set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$repository_root/scripts/visual-capture-lib.sh"
unity_version=$(sed -n 's/^m_EditorVersion: //p' \
    "$repository_root/ProjectSettings/ProjectVersion.txt")
unity_editor=${UNITY_EDITOR:-/Applications/Unity/Hub/Editor/$unity_version/Unity.app/Contents/MacOS/Unity}
scenario_name=
type_name=
output_directory=

usage() {
    cat <<'EOF'
Usage: scripts/scaffold-visual-capture.sh --scenario NAME --type TYPE --output Assets/PATH

Creates a formatted scenario component, its .meta, and an authored scene that
contains one matching scenario plus the reusable Masonry capture shell.
EOF
}

while [ "$#" -gt 0 ]; do
    case $1 in
        --scenario) scenario_name=${2-}; shift 2 ;;
        --type) type_name=${2-}; shift 2 ;;
        --output) output_directory=${2-}; shift 2 ;;
        --help|-h) usage; exit 0 ;;
        *) printf 'Unknown argument: %s\n' "$1" >&2; usage >&2; exit 2 ;;
    esac
done

case $scenario_name in
    ''|*[!A-Za-z0-9._-]*) printf 'A safe --scenario name is required.\n' >&2; exit 2 ;;
esac
case $type_name in
    ''|[!A-Za-z_]*|*[!A-Za-z0-9_]*) printf 'A C# --type name is required.\n' >&2; exit 2 ;;
esac
case $output_directory in
    Assets/*) ;;
    *) printf -- '--output must be a directory under Assets.\n' >&2; exit 2 ;;
esac
case $output_directory in *../*) printf -- '--output may not traverse parents.\n' >&2; exit 2 ;; esac
[ -x "$unity_editor" ] \
    || { printf 'Unity %s was not found at %s.\n' "$unity_version" "$unity_editor" >&2; exit 1; }

script_path="$output_directory/$type_name.cs"
scene_path="$output_directory/$type_name.unity"
if [ -e "$repository_root/$script_path" ] || [ -e "$repository_root/$scene_path" ]; then
    printf 'Refusing to overwrite existing scenario output.\n' >&2
    exit 1
fi
mkdir -p "$repository_root/$output_directory"
cat >"$repository_root/$script_path" <<EOF
#nullable enable

using Masonry.VisualCapture;
using UnityEngine;

public sealed class $type_name : MasonryCaptureScenario
{
    private bool awaitingPress;
    private bool awaitingRelease;

    public override string ScenarioName => "$scenario_name";

    protected override void BeginCapture()
    {
        Object.FindAnyObjectByType<MasonryCaptureShell>().SetPhase("Before interaction");
        awaitingPress = true;
        RequestInput(
            new[] { "initial-state-rendered" },
            CaptureInput.PointerLeftButtonDown,
            new Vector2(0.5f, 0.5f)
        );
    }

    private void Update()
    {
        if (awaitingPress && Input.GetMouseButtonDown(0))
        {
            awaitingPress = false;
            awaitingRelease = true;
            RequestInput(
                new[] { "initial-state-rendered", "requested-press-observed" },
                CaptureInput.PointerLeftButtonUp,
                new Vector2(0.5f, 0.5f)
            );
            return;
        }

        if (!awaitingRelease || !Input.GetMouseButtonUp(0))
        {
            return;
        }

        awaitingRelease = false;
        Object.FindAnyObjectByType<MasonryCaptureShell>().SetPhase("Interaction passed");
        SignalPassed(
            new[]
            {
                "initial-state-rendered",
                "requested-press-observed",
                "requested-release-observed",
            }
        );
    }
}
EOF
guid=$(printf '%s:%s' "$script_path" "$scenario_name" | capture_sha256 | cut -c 1-32)
cat >"$repository_root/$script_path.meta" <<EOF
fileFormatVersion: 2
guid: $guid
EOF

unity_log=$(mktemp "${TMPDIR:-/tmp}/masonry-scaffold.XXXXXX")
trap 'rm -f "$unity_log"' EXIT HUP INT TERM
if ! MASONRY_CAPTURE_SCAFFOLD_SCENE="$scene_path" \
    MASONRY_CAPTURE_SCAFFOLD_SCRIPT="$script_path" \
    MASONRY_CAPTURE_SCAFFOLD_TYPE="$type_name" "$unity_editor" \
    -batchmode -nographics --burst-disable-compilation -quit \
    -projectPath "$repository_root" \
    -executeMethod Masonry.Editor.VisualCaptureScaffold.CreateScene \
    -logFile "$unity_log"; then
    tail -n 120 "$unity_log" >&2
    exit 1
fi
if ! grep -q "MASONRY_CAPTURE_SCAFFOLD_OK:$scene_path" "$unity_log"; then
    tail -n 120 "$unity_log" >&2
    printf 'Unity exited without completing scenario scaffolding.\n' >&2
    exit 1
fi
printf 'Created %s, %s, and matching metadata.\n' "$script_path" "$scene_path"
