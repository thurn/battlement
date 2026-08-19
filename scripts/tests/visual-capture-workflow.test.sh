#!/bin/sh

set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
. "$repository_root/scripts/visual-capture-lib.sh"

fail() {
    printf 'visual capture workflow test failed: %s\n' "$1" >&2
    exit 1
}

assert_equals() {
    [ "$1" = "$2" ] || fail "expected '$1' to equal '$2'"
}

test_default_hold_timing() (
    fake_now=100
    slept=
    capture_now() { printf '%s\n' "$fake_now"; }
    capture_sleep() {
        slept=$1
        fake_now=$(awk -v now="$fake_now" -v duration="$1" \
            'BEGIN { print now + duration }')
    }

    capture_wait_for_initial_hold 100 2
    assert_equals "$slept" "2.000000"
    assert_equals "$fake_now" "102"
)

test_zero_hold_override() (
    capture_now() { printf '100\n'; }
    capture_sleep() { fail "zero-second hold slept for $1 seconds"; }
    capture_wait_for_initial_hold 100 0
)

test_fingerprint_invalidation() (
    fixture=$(mktemp -d "${TMPDIR:-/tmp}/masonry-capture-test.XXXXXX")
    trap 'rm -rf "$fixture"' EXIT HUP INT TERM
    mkdir -p "$fixture/Assets" "$fixture/Packages" "$fixture/ProjectSettings" \
        "$fixture/scripts" "$fixture/crates"
    printf 'initial\n' >"$fixture/Assets/Scenario.cs"
    initial=$(capture_project_fingerprint "$fixture" Assets/Scenario.unity demo none '')
    printf 'changed\n' >"$fixture/Assets/Scenario.cs"
    changed=$(capture_project_fingerprint "$fixture" Assets/Scenario.unity demo none '')
    [ "$initial" != "$changed" ] || fail "relevant input change did not invalidate build"
)

capture_is_nonnegative_number 0 || fail "zero should be accepted"
capture_is_nonnegative_number 2.5 || fail "decimal should be accepted"
if capture_is_nonnegative_number -1; then fail "negative hold should be rejected"; fi
test_default_hold_timing
test_zero_hold_override
test_fingerprint_invalidation
printf 'Visual capture workflow tests passed.\n'
