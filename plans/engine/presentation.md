# Render game snapshots into the existing command queue

Rules publish the moments the player should see. Reactant renders those states
into trees and generates ordinary Battlement commands. Unity works through its
existing command queue, waiting for blocking operations and letting nonblocking
operations continue.

For example, playing a card draws another card and grants one energy:

```text
Rust                                       Unity command queue
state: card played -> tree -> commands      move card to table (blocking)
state: card drawn  -> tree -> commands       reveal and draw card (blocking)
state: energy +1   -> tree -> commands       update energy label
                                           particles continue (nonblocking)
```

Rust can generate all three updates before Unity finishes the first movement.
It does not wait for animation, batch completion, or a rendered frame before
rendering the next state. The command queue already provides the ordering.

Read [execution](execution.md) for publication, prompts, and cancellation, and
[animation](motion.md) for choosing how objects move.

## The snapshot queue carries data

A queue entry contains an immutable `Game::State`, an optional `StateAnimation`,
and an optional presented prompt. Other pages call this a **checkpoint**: it is
simply a snapshot to render. Rules call `present` for intermediate changes and
`choose` for a prompt. Normal return publishes the final state automatically.

The Rust consumer takes entries in order, renders each against the previously
rendered tree, and appends the resulting commands to Battlement. It can
continue as soon as it has handed off those commands and downstream capacity
permits more output. Keep a FIFO of exactly
32 pending snapshots between worker and consumer. Reserve before
cloning/building; release a slot when the consumer takes the entry. Never drop
or coalesce entries. This bounds snapshot count, not bytes or the number of
commands Unity has yet to execute. Measure retained snapshot bytes separately.
Downstream capacity follows the bounded admission contract below; available
capacity allows render-ahead without waiting for animation completion.

The component tree represents the most recently rendered state, which may be
ahead of Unity's playback. Retain that tree for the next diff, not an additional
host-acknowledged tree. The engine does not stream game snapshots into Unity.

## Bound downstream admission

Use the existing FlatBuffers transport and its configured response/allocation
limits. Account for queued and retained buffers through their last host use,
including running operations and exit effects. A full cumulative budget is
backpressure, not session failure: suspend further snapshot consumption and
retain at most the current unsubmitted output. The 32-slot FIFO then naturally
stops the worker before more builders run. Resume consumption when capacity is
released; do not drop/coalesce snapshots or retry already submitted effects.

Admission must reserve capacity before publishing output, and final state stays
Busy/unaccepted until its final output is successfully submitted. Empty output
can finish locally. One response exceeding the per-message limit remains an
explicit error; keep the previous accepted state and report recovery.

Reserve capacity and independent delivery for menu, resume, cancellation, and
replacement work so a full gameplay budget cannot prevent its own release.
Capacity notification is transport flow control, not a per-snapshot success or
rendered-frame receipt. Do not block Unity's thread waiting for capacity. Test
saturation with controlled budgets and verify obsolete buffer ownership is released
after completion, failure, cancellation, and replacement; only live declared
content and explicitly retained effects may keep their required buffers. Task 12 establishes
this behavior; task 28 preserves it while paused.

## Generate ordinary ordered commands

Tree differences supply create, update, reparent, and remove commands. A changed
layout target supplies a blocking movement by default. A `StateAnimation` can
select a custom sequence, such as reveal, flip, then arrive in the hand.

Several movements in a parallel group all finish before its successor starts.
Cosmetic sounds, particles, hover, and loops are nonblocking. A custom sequence
owns its selected properties instead of also generating default movement for
those properties. To continue before a decorative tail ends, make the tail
nonblocking. Optional sequence labels schedule steps or sounds; they are not a
game-progress API.

Preserve ordering across responses with existing `BatchStart` dependencies.
Reactant's current app delivery uses `AfterEarlierBlockingWork` for ordinary
batches and separate `AfterEarlierAssetPreparation` asset batches. Preserve
gameplay ordering while making independent menu/control delivery explicit;
ordinary delivery must not overwrite that classification. Dependencies
between create/move/remove operations belong in normal ordered command groups.

No-change output needs no command and no special frame boundary. If the player
needs time to read a state, author an ordinary finite `TimeWait` in its display
sequence. Rules do not sleep for animation.

## Reuse the scheduler and Motion

[Reactant delivery](../../crates/reactant-core/src/app_delivery.rs)
already submits [commits as
batches](../../crates/reactant-core/src/commit.rs).
[Commands](../../crates/battlement/src/commands/command.rs) carry blocking
flags, and the
[scheduler](../../Packages/com.battlement.client/Runtime/Host/BattlementBatchScheduler.cs)
already waits on `IBattlementCommandOperation`.

Connect generated Motion starts to that existing operation interface: the
operation stays unfinished during playback, rather than completing when its
descriptor is installed. Preserve blocking flags through Reactant lowering and
reuse Motion's playback state, cancellation, and the command operation registry.
The fake must exercise the same queue behavior. This is an adapter, not another
scheduler. No batch-success notification or presentation-completion protocol is
required.

Load assets with existing asset commands and dependencies. Validate declarations
and refs before submitting their dependent commands. Use inactive objects and
ordered activation where a particular visual needs them. There is no general
prepare/ready/commit/discard transaction, display generation, atomic scene swap,
or per-snapshot rendered-frame receipt.

## Prompts and local interaction

A human choice publishes its snapshot and waits for a response. Queue the prompt
controls after earlier blocking gameplay commands, with the existing native
enabled/hit-region properties determining when the player can use them. Submit
answers through request-bound handles. A queued prompt does not need to tell
Rust when it becomes visible; its eventual response is the necessary return
message. Live AI can choose after publication and queue subsequent states
without waiting for animation, subject to downstream admission capacity.

Request-specific actionable children use distinct existing native object IDs
and retain their response handles. Do not reuse a still-visible old prompt's
input target for a newer request. A stale target/ended request is ignored, never
dispatched through a newer prompt's closure. Stable card visuals can remain
identified while their actionable children change. Apply this to pointer,
keyboard, and controller activation.

Menus and card inspection use independent display state. Hearts menu pause uses
the [game presentation pause control](motion.md#pause-gameplay-presentation-without-stopping-rules);
workers keep running while visible gameplay and its command queue are paused. Native hover offsets remain
responsive without rerendering game state. Local menu commands may use existing
independent batches; changes to the gameplay subtree must follow its queued
commands, so a settings rerender cannot reveal the final hand or score early.
Classify delivery by the affected commands, not merely by whether a click or
worker caused the render. Do not resend a complete future game tree to implement
a local hover or menu update.

Render each semantic event once when its queue entry is consumed. Later settings
or selection renders update ordinary props without replaying that event. Reuse
existing batch/command duplicate suppression and Motion playback identity for
transport redelivery; game code needs no checkpoint/effect numbering scheme.

Reflow retargets the currently executing move in place when it can be expressed
as a host layout/target update. Future gameplay moves remain ordered commands.
A rerender based on a future state cannot overtake those commands. Hover and drag
cannot cancel blocking gameplay placement; cosmetic replacement uses existing
controls. There is no transfer of progress requirements between playbacks.

## Rules completion and failure

Normal rules return accepts the final logical state once the Rust consumer has
rendered its final publication and submitted the generated commands. It does not wait for Unity. `Ready` means
the rules can accept another action, not that animation has stopped. Application
code may enqueue the next action; normal player controls become available at
their authored position in the native command queue.

A worker failure retains the previous accepted state. A host failure stops
presentation and offers restart/exit; it does not undo rules actions that have
already been accepted. Restart rebuilds from accepted state without replaying
old events. Handle existing `BatchFailed`/`OperationFailed` messages in the app,
classifying gameplay versus cosmetic failures by their owned work. Old-session
messages cannot affect the replacement.

Stop/replacement discards pending snapshots and cancels only the old game's
queued and running commands. Extend the existing scheduler with game-scoped
work ownership and cancellation; its current whole-session reset clears all
batches, and individual operation cancellation cannot cancel unstarted commands.
Keep persistent app/menu hosts, hooks, and unrelated operations alive. Release
old game buffers/resources and prevent unstarted commands from launching, even
while paused or capacity-blocked. Reuse batch/command identities to attribute
work and failures; do not introduce a second scheduler. Task 12 owns this
boundary before later effects and pause tasks extend it.

## Manual QA

Hold Unity playback on the played card while Rust renders draw and energy.
Release playback and verify the existing queue preserves all three steps and
nonblocking particles continue. Use menus while commands remain queued. Check
that future state does not appear early, old prompt controls cannot answer a
new request, and a new prompt becomes usable in command order. Exercise empty
updates, explicit waits, delayed assets, stop/restart, and host failure after
rules acceptance. Neither normal rendering nor live AI waits for Unity replies.
