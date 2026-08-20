# Masonry basic sample

This standalone Unity project demonstrates a game-owned Rust rules engine using
Masonry's public Rust crates, Unity package, MessagePack protocol, and native ABI.

From the repository root:

```sh
cargo masonry sample build basic
cargo masonry sample run basic
cargo masonry sample build basic --release
cargo masonry sample run basic --release
```

`sample run` remains attached until the player closes and streams Unity logs to
the terminal, so startup failures and Masonry diagnostics are visible where the
command was invoked.

`sample.toml` supplies this project's player name, Unity build method, and
capture scenario to the shared workflow. A future sibling sample can use the
same commands by providing its own manifest and `rules/Cargo.toml`.

The Rust engine creates three gray cubes. Hover a cube to turn it yellow, move
away to restore gray, and click to tween it between two positions over 500 ms.
A separate blue change is delivered by poll. The unobtrusive status panel shows
the connection, transport, most recently observed action and command, and
whether the latest response was immediate or polled.
