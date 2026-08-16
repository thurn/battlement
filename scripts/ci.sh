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

run_step "Restore local .NET tools" dotnet tool restore
run_step "Check C# formatting" dotnet csharpier check .
