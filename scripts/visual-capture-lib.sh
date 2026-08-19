#!/bin/sh

# Shared, side-effect-free helpers for the visual evidence driver and its tests.

capture_is_nonnegative_number() {
    awk -v value="$1" 'BEGIN { exit !(value ~ /^[0-9]+([.][0-9]+)?$/) }'
}

capture_now() {
    perl -MTime::HiRes=time -e 'printf "%.6f\n", time'
}

capture_sleep() {
    sleep "$1"
}

capture_sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$@"
    else
        shasum -a 256 "$@"
    fi
}

capture_wait_for_initial_hold() {
    hold_started_at=$1
    hold_seconds=$2

    while :; do
        hold_remaining=$(awk -v started="$hold_started_at" -v now="$(capture_now)" \
            -v duration="$hold_seconds" \
            'BEGIN { remaining = started + duration - now; printf "%.6f", remaining }')
        if awk -v remaining="$hold_remaining" 'BEGIN { exit !(remaining <= 0) }'; then
            return
        fi
        capture_sleep "$hold_remaining"
    done
}

capture_project_fingerprint() {
    fingerprint_root=$1
    fingerprint_scene=$2
    fingerprint_scenario=$3
    fingerprint_transport=$4
    fingerprint_plugin=${5-}

    fingerprint_manifest=$(mktemp "${TMPDIR:-/tmp}/masonry-fingerprint.XXXXXX")
    (
        cd "$fingerprint_root"
        find Assets Packages ProjectSettings scripts crates -type f \
            ! -path '*/Library/*' ! -path '*/Temp/*' ! -path '*/obj/*' \
            ! -path '*/target/*' -print 2>/dev/null
        for fingerprint_file in Cargo.toml Cargo.lock; do
            [ ! -f "$fingerprint_file" ] || printf '%s\n' "$fingerprint_file"
        done
    ) | LC_ALL=C sort >"$fingerprint_manifest"
    {
        printf '%s\n%s\n%s\n%s\n' "$fingerprint_scene" "$fingerprint_scenario" \
            "$fingerprint_transport" "$fingerprint_plugin"
        while IFS= read -r fingerprint_file; do
            printf '%s\0' "$fingerprint_file"
            capture_sha256 "$fingerprint_root/$fingerprint_file"
        done <"$fingerprint_manifest"
    } | capture_sha256 | awk '{ print $1 }'
    rm -f "$fingerprint_manifest"
}

capture_verify_png_dimensions() {
    image_path=$1
    expected_width=$2
    expected_height=$3
    actual_width=$(sips -g pixelWidth "$image_path" | awk '/pixelWidth/ { print $2 }')
    actual_height=$(sips -g pixelHeight "$image_path" | awk '/pixelHeight/ { print $2 }')
    [ "$actual_width" -eq "$expected_width" ] \
        && [ "$actual_height" -eq "$expected_height" ]
}

capture_tracked_state() {
    git -C "$1" status --porcelain=v1 --untracked-files=all
}
