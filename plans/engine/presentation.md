# Present complete states and wait for the right animation

The display must never show half of a checkpoint or accept input using handlers
from a different visible state. Prepare expensive work first, then update the
visible objects and their handlers together. Keep the previous display usable
while new assets load.

Read this for reconciliation delivery, native resource loading, animation
requirements, and frame acknowledgements. Related pages: [rules and
checkpoints](execution.md), [animation](motion.md), [identity](identity.md), and
[validation](validation.md).

## Prepare before changing what the player sees

A render reads one immutable state snapshot and one version of display stores.
It describes the new tree, matches object identities, computes layout, and
constructs animation/effect requests. Required assets, host properties, and
inactive native objects must be ready before the result becomes visible.

For example, loading artwork for a newly revealed card can span several frames:

```text
visible: face-down card, existing handlers, running hover animation
prepare: load artwork; create inactive face; validate material parameters
commit:  replace face, update handlers, start reveal animation together
```

A **commit** is that complete visible update. A **commit generation** is its
monotonically increasing identifier. Events use the identifier to reject work
from an older visible state.

Each preparation records the run, checkpoint, current commit generation, and
requested display revision. Recheck them immediately before committing. If the
player changes a setting during preparation, discard the obsolete inactive
resources and prepare the new version of that same pending checkpoint. Do not
skip a required checkpoint just because display state changed.

Abandonment also invalidates preparation. Discard reserved playback handles and
inactive resources without starting sounds, particles, or input handlers.

## Native preparation and acknowledgement

Add prepare, ready, commit, and discard operations to the Battlement protocol.
The following is an example exchange; game code does not construct it:

```text
Rust -> Unity: prepare update 18 for checkpoint 4
Unity -> Rust: update 18 ready; all required assets loaded
Rust -> Unity: commit update 18 as generation 9
Unity -> Rust: generation 9 committed
Unity -> Rust: checkpoint 4 had a rendering opportunity in generation 9
```

Preparation IDs are scoped to the session. Requests and responses carry the
session and preparation ID, with the run/checkpoint and revision data needed to
reject obsolete work. A repeated request with the same identity and body has no
additional effect. A different body using an existing identity is invalid.
Missing required dependencies produce an identified preparation failure, not a
successful response with missing resources.

At commit, Unity performs these operations without yielding to another frame or
input callback:

1. Extract and reparent surviving identified objects before removing ancestors.
2. Apply validated properties, attachments, and visibility changes.
3. Install new handler/ref mappings and animation/effect registrations.
4. Publish the new generation and allow input against it.

Rust retains pending and acknowledged trees separately. It adopts the new tree
only after the matching commit acknowledgement. If an input event is queued
before Rust processes that acknowledgement, hold it until the new handlers are
installed, then validate its generation. Never dispatch it through old handlers
or require a reentrant call into Rust.

Large object populations must be constructed inactive during preparation. The
final visible swap must fit in one frame. Predictable failures are caught before
commit. An unexpected Unity failure during the swap stops the session and shows
a failure surface; do not pretend arbitrary native changes can be rolled back.

## Tell the display when a checkpoint may advance

The display collects the animation completions or labels each checkpoint must
wait for. Rules publish snapshots/events; they do not sequence the display. This
collection is its **advancement gate**: all required entries must be satisfied
before the next checkpoint can replace it. Ordinary movement is required by
default; cosmetic animation does not block progress.

For example, one checkpoint moves two cards while a glow continues indefinitely:

```text
card A: wait for its draw sequence's "ready" label
card B: wait until its ordinary movement arrives
glow:   cosmetic; do not wait
advance only when A is ready AND B has arrived
```

An animation registration can choose an earlier label for its own movement. That
replaces its default arrival requirement; it does not add a second requirement
that still waits for arrival. Other cards' requirements remain. After the
earlier label, remaining movement and cosmetic effects may continue.

A checkpoint has at most one semantic event, but any number of display
registrations may interpret it. Choice and final checkpoints have no event;
default movement and frame requirements still apply.

A registration is a callback that builds animation from one typed
`StateAnimation`. The component reads the current checkpoint with
`use_checkpoint::<StateAnimation>()`, obtains scoped animation controls with
`use_animate()`, and creates its compatible card and anchor refs during
rendering. It declares the refs on its world children.

For example, a `CardView` component filters the game's state-animation enum to
draws of that card. The scoped name identifies this callback; the pattern
selects which animations it handles:

```rust
let checkpoint = use_checkpoint::<StateAnimation>();
let animate = use_animate();
checkpoint.on_animation(card_ref.scoped_name("draw"), move |animation, cx| {
    if let StateAnimation::CardDrawn(id) = animation {
        if *id == card_id {
            let playback = animate.start(draw_sequence(card_ref, reveal_ref));
            cx.require(playback.reached("ready"));
        }
    }
});
```

Here `card_id` is the rules card ID; `card_ref` is a typed ref on that
component's native card group, and `reveal_ref` identifies a Rust-created
anchor. The runtime resolves these against the prepared tree before running the
callback. Missing or incompatible required refs fail preparation before any
effect starts.

The registration name is stable within that animation and must be unique. The
`scoped_name` helper combines this object's stable presentation identity with
"draw", so multiple card components can declare the same local name safely. A
parent can instead register one callback that handles several moved cards.
Reject duplicate registrations with the same complete identity.

The callback reserves playback rather than starting Unity work immediately.
Rerenders, preparation retries, and duplicate delivery reuse the
checkpoint/animation/registration identity. Commit installs the playback and
requirement together and starts it once. Event-driven `use_animate` uses the
same preparation machinery without requiring a gameplay checkpoint.

## A rendered frame is also required

The next checkpoint needs both satisfied animation requirements and a rendering
opportunity after satisfaction. Command receipt, a poll, or elapsed time is not
proof that the player could see the state.

Unity sends a post-render/end-of-frame acknowledgement containing run ID,
checkpoint ID, commit generation, and frame sequence. Ignore acknowledgements
for abandoned runs, old generations, or frames before the requirements were
satisfied. The fake's `advance_frame()` produces the equivalent boundary.

```text
frame 10: card is still moving       -> cannot advance
between frames: "ready" is reached   -> still cannot advance
frame 11: checkpoint can render      -> next checkpoint may be committed
```

No-movement and equal-pose checkpoints satisfy animation requirements
immediately, but still need this frame. Initially admit at most one checkpoint
commit per rendered frame. Suspended or minimized hosts must not invent frames
to unblock rules. On resume, use real playback and frame acknowledgements.

## Replace animation without losing required work

A reflow updates a movement's destination without changing what completion
means. An explicitly authored replacement sequence must take over every
unfinished requirement that belonged to the animation it replaces.

For example, a draw waiting for `ready` can switch to a shorter animation:

```text
before replacement: wait for playback 20, generation 1, label "ready"
after replacement:  wait for playback 21, generation 1, label "ready"
late completion from playback 20: ignore
```

Process accepted events and replacement commits serially on the main thread. If
an event satisfied the old requirement before replacement committed, retain that
satisfaction permanently. Otherwise update the requirement to the new playback,
generation, and label/completion atomically with replacement. Even if the old
host animation finished earlier, an event accepted after replacement cannot
satisfy the new requirement.

Start replacement from the actual displayed pose. Cosmetic tracks may stop
freely. Unfinished required work must finish, transfer responsibility, or fail
and abandon the action. Never silently delete a requirement to make it pass.
Inspection seeking and replay events cannot satisfy live requirements.

A failed required track abandons the worker and that action's presentation,
retains the last accepted state, and shows exit/restart. A new game must reject
all late events from the failed run.

## Manual QA

Delay artwork loading, change a setting, and verify only the latest prepared
version becomes visible. During the swap, click the affected object and check
that its visible state and handler agree. Step two no-animation checkpoints one
frame at a time. Replace a required draw before and after `ready`, inject an old
completion event, and verify advancement follows the rules above.
