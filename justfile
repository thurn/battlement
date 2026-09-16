set positional-arguments := true

# List the available project commands. Pass `--unsorted` or other `just --list` flags through.
default *args:
    @just --list "$@"

# Run any Reactant tooling command from this checkout, for example `just rt plugin inspect …`.
rt *args:
    cargo run --quiet -p rt -- "$@"

# Build and run the direct chess sample; append flags such as `--web`.
chess *args:
    cargo run --quiet -p rt -- run --skip-assets --project samples/chess --application "Battlement Chess.app" --manifest-path rules/Cargo.toml --scene Assets/Scenes/Main.unity "$@"

# Build and run the Reactant chess UI sample; append flags such as `--web`.
chess-ui *args:
    cargo run --quiet -p rt -- run --project samples/chess-ui "$@"

# Build and run the Reactant UI laboratory; append flags such as `--web`.
reactant *args:
    cargo run --quiet -p rt -- run --project samples/reactant "$@"

# Build and run the direct basic fixture; append flags such as `--web`.
basic *args:
    cargo run --quiet -p rt -- run --skip-assets --project samples/basic --application "Battlement Basic.app" --manifest-path rules/Cargo.toml --scene Assets/Scenes/BasicSample.unity "$@"

# Build and run the direct tic-tac-toe fixture; append flags such as `--web`.
tictactoe *args:
    cargo run --quiet -p rt -- run --skip-assets --project samples/tictactoe --application "Battlement Tic Tac Toe.app" --manifest-path rules/Cargo.toml --scene Assets/Scenes/TicTacToe.unity "$@"

# Build and run the direct UI fixture; append flags such as `--web`.
ui *args:
    cargo run --quiet -p rt -- run --skip-assets --project samples/ui --application "Battlement UI Lab.app" --manifest-path rules/Cargo.toml --scene Assets/Scenes/UiLab.unity "$@"

# Open the chess Unity project for authoring; append flags such as `--release`.
author *args:
    cargo run --quiet -p rt -- author --skip-assets --project samples/chess --application "Battlement Chess.app" --manifest-path rules/Cargo.toml --scene Assets/Scenes/Main.unity "$@"

# Open the tic-tac-toe Ditto gallery; append gallery flags or use `rt` for another Ditto command.
ditto *args:
    cargo run --quiet -p rt -- ditto --config samples/tictactoe/ditto.toml gallery "$@"

# Generate typed Addressables constants for the chess sample.
generate *args:
    cargo run --quiet -p rt -- addressables generate --project samples/chess "$@"

# Verify the chess sample's typed Addressables constants without writing.
generate-check *args:
    cargo run --quiet -p rt -- addressables check --project samples/chess "$@"

# Preview the Reactant sample's generated assets; append preview flags.
reactant-assets *args:
    cargo run --quiet -p rt -- assets preview --project samples/reactant "$@"

# Run local validation; the focused suite is the default and `just ci --full` runs the complete gate.
ci *args:
    python3 scripts/ci.py "$@"

# Run the complete Ditto CI gate; use `script scripts/ditto_ci.py …` for another subcommand.
ditto-ci *args:
    python3 scripts/ditto_ci.py gate "$@"

# Report recent CI, Codex, and Tollgate performance; defaults to the ten latest completed sessions.
perf-report *args:
    python3 scripts/perf_report.py "$@"

# Prepare a cached chess UI web build for local review; append `--development` for a debug build.
web-demo *args:
    python3 scripts/prepare-web-demo.py --project samples/chess-ui "$@"

# Deploy the complete public sample set from explicit repository projects.
deploy *args:
    python3 scripts/deploy.py --project samples/basic --project samples/chess --project samples/chess-ui --project samples/reactant --project samples/tictactoe --project samples/ui "$@"

# Run any repository Python script with arbitrary arguments.
script path *args:
    python3 "$@"
