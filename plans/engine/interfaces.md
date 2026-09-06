# Minimum public interfaces and implementation starting shapes

Read this with [execution](execution.md), [architecture](architecture.md), and
[presentation](presentation.md) when implementing tasks 02, 09-12, or 25. These
sketches make the data flow concrete for implementors. They are proposed Rust
shapes, not APIs that already exist. Task 02 must replace its sketch with a
compiling example once implemented.

## Game and choice interfaces

The game supplies mutable state, immutable snapshots, ordered changes, typed
prompt/answer envelopes, and a borrowed decision view for simulation policies.
Keep the borrowed decision view separate from the owned display prompt: building
an interactive prompt must never be necessary for a simulation choice.

A suitable starting trait shape is:

~~~rust
trait Game: Sized + 'static {
    type State: Send + 'static;
    type Snapshot: Send + 'static;
    type Change: Send + 'static;
    type Changes: IntoIterator<Item = Self::Change> + Send + 'static;
    type Action: Send + 'static;
    type Prompt: Send + 'static;
    type Answer: Send + 'static;
    type Decision<'a> where Self: 'a;

    fn validate_action(state: &Self::State, action: &Self::Action)
        -> Result<(), ActionRejection>;
    fn fork_state(state: &Self::State) -> Self::State;
    fn final_checkpoint(state: &Self::State)
        -> (Self::Snapshot, Self::Changes);
}
~~~

Use a separate generic execute method/function that takes State, Action, and
Executor<Game, Mode>. A Game registration can bind that function to App without
a trait-object call inside each rules primitive. It is acceptable to erase the
outer action dispatch once per action; simulation's inner choose/present path
must remain statically dispatched.

Each typed choice builds an owned prompt only for interactive mode. A
ChoiceRejection is public invalid-answer feedback, such as wrong variant or
illegal card; it is not a worker failure.

~~~rust
trait ChoiceSpec<G: Game> {
    type Answer;
    fn prompt(&self, state: &G::State) -> G::Prompt;
    fn decision<'a>(&'a self, state: &'a G::State) -> G::Decision<'a>;
    fn validate(&self, state: &G::State, answer: G::Answer)
        -> Result<Self::Answer, ChoiceRejection>;
}
trait ChoicePolicy<G: Game> {
    fn choose(&mut self, decision: G::Decision<'_>) -> G::Answer;
}
~~~

For Hearts, Decision can be a borrowed enum with Pass and Play variants. Pass
exposes the acting player's observation and eligible hand; Play also exposes
public trick/history and a borrowed legal-card view. It must not carry the true
opponents' hands. Game-owned candidate enumeration may allocate; that is
measured separately from executor overhead.

A concrete Simulation<Policy> calls policy.choose(spec.decision(state)), then
spec.validate to recover the typed answer. Invalid policy output is a developer
error and may panic. Interactive invalid user input instead leaves the prompt
unanswered and emits ChoiceRejection. Neither path needs a vtable or mandatory
owned prompt allocation.

## Endpoint records and serial ownership

Use opaque monotonically allocated IDs scoped to their documented lifetime. Do
not recycle IDs within a run or session. Presentation UUIDs remain distinct from
numeric execution/commit identifiers.

~~~text
Worker output:
  Checkpoint { run, checkpoint, snapshot, ordered changes }
  Prompt     { run, checkpoint, request, snapshot, owned prompt }
  Completed  { run, checkpoint, final state, snapshot, ordered changes }
  Failed     { run, public failure }
  Stopped    { run }

Main-thread input:
  Answer { run, request, answer envelope }
  Abandon { run }
~~~

Checkpoint/Prompt/Completed occupy the bounded publication slot. Failed/Stopped
are terminal lifecycle state notifications, not extra checkpoint history: store
the endpoint's terminal state separately so a full publication slot cannot
prevent shutdown/failure notification.

At normal return, reserve the publication slot before invoking final_checkpoint,
check cancellation before/after building it, and publish Completed. This applies
the same laziness/backpressure contract to final output.

The worker validates interactive answer envelopes. The main thread rejects
obviously stale run/request submissions before enqueueing, and the worker
revalidates identity and cancellation after wakeup. A validation failure cannot
consume the prompt's completion.

The application serializes host events, action admission, commit results, and
accepted-state changes on Unity's main-thread engine callbacks. Worker threads
only touch their private State and synchronized endpoint.

## Prepared host and frame interfaces

A suitable protocol request/response family is:

~~~text
Prepare { session, preparation, proposal identity, inactive host plan }
Ready   { session, preparation, resolved host dependencies }
Commit  { session, preparation, commit generation }
Committed { session, preparation, commit generation }
Discard { session, preparation }
Rendered { session, run, checkpoint, commit generation, frame sequence }
~~~

The proposal identity contains run/checkpoint, base commit generation, and
desired render revision. Missing required resources yield a correlated
preparation failure, not Ready with missing members. A repeated Prepare or
Commit with the same identity is idempotent; a mismatched body is rejected.

Input/event delivery cannot overtake Committed on the main-thread response
stream. If the transport queues an event before Rust installs acknowledged
handlers, hold it until that acknowledgement is processed, then validate its
generation. Never dispatch through the old table or require reentrant Rust.

Large assets/objects are prepared inactive; the commit plan contains only the
validated dependency-ordered visible changes, handler/ref identity swap, and
playback/gate registration. Do not encode checkpoint semantics as a chain of
unrelated legacy TimeWait batches.

## Registration and gate interface

A checkpoint registration uses one stable registration slot and receives typed
semantic changes, compatible refs, scoped controls, and a gate builder. It
returns prepared requests; it has no immediate host side effects.

~~~rust
checkpoint.on_change(ChangeKind::Draw, slot("draw"), |change, cx| {
    let playback = cx.animate().start(draw_sequence(change.card));
    cx.require(playback.reached("ready"));
});
~~~

Each contribution identifies its owning movement/registration so an early-label
override replaces that contribution rather than duplicating its default arrival
requirement. The checkpoint gate is the conjunction of all remaining required
contributions. Cosmetic work does not register a contribution.

If a label is already satisfied before an authored replacement is processed,
keep that satisfaction permanently. Otherwise atomically rebind the contribution
to the successor's playback/generation/label. Inspection and replay handles
cannot be passed as live requirements.

## Manual QA

Have a reader implement the small neutral two-choice rules fixture from task 02
using these shapes. Verify that they can explain how the policy chooses without
an owned prompt, how final output reserves capacity, and how host input waits
for the correct committed handler table.
