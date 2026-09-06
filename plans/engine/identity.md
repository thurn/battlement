# Logical identity, ancestry, and retained visuals

Read this for tree matching, portals, refs, effect cleanup, host replacement, or
removal. See [architecture](architecture.md) and
[presentation](presentation.md).

## Matching and moving

Any component or host can declare id(Uuid). The UUID identifies one live logical
presentation across all active roots and both host domains. Ordinary key()
remains sibling-scoped. No string/numeric overload silently substitutes for
UUID.

A **mounted incarnation** identifies one continuous mounted lifetime of that
presentation. A host reference carries both presentation and incarnation
identity, plus its host kind; retained visuals cannot masquerade as new mounts.

Build the proposed UUID index before any visible commit. Reject duplicates
across roots, UI containers, portals, and world objects atomically. Never use a
first-match-wins policy.

Match the UUID-bearing position before evaluating descendants, so an ancestry
move can reuse its hook storage. The algorithm must handle a proposed parent
containing a child formerly under an ancestor that disappears in this commit.

~~~text
old: root -> hand -> Card(id=A, hook counter=3)
new: root -> table -> Card(id=A, hook counter=3)
result: same mounted Card; context and event path now come from table
~~~

Extract moved nodes before disposing unmatched former ancestors. Preserve
compatible descendants, hooks, refs, and native handles. Component type changes
reset that component's hook storage; do not reinterpret slots of another type.

Provider lookup and capture/bubble paths come from the new logical ancestry.
Reevaluate moved context consumers. Clean up effects whose dependencies changed
before installing replacements. Avoid double subscription during the move.

## Host compatibility

Reuse a native host only if kind and property contract are compatible. A stable
world group can keep its transform while its visual children change face/type.
Prepare incompatible visual replacements inactive before committing.

UI-to-world or world-to-UI continuity requires an explicit projection policy
mapping source rendered geometry to the destination plane/camera. Do not infer
pixel-to-world scale. Preserve logical identity while connecting compatible
old/new visuals; lack of a required projection is a preparation error.

## Removal and reappearance

Absence from a committed tree unmounts the logical object immediately. Detach
input, stores, context subscriptions, and callbacks; discard hooks. An object
absent only from abandoned preparation has not unmounted.

**Exit visuals** are frozen host representations with prepared animation,
anchors, and assets retained after logical removal. They do not continue
rendering components or accepting gameplay input.

If the same UUID reappears, allocate a new incarnation and fresh hook state,
even while the old visual exits. Live UUID uniqueness does not count retained
old incarnations. Events and effect targets must validate incarnation as well as
UUID. Old callbacks cannot update or destroy the new object.

Reference-count retained visual dependencies by actual ownership. Release hosts,
material instances, anchors, and assets when their last exit/effect use ends. A
retained projectile can follow its original anchor without retaining the logical
Card component.

## Selectors and display stores

Selectors compare the selected value and suppress component evaluation when it
is equal. Explicit props must produce identical behavior without subscriptions.
A moved component preserves its subscription identity but reads from the new
provider ancestry.

Store notifications are queued. Rendering captures a stable version; writes
during that render become later revisions and cannot tear the proposal. Aborted
proposals must not publish new subscriptions or cleanup committed ones.

## Manual QA

Move a stateful Card between layouts and UI portal containers while changing
providers. Its counter/ref survive, but context and events change ancestry.
Remove it during an exit effect and recreate the same UUID; verify fresh state,
two isolated visual incarnations, and no input on the exiting representation.
