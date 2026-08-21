# Masonry Tic-Tac-Toe sample

This standalone Unity project is an interactive Tic-Tac-Toe game authored entirely in Rust.
Masonry renders the supplied board and marker textures, forwards board clicks to the native
rules engine, and polls for the computer's move. The project contains no game-specific C#.

The player is X and moves first. After each legal player move, input pauses for 500 ms before
the computer selects a random empty square as O. When a round ends, click the board once to
clear it and begin another round.

From the repository root:

```sh
cargo masonry sample build tictactoe
cargo masonry sample run tictactoe
cargo masonry sample run tictactoe --web
cargo masonry sample build tictactoe --release
cargo masonry sample run tictactoe --release
```

The Web command cross-compiles the Rust rules engine with Unity's bundled Emscripten
toolchain, links it into the Unity WebAssembly player, serves the static build locally,
and opens it in the default browser. It remains attached until you press Ctrl-C. No
game server is involved; Python only serves the generated HTML, data, and Wasm files.
