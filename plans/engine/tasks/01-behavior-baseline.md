# 01. Establish behavioral baselines and classify existing tests

The migration has a repeatable external behavior contract before changing any
engine or sample behavior.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Sample migration](../migration.md)
- [Validation](../validation.md)

**Prerequisite:** None. Begin from the current certified release.

**Starting code:** Existing game tests; existing sample declarations; repository
validation.

## Example

Preserve the behavior behind a coupled assertion. For a chess capture, record
what the player sees before deciding which test to keep:

```text
input: capture a piece in a fixed board position
observe: moving piece follows its path; captured visual remains until impact
keep: that path and disappearance timing
replace: assertions about exact command counts
```

## Implementation

1. Enumerate the existing native Ditto selections and Rust gameplay tests for
   all six samples. Record their public inputs, observable assertions, and
   fixture seeds in external review evidence; put durable migration requirements
   in migration.md.

2. Classify command variants/counts, prefab-kind checks, exact host counts, and
   sleep/poll loops. For each, identify the actual behavior it protects, if any.
   Record which assertions need replacement when their owning behavior changes.
   Rework only straightforward assertions that obstruct an imminent change; do
   not rewrite unaffected suites as a prerequisite to engine implementation.

3. Capture current native motion/effect behavior where the fake cannot observe
   time. Leave those coupled timing assertions temporarily in place with task 08
   named as their earliest possible rework owner; migrate the remaining assertions
   with their sample before cutover. Do not invent interpolation evidence.

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

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Temporal fake improvements and remaining timed-assertion replacements belong to
task 08. No game or framework refactor belongs here.

## Manual QA

Replay one tic-tac-toe round and chess start/move/capture/reset with the current
native player. Confirm the baseline evidence describes what the player actually
sees.
