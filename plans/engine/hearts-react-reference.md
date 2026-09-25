# Blind React reference for Hearts

A fresh agent produced this sketch without conversation history and without
inspecting local files. Preserve it as independent comparison material; it does
not override the chosen rules or product behavior in [the design](hearts.md).
The response below is retained with formatting changes only.

## Prompt

> Sketch a Hearts game implementation in React, including data flow, use of hooks,
> and component structure. Do not inspect any local repositories or files. Return
> an idiomatic architecture sketch, with enough illustrative TypeScript/React
> pseudocode to show state ownership, game actions, asynchronous opponents, and
> presentation effects.

## Response

Use a pure game engine behind a React reducer. Keep authoritative game state in
one place, derive the current player's view, and let components submit intentions
such as “pass these cards” or “play this card.”

```text
HeartsApp
├── SetupScreen
└── GameTable
    ├── Scoreboard
    ├── OpponentSeat × 3
    ├── TrickArea
    ├── HumanHand
    ├── PassControls
    ├── RoundResults
    └── GameAnnouncements
```

`GameTable` receives a player-specific view, keeping opponent cards out of
presentation props. Components own only transient interface state: selected
cards, hovered cards, open dialogs, and focus.

```ts
type CardId = string;
type Seat = 0 | 1 | 2 | 3;

type Phase =
  | { kind: "passing"; direction: "left" | "right" | "across" }
  | { kind: "playing"; turn: Seat }
  | { kind: "roundOver" }
  | { kind: "gameOver"; winner: Seat };

type GameState = {
  gameId: string;
  revision: number;
  phase: Phase;
  hands: Record<Seat, CardId[]>;
  submittedPasses: Partial<Record<Seat, CardId[]>>;
  trick: Array<{ seat: Seat; card: CardId }>;
  captured: Record<Seat, CardId[]>;
  heartsBroken: boolean;
  scores: Record<Seat, number>;
  round: number;
};

type Command =
  | { kind: "pass"; seat: Seat; cards: CardId[] }
  | { kind: "play"; seat: Seat; card: CardId }
  | { kind: "nextRound" };

type Action =
  | {
      type: "command";
      gameId: string;
      expectedRevision: number;
      command: Command;
    }
  | { type: "restart"; initial: GameState };

type GameEvent =
  | { kind: "cardPlayed"; seat: Seat; card: CardId }
  | { kind: "trickWon"; winner: Seat; cards: CardId[] }
  | { kind: "roundScored"; points: Record<Seat, number> };
```

The engine handles legality, turn order, trick collection, scoring, and phase
transitions atomically. It should implement following suit, the two-of-clubs
opening, restrictions on hearts and first-trick points, pass rotation, and
shooting the moon under an explicit rules configuration.

```ts
// No React, timers, animation, or uncontrolled randomness here.
function transition(
  state: GameState,
  command: Command,
): { state: GameState; events: GameEvent[] } {
  // Validate command against the authoritative state.
  // Apply it and resolve any immediately resulting rules.
}
```

Create shuffled deals outside the reducer or use a seeded generator whose state
is part of the engine. Reducer execution must be deterministic.

The reducer can retain a bounded presentation-event queue alongside the game:

```ts
type Model = {
  game: GameState;
  presentation: Array<{ id: string; event: GameEvent }>;
};

function reducer(model: Model, action: Action): Model {
  if (action.type === "restart") {
    return { game: action.initial, presentation: [] };
  }

  const game = model.game;
  if (
    action.gameId !== game.gameId ||
    action.expectedRevision !== game.revision
  ) {
    return model; // Ignore an obsolete async response or duplicate action.
  }

  const result = transition(game, action.command);
  return {
    game: result.state,
    presentation: appendEvents(model.presentation, result.events),
  };
}
```

For completeness, the actual reducer also accepts presentation acknowledgments
that remove consumed events without changing the game revision.

A controller hook connects React to the engine:

```tsx
function useHearts(initial: GameState, human: Seat) {
  const [model, dispatch] = useReducer(reducer, {
    game: initial,
    presentation: [],
  });

  const view = selectPlayerView(model.game, human);
  const decision = selectNextComputerDecision(model.game, human);

  useComputerTurn(decision, dispatch);

  function send(command: Command) {
    dispatch({
      type: "command",
      gameId: model.game.gameId,
      expectedRevision: model.game.revision,
      command,
    });
  }

  return { view, events: model.presentation, send, dispatch };
}
```

`selectNextComputerDecision` returns a stable, immutable decision snapshot or
`null`. Its key includes the game ID, revision, and computer seat. Passing can
process pending computer submissions sequentially; card exchange occurs only
once everyone has submitted.

Asynchronous opponents belong in an effect because they synchronize the game
with an external computation or timer:

```tsx
function useComputerTurn(
  decision: ComputerDecision | null,
  dispatch: React.Dispatch<Action>,
) {
  useEffect(() => {
    if (!decision) return;

    const controller = new AbortController();

    void (async () => {
      try {
        await delay(400, controller.signal);
        const command = await chooseComputerMove(
          decision.observation,
          controller.signal,
        );

        if (!controller.signal.aborted) {
          dispatch({
            type: "command",
            gameId: decision.gameId,
            expectedRevision: decision.revision,
            command,
          });
        }
      } catch (error) {
        if (!controller.signal.aborted) {
          reportComputerFailure(error);
          // Surface a retry action, or submit a legal fallback.
        }
      }
    })();

    return () => controller.abort();
  }, [decision, dispatch]);
}
```

Memoize the decision snapshot by its decision key so unrelated renders do not
restart computation. Abort handles cleanup and development Strict Mode; reducer
revision checks handle results that still arrive late. An opponent receives only
its permitted observation. Heavy search can run in a worker through the same
interface.

Keep hand selection local and scoped to the round or passing phase:

```tsx
function HumanHand({ view, onPlay }: HandProps) {
  return (
    <div aria-label="Your hand">
      {view.hand.map(card => (
        <CardButton
          key={card.id}
          card={card}
          disabled={!view.legalPlays.has(card.id)}
          onClick={() => onPlay(card.id)}
        />
      ))}
    </div>
  );
}
```

Presentation effects consume explicit game events. A completed trick event
carries its cards so animation still works after the engine clears the live
trick. A presentation hook sequences dealing, card movement, collection, and
announcements, then acknowledges event IDs. Cancel pending animation on restart
or unmount, and support reduced motion. Animation completion should not determine
who won a trick or alter scores.

Derive legal cards, point totals, labels, and whether controls are enabled
directly from state; avoid effects that copy these values into additional state.
Use `useMemo` for expensive calculations or stable async inputs, `useRef` for DOM
and animation handles, and context only if passing the view and action callbacks
becomes cumbersome.
