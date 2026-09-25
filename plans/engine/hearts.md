# Hearts as a Reactant engine benchmark

## Authority and deliverables

This is the standalone implementation design for Hearts. It replaces the former
Hearts architecture, explicit-save policy, asset prerequisites, and tasks 35–43.
Other games may continue to use the lower-level engine contracts. The numbered
engine migration sequence is not itself a prerequisite: inspect actual code and
follow the [native work graph](hearts-work-graph.md).

The user approved the product direction and clarified that the current request
is **detailed planning and native bead filing only**. Future implementation and
introspection remain natively deferred without expiry. Closing the design bead
does not authorize execution. Explicit user authorization is required to release
the execution gate and planned work.

Deliver a beautiful complete offline Hearts game and meaningful reusable
Reactant improvements. A sample-only implementation fails. Preserve the
[blind React reference](hearts-react-reference.md) as the independent comparison;
do not rewrite it to justify the eventual Rust implementation.

Targets: native macOS, desktop WebGL, native iOS/Android; mouse, touch, keyboard,
controller, portrait, and landscape. Exclude phone browsers, Windows/Linux,
networking, accounts, configurable house rules, undo, hints, tutorial campaigns,
store publishing, and public web deployment. Prioritize familiar React ownership,
reducers, hooks, effects, keys, and declarative presentation in idiomatic Rust;
no JSX macro system or general React parity project.

The user owns physical mobile testing/performance sign-off and final aesthetic
approval. The agent owns builds, instrumentation, reproducible instructions, and
visual/audio evidence. Intermediate visual review proceeds autonomously.

## Current implementation and ownership

Revalidate these facts in the executing task's own worktree before changes.

| Concern | Current owner | Required approach |
| --- | --- | --- |
| Components, hooks, effects | `reactant-core` | Extend the shared runtime, not another component system. |
| Sessions, accepted/rendered state | `reactant` session/output modules | Reuse final-output acceptance and lifecycle ownership. |
| Rules workers and bounded publications | `reactant-rules` | Reuse worker primitives and the 32-slot publication path. |
| World layout/destination identity | `reactant` world layout modules | Preserve existing reparenting and identity mechanisms. |
| Motion/audio/particle sequences | Existing Reactant/Battlement APIs | Add lifecycle ergonomics over current host execution. |
| Persistent-state hook | Synchronous backend/direct file writes | Add asynchronous ordered durable storage. |
| Cached build targets | macOS, WebGL, iOS simulator | Android hooks and simulator proofs do not establish physical delivery. |
| Owned artwork | Chess's KayKit PlayingCards and ForestNaturePack | Inventory existing assets; the missing-card-archive prerequisite is obsolete. |

Rules, composition, orchestration, and tuning belong in Rust. Unity supplies
prepared meshes/materials/audio/effect assets. Shared rendering/storage/build
changes belong in package/tooling code, never sample-specific C# gameplay.
Use authoring and generation tools rather than editing generated Unity outputs.

## Product and art direction

### Composition and assets

Match [the reference](../../docs/hearts-kaykit-forest-reference.png): a bright
low-poly forest clearing, elevated fixed camera, warm soft shadows, oversized
white cards South, navy/gold backs at the remaining edges, and a quiet center.
Use the owned KayKit deck despite its stylized court figures differing from the
mockup's traditional illustrations. Fidelity concerns composition, readability,
colors, scale, lighting, and material treatment, not accidental card/rule states
in the reference. Build from the existing asset pool under the boundary below.

Prepare KayKit card meshes with verified face/back mappings and the minimal
forest subset through import/configuration inputs. Retain source/license records
and deterministic generation. Hearts must not need a chess player/runtime assembly.
Map all 52 faces once and one shared back. Cards have independent hit regions,
small visible thickness, rounded silhouettes, readable corners, and contact
shadows. Compose environment objects in Rust from prepared assets.

Keep trees, bushes, cliffs, grass, and rocks around the clearing without hiding
cards or hit regions. Do not orbit or shake the camera during play. Use a
consistent green/cream/navy/gold palette and light direction. Add compact seat
names/scores, restrained active-seat accents, and a small prompt/action area.
Text identifies turns/passing; color, motion, and sound are supplementary.

Landscape retains four large fans. Portrait recomposes the perimeter and center;
do not scale down the desktop canvas. Preserve legible human ranks using a
horizontally pannable fan when needed, with automatic focus reveal for keyboard
and controller. Opponent fans may compress because counts convey their relevant
information. Respect safe areas, scaled text, and usable touch targets.

### Existing asset boundary

Use the assets already in the repository. Do not commission, purchase, download,
generate, paint, model, record, compose, or synthesize new artistic assets for
Hearts. The only new visual asset exception is simple deterministic geometry
that is straightforward to generate and verify: planes, discs, rectangles,
rings, or basic confetti shapes for surfaces, focus marks, and restrained effects.
No bespoke illustrations, textures, detailed meshes, rigs, or artistic shaders.

Normal implementation work remains in scope: importing and addressing existing
assets, composing scenes/prefabs from them, configuring existing materials and
particle emitters, tint/scale/opacity changes, layout, lighting, animation,
playback timing, gain, and crossfades. Use existing fonts for labels and controls.
Effect composition must stay simple; adapt an existing effect or a few primitive
particles rather than undertaking a custom VFX production task.

The local inventory includes 52 card models, 52 faces and one back, 198 forest
models with their shared texture, 41 NotJam sound effects and four music tracks,
and NOVA particle shaders/example prefabs. All card/forest models and PNGs passed
structural checks; referenced textures and Unity metadata are present. All local
Opus audio decoded successfully. These checks do not establish Hearts import,
rendering, or aesthetic suitability; verify those during implementation.

Select and audition from that pool. Literal paper/card recordings, woodland
ambience, leaf sprites, heart-shaped bursts, and newly composed gentle music are
not acceptance requirements. Use existing selection/contact/result sounds for
card actions; choose the least intrusive existing music and tune its mix. Use a
suitable existing ambient loop only if one is available after listening;
otherwise omit the ambience layer and its control. Keep music and SFX required.
Use simple existing or primitive-particle accents for heart breaking and results;
replace unavailable leaf imagery with sparse neutral particles or omit that
specific decoration. Preserve lifecycle/particle engine work and deterministic
evidence even when a particular decorative treatment is omitted. Record each
chosen source and substitution; an asset gap must not create a new production
bead or block completion on excluded artwork.

### Motion and sound

| Occurrence | Initial treatment | Audio/effect role |
| --- | --- | --- |
| Deal | 45 ms stagger into fans | Quiet existing short SFX with restrained variation |
| Focus/hover/selection | Short lift and spring settle; distinguish selection | Quiet selection tick, never rerender-driven |
| Pass exchange | 450 ms transfer and hand reflow | Existing transfer cue without revealing secret hands |
| Play | 250 ms travel, slight tilt, clean contact | Landing contact cue |
| Complete trick | Approximately 650 ms readable hold | Winner and penalty emphasis |
| Collect | 400 ms into winner's pile | Existing collection cue, sparse existing/primitive accent |
| First heart | Brief red/gold tint/primitive pulse and text | Distinct soft cue |
| Hand/match results | Count-up then stable totals | Existing result cue, restrained existing/primitive celebration |
| Ambient | Sparse existing/primitive particles away from faces | Existing music; suitable existing ambience only if available |

These are tuning defaults, not test sleeps. Use the shared presentation clock
and explicit controlled test times. Ambient effects have deterministic seeds and
controlled clocks and cannot prevent readiness from settling. Select sound roles
under the existing asset boundary; preserve provenance and listen to the mix.
Missing thematic assets require substitutions, not new asset production.

Settings: separate music/SFX volume, ambience volume only when that layer is
used, reduced motion, text scale, animation speed.
Reduced motion substitutes brief state transitions for large travel/bursts while
preserving readable outcomes and order. Volume changes do not restart music;
track replacement crossfades; unmount/backgrounding do not leak playback.

## Rules, model, and information boundaries

### Fixed rules

Four clockwise seats: South human, West, North, East. Deal all 52 cards, thirteen
each; ace high, no trump. Pass cycle left/right/across/hold. Each passing seat
submits three distinct owned cards against unchanged hands; commit all transfers
atomically after all submissions. The two of clubs opens; follow suit if able.

On the first trick, hearts and queen of spades cannot be discarded if a legally
available non-penalty card exists. Forced penalty discards are allowed. Hearts
cannot lead unbroken unless the leader holds only hearts. Queen of spades does
not break hearts. Highest led-suit card wins; winner leads next.

Hearts score one each, queen of spades thirteen. Taking all penalties scores
zero for the shooter and twenty-six for each other seat. Score after thirteen
tricks. End after scoring when any total reaches 100; tied lowest seats share
the win. No sudden-death hand or additional variant settings.

### Logical model and transitions

Create a standalone sample rules workspace with a domain-only module boundary.
`CardId` is stable suit/rank data, ordered clubs/diamonds/spades/hearts then
ascending rank. It is not a Unity UUID. A private projection layer allocates an
opaque random presentation token for
each card in the mounted session; never derive a publicly reversible token
from suit/rank. Cards retain that token through zones and deals within the
session. Restore allocates fresh tokens while retaining domain IDs.

`HeartsState` owns hands, pass submissions/cycle, current trick, captured cards,
public history, leader/turn, hearts-broken flag, hand index, totals, phase, and
saved random streams. Every card occupies one actual zone. Pending passes
reference cards still in hands; they are not another ownership zone. History
references identities without duplicating cards. Retain scoring/results until an
explicit next-hand action.

Use seeded Fisher–Yates with a pinned deterministic PRNG and serialize exact
resumable state. Deck and AI randomness are separate streams; presentation
randomness cannot affect either. Record intentional seeded-behavior changes when
dependencies change. Do not build a save migration framework.

Intentions: `SubmitPass { seat, cards }`, `PlayCard { seat, card }`, `NextHand`.
Human commands wrap an intention with the session/revision guard. AI task output
is a decision envelope containing the intention, seat, expected observation
fingerprint, expected per-seat stream identity/state, and resulting stream
state. The reducer checks that envelope against the accepted input; its private
next state includes both the intention and stream advancement. Final output
acceptance commits both together. Cancellation, rejection, or failure before
final submission commits neither. A later playback failure preserves both.
New Game replaces the session. One played card is one command for human or AI;
there is no multi-turn action waiting for a human and executing AI replies.

The pure reducer takes state/command and emits final state plus ordered semantic
checkpoints. External stale/illegal intentions return typed rejection without
mutation; impossible internal invariants panic. One shared transition function
accepts a presentation sink: live collection captures checkpoints; simulation
uses a no-op sink and avoids snapshot allocation. Do not duplicate rules for AI.

Events: deal, pass exchange, card played, heart broken, trick collected, hand
scored, match ended. Carry domain facts, not asset addresses or timings. A
fourth-card command exposes the completed-trick snapshot before collection,
then collection/scoring checkpoints, then final state. Combine simultaneous
consequences where one event accurately describes them. Only final output
submission accepts the command's logical result.

### Observations

`HumanView`: own cards, legal actions/reasons, public trick/history, counts,
totals, phase, pass direction, and announcements. `AiObservation`: acting hand,
public plays/counts/void suits, and legitimately known passed-card information.
Neither contains another seat's hidden hand or secret pass submission.
`HumanView` contains ordered opaque card tokens for opponent backs and optional
visible faces for public/own cards. Projection owns the private CardId-to-token
mapping. Reveal adds a face to the same token; it does not replace identity.
Project semantic checkpoints/events through the same boundary. Hidden-card
props/materials must not contain their face texture or rank, even if rendered
face down; counts alone are insufficient for stable card animation.

Selectors are pure; do not copy derived values through effects. Give AI only an
observation, not full state with a promise to ignore fields. Diagnostics can
retain seeds/decisions; player text/accessibility cannot reveal private cards.
Saves are internal local data, not a multiplayer security boundary.

## Reactant authoring contracts

These are proposed public shapes, not claims about existing APIs. Rust bounds
and module placement follow current facade/layering. Keep adapters thin and
preserve independently useful lower-level APIs. Shared library code must remove
caller responsibility rather than hiding the same bookkeeping behind new names.

### Reducer sessions and presentation acceptance

`use_game_reducer(session_key, initializer, reducer)` mounts one typed session,
runs initialization once per session, and returns a stable handle. Expose
accepted observation, presented selectors, session/revision, readiness/failure,
and guarded dispatch. Dispatch carries expected session/revision and returns
started, busy, stale, or typed rejection. Delayed ordinary UI events do not panic.

Adapt to existing `Game`, worker, and publication machinery. Do not add an
application event queue, competing scheduler, or sample-owned occurrence counter.
Reuse 32-slot publication capacity and downstream byte bounds. One command may
produce multiple checkpoints. Reserve capacity before constructing publications;
do not build an unbounded eager event list ahead of backpressure. Hearts has a
small bounded number of checkpoints per command.

Final output submission advances accepted state/revision together exactly once.
Prior submitted output retains the preceding presentation work before acceptance.
Submission acknowledgment does not mean animation completion. Busy admits
nothing. Duplicate final submission is harmless. Session replacement invalidates
old handles, tasks, and pending output.

Accepted and presented observations must be clearly named. Saves/AI read
accepted state; cards/scores/input read presented state. Hearts dispatches a new
game action only when preceding presentation settles and visible revision equals
accepted revision. Expose a scoped settled receipt/observation from the engine;
no application timer or guessed frame count. Inspection and menus stay live.

Failure before final output acceptance retains the prior accepted state. A host
playback failure after acceptance retains the newly accepted state; it never
rolls accepted rules backward. Both disable gameplay and offer recovery. Cancel
failed-session host work and
reconstruct from accepted state without historical effects. A partly displayed
command whose final output was not submitted cannot be reported accepted or
saved. A command already accepted remains eligible for saving even when its
animation subsequently fails.

### Cancellable asynchronous work

`use_task(key, work)` exposes idle/pending/ready/failed states and owns cleanup.
Disabled keys start nothing. Replacement/unmount invalidate results immediately.
Work receives a cooperative cancellation token; completion returns through the
component runtime without UI-thread joins.

Build the computation lane from existing worker primitives, separate from the
rules-action slot. Hearts permits one active search and one latest replacement;
no unbounded jobs or per-seat thread fleet. A replacement waits for cooperative
cleanup, without blocking UI. Check cancellation at assignment steps and every
simulated card transition. Verify finite worker admission in desktop WebGL.

`use_hearts` composes reducer, selectors, AI task, and presentation readiness.
AI keys contain session, seat, and observation fingerprint. Another seat's secret
pass submission does not restart unchanged work. Revalidate ready results against
current phase/observation and use current revision at guarded dispatch. Obsolete
results cannot commit.

### Presentation, audio, and particles

Use existing `use_animate` and sequence execution. Occurrence identity is session
plus publication sequence; rerender/settings/resize cannot replay it. Add typed
composition of event motion/sound/particles and settled receipts over existing
ownership, clocks, and pause mechanisms.

World layout owns card destinations; keys/refs remain stable through hand, trick,
and captured pile. Extend existing destination identity/world-preserving
reparenting; no second shared-layout registry. Improve fan measurement, clipping,
focus reveal, or retargeting only where the sample demonstrates missing behavior.

Provide lifecycle-owned audio/ambient hooks or components with typed asset,
enabled/paused, gain, loop, crossfade, seed, and reduced-motion inputs. Own and
dispose handles on unmount. One-shots belong to occurrences; ambience belongs to
component lifetime. Scene replacement stops both scopes. Eliminate uncontrolled
Unity particle randomness/time from deterministic Ditto evidence.

### Component composition

```text
HeartsApp
  StartScreen: Continue, New Game, Settings, Rules
  HeartsProvider / use_hearts
    ForestTable
      OpponentSeat × 3
      TrickArea
      HumanHand -> Card × remaining cards
    Scoreboard, PassingControls, Announcements
    HandResults, MatchResults
    PauseMenu, Settings, Rules
```

Selection resets on phase/session changes or when selected cards leave the hand;
opening a modal does not clear it. Components receive observations and intention
callbacks, not mutable full rules state. Context avoids deep prop plumbing,
selectors limit updates, and local hooks own transient interaction. Do not move
all UI state into another global controller.

## Interaction and application lifecycle

Startup shows storage loading/error until resolved, then Continue if a valid
save exists. New Game, Settings, and Rules are available. Contextual teaching
explains passing/penalties at first use. Results offer Next Hand or New Game.

Click/tap selects and raises; second activation or Play commits. Dragging a legal
card into the center commits. Invalid drop, capture loss, and cancellation return
to the latest layout destination. Never commit twice across animation/task
completion. Passing toggles cards; Pass requires exactly three.

Arrows/D-pad/stick navigate; Enter/primary selects then confirms; Escape/back
cancels or closes the top overlay. Illegal cards stay inspectable with a reason.
Hover/focus lift; touch has Inspect. Focus and selection are distinct without
relying only on color. One activation cannot both select and activate a newly
mounted confirmation control.

Modals trap focus, block table hit testing, preserve selection, and return focus
to the invoker or valid successor. Pause freezes gameplay presentation and stops
new actions/search while menus/settings remain responsive. Resume continues the
same occurrence. Backgrounding cancels compute, suspends audio, and attempts a
pending-save flush. Foregrounding revalidates input and state before scheduling.

Reorientation preserves identity, selection, and event progress; retarget
endpoints without reissuing occurrences. Keep scores/prompts/controls readable
with larger text and safe areas. Semantic labels must not expose hidden hands.

## Fair AI and shared simulation

Ship one difficulty. Evaluate each legal play on 32 plausible hidden deals. For
passing, score all three-card combinations with a deterministic danger heuristic,
retain eight, and evaluate them on the same 32 deals. Stable card/combo ordering
breaks ties.

Build each hidden deal only from observation: remove played/known cards, fix
legitimately known owners, expand remaining hand capacities into slots, and solve
a randomized bipartite matching between unknown cards and allowed slots. Shuffle
traversal using the AI stream; augmenting-path search is finite over at most 52
cards. This is plausible sampling, not a uniform-distribution claim. Infeasible
matching indicates an observation invariant failure; never relax constraints or
inspect the real hidden assignment.

Keep separately persisted per-seat AI streams independent of deck randomness.
An accepted decision changes only its acting seat's stream. Every candidate uses
the same sampled deals/rollout seeds. Cancellation advances no accepted random
state. Commit stream advancement alongside the accepted decision so resume is
reproducible.

Rollouts call the shared rules with a no-op presentation sink. A cheap legal
policy follows suit cheaply when losing, avoids penalties where possible, and
discards dangerous cards when void. Each simulated actor reasons from its own
sampled observation, never omniscient state. Passing forces the candidate and
chooses other passes independently from their sampled observations, then exchanges
simultaneously. Never use actual secret submitted passes.

Apply ordinary end-of-hand and moon scoring; minimize mean added penalty to the
acting player. Do not simulate match win probability. Bound work by operation
counts, never elapsed-time cutoffs. Record measured strength/latency tradeoffs
when tuning sample counts. The UI remains responsive during long searches.

## Durable storage and resume

Add an asynchronous typed persistent-state interface over platform backends.
Distinguish loading, absent, loaded, pending write, committed, and recoverable
error. Stable callbacks and completion delivery belong to the normal runtime;
renders never perform blocking storage or sample-specific polling.

One owner serializes a save slot across sessions. Requests capture immutable
values and session/revision. Coalesce unstarted writes to latest desired state;
never run replacements concurrently. Old completion cannot update new-session UI
or overwrite newer durable state. Track committed separately from accepted
revision. Retry latest desired state rather than resurrecting stale work.
Settings have their own slot/lifecycle.

Native protocol: serialize, write same-directory temporary file, synchronize it,
atomically replace destination, synchronize the containing directory where
supported, then acknowledge. Surface failures. WebGL protocol: store the complete
record in one IndexedDB readwrite transaction; acknowledge transaction completion.
Initialize/read asynchronously. In-memory filesystem writes are not browser
storage success.

Autosave the initial accepted deal and each accepted action, including partial
passing submissions. Restore the latest complete committed record with fresh
session/task/host identities and no transient backlog. Abrupt termination can
lose actions not yet committed. A complete commit is valid even if termination
prevented its success callback; no extra acknowledgment journal.

Pause permits save completion. Background flush is best effort; the OS need not
wait. Application-controlled exit awaits pending storage or offers retry/exit
without finishing. New Game confirms replacement of resumable progress; ordered
storage prevents older pending writes overwriting it.

Validate loaded structure/game invariants before mounting. Corrupt/unavailable
storage and write failure retain previous valid storage where possible and
playable in-memory state. Explain failure and offer retry or explicit New Game;
never silently overwrite unreadable data. No save migration system. Diagnostic
logs are actionable without dumping private cards into player UI.

## Introspection, reuse, and delivery

Follow the [work graph](hearts-work-graph.md). Every bounded implementation has a
separate blocking introspection bead; the next feature depends on introspection,
not just delivered code. All execution is deferred until explicitly authorized.

Each review must record evidence and answer:

1. What was awkward/repetitive, with concrete code or tool output?
2. How would the independent React sketch express that responsibility?
3. Why was React simpler, or why is the Rust difference justified?
4. Does the remedy belong in Reactant, shared host/tooling, or game domain code?
5. What reusable fix was implemented/adopted, or why is no change justified?

Vague future-work lists do not close introspection. Classify every finding:

- **Required for current acceptance:** implement the bounded repair before closing
  introspection. If it needs a separate implementation bead, link it as a blocker,
  pair it with introspection, and adopt/validate the result before continuing.
- **Larger-scope improvement:** file a concrete native follow-up for architecture,
  tooling reliability/performance, library APIs, or another evidenced concern.
  Record the observed problem and code/log evidence, proposed outcome, bounded
  scope/non-goals, acceptance criteria, validation, priority, intended project,
  creator thread, and known prerequisites. Link it back to the exposing
  implementation and introspection using discovery relationships. Search for an
  existing equivalent bead before filing and link/update that work instead of
  duplicating it.

Broader follow-ups do not automatically block Hearts or gain implementation
permission. Make one blocking only when its absence prevents a stated acceptance
criterion, and document that reason. If such a fix materially exceeds authorized
scope, retain the blocker and seek explicit scope authorization rather than
silently expanding the task. Otherwise keep broader work independently deferred
without expiry until authorized. Unknown project ownership must be reported,
not guessed into an unrelated project's ready queue.

Every introspection's completion evidence lists the actual follow-up bead IDs
and dispositions, or explicitly records that no larger-scope finding was found.
A prose TODO is not a filed follow-up. Read-only no-change conclusions need no
recursive review, and findings must not be invented merely to fill a quota.
Split oversized implementation before execution and preserve the blocking edges.

Prove reuse through focused chess music and asynchronous-opponent migrations.
Compare before/after caller code and cleanup. Do not redesign chess, alter its
rules, or force unnecessary migrations. Keep progress/evidence in beads, not
maintained guidance. A new abstraction must remove real caller responsibility.

Use task-owned isolated worktrees, native claiming, staged validation, repository
review policy, exact Tollgate candidates, certified promotion, and synchronization.
The product plan never waives a failed required check.

## Validation and completion

### Automated contracts

| Layer | Required cases |
| --- | --- |
| Rules | All passes, atomic exchange, conservation, first lead, follow suit, forced penalty, heart/queen exceptions, all-hearts lead, trick leadership, moon, threshold, ties. |
| Reproducibility/privacy | Seeded replay, save equivalence, explicit rare deals, hidden UI exclusion, AI hidden-hand invariance with fixed observation and AI seed. |
| Reducer/output | Busy/full capacity, stale/duplicate input, intermediate/final submission failure, exactly-once acceptance/effects, old-session invalidation, settled versus submitted. |
| Tasks | Dependency changes, pause/restart/unmount, cancellation within sampling/rollouts, bounded workers, failure/retry, no UI search/join. |
| Input/presentation | Passing, inspect/reasons, drag cancellation, keyboard/controller confirmation, modal focus, pause/reorientation/reduced motion, stable identity. |
| Storage | Absent/valid/corrupt, native/browser failure/commit, coalescing, old/new races, retry, termination boundaries, no partial load. |
| Platform | Mobile builds/lifecycle; desktop WebGL input/shaders/audio activation/storage/reload/finite worker pool. |

Prefer black-box behavior/native Ditto to implementation-mirroring tests. Complex
pure rules/sampling tests are useful. Use explicit rare deals, not CI seed search.

### Visual, audio, and device evidence

Reserve three sequential scene-to-reference improvement passes (V01–V03 in the
[work graph](hearts-work-graph.md#sequential-reference-refinement)) after H29/R29
and before final evidence H30. Each pass freshly compares the latest native scene
with the mockup, corrects its most visible remaining differences using existing
assets, and retains comparable before/after captures. Each has a blocking
introspection bead (RV01–RV03), including concrete larger-scope follow-up filing.
Keep the loop bounded and protect readability, portrait use and performance;
record an evidence-backed no-change outcome if no worthwhile in-scope adjustment
remains. These passes do not replace user-owned final aesthetic approval.

Retain native landscape/portrait captures for all hands, selection/passing,
in-flight play, complete trick, collection, results, modal focus, and reorientation.
Use controlled midpoints, not only endpoints. Compare directly with the reference
before baseline updates; pixel equality alone does not prove aesthetic quality.
Listen to the actual mix and timing. Repeated lifecycle changes must not leak or
repeat audio. Verify deterministic particles stay off faces, and inspect clarity,
hit targets, safe areas, lighting, clipping, depth order, and visible focus.

Deliver Android APKs and native iOS device builds through the user's signing
route. Establish SDK/signing/installation prerequisites in the first audit.
Missing credentials/access are external dependencies; unsigned projects and
simulator evidence do not prove physical delivery. No store publishing.

Provide a physical-test packet with exact build identity, installation steps,
fixtures, timing overlay/log export, and expectations for a full hand, AI,
effects, rotation, background/resume, failure/retry, controller, and touch.
Target sustained 60 fps on iPhone 12 / Pixel 6 class hardware. Measure a full hand
after warmup, with frame-time distributions/missed frames, not a static scene.
Reduce decoration before clarity. The user performs device testing and sign-off;
keep physical/performance and aesthetic gates open until actual results arrive.

### Five-minute CI contract

Establish a fresh baseline on the normal runner. Full warm automated pre-promotion
validation must finish within 300 seconds: changed-source compilation, selected
generated-input checks, formatting/lint, Rust/Unity/tooling tests, required native
scenarios, and selected automated browser checks. Time actual execution start to
terminal validation. Report queue/lease waits, cold toolchain preparation, and
cold player construction separately; do not hide normal source rebuilds as setup.

Human visual/audio review, device testing, and sustained performance runs are
separate required milestone evidence. Invalidate retained evidence for relevant
source/assets/build recipes/toolchains/inputs/scenario changes. Demonstrate warm
changed-source sample, engine, and host runs; unchanged cache hits are not proof.

Tool failures and over-budget warm runs block dependent features. Retain handles
and timings, isolate the boundary, and repair observed bottlenecks. Prefer shared
setup, correct caches, parallel independent checks, and removal of redundant
checks/waits with coverage rationale. Do not skip failures, inflate timeouts,
mislabel cold work, or silently delete unique coverage. Surface materially
unrelated repair scope or irreducible coverage/budget conflicts rather than
starting an unlimited rewrite.

### Acceptance

Completion requires a polished full match, substantive Reactant improvements used
by Hearts/chess, closed blocking introspections, passing automated validation
within budget, visual/audio evidence, mobile deliverables, and user-owned device/
performance and final aesthetic sign-off. Planning completion requires this
document, preserved independent reference, and verified deferred native graph to
be reviewed and delivered; it does not claim future implementation already exists.
