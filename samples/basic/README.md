# Masonry basic sample

This standalone Unity project demonstrates a game authored entirely in Rust using
Masonry's public Rust crates, Unity package, MessagePack protocol, and native ABI.
The project contains no game-specific C#: its Unity scene uses Masonry's reusable
bootstrap component, while Rust creates the game objects, diagnostics, and behavior.

From the repository root:

```sh
cargo masonry sample build basic
cargo masonry sample run basic
cargo masonry sample run basic --web
cargo masonry sample build basic --release
cargo masonry sample run basic --release
```

`sample run` remains attached until the player closes and streams Unity logs to
the terminal, so startup failures and Masonry diagnostics are visible where the
command was invoked.

`sample.toml` supplies this project's player identity and bootstrap scene to the
shared workflow. A future sibling sample can use the same commands by providing
its own manifest and `rules/Cargo.toml`.

The Rust engine creates three white cubes. Hover a cube to turn it yellow and
move away to restore white. Drag cube A to see its center snap to the pointer;
drag cube B to retain the exact pickup offset. Cube C remains clickable and
tweens between two positions over 500 ms. A separate blue change is delivered
by poll. Rust receives both drag lifecycle actions and commits the final world
position. It also creates the labels and status panel showing the connection,
transport, most recently observed action and command, and whether the latest
response was immediate or polled.
