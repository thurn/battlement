# A React-shaped architecture for the chess sample

The chess sample can be made structurally very close to a single-player React
game without pretending that Rust, a native retained host, and Reactant's rules
worker are a browser. The target should preserve the same mental model:

1. immutable chess state is the source of truth;
2. components render that state declaratively;
3. input produces typed intents rather than editing presentation objects;
4. the rules boundary validates every human and computer move;
5. effects react to committed state; and
6. expensive computer play runs away from the presentation thread.

The current sample already follows much of this model. Its main source of
unfamiliarity is not its chess logic, but coordination split across
`ChessGame`, `AppControl`, and `ReactantChessApp::maintain`. Reactant can bring
those responsibilities closer to the component tree while retaining the few
boundaries required by its host and publication model.

## Correspondence with the React design

| React concept | Current chess implementation | Desired Reactant expression |
| --- | --- | --- |
| Rules state and reducer | [`ChessState`](../../samples/chess/rules/src/reactant_game.rs) and `Game::execute` | Keep this pair as the authoritative game reducer. Give actions and state names that describe player intent and game phase. |
| Component tree | [`BoardView` and `PieceView`](../../samples/chess/rules/src/reactant_view.rs) | Keep declarative components, but split controls and overlays into small components rather than assembling the whole screen in `BoardView`. |
| UI state | [`LocalState` and `AppControl`](../../samples/chess/rules/src/reactant_input.rs) | Keep non-rules state outside `ChessState`, but update it through a reducer-shaped action API. |
| Event handlers | `activate_square`, drag handlers, key routing, and controller routing | Normalize all sources into the same small set of UI intents, then derive a game action when an intent completes a legal move. |
| Effects | [`EffectsView`](../../samples/chess/rules/src/reactant_effects.rs) plus `ReactantChessApp::maintain` | Keep audio and diagnostics as component effects. Move ordinary turn coordination into a component effect; reserve app lifecycle code for accepted-state and session replacement work. |
| Web Worker | Reactant [`RulesWorker`](../../crates/reactant/src/game_app.rs) | Keep it. The AI already runs off the presentation thread as part of `ChessAction::AiMove`. |
| DOM/CSS board | Reactant world nodes, prefabs, hit regions, and a small UI document | Keep the native world representation. It is the host equivalent of a DOM projection, not application state. |

This is close enough that a React developer should recognize the direction of
data flow even though the syntax and host elements differ.

## State ownership

A browser implementation can often use one reducer because its rules, UI, and
effects all run in one JavaScript realm. Reactant has three legitimate state
lifetimes:

- `ChessState` is worker-owned, deterministic, and accepted only after its
  output has been submitted successfully.
- UI interaction state such as selection, keyboard cursor, pause visibility,
  and volume belongs to the presentation process.
- session state such as an active worker, replacement generation, and durable
  persistence belongs to the application lifecycle.

Those lifetimes justify more than one storage location. They do not justify
three unrelated command vocabularies. The application-facing shape should be
approximately:

```rust
enum ChessAction {
  Start(StartMode),
  Move(PlayerMove),
  Resign(Color),
  RequestComputerMove,
}

enum UiAction {
  Select(Square),
  Activate(Square),
  ChoosePromotion(Piece),
  Cancel,
  TogglePause,
  RequestNewGame,
  ConfirmNewGame,
  SetVolume(f64),
}
```

`ChessAction` is analogous to actions sent to a game reducer. `UiAction` is
analogous to a component reducer. Pointer, keyboard, controller, and drag input
should all dispatch `UiAction`; they should not each implement a parallel
interaction policy.

The current `AppControl` is still justified as app-owned storage because native
core input arrives outside component event callbacks and its settings survive a
`GameRoot` replacement. Its methods currently mix reduction, validation,
effects, and game dispatch. A reducer-shaped `dispatch(UiAction, &GameHandle)`
would make that necessary external store feel like React context plus
`useReducer`, rather than like an imperative controller. Reactant's ordinary
`use_reducer` remains appropriate for state whose lifetime is wholly inside a
mounted component.

An explicit mutable `phase` is not necessary for states Reactant already knows.
`GameStatus::Busy`, the side to move, terminal outcome, and a pending promotion
can derive `HumanTurn`, `PromotionChoice`, `ComputerThinking`, and `GameOver`.
Derivation avoids two sources of truth. A small public derived enum would still
be useful for rendering and tests.

## Rendering

React renders DOM nodes; the sample renders native UI elements and scene
objects. [`BoardView::render`](../../samples/chess/rules/src/reactant_view.rs)
already behaves like a React component: it reads immutable game state and an
external UI store, maps pieces to keyed children, derives legal highlights,
and returns a tree. Stable piece IDs and object references are required because
native animations address retained objects across snapshots. React keys alone
do not provide that host addressability, so this difference should remain.

The board should continue to be rendered from state rather than mutated in
response to input. `ChessPosition` contains both the `cozy_chess::Board` and
stable presentation identities, while [`Movement`](../../samples/chess/rules/src/position.rs)
describes how a logical move should animate. This is more presentation metadata
than a DOM chessboard needs, but it is justified by capture effects, two-piece
castling, knight paths, and prefab continuity. It should not become a second
rules engine: `cozy-chess` remains authoritative for legality and board state.

`BoardView` currently owns the board, title control, reset control, status
label, legal highlights, cursor, diagnostics, and animation subscription. A
more familiar component tree would be:

```text
ChessScreen
├── GameEffects
├── StatusOverlay
├── Board
│   ├── Pieces
│   ├── LegalTargets
│   └── Cursor
├── PromotionDialog
└── GameControls
```

These are Rust `Component` values returning `impl Render`, rather than
functions returning JSX. Owned `'static` props and explicit `Child`/`Children`
values are required by Rust ownership and Reactant's retained tree. That syntax
difference is justified; large monolithic render functions are not.

## Actions and turn coordination

The current human path is sound in outline:

```text
native event → AppControl → ChessAction::Move → rules worker
             → accepted immutable snapshot → component render
```

Every move is checked before dispatch and checked again by `Game` admission.
The second check should remain. It makes the worker boundary authoritative and
protects keyboard, pointer, controller, tests, and future inputs equally.

Computer turns are currently discovered by
[`ReactantChessApp::maintain`](../../samples/chess/rules/src/reactant_app.rs),
which polls accepted state and dispatches `AiMove`. This works, but it hides an
ordinary state-driven effect in engine lifecycle code. A `TurnCoordinator`
component could subscribe to game status and the side to move, then dispatch
`RequestComputerMove` from an effect when the accepted game is ready. That is
the direct analogue of a React `useEffect` watching `sideToMove`.

The search itself should remain in the rules worker. Unlike a browser Web
Worker that returns an untrusted message tagged with a position ID, a Reactant
game session owns the worker run, accepts one action at a time, and stops the
old run when the session is replaced. [`GameRoot`](../../crates/reactant/src/game_hooks.rs)
also keys the rendered subtree by session identity. These mechanisms provide
the stale-result protection that the React version would implement with a
request generation. Duplicating a position ID in chess application code would
be redundant.

Animations are another justified difference. A React DOM game might update the
position immediately and animate a CSS transition. Reactant publishes semantic
`ChessAnimation` checkpoints, turns them into native sequences with
`use_animate`, and accepts the final state through bounded output submission.
This couples game progress to successful host delivery and prevents subsequent
work from overtaking required presentation. It is native transaction and
backpressure behavior, not incidental chess complexity.

## Special rules and terminal actions

Castling and en passant are already handled at the correct level.
`cozy-chess` validates the move, while `ChessPosition` translates its compact
representation into visible piece updates. In particular, `cozy-chess`
represents castling as a king move onto its own rook; the input adapter exposes
the conventional king destination and `ChessPosition` moves both native
pieces. That adapter is a library-boundary necessity and should remain
contained in the position module.

Promotion is not yet React-like or feature-complete. `player_move` currently
inserts `Piece::Queen` whenever a pawn reaches the eighth rank. The target
design should leave the board unchanged, store a pending move in UI state,
render `PromotionDialog`, and dispatch the completed move only after the player
chooses queen, rook, bishop, or knight. The promotion piece belongs in the
typed move sent to the worker, where legality is checked again. There is no
host or Rust constraint that justifies forced queen promotion.

Resignation is absent. It should be a game action, not a synthetic board move
or UI-only flag. Because `cozy_chess::Board` cannot represent resignation,
`ChessState` needs an explicit outcome alongside the board. The same outcome
type can normalize checkmate, draw, and resignation for rendering:

```rust
enum GameOutcome {
  Checkmate { winner: Color },
  Draw,
  Resignation { winner: Color },
}
```

Once an outcome exists, move and AI actions are illegal. A resign button merely
dispatches `ChessAction::Resign(Color::White)`. This mirrors the React design
and keeps terminal truth inside the rules state.

The sample does not retain move history, expose undo, or distinguish draw
reasons. Its persisted format stores only the current FEN. That is acceptable
only as a deliberate product boundary: the current sample has no move list,
undo command, or threefold-repetition feature. If any of those are added,
history must join worker-owned `ChessState`; it should not be reconstructed by
components or hidden in the persistence layer.

## Effects, persistence, and replacement

Audio, vibration, and diagnostics already use component effects and should stay
there. They are reactions to committed presentation state and have cleanup or
dependency semantics familiar from React.

Persistence is different. Reactant renders a worker checkpoint before that
checkpoint becomes the accepted rules state; successful native submission is
what advances acceptance. An ordinary component effect could therefore save a
snapshot that the session has not accepted. The current app-level persistence
step is justified until Reactant exposes an accepted-state effect or
subscription. Such an API would let application code express “save after this
revision is accepted” declaratively without weakening the transaction
boundary.

New game and restart currently replace the entire rules session. That is more
infrastructure than a React reducer reset, but it deliberately cancels an
active worker, abandons its pending publications, remounts the game-owned
subtree, and prevents stale AI or animation output from entering the new game.
Session replacement should remain. The application-specific request plumbing
can shrink if Reactant exposes replacement as an effect-safe command, but a
plain state assignment is not equivalent.

## Recommended boundary

Reactant can reasonably aim for the following level of familiarity:

- game authors define immutable state, typed actions, and one rules reducer;
- components select state and return declarative native trees;
- all input sources converge on a reducer-shaped UI intent API;
- ordinary orchestration is expressed with hooks and effects;
- asynchronous game work is dispatched through a stable handle and rendered
  from immutable snapshots; and
- host delivery, accepted-state persistence, cancellation, and native object
  identity remain framework-managed differences.

That would make the chess application read like the Rust equivalent of the
React sketch. The irreducible differences would each have a concrete reason:
Rust ownership, a native retained scene, transactional output delivery, or
worker/session isolation. Forced queen promotion, monolithic rendering, and
routine turn effects hidden in `maintain` do not meet that standard and should
not be defended as platform necessities.
