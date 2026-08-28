# Battlement Chess sample

This standalone Unity project is a complete player-versus-computer chess game implemented in
Rust. Battlement loads the authored board scene and KayKit piece models through Unity Addressables;
the sample contains no game-specific C#.

Play white with the mouse by clicking a piece and then its destination square, or by dragging a
piece there. For keyboard-only play, press Enter or Space on the opening screen, move the glowing
board cursor with the arrow keys, and press Enter, Numpad Enter, or Space to select a piece or move
it to the cursor. Escape cancels the current selection. Clicking a piece
moves the same cursor to its square, so mouse and keyboard input can be freely mixed. The cursor is
hidden during mouse-only play and appears on the first board keyboard action; after that it remains
available while Black thinks.

On a controller, A/Cross starts the game, selects a piece, and confirms its destination. B/Circle
cancels and returns the cursor to the selected piece. The D-pad and dominant axis of the left stick
move exactly one square, with held-stick repeat after a short delay. LB/L1 and RB/R1 wrap through
white pieces with legal moves, or through legal destinations after selection. Start (Menu/Options)
opens the pause controls: A/Cross requests New Game and requires a second confirmation, B/Circle
resumes, and the shoulder buttons adjust music volume. Right stick and triggers are intentionally
unused.

Illegal drags return to their starting square. Pawns automatically promote to queens, and castling
is performed by moving the king to `c1` or `g1`. The rules, including captures, check, checkmate,
castling, and en passant, come from `cozy-chess`. The current position is serialized after every
move beneath Unity's persistent data path and opens automatically on the next launch, including in
Web builds. Open the pause controls and confirm the refresh button twice to discard that position
and start a new game. To immediately clear the saved position and replay the opening animation,
press Command-Shift-R on macOS or Control-Shift-R on Windows and Linux.

Clicking Play brings both armies onto the board in independent random orders over roughly two
seconds. Each arriving piece plays a one-second NOVA Shader effect sized to one board square;
piece input becomes available after the stagger completes.

Black uses a roughly two-second iterative-deepening negamax search with alpha-beta pruning,
quiescence search, move ordering, and a positional evaluation. Rayon searches the root moves in
parallel on native platforms and desktop Web browsers. Mobile Web builds keep the shared
`par_iter` search but run it sequentially in a current-thread Rayon pool, temporarily occupying
Unity's thread while the computer thinks. Mobile browsers can hang while synchronously
bootstrapping a nested WebAssembly worker during scene startup.

Native and Web builds always use Rayon. The Web build compiles Rust and its standard library with
WebAssembly atomics through Unity's bundled Emscripten toolchain and writes a
`Build/<profile>/WebThreads` player. Its local server uses HTTP localhost and sends the COOP, COEP,
and CORP headers required by `SharedArrayBuffer`. Deploy the directory from an HTTPS origin with
equivalent headers, and keep every embedded resource same-origin or explicitly CORS/CORP
compatible.

The sample requires WebAssembly thread support. Its loading page reports an error without starting
Unity when the browser or host cannot provide `SharedArrayBuffer` in a cross-origin-isolated page.

The CC0 NotJam soundtrack plays in this order: “Critical”, “Switch with Me”,
“Breakbeat Chips”, and “Drag and Dread”. Each loop plays for two minutes before a
five-second crossfade. Use the minus and equal/plus keys to adjust the background-music volume from
the Rust rules engine.

The checked-in music sources use the Opus codec. Importing them requires `ffmpeg` on
`PATH`, or an explicit `BATTLEMENT_FFMPEG` path, so Battlement's editor importer can create
Unity AudioClips for desktop and web builds.

From the repository root:

```sh
cargo battlement author --project samples/chess
cargo battlement sample build chess
cargo battlement sample run chess
cargo battlement sample run chess --web # threaded Rayon build
cargo battlement sample build chess --release
cargo battlement sample run chess --release
```

`author` opens the project in Unity and enters Play mode. Use **Battlement > Play Game** to replay
after editing the scene.
