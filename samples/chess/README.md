# Masonry Chess sample

This standalone Unity project renders a decorated chessboard and all 32 pieces in their
standard starting positions from Rust. Masonry loads the authored board scene and the KayKit
piece models through Unity Addressables. The sample contains no game-specific C#.

From the repository root:

```sh
cargo masonry sample build chess
cargo masonry sample run chess
cargo masonry sample run chess --web
cargo masonry sample build chess --release
cargo masonry sample run chess --release
```
