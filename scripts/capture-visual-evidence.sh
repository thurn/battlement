#!/bin/sh

set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repository_root"
unity_version=$(sed -n 's/^m_EditorVersion: //p' \
    "$repository_root/ProjectSettings/ProjectVersion.txt")
unity_editor=${UNITY_EDITOR:-/Applications/Unity/Hub/Editor/$unity_version/Unity.app/Contents/MacOS/Unity}
task_id=
scenario=
scene=
plugin=
cargo_package=
transport=none
artifact_root="$repository_root/artifacts/visual-evidence"
capture_kind=png
width=1280
height=720
video_seconds=5
interaction_timeout=15
run_id=$(date -u '+%Y%m%dT%H%M%SZ')-$$
show_overlay=0

usage() {
    cat <<'EOF'
Usage: scripts/capture-visual-evidence.sh --task ID --scenario NAME [options]

Options:
  --scene PATH           Authored Unity scenario scene under Assets (required)
  --plugin PATH          Prebuilt host-architecture libmasonry_rules.dylib
  --cargo-package NAME   Cargo package that builds libmasonry_rules.dylib
  --transport NAME       native, http, or none (default: none)
  --artifact-root PATH   Evidence root (default: artifacts/visual-evidence)
  --capture KIND         png, video, or both (default: png)
  --dimensions WIDTHxHEIGHT (default: 1280x720)
  --video-seconds N      Video duration from 1 to 60 seconds (default: 5)
  --interaction-timeout N Seconds allowed for input requests (default: 15)
  --run-id ID            Unique run name (default: UTC timestamp and process ID)
  --show-overlay         Show capture diagnostics in the player
EOF
}

while [ "$#" -gt 0 ]; do
    case $1 in
        --task) task_id=${2-}; shift 2 ;;
        --scenario) scenario=${2-}; shift 2 ;;
        --scene) scene=${2-}; shift 2 ;;
        --plugin) plugin=${2-}; shift 2 ;;
        --cargo-package) cargo_package=${2-}; shift 2 ;;
        --transport) transport=${2-}; shift 2 ;;
        --artifact-root) artifact_root=${2-}; shift 2 ;;
        --capture) capture_kind=${2-}; shift 2 ;;
        --dimensions)
            width=${2%x*}
            height=${2#*x}
            shift 2
            ;;
        --video-seconds) video_seconds=${2-}; shift 2 ;;
        --interaction-timeout) interaction_timeout=${2-}; shift 2 ;;
        --run-id) run_id=${2-}; shift 2 ;;
        --show-overlay) show_overlay=1; shift ;;
        --help|-h) usage; exit 0 ;;
        *) printf 'Unknown argument: %s\n' "$1" >&2; usage >&2; exit 2 ;;
    esac
done

case $task_id in ''|*[!A-Za-z0-9._-]*) printf 'A safe --task ID is required.\n' >&2; exit 2 ;; esac
case $scenario in ''|*[!A-Za-z0-9._-]*) printf 'A safe --scenario name is required.\n' >&2; exit 2 ;; esac
case $cargo_package in *[!A-Za-z0-9._-]*) printf 'Invalid --cargo-package name.\n' >&2; exit 2 ;; esac
case $run_id in ''|*[!A-Za-z0-9._-]*) printf 'A safe --run-id is required.\n' >&2; exit 2 ;; esac
case $capture_kind in png|video|both) ;; *) printf 'Invalid --capture kind.\n' >&2; exit 2 ;; esac
case $transport in native|http|none) ;; *) printf 'Invalid --transport name.\n' >&2; exit 2 ;; esac
case $scene in Assets/*.unity) ;; *) printf -- '--scene must name an Assets/*.unity file.\n' >&2; exit 2 ;; esac
case $scene in *../*) printf -- '--scene may not traverse parent directories.\n' >&2; exit 2 ;; esac
if [ ! -f "$repository_root/$scene" ]; then
    printf 'Capture scene was not found: %s\n' "$scene" >&2
    exit 2
fi
if [ -n "$plugin" ] && [ -n "$cargo_package" ]; then
    printf 'Use only one of --plugin and --cargo-package.\n' >&2
    exit 2
fi
if { [ -n "$plugin" ] || [ -n "$cargo_package" ]; } && [ "$transport" != native ]; then
    printf 'A native plugin requires --transport native.\n' >&2
    exit 2
fi
if [ "$transport" = native ] && [ -z "$plugin" ] && [ -z "$cargo_package" ]; then
    printf -- '--transport native requires --plugin or --cargo-package.\n' >&2
    exit 2
fi
case $width:$height in *[!0-9:]*|:*|*:) printf 'Dimensions must be WIDTHxHEIGHT.\n' >&2; exit 2 ;; esac
if [ "$width" -lt 320 ] || [ "$height" -lt 240 ]; then
    printf 'Capture dimensions must be at least 320x240.\n' >&2
    exit 2
fi
case $video_seconds in ''|*[!0-9]*) printf 'Video duration must be an integer.\n' >&2; exit 2 ;; esac
if [ "$video_seconds" -lt 1 ] || [ "$video_seconds" -gt 60 ]; then
    printf 'Video duration must be between 1 and 60 seconds.\n' >&2
    exit 2
fi
case $interaction_timeout in ''|*[!0-9]*) printf 'Interaction timeout must be an integer.\n' >&2; exit 2 ;; esac
if [ "$interaction_timeout" -lt 1 ] || [ "$interaction_timeout" -gt 120 ]; then
    printf 'Interaction timeout must be between 1 and 120 seconds.\n' >&2
    exit 2
fi
if [ "$(uname -s)" != Darwin ]; then
    printf 'Release-player capture is supported on macOS only.\n' >&2
    exit 1
fi
if [ ! -x "$unity_editor" ]; then
    printf 'Unity %s was not found at %s.\n' "$unity_version" "$unity_editor" >&2
    exit 1
fi

revision=$(git -C "$repository_root" rev-parse HEAD)
output_directory="$artifact_root/$revision/$task_id/$run_id"
if [ -e "$output_directory" ]; then
    printf 'Refusing to overwrite capture run: %s\n' "$output_directory" >&2
    exit 1
fi
mkdir -p "$output_directory"
run_log="$output_directory/$run_id.log"
temporary_root=$(CDPATH= cd -- "$(mktemp -d "${TMPDIR:-/tmp}/masonry-capture.XXXXXX")" \
    && pwd -P)
build_path="$temporary_root/Masonry Capture.app"
status_path="$temporary_root/player-status.json"
unity_log="$temporary_root/unity-build.log"
player_log="$temporary_root/player.log"
helper="$temporary_root/macos-capture"
plugin_root="$repository_root/Assets/Plugins"
plugin_directory="$repository_root/Assets/Plugins/macOS"
plugin_path="$plugin_directory/libmasonry_rules.dylib"
player_pid=
caffeinate_pid=
recorder_pid=
plugin_staged=0
pointer_button_down=0
last_pointer_x=0
last_pointer_y=0

log() {
    printf '%s\n' "$*" | tee -a "$run_log"
}

cleanup() {
    if [ -n "$recorder_pid" ] && kill -0 "$recorder_pid" 2>/dev/null; then
        kill "$recorder_pid" 2>/dev/null || true
        wait "$recorder_pid" 2>/dev/null || true
    fi
    if [ "$pointer_button_down" -eq 1 ] && [ -x "$helper" ] \
        && [ -n "$player_pid" ]; then
        "$helper" pointer-left-button-up "$player_pid" \
            "$last_pointer_x" "$last_pointer_y" 2>/dev/null || true
    fi
    if [ -n "$player_pid" ] && kill -0 "$player_pid" 2>/dev/null; then
        kill "$player_pid" 2>/dev/null || true
        wait "$player_pid" 2>/dev/null || true
    fi
    if [ -n "$caffeinate_pid" ] && kill -0 "$caffeinate_pid" 2>/dev/null; then
        kill "$caffeinate_pid" 2>/dev/null || true
        wait "$caffeinate_pid" 2>/dev/null || true
    fi
    if [ "$plugin_staged" -eq 1 ]; then
        rm -rf "$plugin_root" "$repository_root/Assets/Plugins.meta"
    fi
    rm -rf "$build_path"
    rm -rf "$temporary_root"
}
trap cleanup EXIT HUP INT TERM

log "capture run $run_id"
log "revision $revision"
log "task $task_id scenario $scenario kind $capture_kind dimensions ${width}x${height}"
log "scene $scene transport $transport"

swiftc "$repository_root/scripts/macos-capture.swift" -o "$helper" \
    -framework AppKit -framework ApplicationServices -framework AVFoundation \
    -framework ScreenCaptureKit
"$helper" preflight >>"$run_log" 2>&1
caffeinate -dims -w $$ &
caffeinate_pid=$!
caffeinate -u -t 2

if [ -n "$plugin" ] || [ -n "$cargo_package" ]; then
    if [ -e "$plugin_root" ]; then
        log "Generated plugin staging path already exists; refusing to overwrite it."
        exit 1
    fi
    if [ -n "$cargo_package" ]; then
        cargo build --quiet --release -p "$cargo_package" \
            --target-dir "$temporary_root/rust-target"
        plugin="$temporary_root/rust-target/release/libmasonry_rules.dylib"
    elif [ ! -f "$plugin" ]; then
        log "Native plugin was not found: $plugin"
        exit 1
    fi
    plugin_staged=1
    mkdir -p "$plugin_directory"
    cp "$plugin" "$plugin_path"
    if ! lipo -archs "$plugin_path" | tr ' ' '\n' | grep -qx "$(uname -m)"; then
        log "The native plugin does not contain the host architecture $(uname -m)."
        exit 1
    fi
fi

log "building non-Development macOS player with Unity $unity_version"
if ! MASONRY_CAPTURE_BUILD_PATH="$build_path" \
    MASONRY_CAPTURE_SCENE_PATH="$scene" \
    MASONRY_CAPTURE_SCENARIO="$scenario" "$unity_editor" \
    -batchmode -nographics --burst-disable-compilation -quit \
    -projectPath "$repository_root" \
    -executeMethod Masonry.Editor.VisualCaptureBuild.Build \
    -logFile "$unity_log"; then
    tail -n 120 "$unity_log" >>"$run_log"
    log "Unity release player build failed."
    exit 1
fi
if ! grep -q "MASONRY_CAPTURE_BUILD_OK:$build_path" "$unity_log"; then
    tail -n 120 "$unity_log" >>"$run_log"
    log "Unity exited without the capture build success marker."
    exit 1
fi

executable_name=$(plutil -extract CFBundleExecutable raw \
    "$build_path/Contents/Info.plist")
player_executable="$build_path/Contents/MacOS/$executable_name"
packaged_plugin="$build_path/Contents/PlugIns/libmasonry_rules.dylib"
if [ ! -x "$player_executable" ]; then
    log "The player executable is missing."
    exit 1
fi
if [ "$transport" = native ]; then
    if [ ! -f "$packaged_plugin" ]; then
        log "The bundled native plugin is missing."
        exit 1
    fi
    if ! lipo -archs "$packaged_plugin" | tr ' ' '\n' | grep -qx "$(uname -m)"; then
        log "The packaged dylib does not contain the host architecture $(uname -m)."
        exit 1
    fi
fi

log "launching packaged player without Editor library search paths"
env -u DYLD_LIBRARY_PATH -u DYLD_FRAMEWORK_PATH open -n "$build_path" --args \
    -popupwindow -screen-fullscreen 0 -screen-width "$width" -screen-height "$height" \
    -masonryCaptureScenario "$scenario" -masonryCaptureStatus "$status_path" \
    -logFile "$player_log" \
    $(if [ "$show_overlay" -eq 1 ]; then printf '%s' '-masonryCaptureOverlay'; fi)
pid_deadline=$(( $(date +%s) + 10 ))
while [ "$(date +%s)" -lt "$pid_deadline" ]; do
    player_pid=$(pgrep -f "$player_executable" | head -n 1 || true)
    [ -n "$player_pid" ] && break
    sleep 0.1
done
if [ -z "$player_pid" ]; then
    log "The launched player process could not be found."
    exit 1
fi

deadline=$(( $(date +%s) + 45 ))
phase=
while [ "$(date +%s)" -lt "$deadline" ]; do
    if ! kill -0 "$player_pid" 2>/dev/null; then
        wait "$player_pid" || true
        tail -n 120 "$player_log" >>"$run_log"
        log "Player exited before the ready signal."
        exit 1
    fi
    if [ -s "$status_path" ]; then
        phase=$(jq -r '.phase // empty' "$status_path" 2>/dev/null || true)
        case $phase in
            ready) break ;;
            failed)
                jq -r '.failure // "unknown player failure"' "$status_path" >>"$run_log"
                log "Player failed before the ready signal."
                exit 1
                ;;
        esac
    fi
    sleep 0.1
done
if [ "$phase" != ready ]; then
    tail -n 120 "$player_log" >>"$run_log"
    log "Timed out waiting for the player ready signal."
    exit 1
fi

window_data=
window_deadline=$(( $(date +%s) + 10 ))
while [ "$(date +%s)" -lt "$window_deadline" ]; do
    window_data=$("$helper" window "$player_pid" 2>/dev/null || true)
    [ -n "$window_data" ] && break
    sleep 0.1
done
if [ -z "$window_data" ]; then
    log "Could not locate the player content window."
    exit 1
fi
set -- $window_data
window_id=$1
window_x=$2
window_y=$3
window_width=$4
window_height=$5
"$helper" focus "$player_pid"

video_path="$output_directory/$run_id.mp4"
png_path="$output_directory/$run_id.png"
recording_ready="$temporary_root/recording-ready"
if [ "$capture_kind" = video ] || [ "$capture_kind" = both ]; then
    log "recording H.264 interaction video"
    "$helper" record-window "$window_id" "$video_path" "$video_seconds" \
        "$width" "$height" "$recording_ready" &
    recorder_pid=$!
    recording_deadline=$(( $(date +%s) + 10 ))
    while [ ! -e "$recording_ready" ] \
        && [ "$(date +%s)" -lt "$recording_deadline" ]; do
        if ! kill -0 "$recorder_pid" 2>/dev/null; then
            wait "$recorder_pid" || true
            recorder_pid=
            log "Video capture failed before recording began."
            exit 1
        fi
        sleep 0.05
    done
    if [ ! -e "$recording_ready" ]; then
        log "Timed out waiting for video recording to begin."
        exit 1
    fi
fi

deadline=$(( $(date +%s) + interaction_timeout ))
phase=
last_request=0
while [ "$(date +%s)" -lt "$deadline" ]; do
    if ! kill -0 "$player_pid" 2>/dev/null; then
        tail -n 120 "$player_log" >>"$run_log"
        log "Player crashed before completing assertions."
        exit 1
    fi
    if [ -n "$recorder_pid" ] && ! kill -0 "$recorder_pid" 2>/dev/null; then
        wait "$recorder_pid" || true
        recorder_pid=
        log "Video ended before the scenario completed."
        exit 1
    fi
    status_data=$(jq -r \
        '[.phase // "", .requestId // -1, .input // "", .pointerX // -1, .pointerY // -1] | @tsv' \
        "$status_path" 2>/dev/null || true)
    set -- $status_data
    phase=${1-}
    case $phase in
        passed) break ;;
        failed) break ;;
        ready)
            request_id=${2-}
            input=${3-}
            pointer_x=${4-}
            pointer_y=${5-}
            if [ "$request_id" != "$last_request" ]; then
                case $request_id in
                    ''|*[!0-9]*) log "Input request ID was invalid."; exit 1 ;;
                esac
                if [ "$request_id" -ne "$((last_request + 1))" ]; then
                    log "Input request IDs must be consecutive."
                    exit 1
                fi
                case $input in
                    pointer-move|pointer-left-button-down|pointer-left-button-up) ;;
                    *) log "Unsupported capture input: $input"; exit 1 ;;
                esac
                if ! awk -v x="$pointer_x" -v y="$pointer_y" \
                    'BEGIN { exit !(x != "" && y != "" && x >= 0 && x <= 1 && y >= 0 && y <= 1) }'; then
                    log "Input request did not provide normalized pointer coordinates."
                    exit 1
                fi
                last_pointer_x=$(awk -v origin="$window_x" -v size="$window_width" \
                    -v value="$pointer_x" 'BEGIN { printf "%d", origin + (size * value) }')
                last_pointer_y=$(awk -v origin="$window_y" -v size="$window_height" \
                    -v value="$pointer_y" 'BEGIN { printf "%d", origin + (size * value) }')
                case $input in
                    pointer-left-button-down)
                        if [ "$pointer_button_down" -eq 1 ]; then
                            log "Primary pointer button was already pressed."
                            exit 1
                        fi
                        ;;
                    pointer-left-button-up)
                        if [ "$pointer_button_down" -eq 0 ]; then
                            log "Primary pointer button was not pressed."
                            exit 1
                        fi
                        ;;
                esac
                dispatched_input=$input
                if [ "$input" = pointer-move ] && [ "$pointer_button_down" -eq 1 ]; then
                    dispatched_input=pointer-left-drag
                fi
                "$helper" "$dispatched_input" "$player_pid" \
                    "$last_pointer_x" "$last_pointer_y"
                case $input in
                    pointer-left-button-down) pointer_button_down=1 ;;
                    pointer-left-button-up) pointer_button_down=0 ;;
                esac
                last_request=$request_id
            fi
            ;;
    esac
    sleep 0.1
done
if [ "$phase" != passed ]; then
    jq -r '.failure // "capture assertions timed out"' "$status_path" >>"$run_log"
    log "Player assertions did not pass."
    exit 1
fi
if [ "$pointer_button_down" -ne 0 ]; then
    log "Scenario completed while the primary pointer button was still pressed."
    exit 1
fi

if [ -n "$recorder_pid" ]; then
    if ! wait "$recorder_pid"; then
        recorder_pid=
        log "Video capture failed."
        exit 1
    fi
    recorder_pid=
fi

if [ "$capture_kind" = png ] || [ "$capture_kind" = both ]; then
    /usr/sbin/screencapture -x -o -l "$window_id" "$png_path"
    image_width=$(sips -g pixelWidth "$png_path" | awk '/pixelWidth/ { print $2 }')
    image_height=$(sips -g pixelHeight "$png_path" | awk '/pixelHeight/ { print $2 }')
    if [ "$image_width" -ne "$width" ] || [ "$image_height" -ne "$height" ]; then
        log "PNG was ${image_width}x${image_height}; requested ${width}x${height}."
        exit 1
    fi
fi

video_details=
if [ "$capture_kind" = video ] || [ "$capture_kind" = both ]; then
    video_details=$("$helper" inspect-video "$video_path")
    set -- $video_details
    if [ "$1" -ne "$width" ] || [ "$2" -ne "$height" ] || [ "$4" != avc1 ]; then
        log "MP4 inspection failed: $video_details"
        exit 1
    fi
    if ! awk -v rate="$3" 'BEGIN { exit !(rate >= 28 && rate <= 31) }'; then
        log "MP4 frame rate was $3 rather than 30 fps."
        exit 1
    fi
fi

assertions=$(jq -c '.assertions' "$status_path")
log "assertions passed: $(printf '%s' "$assertions" | jq -r 'join(", ")')"
log "evidence retained at $output_directory"
