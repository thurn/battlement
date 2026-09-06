# 01. Establish behavioral baselines and classify existing tests

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Migration contract](../migration.md)
- [Validation contract](../validation.md)

**Prerequisite:** None. Begin from the current certified release.

**Source roles:** Existing game tests; existing sample declarations; repository
validation. Resolve these through source-map.md; its links track the current
owner after crate moves. Inspect the concrete caller and host/fake counterpart
before editing.

## Result

The migration has a repeatable external behavior contract before changing any
engine or sample behavior.

## Implementation

1. Enumerate the existing native Ditto selections and Rust gameplay tests for
   all six samples. Record their public inputs, observable assertions, and
   fixture seeds in external review evidence; put durable migration requirements
   in migration.md.

2. Classify command variants/counts, prefab-kind checks, exact host counts, and
   sleep/poll loops. For each, identify the actual behavior it protects, if any.
   Rework straightforward cases to existing public world/text/input observations
   and delete tests that protect only internal representation.

3. Capture current native motion/effect behavior where the fake cannot observe
   time. Leave those coupled timing assertions temporarily in place with task 08
   named as their rework owner; do not invent interpolation evidence.

4. Retain existing native baselines. Run focused old suites before and after
   test-only edits so the replacements demonstrably pass against the original
   implementation.

## Acceptance

- Both gameplay suites and the currently implemented Reactant/chess-ui scenarios
  still pass with production code unchanged.

- Every deleted or reworked assertion has an explicit retained behavior or a
  reason that it asserted no external guarantee.

- Review evidence identifies the exact initial/changed/reset selections needed
  for later migrations, including chess capture paths, spawn beats,
  save/restore, and input modes.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Temporal fake improvements and remaining timed-assertion replacements belong to
task 08. No game or framework refactor belongs here.

## Manual QA

Replay one tic-tac-toe round and chess start/move/capture/reset with the current
native player. Confirm the baseline evidence describes what the player actually
sees.
