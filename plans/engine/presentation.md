# Preparation, commit, and checkpoint advancement

Read this when changing reconciliation delivery, assets, frame acknowledgements,
required animation work, or failure recovery. See [execution](execution.md),
[motion](motion.md), and [identity](identity.md).

## Generations and preparation

A **commit generation** identifies one complete visible host tree and handler
table. A prepared proposal carries run ID, checkpoint ID, base commit
generation, and desired render revision. Treat these as a structured identity,
not a timestamp.

Rendering reads one presented/proposed snapshot and stable store version. It
resolves identity, layout, assets, effect registrations, and inactive host
resources before changing visible objects. The previous committed generation
remains visible and interactive while preparation spans frames.

Immediately before committing, revalidate the proposal identity on the serial
main-thread path. A newer store revision, replacement session, or different base
commit invalidates obsolete preparation. Release its inactive resources and
reserved playbacks, then prepare the newest revision for the same pending
checkpoint. Do not skip a required checkpoint because display state changed.

## Host transaction

Add typed prepare/ready/commit/discard operations and correlated results to the
Battlement protocol. A preparation ID names the host-side inactive resource set.
Each operation also carries the session and proposed commit identity. Preparing
must not start animation, emit effects, or expose new input handlers.

At commit:
1. Recheck the run and desired revision.
2. Extract/reparent surviving identified hosts before destroying ancestors.
3. Apply validated properties, attachment changes, and visibility changes.
4. Install handler/ref mappings, playbacks, occurrences, and checkpoint gates.
5. Publish the new generation and only then admit input against it.

Unity executes the visible swap without yielding to another frame or input
callback. Resource loading and large construction belong to preparation. This is
not a promise of database rollback after arbitrary Unity failure: validate
predictable failures before commit; an unexpected apply failure stops the
session and exposes its failure surface.

Rust's committed state advances on the correlated successful host commit.
Maintain pending versus acknowledged trees explicitly. Until acknowledgement, do
not dispatch new-generation events against the old handler table or accept a
later commit. The fake must exercise the same prepare/commit/ack ordering.

## Required advancement gate

A checkpoint has one advancement gate, composed from its required registrations.
By default it waits for all required movement introduced by that checkpoint. A
registration can bind its movement contribution to an earlier sequence label;
other required contributions still apply. Cosmetic work never blocks it.

For example:

~~~text
checkpoint 12:
  card A -> sequence draw-A label ready
  card B -> default layout movement complete
  aura   -> cosmetic, excluded
gate = draw-A.ready AND card-B.arrived
~~~

An empty/equal-pose gate is immediately satisfied, but the rendering opportunity
is still required. A checkpoint with no movement is not skipped in the same
frame. Admit at most one checkpoint commit per rendered frame initially.

## Rendering opportunity

Unity reports a frame generation only after the committed checkpoint has had an
opportunity to render with the gate already satisfied. A pre-gate frame does not
count, nor does a frame from an earlier commit or abandoned run.

Use a post-render/end-of-frame acknowledgement containing run, checkpoint,
commit generation, and frame sequence. Do not treat command receipt, poll(), or
elapsed wall-clock time as proof of presentation. The fake's advance_frame()
performs the equivalent boundary and produces the same acknowledgement.

Suspended/minimized hosts do not invent frames to unblock gameplay. Local pause
policy may freeze playback; on resume the normal acknowledgement resumes.

## Replacing required work

A validated event permanently satisfies its contribution. Reflow keeps the
existing playback/dependency; an authored replacement atomically rebinds an
unsatisfied contribution to a successor playback ID, generation, and target
label/completion.

Accept host events and replacement commits on one serial path. An event accepted
before replacement may satisfy the old contribution; after replacement, the old
generation is stale even if it completed earlier on the host. A successor starts
from the displayed pose. Never convert interrupted required work to success.

Required-track failure abandons the action. Cosmetic tracks may stop freely. A
preparation error cannot silently omit required assets, targets, or gates.

## Checkpoint registrations

Provide a presentation-effect registration keyed by checkpoint, change index,
and stable registration slot. It runs during preparation with typed changes,
refs, and scoped animation controls. start() reserves a handle; require() binds
its gate contribution. Playback starts only at successful commit.

Retrying preparation or rerendering reuses the occurrence identity. Do not start
playback from ordinary render side effects. Event-driven animations use the same
preparation machinery without needing a rules checkpoint.

## Manual QA

Hold asset preparation across frames while opening a menu. Replace that proposal
and verify it never flashes onscreen or plays sound. Present two zero-duration
checkpoints and confirm each receives its own rendered frame. Replace required
motion just before and after a completion event and inspect the accepted gate.
