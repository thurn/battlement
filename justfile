set positional-arguments := true

# List the available project commands. Pass `--unsorted` or other `just --list` flags through.
default *args:
    @just --list "$@"

# Run any Battlement CLI command from this checkout, for example `just battlement plugin inspect …`.
battlement *args:
    cargo run --quiet -p battlement-cli -- "$@"

# Build and run the chess sample; append flags such as `--web` or use `battlement` for another sample.
sample *args:
    cargo run --quiet -p battlement-cli -- sample run chess "$@"

# Open the chess Unity project for authoring; append flags such as `--release`.
author *args:
    cargo run --quiet -p battlement-cli -- author --project samples/chess "$@"

# Open the tic-tac-toe Ditto gallery; append gallery flags or use `battlement` for another Ditto command.
ditto *args:
    cargo run --quiet -p battlement-cli -- ditto --config samples/tictactoe/ditto.toml gallery "$@"

# Generate typed Addressables constants for the chess sample; append `--check` to verify without writing.
generate *args:
    cargo run --quiet -p battlement-cli -- generate samples/chess "$@"

# Preview the chess sample's Reactant assets; append preview flags or use `battlement` for another action.
reactant-assets *args:
    cargo run --quiet -p battlement-cli -- reactant assets preview --project samples/chess "$@"

# Run local validation; the focused suite is the default and `just ci --full` runs the complete gate.
ci *args:
    python3 scripts/ci.py "$@"

# Run the complete Ditto CI gate; use `script scripts/ditto_ci.py …` for another subcommand.
ditto-ci *args:
    python3 scripts/ditto_ci.py gate "$@"

# Report recent CI, Codex, and Tollgate performance; defaults to the ten latest completed sessions.
perf-report *args:
    python3 scripts/perf_report.py "$@"

# Prepare a cached chess UI web build for local review; append flags such as `--release`.
web-demo *args:
    python3 scripts/prepare-web-demo.py chess-ui "$@"

# Run any repository Python script with arbitrary arguments.
script path *args:
    python3 "$@"
