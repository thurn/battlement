# A playable four-player Hearts sample

Hearts demonstrates the engine in a complete game: one human and three AI
players, a 3D table, UI menus and scores, and explicit durable save/load. Read
this for rules, interaction, AI, and persistence requirements. See
[execution](execution.md), [world](world.md), and [validation](validation.md).
The focused engine laboratory is specified in [fixtures](fixtures.md).

## Experience and assets

Create samples/hearts as a standalone Unity/Rust sample. One human plays with
three AI opponents around a fixed angled 3D table. Use native UI for menus,
scores, prompts, and results; cards and their motion live in world space.

Use the CC0 KayKit Board Game Bits EXTRA card faces/backs and suitable table
props. The [publisher page](https://kaylousberg.itch.io/board-game-bits)
identifies playing-card textures/models as EXTRA content. The existing chess
sample has an EXTRA subset but no card deck was found during planning.

Task 35 needs the actual asset archive: locate an owned archive or obtain it
through the user's access, record its hash and included license, and import the
needed files through sample authoring tools. Do not purchase assets without
authorization or claim the archive was checked before obtaining it. Earlier
engine tasks do not depend on this archive.

Cards use Rust-created face/back surfaces with prepared textures and independent
hit regions. Choose the regular two-color 52-card deck and one shared back.
Retain source names/license and deterministic Addressables generation. Use
existing licensed project fonts/audio when suitable; optional sound may be
omitted until a licensed source is present. Required engine audio tests use
synthetic fixtures independent of sample music.

## Fixed rules

This sample implements the following explicit variant. Published references such
as [Bicycle Hearts](https://bicyclecards.com/how-to-play/hearts) provide
context; the rules below resolve variations and are authoritative here.

- Four seats clockwise, human South, AI West/North/East. Deal all 52 cards,
  thirteen each. Ace is high; there is no trump.
- Passing rotates left, right, across, hold. Each passing player selects three
  distinct cards simultaneously; commit all four transfers together.
- The two of clubs leads the first trick. Players must follow suit if able.
- On the first trick, no heart or queen of spades may be discarded unless every
  card legally available is a penalty card.
- Hearts cannot be led until a heart has been played, unless the leader holds
  only hearts. The queen of spades alone does not break hearts.
- Highest card of the led suit wins the trick and leads the next.
- Each heart scores 1; queen of spades scores 13. Taking all penalty cards
  scores 0 for that player and adds 26 to each other player.
- Score the hand after thirteen tricks. End the match when any total reaches 100
  or more. Lowest total wins; tied lowest players share the win.
- No house-rule settings, networking, account system, or matchmaking.

Use a fixed suit/rank ordering and documented seeded shuffle algorithm in the
game. Save the complete PRNG state and passing-cycle index. Tests use explicit
deals for rare cases rather than searching random seeds.

## Actions and checkpoints

`App::start_game` accepts a prepared new deal or a loaded state and creates
`HeartsContext` with the session connection. Initial entry presentation must
finish before gameplay dispatch. Starting does not itself execute an action or
save anything.

`ResolvePassing` is one action with human and AI choices against unchanged
pre-exchange hands; commit all transfers together. `PlayTurn` resolves any
leading AI plays, one human card choice, and the following consecutive AI plays
until the next human choice or a hand boundary. Each AI decision runs its policy
inside that same `execute()` call; there is no per-card handle dispatch. If a
card completes a trick or hand, collect, score, and deal through ordered
checkpoints before acceptance. A completed hand ends the action even if it ends
before that action reaches its human choice.

`HeartsState` includes hands, current trick, captured cards, turn/leader, broken
hearts, scores, passing phase, and saved random state. The display receives a
full immutable logical clone. It is responsible for showing opponent backs and
for omitting private values from player inspection. Do not add a separate
filtered view type or serialize full state into Unity.

For a fourth card, publish card play before trick collection, then score/deal
checkpoints as needed. One semantic event can describe all related effects.
Normal return adds a final snapshot without an event; acceptance waits for final
required animation and a subsequent rendered frame.

Schedule the next action when `use_game_status::<HeartsGame>()` becomes `Ready`,
using a fresh `accepted_state()` copy or the matching displayed snapshot:

```rust
match accepted.phase {
    Phase::PassingDue => game.dispatch(Action::ResolvePassing),
    Phase::Playing => game.dispatch(Action::PlayTurn),
    Phase::MatchComplete => return,
};
```

Schedule in an app callback/effect, not by mutating rules during render. One
UI/app controller owns next-action scheduling; AI policies never receive a
`GameHandle`. Do not dispatch repeatedly while busy or unconditionally play
after a last-card action: the next phase may require passing. Match results wait
for New Game. Menus remain responsive.

The context routes each live choice using its acting seat and prompt. Human
choices call `DisplayConnection::choose`; AI choices call `choose_with_policy`.
For passing, identify the seat currently choosing, not only the future trick
leader. Human confirmation submits a typed response to that request, never
another competing `PlayTurn` action.

Both paths retain the typed prompt for validation and publish one owned clone
inside `HeartsPrompt<'static>` with the snapshot. Policies borrow a
`HeartsPrompt<'_>` wrapper around the original; there is no reverse conversion.
After its snapshot is queued, a live AI policy immediately runs on the rules
worker and returns an option index. Animation and prompt visibility do not delay
search while fewer than 32 checkpoint slots are occupied. Human handles cannot
resolve AI-owned requests. Display code may identify West's decision while
hiding West's card choices. A queued AI prompt may already be resolved when
displayed; it is informational, not a new input request or a claim that search
is still running. There is no private controller-message channel or
engine-created independent AI job.

A simulation context always chooses inline using its configured policy. Game
code may schedule independent simulations, but the engine does not require it.
An abandoned live policy may finish bounded computation; its result is discarded
at the choice boundary and cannot affect a replacement.

Give every card a stable presentation UUID for its deal lifetime. Transfers
preserve it; inspection copies and new deals get separate identities.

## Interaction and layout

South's hand is a readable fan; opponent hands show backs. Display the current
trick in the center, captured piles near each seat, totals beside seats, and the
active player/passing direction in text as well as visual emphasis.

During passing, click/tap or keyboard/controller activation toggles a card. Show
exactly three selected cards and enable Pass only at three. Confirming
dispatches one answer; selection changes alone never mutate rules.

During play, click/tap a legal card to select it, then use Play or activate it
again to confirm. Dragging a legal card to the central play region confirms;
invalid drops/capture loss return it to its latest hand destination. Hover/focus
lifts a card and can reveal an enlarged inspection presentation. Touch
inspection uses an explicit Inspect control after selection.

Keyboard arrows/controller directions navigate cards and UI. Enter/primary
button activates; Escape/back closes inspection or menus and returns focus to
its invoker. Illegal cards remain inspectable but cannot be played. Modal menus
block table actions, preserve selection, and expose Resume/Save/New Game/Exit.

Adapt hand spacing and camera framing to portrait and landscape viewports. Keep
card faces readable at the selected native review resolutions. Handle
reorientation during an in-flight pass without restarting that occurrence.

## AI and shared simulation

Use seeded, bounded Monte Carlo rollouts through the same `Game::execute`
method. The policy receives `&HeartsState` and `&HeartsPrompt<'_>`. Its
game-owned sampler uses the acting seat's known cards, public play history, and
observed void suits to randomize hidden hands before search. The sampler must
not use the real hidden assignment as knowledge. The engine neither sanitizes
state nor creates an observation type.

For each decision, sample 32 possible deals consistent with what the player
knows. A **rollout** plays the rest of a sampled hand using a cheap legal policy
to estimate a candidate's result. Run one rollout per candidate on each deal.
For passing, score all three-card combinations with a cheap danger heuristic,
then evaluate the best eight through those same 32 sampled deals. Resolve ties
by stable card/combo ordering. These are engineering defaults, not performance
acceptance thresholds.

Rollout play uses a deterministic legal heuristic with seeded tie variation:
follow suit cheaply when losing, avoid taking penalties when possible, and
discard dangerous cards when void. Use the fixed scoring rules through the
normal execution primitives. Avoid a second shadow implementation of Hearts.

Evaluate each candidate by playing out the remainder of the current hand, then
applying ordinary hand scoring including shooting the moon. Minimize the mean
additional penalty assigned to the acting player across the sampled deals; do
not simulate the entire match or optimize an unspecified win probability.
Passing and play use this same objective. Reuse each sampled deal and rollout
seed across candidates so they are compared on the same hidden information.

For a passing candidate, force the acting player's three cards and choose the
other players' passes with the cheap rollout policy using their own information
in the sampled state; commit the exchange simultaneously. Rollouts then play
through the end of that hand. Rollout heuristics must respect the acting seat's
information.

Run live decisions on the rules worker, off the Unity thread. Keep search
bounded. Abandonment invalidates output immediately; ordinary search can finish
before the next choice/return boundary. Game-owned batch cancellation is
allowed, but Reactant exposes no explicit cancellation-check primitive. Fixtures
can override the work count with a fixed smaller count. Record decisions/seeds
for reproduction, not private cards in the player UI.

For example, compare playing a low club and a high club on the same possible
deals. Apply normal moon scoring before comparing penalties:

```text
candidate low club:  final extra penalties [0, 3, 1, ...]
candidate high club: final extra penalties [5, 0, 8, ...]
choose the candidate with the lower mean; break ties by stable card order
```

The sampled hands are simulation inputs, not knowledge granted to later acting
players. Each simulated choice receives sampled state and the shared prompt
enum; game-owned heuristics control which information they inspect.

## Explicit save and resume

V1 has no autosave, acceptance callback, or implicit save-on-exit. Save is an
explicit menu command. It calls `game.accepted_state()` and writes that owned
copy outside the rules worker. During an action it captures the previous
accepted boundary; before any action, it captures the initial deal. Tell the
player which completed position was saved rather than implying in-flight work
was accepted.

The game owns its save schema, including all data required to restore future
seeded behavior. Use atomic temporary-write/replace where supported. WebGL must
acknowledge a durable storage flush; an in-memory filesystem write is not
enough. Permit one explicit save write at a time and disable duplicate Save
while it is pending. Report success only for that exact captured state after
acknowledgement. A later game/action does not retroactively change the saved
copy.

Startup offers Continue when a valid save exists, otherwise New Game. Load calls
`App::start_game` with the saved state and a freshly constructed context.
Restore current visuals without replaying old transient events, then schedule
from the accepted phase after entry presentation. A restored human choice gets
fresh session/run/request identity.

Starting New Game does not overwrite an existing save. Exit does not capture or
save a newer state automatically. If an explicit write is pending, normal Exit
waits for that write, with retry or exit-without-finishing on failure. Forced
termination restores the last durably acknowledged explicit save.

Write/flush failures preserve playable in-memory state and show retry feedback.
Corrupt/incompatible saves show an explanation and offer New Game without
silently overwriting the only save. No save-version migration framework is
required. The existing chess sample's save behavior is preserved through its own
application logic, not by adding an engine autosave service.

## Manual QA

Play passing, a complete hand, moon scoring, and tied match results using
explicit fixture deals and all input methods. Save during a human prompt and
during trick collection; reload the captured accepted boundary with fresh
response handles. Verify new deals and completed actions do not save
automatically. Inject explicit write/flush failure, retry, and confirm normal
play and the previous durable save remain valid. Change only real hidden
opponent hands and verify AI sampling and decision distributions do not gain
knowledge from them.
