#!/bin/sh

set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$repository_root/scripts/visual-capture-lib.sh"
cd "$repository_root"
unity_version=$(sed -n 's/^m_EditorVersion: //p' ProjectSettings/ProjectVersion.txt)
unity_editor=${UNITY_EDITOR:-/Applications/Unity/Hub/Editor/$unity_version/Unity.app/Contents/MacOS/Unity}
task_id= scenario= scene= plugin= cargo_package=
transport=none
artifact_root="$repository_root/artifacts/visual-evidence"
build_cache_root=
capture_kind=png
mode=capture
width=1280
height=720
video_seconds=5
initial_hold_seconds=2
interaction_timeout=15
run_id=$(date -u '+%Y%m%dT%H%M%SZ')-$$
show_overlay=0

usage() {
    cat <<'EOF'
Usage: scripts/capture-visual-evidence.sh --task ID --scenario NAME [options]

Options:
  --scene PATH             Authored Unity scenario scene under Assets (required)
  --plugin PATH            Prebuilt host-architecture libmasonry_rules.dylib
  --cargo-package NAME     Cargo package that builds libmasonry_rules.dylib
  --transport NAME         native, http, or none (default: none)
  --artifact-root PATH     Evidence root (default: artifacts/visual-evidence)
  --build-cache PATH       Content-addressed player cache (default: ROOT/.build-cache)
  --capture KIND           png, video, or both (default: png)
  --smoke                  Run the packaged protocol without producing media
  --dimensions WIDTHxHEIGHT (default: 1280x720)
  --video-seconds N        Video duration from 1 to 60 seconds (default: 5)
  --initial-hold-seconds N Stable video opening before first input (default: 2)
  --interaction-timeout N Seconds allowed for input requests (default: 15)
  --run-id ID              Unique run name (default: UTC timestamp and process ID)
  --show-overlay           Show capture diagnostics in the player
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
        --build-cache) build_cache_root=${2-}; shift 2 ;;
        --capture) capture_kind=${2-}; shift 2 ;;
        --smoke) mode=smoke; shift ;;
        --dimensions) width=${2%x*}; height=${2#*x}; shift 2 ;;
        --video-seconds) video_seconds=${2-}; shift 2 ;;
        --initial-hold-seconds) initial_hold_seconds=${2-}; shift 2 ;;
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
[ -f "$repository_root/$scene" ] || { printf 'Capture scene was not found: %s\n' "$scene" >&2; exit 2; }
if [ -n "$plugin" ] && [ -n "$cargo_package" ]; then
    printf 'Use only one of --plugin and --cargo-package.\n' >&2; exit 2
fi
if { [ -n "$plugin" ] || [ -n "$cargo_package" ]; } && [ "$transport" != native ]; then
    printf 'A native plugin requires --transport native.\n' >&2; exit 2
fi
if [ "$transport" = native ] && [ -z "$plugin" ] && [ -z "$cargo_package" ]; then
    printf -- '--transport native requires --plugin or --cargo-package.\n' >&2; exit 2
fi
case $width:$height in *[!0-9:]*|:*|*:) printf 'Dimensions must be WIDTHxHEIGHT.\n' >&2; exit 2 ;; esac
if [ "$width" -lt 320 ] || [ "$height" -lt 240 ]; then
    printf 'Capture dimensions must be at least 320x240.\n' >&2; exit 2
fi
case $video_seconds in ''|*[!0-9]*) printf 'Video duration must be an integer.\n' >&2; exit 2 ;; esac
if [ "$video_seconds" -lt 1 ] || [ "$video_seconds" -gt 60 ]; then
    printf 'Video duration must be between 1 and 60 seconds.\n' >&2; exit 2
fi
capture_is_nonnegative_number "$initial_hold_seconds" \
    || { printf 'Initial hold must be a nonnegative number.\n' >&2; exit 2; }
case $interaction_timeout in ''|*[!0-9]*) printf 'Interaction timeout must be an integer.\n' >&2; exit 2 ;; esac
if [ "$interaction_timeout" -lt 1 ] || [ "$interaction_timeout" -gt 120 ]; then
    printf 'Interaction timeout must be between 1 and 120 seconds.\n' >&2; exit 2
fi
[ "$(uname -s)" = Darwin ] \
    || { printf 'Release-player capture is supported on macOS only.\n' >&2; exit 1; }
[ -x "$unity_editor" ] \
    || { printf 'Unity %s was not found at %s.\n' "$unity_version" "$unity_editor" >&2; exit 1; }
if [ -n "$plugin" ] && [ ! -f "$plugin" ]; then
    printf 'Native plugin was not found: %s\n' "$plugin" >&2; exit 2
fi

[ -n "$build_cache_root" ] || build_cache_root="$artifact_root/.build-cache"
revision=$(git rev-parse HEAD)
plugin_identity=
if [ -n "$plugin" ]; then
    plugin_identity=$(capture_sha256 "$plugin" | awk '{ print $1 }')
elif [ -n "$cargo_package" ]; then
    plugin_identity="cargo:$cargo_package"
fi
content_fingerprint=$(capture_project_fingerprint "$repository_root" "$scene" \
    "$scenario" "$transport" "$plugin_identity")
identity="${revision}-$(printf '%.12s' "$content_fingerprint")"
output_directory="$artifact_root/$identity/$task_id/$run_id"
[ ! -e "$output_directory" ] \
    || { printf 'Refusing to overwrite capture run: %s\n' "$output_directory" >&2; exit 1; }
mkdir -p "$output_directory"
run_log="$output_directory/$run_id.log"
temporary_root=$(CDPATH= cd -- "$(mktemp -d "${TMPDIR:-/tmp}/masonry-capture.XXXXXX")" && pwd -P)
initial_tracked_state="$temporary_root/tracked-state.before"
capture_tracked_state "$repository_root" >"$initial_tracked_state"
isolated_project="$temporary_root/project"
status_path="$temporary_root/player-status.json"
unity_log="$temporary_root/unity-build.log"
player_log="$temporary_root/player.log"
helper="$temporary_root/macos-capture"
build_key=$(printf '%s\0%s\0' "$content_fingerprint" "$unity_version" \
    | capture_sha256 | awk '{ print $1 }')
cache_directory="$build_cache_root/$build_key"
build_path="$cache_directory/Masonry Capture.app"
cache_manifest="$cache_directory/manifest.json"
player_pid= caffeinate_pid= recorder_pid=
pointer_button_down=0
last_pointer_x=0
last_pointer_y=0

log() { printf '%s\n' "$*" | tee -a "$run_log"; }

cleanup() {
    cleanup_status=$?
    trap - EXIT HUP INT TERM
    if [ -n "$recorder_pid" ] && kill -0 "$recorder_pid" 2>/dev/null; then
        kill "$recorder_pid" 2>/dev/null || true; wait "$recorder_pid" 2>/dev/null || true
    fi
    if [ "$pointer_button_down" -eq 1 ] && [ -x "$helper" ] && [ -n "$player_pid" ]; then
        "$helper" pointer-left-button-up "$player_pid" "$last_pointer_x" "$last_pointer_y" 2>/dev/null || true
    fi
    if [ -n "$player_pid" ] && kill -0 "$player_pid" 2>/dev/null; then
        kill "$player_pid" 2>/dev/null || true; wait "$player_pid" 2>/dev/null || true
    fi
    if [ -n "$caffeinate_pid" ] && kill -0 "$caffeinate_pid" 2>/dev/null; then
        kill "$caffeinate_pid" 2>/dev/null || true; wait "$caffeinate_pid" 2>/dev/null || true
    fi
    final_tracked_state="$temporary_root/tracked-state.after"
    capture_tracked_state "$repository_root" >"$final_tracked_state"
    if ! cmp -s "$initial_tracked_state" "$final_tracked_state"; then
        printf 'Unexpected tracked repository changes occurred during capture:\n' | tee -a "$run_log" >&2
        diff -u "$initial_tracked_state" "$final_tracked_state" | tee -a "$run_log" >&2 || true
        cleanup_status=1
    else
        printf 'repository cleanliness check passed\n' | tee -a "$run_log"
    fi
    rm -rf "$temporary_root"
    exit "$cleanup_status"
}
trap cleanup EXIT HUP INT TERM

log "capture run $run_id"
log "source commit $revision"
log "content fingerprint $content_fingerprint"
log "artifact identity $identity"
log "task $task_id scenario $scenario mode $mode dimensions ${width}x${height}"
log "scene $scene transport $transport"

swiftc scripts/macos-capture.swift -o "$helper" -framework AppKit \
    -framework ApplicationServices -framework AVFoundation -framework ScreenCaptureKit
if [ "$mode" = smoke ]; then "$helper" preflight-input >>"$run_log" 2>&1
else "$helper" preflight >>"$run_log" 2>&1; fi
caffeinate -dims -w $$ &
caffeinate_pid=$!
caffeinate -u -t 2

cache_valid=0
if [ -f "$build_path/Contents/Info.plist" ] && [ -f "$cache_manifest" ] \
    && jq -e --arg fingerprint "$content_fingerprint" --arg scene "$scene" \
        --arg scenario "$scenario" --arg unity "$unity_version" \
        '.fingerprint == $fingerprint and .scene == $scene
            and .scenario == $scenario and .unity == $unity' \
        "$cache_manifest" >/dev/null; then
    cache_valid=1
    log "reusing verified packaged player $build_path"
fi
if [ "$cache_valid" -eq 0 ] && [ -e "$cache_directory" ]; then
    log "discarding an incomplete or invalid cache entry $cache_directory"
    rm -rf "$cache_directory"
fi

if [ "$cache_valid" -eq 0 ]; then
    log "building isolated non-Development macOS player with Unity $unity_version"
    mkdir -p "$isolated_project"
    rsync -a --exclude .git --exclude .worktrees --exclude Library --exclude Temp \
        --exclude Logs --exclude obj --exclude target --exclude artifacts \
        "$repository_root/" "$isolated_project/"
    if [ -n "$plugin" ] || [ -n "$cargo_package" ]; then
        isolated_plugin_directory="$isolated_project/Assets/Plugins/macOS"
        isolated_plugin_path="$isolated_plugin_directory/libmasonry_rules.dylib"
        mkdir -p "$isolated_plugin_directory"
        if [ -n "$cargo_package" ]; then
            cargo build --quiet --release -p "$cargo_package" \
                --manifest-path "$isolated_project/Cargo.toml" \
                --target-dir "$temporary_root/rust-target"
            plugin="$temporary_root/rust-target/release/libmasonry_rules.dylib"
        fi
        cp "$plugin" "$isolated_plugin_path"
        lipo -archs "$isolated_plugin_path" | tr ' ' '\n' | grep -qx "$(uname -m)" \
            || { log "The native plugin lacks host architecture $(uname -m)."; exit 1; }
    fi
    uncached_build="$temporary_root/Masonry Capture.app"
    if ! MASONRY_CAPTURE_BUILD_PATH="$uncached_build" MASONRY_CAPTURE_SCENE_PATH="$scene" \
        MASONRY_CAPTURE_SCENARIO="$scenario" "$unity_editor" -batchmode -nographics \
        --burst-disable-compilation -quit -projectPath "$isolated_project" \
        -executeMethod Masonry.Editor.VisualCaptureBuild.Build -logFile "$unity_log"; then
        tail -n 120 "$unity_log" >>"$run_log"; log "Unity release player build failed."; exit 1
    fi
    grep -q "MASONRY_CAPTURE_BUILD_OK:$uncached_build" "$unity_log" \
        || { tail -n 120 "$unity_log" >>"$run_log"; log "Unity omitted the build success marker."; exit 1; }
    cache_staging="$temporary_root/cache"
    mkdir -p "$cache_staging"
    ditto "$uncached_build" "$cache_staging/Masonry Capture.app"
    jq -n --arg fingerprint "$content_fingerprint" --arg revision "$revision" \
        --arg scene "$scene" --arg scenario "$scenario" --arg unity "$unity_version" \
        '{fingerprint: $fingerprint, revision: $revision, scene: $scene,
            scenario: $scenario, unity: $unity}' >"$cache_staging/manifest.json"
    mkdir -p "$build_cache_root"
    [ -e "$cache_directory" ] || mv "$cache_staging" "$cache_directory"
    log "cached packaged player $build_path"
fi

executable_name=$(plutil -extract CFBundleExecutable raw "$build_path/Contents/Info.plist")
player_executable="$build_path/Contents/MacOS/$executable_name"
packaged_plugin="$build_path/Contents/PlugIns/libmasonry_rules.dylib"
[ -x "$player_executable" ] || { log "The player executable is missing."; exit 1; }
if [ "$transport" = native ]; then
    [ -f "$packaged_plugin" ] || { log "The bundled native plugin is missing."; exit 1; }
    lipo -archs "$packaged_plugin" | tr ' ' '\n' | grep -qx "$(uname -m)" \
        || { log "The packaged dylib lacks host architecture $(uname -m)."; exit 1; }
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
    [ -z "$player_pid" ] || break
    sleep 0.1
done
[ -n "$player_pid" ] || { log "The launched player process could not be found."; exit 1; }

deadline=$(( $(date +%s) + 45 ))
phase=
while [ "$(date +%s)" -lt "$deadline" ]; do
    if ! kill -0 "$player_pid" 2>/dev/null; then
        wait "$player_pid" || true; tail -n 120 "$player_log" >>"$run_log"
        log "Player exited before the ready signal."; exit 1
    fi
    if [ -s "$status_path" ]; then
        phase=$(jq -r '.phase // empty' "$status_path" 2>/dev/null || true)
        case $phase in
            ready) break ;;
            failed) jq -r '.failure // "unknown player failure"' "$status_path" >>"$run_log"
                log "Player failed before the ready signal."; exit 1 ;;
        esac
    fi
    sleep 0.1
done
[ "$phase" = ready ] || { tail -n 120 "$player_log" >>"$run_log"; log "Ready signal timed out."; exit 1; }

window_data=
window_deadline=$(( $(date +%s) + 10 ))
while [ "$(date +%s)" -lt "$window_deadline" ]; do
    window_data=$("$helper" window "$player_pid" 2>/dev/null || true)
    [ -z "$window_data" ] || break
    sleep 0.1
done
[ -n "$window_data" ] || { log "Could not locate the player content window."; exit 1; }
set -- $window_data
window_id=$1 window_x=$2 window_y=$3 window_width=$4 window_height=$5
"$helper" focus "$player_pid"

video_path="$output_directory/$run_id.mp4"
before_png_path="$output_directory/$run_id-before.png"
after_png_path="$output_directory/$run_id-after.png"
recording_ready="$temporary_root/recording-ready"
hold_started_at=
if [ "$mode" = capture ] && { [ "$capture_kind" = video ] || [ "$capture_kind" = both ]; }; then
    log "recording H.264 interaction video with ${initial_hold_seconds}s initial hold"
    "$helper" record-window "$window_id" "$video_path" "$video_seconds" \
        "$width" "$height" "$recording_ready" &
    recorder_pid=$!
    recording_deadline=$(( $(date +%s) + 10 ))
    while [ ! -e "$recording_ready" ] && [ "$(date +%s)" -lt "$recording_deadline" ]; do
        if ! kill -0 "$recorder_pid" 2>/dev/null; then
            wait "$recorder_pid" || true; recorder_pid=
            log "Video capture failed before recording began."; exit 1
        fi
        sleep 0.05
    done
    [ -e "$recording_ready" ] || { log "Timed out waiting for recording."; exit 1; }
    hold_started_at=$(capture_now)
fi

if [ "$mode" = capture ] && { [ "$capture_kind" = png ] || [ "$capture_kind" = both ]; }; then
    /usr/sbin/screencapture -x -o -l "$window_id" "$before_png_path"
    capture_verify_png_dimensions "$before_png_path" "$width" "$height" \
        || { log "Before PNG dimensions did not match ${width}x${height}."; exit 1; }
    log "before PNG $before_png_path"
fi
[ -z "$hold_started_at" ] || capture_wait_for_initial_hold "$hold_started_at" "$initial_hold_seconds"

deadline=$(( $(date +%s) + interaction_timeout ))
phase= last_request=0 first_dispatch_at=
while [ "$(date +%s)" -lt "$deadline" ]; do
    kill -0 "$player_pid" 2>/dev/null \
        || { tail -n 120 "$player_log" >>"$run_log"; log "Player crashed before assertions."; exit 1; }
    if [ -n "$recorder_pid" ] && ! kill -0 "$recorder_pid" 2>/dev/null; then
        wait "$recorder_pid" || true; recorder_pid=
        log "Video ended before the scenario completed."; exit 1
    fi
    status_data=$(jq -r \
        '[.phase // "", .requestId // -1, .input // "", .pointerX // -1, .pointerY // -1] | @tsv' \
        "$status_path" 2>/dev/null || true)
    set -- $status_data
    phase=${1-}
    case $phase in
        passed|failed) break ;;
        ready)
            request_id=${2-} input=${3-} pointer_x=${4-} pointer_y=${5-}
            if [ "$request_id" != "$last_request" ]; then
                case $request_id in ''|*[!0-9]*) log "Input request ID was invalid."; exit 1 ;; esac
                [ "$request_id" -eq "$((last_request + 1))" ] \
                    || { log "Input request IDs must be consecutive."; exit 1; }
                case $input in pointer-move|pointer-left-button-down|pointer-left-button-up) ;;
                    *) log "Unsupported capture input: $input"; exit 1 ;; esac
                awk -v x="$pointer_x" -v y="$pointer_y" \
                    'BEGIN { exit !(x != "" && y != "" && x >= 0 && x <= 1 && y >= 0 && y <= 1) }' \
                    || { log "Input request lacks normalized coordinates."; exit 1; }
                last_pointer_x=$(awk -v o="$window_x" -v s="$window_width" -v v="$pointer_x" \
                    'BEGIN { printf "%d", o + (s * v) }')
                last_pointer_y=$(awk -v o="$window_y" -v s="$window_height" -v v="$pointer_y" \
                    'BEGIN { printf "%d", o + (s * v) }')
                case $input in
                    pointer-left-button-down) [ "$pointer_button_down" -eq 0 ] \
                        || { log "Primary pointer button was already pressed."; exit 1; } ;;
                    pointer-left-button-up) [ "$pointer_button_down" -eq 1 ] \
                        || { log "Primary pointer button was not pressed."; exit 1; } ;;
                esac
                dispatched_input=$input
                if [ "$input" = pointer-move ] && [ "$pointer_button_down" -eq 1 ]; then
                    dispatched_input=pointer-left-drag
                fi
                if [ -z "$first_dispatch_at" ]; then
                    first_dispatch_at=$(capture_now)
                    if [ -n "$hold_started_at" ]; then
                        hold_elapsed=$(awk -v start="$hold_started_at" -v finish="$first_dispatch_at" \
                            'BEGIN { printf "%.3f", finish - start }')
                        log "first input dispatched ${hold_elapsed}s after recording started"
                    fi
                fi
                "$helper" "$dispatched_input" "$player_pid" "$last_pointer_x" "$last_pointer_y"
                case $input in pointer-left-button-down) pointer_button_down=1 ;;
                    pointer-left-button-up) pointer_button_down=0 ;; esac
                last_request=$request_id
            fi ;;
    esac
    sleep 0.1
done
if [ "$phase" != passed ]; then
    jq -r '.failure // "capture assertions timed out"' "$status_path" >>"$run_log"
    log "Player assertions did not pass."; exit 1
fi
[ "$pointer_button_down" -eq 0 ] \
    || { log "Scenario passed with the primary pointer button pressed."; exit 1; }

if [ "$mode" = capture ] && { [ "$capture_kind" = png ] || [ "$capture_kind" = both ]; }; then
    /usr/sbin/screencapture -x -o -l "$window_id" "$after_png_path"
    capture_verify_png_dimensions "$after_png_path" "$width" "$height" \
        || { log "After PNG dimensions did not match ${width}x${height}."; exit 1; }
    log "after PNG $after_png_path"
fi
if [ -n "$recorder_pid" ]; then
    if ! wait "$recorder_pid"; then recorder_pid=; log "Video capture failed."; exit 1; fi
    recorder_pid=
fi
if [ "$mode" = capture ] && { [ "$capture_kind" = video ] || [ "$capture_kind" = both ]; }; then
    video_details=$("$helper" inspect-video "$video_path")
    set -- $video_details
    [ "$1" -eq "$width" ] && [ "$2" -eq "$height" ] && [ "$4" = avc1 ] \
        || { log "MP4 inspection failed: $video_details"; exit 1; }
    awk -v rate="$3" 'BEGIN { exit !(rate >= 28 && rate <= 31) }' \
        || { log "MP4 frame rate was $3 rather than 30 fps."; exit 1; }
    log "video $video_path"
fi

assertions=$(jq -c '.assertions' "$status_path")
log "assertions passed: $(printf '%s' "$assertions" | jq -r 'join(", ")')"
if [ "$mode" = smoke ]; then log "smoke validation passed; no media produced"
else log "evidence retained at $output_directory"; fi
