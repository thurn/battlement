# Implement one task at a time

The numbered leaf task is the assignment. Some numbers name a task group whose
lettered assignments run serially; the group page is an index, not one large
implementation task. Read the leaf assignment and its linked contracts, then
complete and integrate it before starting the next leaf in the
[implementation order](README.md#implementation-order).

Related pages: [starting code](source-map.md), [validation](validation.md),
[migration](migration.md), and the repository's [instructions](../../AGENTS.md).

## Begin with the existing behavior

Each task identifies the source areas it changes. Follow the source map to the
current files, then read the relevant caller and native/fake implementation.
Revalidate these pointers in the task's own worktree before making changes.

Run the smallest existing check that covers the behavior first. For a migration,
identify what the player currently sees before changing the implementation. For
example, preserve a chess capture's timing rather than the command sequence that
happens to produce it.

Use the repository's worktree and validation workflow. This plan does not change
promotion permissions or authorize remote branches. Keep implementation, review
evidence, and resource cleanup within the task you are executing.

## Complete the task's working behavior

Implementation requirements describe necessary changes. Acceptance examples
specify observable results. Add the task's smallest working public scenario
before extending it to the full set of cases.

- Update Rust protocol, Unity execution, and the fake together when a capability
  crosses those boundaries.
- Keep existing callers compiling when changing an API. Later sample tasks
  replace behavior and ownership, not broken imports.
- Use the specified later task for explicitly excluded work. Do not treat that
  as permission to leave the current task's behavior broken.
- Give temporary adapters a named removal task. They cannot become a permanent
  second component or animation runtime.
- Do not claim an unavailable test scene, ignored test, or compiling interface
  as completed behavior.

Task examples describe the interface to implement unless explicitly identified
as existing code. Replace illustrative spellings with compiling examples as
those APIs become available, and update affected topic/task pages together.

## Improve the API at its call sites

Use initiative to simplify authoring and add reusable capabilities when the
assigned behavior exposes a gap. Inspect actual game components rather than
judging ergonomics only from engine types.

For example, a card transfer should be expressed by moving the declaration:

```rust
Hand::new().child(Card::new().id(card_id))
// Next render places the same identified component elsewhere.
Table::new().child(Card::new().id(card_id))
```

If every sample must manually create native commands or bookkeeping for that
move, improve the engine. Defaults should make simple games simple. Preserve
specified state ownership, ordering, cancellation, animation, and input behavior
when changing API names or implementation details.

Removing a requirement, changing game behavior, or expanding the project beyond
its defined scope needs a user decision. An ergonomic improvement that preserves
those requirements does not require section-by-section plan approval.

## Validate and leave reproducible evidence

Follow [validation](validation.md) for public scenarios, native checks, and the
required aggregate CI run. Retain commands, seed/deal inputs, results, and
rendered evidence so a reviewer can repeat the important cases.

Every acceptance behavior needs evidence or a specific unresolved blocker. One
existing scenario may cover several bullets and tasks; a bullet does not require
a new test. Later integration tasks reuse that evidence and fill demonstrated
gaps. A test rewrite explains the behavior it preserves. An API change includes
an actual authoring example. Complete review and validation before submitting a
candidate through the repository workflow.

Keep current code guidance accurate when files move. Update the source map for
later tasks and replace incorrect instructions rather than appending conflicting
history. Keep release evidence and temporary planning records outside maintained
source guidance.

## Manual QA

Select a task and follow its links to the current code. Exercise its specified
initial state, interaction, and expected result. Confirm that another reader can
repeat that flow without needing a conversation or an unwritten sequencing rule.
