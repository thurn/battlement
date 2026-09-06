# Serial task execution and Luna handoff

Read this before selecting or implementing a task. The [entry point](README.md)
is the authoritative order; the selected task and its linked shared contracts
are the complete assignment.

## Start from one selected task

Tasks execute serially. Start task N only after task N-1 and its required
follow-ups are validated and integrated through the repository's wt/Tollgate
workflow. Do not start speculative implementation in another task's worktree.

A coordinator can give a Luna implementor this assignment:

~~~text
Implement task NN from plans/engine/README.md.
Read that task, workflow.md, and its listed shared contracts.
Follow repository AGENTS.md and wt. Resolve source roles via source-map.md.
Implement all acceptance scenarios; named deferrals belong to later tasks.
Improve ergonomics within the fixed contracts and update affected plan pages.
Return the exact candidate and evidence; do not authorize promotion.
~~~

This plan does not itself authorize creating Codex tasks, promotion, remote
branches, or bypassing repository validation. Use subagents within the
coordinator's authorized work; user-owned Codex tasks are created only on
request.

## Concrete execution steps

1. Read root/crate/sample guidance and the selected task. Build a compact
   requirement-to-evidence checklist outside tracked source.
2. Create a fresh task-owned worktree through the wt skill. Revalidate the
   source pointers against that worktree; do not edit the primary checkout.
3. Run the task's smallest existing behavioral check before modifying it.
   Identify which public behavior each new assertion protects.
4. Implement the ordered work in the task, including protocol, Unity, and fake
   sides when applicable. Add its smallest visible/public-driver specimen before
   expanding feature coverage.
5. Run the task's named acceptance scenarios and affected regression checks.
   Follow validation.md for native/web/release requirements.
6. Inspect API ergonomics in the actual sample callsite. Move reusable behavior
   into the engine instead of adding per-sample imperative command plumbing.
7. Update the owning shared contract and downstream task pointers if an allowed
   API improvement changes them. Remove incorrect guidance instead of appending
   contradictory history.
8. Complete applicable review, stage changes, pass ./scripts/ci.py, commit once
   with Conventional Commits, and immediately submit tg candidate HEAD without
   promotion authority.
9. Hand off the candidate and concrete review evidence. Promotion still needs
   the explicit mandate required by AGENTS.md and wt.

The repository's workflow instructions remain authoritative if tooling changes.
Do not copy stale build invocations out of another worktree's evidence.

## Task scope and deferrals

Task pages name source roles, a result, ordered implementation steps,
acceptance, and deferred work. Read only the required contracts; do not load all
48 pages.

A deferral is permission to omit only the named later capability, not to leave
the current scenario broken. Keep intermediate tasks runnable. Temporary
adapters need one named removal task and cannot become a parallel permanent
runtime. Never claim later coverage from a placeholder or ignored test.

If a task exceeds a comfortable single-agent change, keep its stated acceptance
boundary and use focused internal commits only if repository policy permits.
Otherwise complete one reviewed candidate; do not silently split promotion
boundaries or waive checks. Public API work can exceed the old chess plan's
approximate line target when the design needs it.

## Evidence and review

Retain exact base/source identities, test commands/results, fixture seeds,
public observations, and native artifacts outside maintained guidance. Each
acceptance bullet needs evidence or an explicit unresolved blocker.

A reviewer should be able to repeat the task from the specified starting state.
Test rewrites must explain the preserved behavior. API changes must include a
before/after authoring example in review evidence, not merely a renamed type.

## Manual QA

A cold reader should select one task, open only its linked contracts, locate the
current code, and explain what to implement and how to prove it. If the reader
must invent a concurrency rule or acceptance condition, repair the plan before
assigning that task.
