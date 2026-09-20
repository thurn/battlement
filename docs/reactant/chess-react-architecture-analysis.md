# A React-shaped architecture for the chess sample

The chess sample can be made structurally close to a single-player React game
without hiding Reactant's native host or rules worker. Application authors
should see the same basic model:

1. immutable game state is authoritative;
2. components declaratively render that state;
3. input produces typed intents;
4. one rules boundary validates human and computer actions;
5. effects react to committed state; and
6. long-running work executes away from the presentation thread.

The current sample already has these ingredients, but their application-facing
shape is spread across `ChessGame`, `AppControl`, `BoardView`, and
`ReactantChessApp::maintain`. The target is not merely React-like data flow. Its
state, component, action, and effect boundaries should also be recognizable to
a React author.

## Direct correspondence

| React application | Target Reactant application | Current implementation |
| --- | --- | --- |
| Game reducer | `ChessState`, `ChessAction`, and `Game::execute` | Present, but rules state also carries presentation and persistence coordination. |
| UI reducer or context | `ChessUiState`, `UiAction`, and one shared dispatcher | `LocalState` and imperative `AppControl` methods. |
| `<ChessGame>` | `ChessApp` plus `GameRoot::new(ChessScreen)` | Construction in `reactant_view::app` and coordination in `ReactantChessApp`. |
| `<ChessBoard>` | `ChessBoard` | Most responsibilities are combined in `BoardView`. |
| `<Square>` | A keyed, usually hostless `Square` component | Legal-square planes are constructed directly by `BoardView`. |
| `<Piece>` | An entity-identified `Piece` logically nested under its square | `PieceView` is in one flat keyed list. |
| Promotion modal state | A typed rules prompt rendered by `PromotionDialog` | Every player promotion is currently forced to queen. |
| `useEffect` for computer turns | `TurnCoordinator` observing a ready final snapshot | `ReactantChessApp::maintain` dispatches `AiMove`. |
| Web Worker | Reactant `RulesWorker` and game session | Already present. |

## Target application shape

The component tree should look like application structure rather than host
plumbing:

```text
ChessApp
├── GameEffects
├── PersistentControls
└── TitleScreen | GameRoot::new(ChessScreen)
                  ├── GameStatus
                  ├── ChessBoard
                  │   ├── Square(a1).key(a1)
                  │   │   └── Piece(white rook).id(rook_entity)
                  │   ├── Square(b1).key(b1)
                  │   │   └── Piece(white knight).id(knight_entity)
                  │   └── ...
                  ├── PromotionDialog        when a promotion prompt is active
                  └── GameControls
```

`GameEffects` owns audio, vibration, diagnostics, and other reactions that do
not render visible structure. `PersistentControls` contains state that survives
a `GameRoot` replacement. `ChessScreen` composes the current game. Its children
receive narrow props and callbacks rather than the full `AppControl` and
`GameHandle`.

Rust components return `impl Render` rather than JSX and must own `'static`
props. Those are language-level differences. They do not justify combining the
screen, board, controls, status, diagnostics, and animation subscription in one
component.

A corresponding source layout could stay flat and explicit:

```text
reactant_app.rs       native Engine adapter and session replacement
chess_game.rs         ChessState, ChessAction, and Game implementation
chess_prompt.rs       ChessPrompt and PromotionPrompt
chess_ui_state.rs     ChessUiState, UiAction, and input normalization
chess_app.rs          title-versus-game composition
chess_screen.rs       current-game composition and focused selectors
chess_board.rs        board projection
chess_square.rs       square surface, semantics, and activation
chess_piece.rs        entity host, input, and motion reference
promotion_dialog.rs   typed prompt presentation
game_controls.rs      resignation and new-game controls
game_effects.rs       audio, vibration, and diagnostics
position.rs           cozy-chess adapter and entity projection
ai.rs                 existing computer search
```

The exact filenames matter less than keeping the component names and ownership
visible. `reactant_app.rs` should stop being the place ordinary game behavior
accumulates.

## State model

The React sketch used one reducer state because rules, interaction, and worker
messages commonly coexist in one JavaScript realm. Reactant has two
application-visible state lifetimes and one framework lifetime:

```rust
struct ChessState {
  position: ChessPosition,
  outcome: Option<GameOutcome>,
}

struct ChessUiState {
  screen: AppScreen,
  selected: Option<Square>,
  cursor: Square,
  cursor_visible: bool,
  overlay: Option<Overlay>,
  volume: f64,
}
```

Reactant's game session separately owns worker status, output publications,
accepted revision, cancellation, and replacement identity. Those values should
normally be observed through hooks rather than copied into chess state.

This split is deliberate:

- `ChessState` is deterministic, worker-owned, and accepted only after its
  output is successfully submitted.
- `ChessUiState` contains local, unfinished interaction and settings.
- the game session contains execution infrastructure rather than chess domain
  state.

The current `ChessState` also stores `started`, `spawning`, `visual_state`,
`starting_board`, and persistence directives. These do not belong in the
long-term domain model. Starting and replacement belong to application
lifecycle; spawning and visual classification belong to presentation;
persistence should observe accepted states. Moving them out makes the reducer
look substantially more like the React version.

`AppScreen::Title` renders `TitleScreen` without pretending that “not started”
is a chess position. Choosing Play creates the game session and switches to
`AppScreen::Game`; a confirmed new game replaces that session.

Move history should join `ChessState` only if the product supports notation,
undo, replay, or repetition detection. Its absence is a deliberate product
boundary, not a constraint imposed by Reactant.

### Orthogonal state instead of one game phase

Reactant should not combine rules state, execution status, prompts, and
presentation into a `GamePhase` enum. These facts are independent:

```text
rules outcome       ongoing or terminal
side to move        white or black
execution status    ready, busy, failed, or stopped
active prompt       none or Promotion(...)
presentation        idle, animating, or paused
```

During promotion the position is still the pre-move position, the player's
action is busy, and `use_game_prompt` returns `Promotion`. The prompt is the
authoritative pending choice; the game has not entered a separate
`PromotionChoice` phase.

Components should derive focused values for the decision they are making:

```rust
let board_interactive = status == GameStatus::Ready
  && state.outcome().is_none()
  && state.board().side_to_move() == Color::White;

let active_promotion = prompt.and_then(|presented| match &presented.prompt {
  ChessPrompt::Promotion(data) => Some((data.clone(), presented.handle.clone())),
});

let computer_turn_ready = status == GameStatus::Ready
  && state.outcome().is_none()
  && state.board().side_to_move() == Color::Black;
```

`ChessBoard` consumes `board_interactive`, `PromotionDialog` renders only from
`active_promotion`, and `TurnCoordinator` reacts only to
`computer_turn_ready`. A pause overlay and an active prompt can coexist without
inventing precedence rules for a flattened phase enum. A small derived
`InteractionMode` may be useful for shared styling or accessibility, but it is
optional UI vocabulary and does not belong in `ChessState`.

## Domain actions and UI actions

Game actions should describe complete rules operations:

```rust
enum ChessAction {
  MoveTo { from: Square, to: Square },
  ComputerMove,
  Resign { player: Color },
}
```

`MoveTo` deliberately omits a promotion piece. It expresses the player's
visible-board intent; if the destination requires promotion, the active action
obtains the missing information through a typed prompt. `ComputerMove` runs the
existing search and applies its complete `cozy_chess::Move`. `Resign` records a
terminal outcome and is not represented as a board move.

Starting, refreshing, and restarting create or replace a game session, so they
should be application commands rather than `ChessAction` variants.

Presentation input should converge on one vocabulary:

```rust
enum UiAction {
  Select(Square),
  Activate(Square),
  BeginDrag(PieceEntityId),
  EndDrag(PieceEntityId, Square),
  CancelSelection,
  TogglePause,
  RequestNewGame,
  ConfirmNewGame,
  SetVolume(f64),
}
```

Pointer, keyboard, controller, and drag handlers dispatch these actions rather
than implementing parallel selection policies. The UI reducer may update
selection, open an overlay, or dispatch one `ChessAction`. The rules boundary
checks legality again, making it authoritative for every input source.

`AppControl` remains useful because native core input arrives outside component
callbacks and some settings survive `GameRoot` replacement. It should expose a
reducer-shaped dispatch API rather than public methods that each combine state
mutation, validation, effects, and game dispatch. State wholly owned by a
mounted subtree can use Reactant's ordinary `use_reducer`.

## Squares, pieces, and entity identity

A chessboard should render sixty-four `Square` components. Each square derives
its selected, legal-target, focus, and activation presentation from props:

```rust
struct Square {
  square: cozy_chess::Square,
  selected: bool,
  legal_target: bool,
  piece: Option<ChessPiece>,
  on_activate: Callback<cozy_chess::Square>,
}
```

The board maps squares just as a React component would:

```rust
cozy_chess::Square::ALL
  .map(|square| {
    SquareView::new(square, state.piece(square))
      .selected(ui.selected == Some(square))
      .legal_target(legal.contains(&square))
      .on_activate(on_activate.clone())
      .key(square)
  })
```

Square identity and piece identity are different. A square has stable sibling
identity based on its coordinate. A piece is a game entity that moves between
squares. The existing `ChessPiece::id` should be treated and named as a
`PieceEntityId`; its UUID is supplied to Reactant's application-wide
presentation identity:

```rust
self.piece.map(|piece| {
  PieceView::new(piece, self.square)
    .id(*piece.entity_id.as_uuid())
})
```

Reactant's public implementation currently calls this a presentation ID rather
than an entity ID. For game authoring, the intended role is entity identity:
the ID is stable for the life of one logical piece, independent of its current
square or native attachment.

Reactant's `.key(...)` matches within one sibling list. Its `.id(UUID)` matches
a component or structural contribution across logical parents. The latter is
the entity behavior chess needs: when a rook moves from `a1` to `a4`, the same
identified `PieceView` moves from the `Square(a1)` subtree to the `Square(a4)`
subtree without losing compatible component state or its retained native host.
The relevant contract is
[`IdentityRenderExt`](../../crates/reactant-core/src/identity.rs), and the
[`reconciler`](../../crates/reactant-core/src/reconcile.rs) already emits native
reparent operations for retained hosts.

The identity should be applied once at the `PieceView` component boundary. Its
inner `BoxHitRegion` should not declare the same UUID again. The identified
component retains its descendant host and gives presentation inspection and
motion one stable entity boundary.

`SquareView` should normally be hostless. It can return its square surface and
optional piece without creating a native transform container. The piece is then
a logical child of the square but remains physically attached to the board's
world root, preserving simple absolute board coordinates. Logical and physical
parenting need not be identical. If squares instead become native transform
parents, Reactant can still reparent the piece host, but movement must account
for the change in local coordinate space.

This structure exercises a capability the current flat `PieceView` list does
not: application-wide identity across parent changes. It also restores the
component boundary a React author expects.

## Promotion as a first-class prompt

Promotion should use Reactant's typed prompt system rather than
`ChessUiState::pending_promotion`. It is missing information required to finish
one rules action, not merely an open modal.

The game defines a prompt enum and typed prompt data:

```rust
enum ChessPrompt<'a> {
  Promotion(Cow<'a, PromotionPrompt>),
}

#[derive(Clone)]
struct PromotionPrompt {
  from: Square,
  to: Square,
  choices: [Piece; 4],
}
```

`PromotionPrompt` implements `PromptData<ChessGame>` with `Piece` as its
response type. Its stable option order is queen, rook, bishop, then knight, and
`is_valid_response` accepts only those four values. The interactive
`ChoicePolicy` assigns this prompt to `ChoiceOwner::Human`; simulation policies
can select an index deterministically.

The rules flow is:

1. `MoveTo { from, to }` is admitted when at least one legal move matches the
   visible destination.
2. `Game::execute` recomputes those candidates on its private state.
3. A normal destination produces one move. A promotion destination produces
   four moves distinguished by promotion piece.
4. Before mutating state, execution calls
   `context.execution.choose(state, PromotionPrompt { ... })`.
5. Reactant publishes the unchanged position and typed prompt, then blocks that
   rules action on the worker.
6. `PromotionDialog` reads the prompt with `use_game_prompt::<ChessGame>()` and
   renders four buttons.
7. A button submits its `Piece` through the exact `ResponseHandle`.
8. The suspended action resumes, selects the matching legal move, applies it,
   and publishes the normal promotion animation and final state.

No provisional pawn move is applied. The accepted state remains the pre-move
position until the completed action's final output is submitted. Restarting or
replacing the session abandons the request and unwinds the waiting action;
responses through stale handles cannot affect the replacement game.

The dialog should be keyed by its response handle, following the established
pattern in
[`prompt_proof.rs`](../../samples/reactant/rules/src/prompt_proof.rs). This
prevents a queued callback belonging to an earlier promotion from acquiring a
new prompt's handler. Ordinary board input remains disabled while the game is
busy, while prompt controls remain interactive through their response handle.
The button callback submits directly to that request:

```rust
let ChessPrompt::Promotion(data) = &presented.prompt;
presented.handle.submit(data.as_ref(), Piece::Knight);
```

The AI does not use this prompt. Its search already returns a complete move,
including promotion. The prompt exists only because a player's destination
click is intentionally incomplete.

## Rendering moves and special rules

The board remains a projection of immutable state. Components never directly
move native objects and then repair the model afterward.

`cozy-chess` remains authoritative for legality, check, mate, castling rights,
and en passant. The application still needs a small boundary adapter because
the crate represents castling as the king moving onto its rook. UI code exposes
the conventional king destination; the position adapter translates it and
describes the two entity movements for presentation.

`ChessPosition` currently stores the rules board plus stable piece identities,
and `Movement` describes entity motion, captures, castling, and promotion.
That extra presentation projection is justified by native animation and prefab
continuity, but it must not become a second legality implementation.

Resignation requires an explicit outcome because it is not representable in a
`cozy_chess::Board`:

```rust
enum GameOutcome {
  Checkmate { winner: Color },
  Draw,
  Resignation { winner: Color },
}
```

Once an outcome exists, move and computer actions are illegal. `GameStatus`
renders the outcome; `GameControls` dispatches resignation.

## Computer turns, effects, and lifecycle

A `TurnCoordinator` component should observe rendered game state and session
status. When the session is ready, that rendered final snapshot has been
accepted; if the game is ongoing and black is to move, an effect dispatches
`ChessAction::ComputerMove`. This is the Reactant equivalent of a React
`useEffect` watching `sideToMove`.

The search itself remains inside the rules worker. A browser implementation
might create a Web Worker and tag replies with a position generation. Reactant
already serializes actions, owns cancellation, associates publications with a
session, and stops the old run during replacement. Application-level request
IDs would duplicate those guarantees.

`GameEffects` should retain audio, vibration, and diagnostics effects. Semantic
`ChessAnimation` checkpoints remain justified because required native
presentation can block later gameplay output. Reactant's output protocol makes
rendering a checkpoint distinct from accepting its final state.

That distinction also explains why persistence cannot yet be an ordinary
component effect. A rendered checkpoint may not be accepted. The current
app-level persistence step should remain until Reactant exposes an
accepted-state subscription or effect, after which chess-specific persistence
coordination can leave `ReactantChessApp::maintain`.

New game and restart replace the rules session rather than assigning a new
reducer value. Replacement cancels worker work, abandons prompts and queued
publications, remounts the game-owned subtree, and prevents stale AI or motion
output from entering the new game. This lifecycle difference should remain,
although Reactant can provide a more declarative command for requesting it.

## Deliberate differences that remain

| Difference from a typical React web game | Justification |
| --- | --- |
| Rules state, UI state, and session state are separate | They have different threads, acceptance points, and lifetimes. |
| Logical component parents may differ from native transform parents | Hostless components let entity structure remain expressive without complicating world coordinates. |
| Pieces carry stable entity IDs in addition to square coordinates | Native animation, references, and retained hosts must follow a piece across logical parents. |
| Rendering produces native UI and world hosts instead of DOM elements | Reactant targets Battlement's Unity client rather than a browser. |
| Promotion suspends one rules action through a typed prompt | The choice completes an atomic move and must be validated by the worker. |
| AI runs as a rules-worker action rather than an application-owned Web Worker | Reactant already provides serialization, cancellation, and stale-result isolation. |
| Rendered and accepted state are distinct | Native delivery and required presentation can fail or remain pending. |
| Game replacement is a session operation | It must stop active worker work and invalidate prompts and publications. |
| Component values own `'static` props | Rust ownership and retained rendering require owned values. |
| Castling crosses a small library adapter | `cozy-chess` uses a non-visual castling representation. |
| Move history may be absent | This is justified only while notation, undo, replay, and repetition are outside product scope. |

The remaining differences have concrete runtime, host, or product reasons.
Forced queen promotion, a flat piece list, the absence of `Square`, a
monolithic `BoardView`, presentation fields in rules state, and routine turn
effects hidden in `maintain` do not. Those are the main opportunities for the
sample to demonstrate that Reactant is immediately familiar to React users.
