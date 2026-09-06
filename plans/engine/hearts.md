# A playable four-player Hearts sample

Hearts demonstrates the engine in a complete game: one human and three AI
players, a 3D table, UI menus and scores, and durable save/resume. Read this for
rules, interaction, AI, and persistence requirements. See
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

An action becomes accepted after its final required animation and rendered
frame. Keep actions small enough that autosave can preserve useful progress.
NewGame prepares a dealt initial state, presents its entry behavior, and accepts
it after the same animation-and-rendered-frame requirements. Autosave that
initial accepted deal before any passing/card action has completed.
ResolvePassing is one action with a human typed choice and AI selections; it
accepts only after the simultaneous exchange has been presented. PlayTurn
accepts one card play. If it completes a trick or hand, that same action also
resolves collection, scores, and any next-hand deal through ordered checkpoints
before acceptance.

`HeartsState` includes hands, current trick, captured cards, turn/leader, broken
hearts, hand/match scores, passing phase, and PRNG state. Display snapshots
contain only the human-visible information; opponent faces remain hidden. Keep
full private state out of normal UI props and the player inspector.

~~~text
fourth card played -> full trick visible -> collect trick
-> update hand score if final trick -> results/deal if continuing
-> final checkpoint rendered -> accept/save -> next action
~~~

After every accepted boundary or completed restoration, inspect the accepted
phase before scheduling: PassingDue starts ResolvePassing, Playing starts one
PlayTurn, and MatchComplete starts no action. Never unconditionally start
PlayTurn after a card action: its final checkpoint may already contain a newly
dealt hand requiring a pass. Do not schedule while another action, failure
surface, or restoration presentation is active. Show hand summaries without
requiring a separate continuation action; match results wait for New Game.

The worker publishes a typed prompt for the acting seat. A human card
confirmation answers that prompt; it must not dispatch a second competing
PlayTurn action.

An AI-owned prompt is answered by an application-owned simulation job only after
the prompt snapshot is presented. That job receives an owned observation for its
acting seat, never the full private state, and returns through the same
run/request validation path as a human answer. This is an independent AI worker
permitted in addition to the display thread and waiting rules worker. Cancel its
bounded work on abandonment/request replacement and discard stale results.

Route private AI observation data only to the application controller that starts
the AI job. UI components receive a separate human-visible prompt, such as "West
is choosing a card", and no hidden hand data. The simulation executor chooses
inline and never starts these interactive automation jobs. During passing,
collect choices against the unchanged pre-exchange hands, then apply all
transfers together.

Give every card a stable presentation UUID for its deal lifetime. Transfers
preserve it. An inspection copy gets another UUID. A new deal gets new
identities even when rank/suit repeats.

For example, the application chooses its next action from the accepted phase:

```rust
match accepted.phase {
    Phase::PassingDue => dispatch(Action::ResolvePassing),
    Phase::Playing => dispatch(Action::PlayTurn),
    Phase::MatchComplete => show_results(),
}
```

This dispatch runs only when the application is idle and restoration has
finished. A human confirms an answer to the resulting prompt. An AI job answers
that same kind of pending request through its typed handle.

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
block table actions, preserve selection, and expose Resume/New Game/Exit.

Adapt hand spacing and camera framing to portrait and landscape viewports. Keep
card faces readable at the selected native review resolutions. Handle
reorientation during an in-flight pass without restarting that occurrence.

## AI and shared simulation

Use seeded, bounded Monte Carlo rollouts through the same rules functions.
Sample opponent hands consistent with known cards, played cards, and observed
void suits. An AI receives its own observation, not the full private state. Do
not accidentally expose opponents' hands via a choice specification.

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
other players' passes with the cheap rollout policy on their own sampled
observations; commit the exchange simultaneously. Rollouts then play through the
end of that hand. All subsequent actors still use their own observations.

Run decisions off the Unity thread. Check cancellation between bounded rollout
batches; simulation primitives themselves retain the no-op cancellation fast
path. Fixtures can override the work count with a fixed smaller count. Record
decisions/seeds for reproduction, not private cards in the player UI.

For example, compare playing a low club and a high club on the same possible
deals. Apply normal moon scoring before comparing penalties:

```text
candidate low club:  final extra penalties [0, 3, 1, ...]
candidate high club: final extra penalties [5, 0, 8, ...]
choose the candidate with the lower mean; break ties by stable card order
```

The sampled hands are simulation inputs, not knowledge granted to later acting
players. Each simulated choice still receives only its actor's observation.

## Autosave and resume

After the initial accepted NewGame deal, accepted passing, and each accepted
PlayTurn, save the newest accepted state asynchronously outside the worker. The
game owns its save schema. Serialize saves in acceptance order; use temporary
write plus atomic replacement where supported. On WebGL provide an explicit
durable storage flush/acknowledgement; an in-memory filesystem write alone is
not a completed save.

Use one writer that handles saves in order. Identify each write by match ID and
accepted-state sequence number. Starting New Game invalidates queued older-match
writes; let an already running write finish before the new initial save replaces
it. An old completion cannot mark a newer state durably saved. The normal Exit
flow asynchronously flushes the newest accepted save before closing, with
retry/exit-without-saving choices on failure. Forced process termination
restores the last durable acknowledgement, not an unacknowledged in-memory
state.

Startup offers Continue when a valid save exists, otherwise New Game. Resume
reconstructs the current visible state without replaying old transient effects
and then schedules an AI turn if needed. Opening a human prompt creates a new
run/request identity; stale answers from before restoration are invalid.

Save failures show nonblocking feedback and allow retry while in-memory play
remains valid. Corrupt/incompatible saves show an explanation and offer New
Game; do not silently overwrite the only save before that choice. No backward
compatibility/version migration framework is required.

For example, New Game must not let an older save overwrite the new deal:

```text
old match write is running; another old write is queued
New Game: discard the queued old write; queue the accepted new deal
running old write finishes; new-deal write runs next
Exit: wait for the newest accepted state to be durably acknowledged
```

A completion identifies the exact write it finished. It cannot mark a newer
state as saved merely because the writer became idle.

## Manual QA

Play through passing, a complete hand, scoring, and a new deal with all input
methods. Exercise moon scoring and a tied match using explicit fixture deals.
Exit during passing and during trick collection, then resume the last accepted
boundary. Inject a save failure and confirm play continues with visible
feedback.
