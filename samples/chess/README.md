# Masonry Chess sample

This standalone Unity project is a complete player-versus-computer chess game implemented in
Rust. Masonry loads the authored board scene and KayKit piece models through Unity Addressables;
the sample contains no game-specific C#.

Play white by dragging a piece to its destination square. Illegal moves return to their starting
square. Pawns automatically promote to queens, and castling is performed by dragging the king to
`c1` or `g1`. The rules, including captures, check, checkmate, castling, and en passant, come from
`cozy-chess`. The current position is serialized after every move beneath Unity's persistent data
path and opens automatically on the next launch, including in Web builds. Use the refresh button
in the top-right corner to discard that position and start a new game.

Clicking Play brings both armies onto the board in independent random orders over roughly five
seconds. Each arriving piece plays a one-second NOVA Shader effect sized to one board square;
piece input becomes available after the stagger completes.

Black uses a roughly two-second iterative-deepening negamax search with alpha-beta pruning,
quiescence search, move ordering, and a positional evaluation. Rayon searches the root moves in
parallel on its worker pool. Search runs asynchronously and Masonry continues polling while the
computer thinks, so rendering remains responsive even though player input is temporarily disabled.

Native and Web builds always use Rayon. The Web build compiles Rust and its standard library with
WebAssembly atomics through Unity's bundled Emscripten toolchain and writes a
`Build/<profile>/WebThreads` player. Its local server uses HTTP localhost and sends the COOP, COEP,
and CORP headers required by `SharedArrayBuffer`. Deploy the directory from an HTTPS origin with
equivalent headers, and keep every embedded resource same-origin or explicitly CORS/CORP
compatible.

The sample requires WebAssembly thread support. Browsers or hosts that cannot provide
`SharedArrayBuffer` are unsupported.

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
cargo masonry sample run chess --web # threaded Rayon build
cargo masonry sample build chess --release
cargo masonry sample run chess --release
```
