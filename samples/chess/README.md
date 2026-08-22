# Masonry Chess sample

This standalone Unity project renders a decorated chessboard and all 32 pieces in their
standard starting positions from Rust. Masonry loads the authored board scene and the KayKit
piece models through Unity Addressables. The sample contains no game-specific C#.

The CC0 NotJam soundtrack plays in this order: “Critical”, “Switch with Me”,
“Breakbeat Chips”, and “Drag and Dread”. Each loop plays for two minutes before a
five-second crossfade. Use the up and down arrow keys to adjust the background-music
volume from the Rust rules engine.

The checked-in music sources use the Opus codec. Importing them requires `ffmpeg` on
`PATH`, or an explicit `MASONRY_FFMPEG` path, so Masonry's editor importer can create
Unity AudioClips for desktop and web builds.

From the repository root:

```sh
cargo masonry sample build chess
cargo masonry sample run chess
cargo masonry sample run chess --web
cargo masonry sample build chess --release
cargo masonry sample run chess --release
```
